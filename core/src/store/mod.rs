//! SQLite-backed session persistence (WAL, append-only messages, compression events).
//!
//! **Cross-process**: WAL on `sessions.db` (M3) — TUI, gateway, and CLI can share the file.
//!
//! **In-process** (M12): one **write** connection (turn context load, append, compression,
//! knot writes) plus a **read pool** of read-only connections (`[storage].read_pool_size`,
//! default 4) for list/history-style reads — `list_sessions`, `load_all_messages`, knot queries.
//! All adapters share one `Arc<SessionStore>` via `HiServices`; see
//! `docs/exec-plans/active/m12-sqlite-concurrent-access.md`.

mod blocking;
mod compressions;
mod knots;
mod messages;
mod read_pool;
mod schema;
mod sessions;
mod time;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::Connection;

pub use blocking::run as run_blocking;
pub use compressions::{NewSessionCompression, SessionCompression};
pub use knots::{KnotProvenance, MergeKnotResult};
pub use messages::StoredMessage;
pub use read_pool::ReadPool;
pub use sessions::SessionSummary;
pub use time::now_unix;

use crate::config::DEFAULT_READ_POOL_SIZE;
use crate::error::{Error, Result};
use crate::llm::ChatMessage;
use crate::memory::{Knot, NewKnot, OwnerId};
use crate::{SessionHandle, SessionId, UserId};

use compressions::{get_by_id, insert as insert_compression, list_for_session};
use knots::{
    forget as forget_knot, get as get_knot, insert as insert_knot, list_active,
    list_all as list_all_knots, merge_knot, reinforce as reinforce_knot,
};
use messages::{
    append, load_all, load_context, load_range, mark_ids_out_of_context, mark_out_of_context,
    reset_context, update_message_content, update_system_content,
};
use schema::ensure_compatible;
use sessions::{get_or_create, list_all, purge, touch};

/// Author: gz
pub struct SessionStore {
    write: Mutex<Connection>,
    read_pool: ReadPool,
    path: String,
}

