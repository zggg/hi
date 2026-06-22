//! SQLite DDL — single source of truth for `sessions.db`.
//!
//! No incremental migrations: incompatible older files must be removed and recreated.

use rusqlite::Connection;

use crate::error::{Error, Result};
use crate::messages::MessageId;

/// Bump when DDL shape changes; older databases must be rebuilt.
pub const SCHEMA_VERSION: i32 = 1;

/// Author: gz
pub const INIT_SQL: &str = r"
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    wecom_user_id TEXT UNIQUE,
    display_name TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS memory_owners (
    id TEXT PRIMARY KEY,
    display_name TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    working_directory TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_compression_at INTEGER
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_call_id TEXT,
    tool_calls_json TEXT,
    tool_name TEXT,
    reasoning_content TEXT,
    in_context INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_session_context
    ON messages(session_id, in_context, id);

CREATE TABLE IF NOT EXISTS channel_identities (
    channel TEXT NOT NULL,
    external_id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    PRIMARY KEY (channel, external_id)
);

CREATE TABLE IF NOT EXISTS session_compressions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    message_id_from INTEGER NOT NULL REFERENCES messages(id),
    message_id_to INTEGER NOT NULL REFERENCES messages(id),
    message_count INTEGER NOT NULL,
    token_estimate INTEGER,
    summary_text TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_session_compressions_session
    ON session_compressions(session_id, created_at DESC);

CREATE TABLE IF NOT EXISTS knots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id TEXT NOT NULL REFERENCES memory_owners(id),
    scope TEXT NOT NULL DEFAULT 'owner',
    session_id TEXT,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    task_status TEXT,
    confidence TEXT NOT NULL DEFAULT 'inferred',
    clarity REAL NOT NULL DEFAULT 0.7,
    permanent INTEGER NOT NULL DEFAULT 0,
    visibility TEXT NOT NULL DEFAULT 'inject',
    content_hash TEXT NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    superseded_by INTEGER REFERENCES knots(id)
);

CREATE INDEX IF NOT EXISTS idx_knots_owner_active
    ON knots(owner_id, status, kind);

CREATE INDEX IF NOT EXISTS idx_knots_owner_clarity
    ON knots(owner_id, clarity DESC);

CREATE TABLE IF NOT EXISTS knot_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    knot_id INTEGER REFERENCES knots(id),
    owner_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    detail_json TEXT,
    source_session_id TEXT,
    source_compression_id INTEGER REFERENCES session_compressions(id),
    source_message_id_from INTEGER,
    source_message_id_to INTEGER,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS knot_provenance (
    knot_id INTEGER NOT NULL REFERENCES knots(id),
    compression_id INTEGER REFERENCES session_compressions(id),
    session_id TEXT NOT NULL,
    message_id_from INTEGER,
    message_id_to INTEGER,
    PRIMARY KEY (knot_id, compression_id)
);

PRAGMA user_version = 1;
";

/// Returns true if `messages.in_context` and `knots` exist (current M7 shape).
pub fn has_current_shape(conn: &Connection) -> Result<bool> {
    let messages_ok = conn
        .prepare("SELECT in_context FROM messages LIMIT 0")
        .is_ok();
    let knots_ok = conn.prepare("SELECT id FROM knots LIMIT 0").is_ok();
    Ok(messages_ok && knots_ok)
}

pub fn database_has_user_tables(conn: &Connection) -> Result<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'messages'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| Error::Message(format!("schema probe: {e}")))?;
    Ok(count > 0)
}

pub fn incompatible_error(db_path: &str) -> Error {
    Error::with_arg(
        MessageId::SchemaIncompatible,
        format!(
            "sessions.db schema 不兼容（需要 version {SCHEMA_VERSION}）。\
         请备份后删除 {db_path} 并重新运行 hi（将自动重建）。\
         导出全文: hi session export"
        ),
    )
}

pub fn ensure_compatible(conn: &Connection, db_path: &str) -> Result<()> {
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| Error::Message(format!("read user_version: {e}")))?;

    if version == SCHEMA_VERSION {
        return Ok(());
    }

    if version == 0 {
        if database_has_user_tables(conn)? {
            if !has_current_shape(conn)? {
                return Err(incompatible_error(db_path));
            }
            conn.execute_batch(INIT_SQL)
                .map_err(|e| Error::Message(format!("refresh schema: {e}")))?;
            return Ok(());
        }
        conn.execute_batch(INIT_SQL)
            .map_err(|e| Error::Message(format!("init schema: {e}")))?;
        return Ok(());
    }

    Err(incompatible_error(db_path))
}

#[cfg(test)]
#[path = "../../test/unit/store/schema.rs"]
mod tests;
