use rusqlite::{params, Connection, Transaction};

use crate::error::{Error, Result};
use crate::SessionId;

/// Record of one context compression (messages remain in DB; range marked `in_context = 0`).
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCompression {
    pub id: i64,
    pub session_id: SessionId,
    pub message_id_from: i64,
    pub message_id_to: i64,
    pub message_count: u32,
    pub token_estimate: Option<u32>,
    pub summary_text: Option<String>,
    pub created_at: i64,
}

/// Author: gz
pub struct NewSessionCompression {
    pub message_id_from: i64,
    pub message_id_to: i64,
    pub message_count: u32,
    pub token_estimate: Option<u32>,
    pub summary_text: Option<String>,
}

/// Insert compression event and touch session metadata. Runs inside caller's transaction.
///
/// Author: gz
pub fn insert(
    tx: &Transaction<'_>,
    session_id: &SessionId,
    record: &NewSessionCompression,
    now: i64,
) -> Result<i64> {
    tx.execute(
        "INSERT INTO session_compressions
         (session_id, message_id_from, message_id_to, message_count, token_estimate, summary_text, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            session_id.0,
            record.message_id_from,
            record.message_id_to,
            record.message_count,
            record.token_estimate,
            record.summary_text,
            now,
        ],
    )
    .map_err(|e| Error::Message(format!("insert session_compression: {e}")))?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "UPDATE sessions SET last_compression_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, session_id.0],
    )
    .map_err(|e| Error::Message(format!("touch session after compression: {e}")))?;
    Ok(id)
}

/// Author: gz
pub fn list_for_session(
    conn: &Connection,
    session_id: &SessionId,
) -> Result<Vec<SessionCompression>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, message_id_from, message_id_to, message_count,
                    token_estimate, summary_text, created_at
             FROM session_compressions
             WHERE session_id = ?1
             ORDER BY created_at DESC",
        )
        .map_err(|e| Error::Message(format!("prepare list compressions: {e}")))?;

    let rows = stmt
        .query_map(params![session_id.0], |row| {
            Ok(SessionCompression {
                id: row.get(0)?,
                session_id: SessionId(row.get(1)?),
                message_id_from: row.get(2)?,
                message_id_to: row.get(3)?,
                message_count: row.get::<_, i64>(4)? as u32,
                token_estimate: row
                    .get::<_, Option<i64>>(5)?
                    .map(|n| n.max(0) as u32),
                summary_text: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| Error::Message(format!("list compressions: {e}")))?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Message(format!("compression row: {e}")))
}

/// Author: gz
pub fn get_by_id(conn: &Connection, compression_id: i64) -> Result<SessionCompression> {
    conn.query_row(
        "SELECT id, session_id, message_id_from, message_id_to, message_count,
                token_estimate, summary_text, created_at
         FROM session_compressions WHERE id = ?1",
        params![compression_id],
        |row| {
            Ok(SessionCompression {
                id: row.get(0)?,
                session_id: SessionId(row.get(1)?),
                message_id_from: row.get(2)?,
                message_id_to: row.get(3)?,
                message_count: row.get::<_, i64>(4)? as u32,
                token_estimate: row
                    .get::<_, Option<i64>>(5)?
                    .map(|n| n.max(0) as u32),
                summary_text: row.get(6)?,
                created_at: row.get(7)?,
            })
        },
    )
    .map_err(|e| Error::Message(format!("get compression {compression_id}: {e}")))
}