impl SessionStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_pool(path, DEFAULT_READ_POOL_SIZE)
    }

    pub fn open_with_pool(path: impl AsRef<Path>, read_pool_size: u32) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Message(format!("create data dir {}: {e}", parent.display()))
            })?;
        }
        let path_str = path.display().to_string();

        let write = Connection::open(path)
            .map_err(|e| Error::Message(format!("open database {}: {e}", path_str)))?;
        write
            .busy_timeout(Duration::from_secs(5))
            .map_err(|e| Error::Message(format!("write busy_timeout: {e}")))?;

        ensure_compatible(&write, &path_str)?;

        let read_pool = ReadPool::open(path, &path_str, read_pool_size)?;

        Ok(Self {
            write: Mutex::new(write),
            read_pool,
            path: path_str,
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    fn write_conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.write
            .lock()
            .map_err(|e| Error::Message(format!("database write lock poisoned: {e}")))
    }

    pub fn ensure_user(&self, user_id: &UserId) -> Result<()> {
        let conn = self.write_conn()?;
        sessions::ensure_user(&conn, &user_id.0)
    }

    /// Load or create session (`tui:main` / `chat:main` / `wecom:{userid}` — isolated transcripts).
    pub fn get_or_create_session(
        &self,
        session_id: &SessionId,
        working_directory: &str,
    ) -> Result<SessionHandle> {
        let user_id = UserId(session_id.0.clone());
        let conn = self.write_conn()?;
        get_or_create(&conn, session_id, working_directory)?;
        Ok(SessionHandle {
            user_id,
            session_id: session_id.clone(),
        })
    }

    /// Full transcript (all rows, including compressed-out-of-context).
    pub fn load_all_messages(&self, session_id: &SessionId) -> Result<Vec<StoredMessage>> {
        self.read_pool
            .with_conn(|conn| load_all(conn, session_id))
    }

    /// Messages in an id range (for compression-time knot extraction).
    pub fn load_messages_range(
        &self,
        session_id: &SessionId,
        id_from: i64,
        id_to: i64,
    ) -> Result<Vec<StoredMessage>> {
        let conn = self.write_conn()?;
        load_range(&conn, session_id, id_from, id_to)
    }

    /// Messages currently visible to the agent (`in_context = 1`).
    pub fn load_context_messages(&self, session_id: &SessionId) -> Result<Vec<StoredMessage>> {
        let conn = self.write_conn()?;
        load_context(&conn, session_id)
    }

    /// Append-only insert. Returns new row ids.
    pub fn append_messages(
        &self,
        session_id: &SessionId,
        messages: &[ChatMessage],
    ) -> Result<Vec<i64>> {
        if messages.is_empty() {
            return Ok(vec![]);
        }
        let now = now_unix();
        let conn = self.write_conn()?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| Error::Message(format!("begin transaction: {e}")))?;
        let ids = append(&tx, session_id, messages, now)?;
        touch(&tx, session_id, now)?;
        tx.commit()
            .map_err(|e| Error::Message(format!("commit messages: {e}")))?;
        Ok(ids)
    }

    pub fn update_system_message(&self, session_id: &SessionId, content: &str) -> Result<()> {
        let conn = self.write_conn()?;
        update_system_content(&conn, session_id, content)?;
        Ok(())
    }

    pub fn update_message_content(&self, message_id: i64, content: &str) -> Result<()> {
        let conn = self.write_conn()?;
        update_message_content(&conn, message_id, content)?;
        Ok(())
    }

    /// Hide all non-system messages from the agent while keeping full transcript rows.
    pub fn reset_session_context(&self, session_id: &SessionId) -> Result<u64> {
        let conn = self.write_conn()?;
        reset_context(&conn, session_id)
    }

    pub fn mark_message_ids_out_of_context(
        &self,
        session_id: &SessionId,
        ids: &[i64],
    ) -> Result<()> {
        let conn = self.write_conn()?;
        mark_ids_out_of_context(&conn, session_id, ids)
    }

    /// Mark message range out of context and record compression event (single transaction).
    pub fn apply_compression(
        &self,
        session_id: &SessionId,
        record: NewSessionCompression,
    ) -> Result<i64> {
        let now = now_unix();
        let conn = self.write_conn()?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| Error::Message(format!("begin compression transaction: {e}")))?;
        mark_out_of_context(
            &tx,
            session_id,
            record.message_id_from,
            record.message_id_to,
        )?;
        let compression_id = insert_compression(&tx, session_id, &record, now)?;
        tx.commit()
            .map_err(|e| Error::Message(format!("commit compression: {e}")))?;
        Ok(compression_id)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        self.read_pool.with_conn(list_all)
    }

    pub fn list_compressions(&self, session_id: &SessionId) -> Result<Vec<SessionCompression>> {
        self.read_pool
            .with_conn(|conn| list_for_session(conn, session_id))
    }

    pub fn get_compression(&self, compression_id: i64) -> Result<SessionCompression> {
        self.read_pool
            .with_conn(|conn| get_by_id(conn, compression_id))
    }

    /// Explicit user-initiated purge — the only code path that deletes messages.
    pub fn purge_session(&self, session_id: &SessionId) -> Result<()> {
        let conn = self.write_conn()?;
        purge(&conn, session_id)
    }

    pub fn ensure_memory_owner(&self, owner_id: &OwnerId) -> Result<()> {
        let conn = self.write_conn()?;
        knots::ensure_owner(&conn, owner_id)
    }

    pub fn list_knots(&self, owner_id: &OwnerId) -> Result<Vec<Knot>> {
        self.read_pool
            .with_conn(|conn| list_active(conn, owner_id))
    }

    pub fn list_all_knots(&self, owner_id: &OwnerId) -> Result<Vec<Knot>> {
        self.read_pool
            .with_conn(|conn| list_all_knots(conn, owner_id))
    }

    pub fn get_knot(&self, knot_id: i64) -> Result<Knot> {
        self.read_pool.with_conn(|conn| get_knot(conn, knot_id))
    }

    pub fn add_knot(&self, knot: &NewKnot) -> Result<i64> {
        let conn = self.write_conn()?;
        insert_knot(&conn, knot)
    }

    pub fn forget_knot(&self, knot_id: i64) -> Result<()> {
        let conn = self.write_conn()?;
        forget_knot(&conn, knot_id)
    }

    pub fn reinforce_knot(&self, knot_id: i64, permanent: bool) -> Result<()> {
        let conn = self.write_conn()?;
        reinforce_knot(&conn, knot_id, permanent)
    }

    pub fn apply_knot_decay(&self, owner_id: &OwnerId, config: &crate::config::MemoryConfig) -> Result<()> {
        let conn = self.write_conn()?;
        knots::apply_decay(&conn, owner_id, config)
    }

    pub fn record_knot_injection(&self, knot_ids: &[i64]) -> Result<()> {
        let conn = self.write_conn()?;
        knots::record_injection(&conn, knot_ids)
    }

    pub fn merge_knot(
        &self,
        knot: &NewKnot,
        supersedes_hash: Option<&str>,
        provenance: &KnotProvenance,
    ) -> Result<MergeKnotResult> {
        let conn = self.write_conn()?;
        let (result, _id) = merge_knot(&conn, knot, supersedes_hash, provenance)?;
        Ok(result)
    }
}

#[cfg(test)]
#[path = "../../test/unit/store/mod.rs"]
mod tests;
