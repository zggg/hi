use rusqlite::{params, Connection};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::SessionId;

use super::time::now_unix;

/// Summary row for session listing.
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionSummary {
    pub session_id: SessionId,
    pub working_directory: String,
    pub updated_at: i64,
    pub message_total: u64,
    pub message_in_context: u64,
    pub last_compression_at: Option<i64>,
}

/// Author: gz
pub fn ensure_user(conn: &Connection, user_id: &str) -> Result<()> {
    let now = now_unix();
    conn.execute(
        "INSERT OR IGNORE INTO users (id, wecom_user_id, display_name, created_at) VALUES (?1, NULL, ?1, ?2)",
        params![user_id, now],
    )
    .map_err(|e| Error::Message(format!("ensure_user: {e}")))?;
    Ok(())
}

/// Author: gz
pub fn get_or_create(
    conn: &Connection,
    session_id: &SessionId,
    working_directory: &str,
) -> Result<()> {
    let user_id = session_id.0.clone();
    ensure_user(conn, &user_id)?;
    let now = now_unix();
    conn.execute(
        "INSERT OR IGNORE INTO sessions (id, user_id, working_directory, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![session_id.0, user_id, working_directory, now],
    )
    .map_err(|e| Error::Message(format!("create session: {e}")))?;
    conn.execute(
        "UPDATE sessions SET working_directory = ?1, updated_at = ?2 WHERE id = ?3",
        params![working_directory, now, session_id.0],
    )
    .map_err(|e| Error::Message(format!("update session: {e}")))?;
    Ok(())
}

/// Author: gz
pub fn list_all(conn: &Connection) -> Result<Vec<SessionSummary>> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.working_directory, s.updated_at, s.last_compression_at,
                    (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id) AS total,
                    (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id AND m.in_context = 1) AS in_ctx
             FROM sessions s
             ORDER BY s.updated_at DESC",
        )
        .map_err(|e| Error::Message(format!("prepare list sessions: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SessionSummary {
                session_id: SessionId(row.get(0)?),
                working_directory: row.get(1)?,
                updated_at: row.get(2)?,
                last_compression_at: row.get(3)?,
                message_total: row.get::<_, i64>(4)? as u64,
                message_in_context: row.get::<_, i64>(5)? as u64,
            })
        })
        .map_err(|e| Error::Message(format!("list sessions: {e}")))?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Message(format!("session row: {e}")))
}

/// Permanently delete a session and all messages/compressions. The only production DELETE path.
///
/// Author: gz
pub fn purge(conn: &Connection, session_id: &SessionId) -> Result<()> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| Error::Message(format!("begin purge transaction: {e}")))?;
    tx.execute(
        "DELETE FROM knot_provenance WHERE session_id = ?1",
        params![session_id.0],
    )
    .map_err(|e| Error::Message(format!("purge knot_provenance: {e}")))?;
    tx.execute(
        "DELETE FROM session_compressions WHERE session_id = ?1",
        params![session_id.0],
    )
    .map_err(|e| Error::Message(format!("purge compressions: {e}")))?;
    tx.execute(
        "DELETE FROM messages WHERE session_id = ?1",
        params![session_id.0],
    )
    .map_err(|e| Error::Message(format!("purge messages: {e}")))?;
    tx.execute("DELETE FROM sessions WHERE id = ?1", params![session_id.0])
        .map_err(|e| Error::Message(format!("purge session: {e}")))?;
    tx.commit()
        .map_err(|e| Error::Message(format!("commit purge: {e}")))?;
    Ok(())
}

/// Author: gz
pub fn touch(conn: &Connection, session_id: &SessionId, now: i64) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
        params![now, session_id.0],
    )
    .map_err(|e| Error::Message(format!("touch session: {e}")))?;
    Ok(())
}
