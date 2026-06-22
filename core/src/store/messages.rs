use rusqlite::{params, Connection, Transaction};

use crate::error::{Error, Result};
use crate::llm::{ChatMessage, Role, ToolCall};
use crate::SessionId;

/// A persisted chat row with stable id (append-only; `in_context` toggles LLM visibility).
///
/// Author: gz
#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub id: i64,
    pub message: ChatMessage,
    pub in_context: bool,
}

/// Author: gz
pub fn load_all(conn: &Connection, session_id: &SessionId) -> Result<Vec<StoredMessage>> {
    query_messages(conn, session_id, None)
}

/// Rows visible to the agent (`in_context = 1`).
///
/// Author: gz
pub fn load_context(conn: &Connection, session_id: &SessionId) -> Result<Vec<StoredMessage>> {
    query_messages(conn, session_id, Some(true))
}

fn query_messages(
    conn: &Connection,
    session_id: &SessionId,
    in_context_only: Option<bool>,
) -> Result<Vec<StoredMessage>> {
    let sql = match in_context_only {
        None => {
            "SELECT id, role, content, tool_call_id, tool_calls_json, reasoning_content, in_context
             FROM messages WHERE session_id = ?1 ORDER BY id ASC"
        }
        Some(true) => {
            "SELECT id, role, content, tool_call_id, tool_calls_json, reasoning_content, in_context
             FROM messages WHERE session_id = ?1 AND in_context = 1 ORDER BY id ASC"
        }
        Some(false) => {
            "SELECT id, role, content, tool_call_id, tool_calls_json, reasoning_content, in_context
             FROM messages WHERE session_id = ?1 AND in_context = 0 ORDER BY id ASC"
        }
    };

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| Error::Message(format!("prepare messages query: {e}")))?;

    let rows = stmt
        .query_map(params![session_id.0], map_row)
        .map_err(|e| Error::Message(format!("query messages: {e}")))?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Message(format!("messages row: {e}")))
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredMessage> {
    let id: i64 = row.get(0)?;
    let role: String = row.get(1)?;
    let content: String = row.get(2)?;
    let tool_call_id: Option<String> = row.get(3)?;
    let tool_calls_json: Option<String> = row.get(4)?;
    let reasoning_content: Option<String> = row.get(5)?;
    let in_context: i64 = row.get(6)?;
    let role = match role.as_str() {
        "system" => Role::System,
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        other => {
            return Err(rusqlite::Error::InvalidColumnType(
                1,
                other.to_string(),
                rusqlite::types::Type::Text,
            ));
        }
    };
    let tool_calls = tool_calls_json
        .map(|json| serde_json::from_str::<Vec<ToolCall>>(&json))
        .transpose()
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    Ok(StoredMessage {
        id,
        message: ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
            reasoning_content,
        },
        in_context: in_context != 0,
    })
}

/// Append messages (INSERT only). Returns the new row ids in order.
///
/// Author: gz
pub fn append(
    tx: &Transaction<'_>,
    session_id: &SessionId,
    messages: &[ChatMessage],
    now: i64,
) -> Result<Vec<i64>> {
    let mut ids = Vec::with_capacity(messages.len());
    for msg in messages {
        let role = role_to_str(msg.role);
        let tool_calls_json = msg
            .tool_calls
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| Error::Message(format!("serialize tool_calls: {e}")))?;
        let tool_name = msg.tool_calls.as_ref().and_then(|calls| {
            if calls.len() == 1 {
                Some(calls[0].name.as_str())
            } else {
                None
            }
        });
        tx.execute(
            "INSERT INTO messages (session_id, role, content, tool_call_id, tool_calls_json, tool_name, reasoning_content, in_context, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)",
            params![
                session_id.0,
                role,
                msg.content,
                msg.tool_call_id,
                tool_calls_json,
                tool_name,
                msg.reasoning_content,
                now,
            ],
        )
        .map_err(|e| Error::Message(format!("insert message: {e}")))?;
        ids.push(tx.last_insert_rowid());
    }
    Ok(ids)
}

/// Update the earliest system message for a session (workdir sync).
///
/// Author: gz
pub fn update_system_content(
    conn: &Connection,
    session_id: &SessionId,
    content: &str,
) -> Result<bool> {
    let updated = conn
        .execute(
            "UPDATE messages SET content = ?1
             WHERE id = (
                 SELECT id FROM messages
                 WHERE session_id = ?2 AND role = 'system'
                 ORDER BY id ASC LIMIT 1
             )",
            params![content, session_id.0],
        )
        .map_err(|e| Error::Message(format!("update system message: {e}")))?;
    Ok(updated > 0)
}

/// Load messages in an id range (inclusive), ordered by id.
///
/// Author: gz
pub fn load_range(
    conn: &Connection,
    session_id: &SessionId,
    id_from: i64,
    id_to: i64,
) -> Result<Vec<StoredMessage>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, role, content, tool_call_id, tool_calls_json, reasoning_content, in_context
             FROM messages
             WHERE session_id = ?1 AND id >= ?2 AND id <= ?3
             ORDER BY id ASC",
        )
        .map_err(|e| Error::Message(format!("prepare message range: {e}")))?;

    let rows = stmt
        .query_map(params![session_id.0, id_from, id_to], map_row)
        .map_err(|e| Error::Message(format!("query message range: {e}")))?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Message(format!("message range row: {e}")))
}
///
/// Author: gz
pub fn mark_out_of_context(
    tx: &Transaction<'_>,
    session_id: &SessionId,
    message_id_from: i64,
    message_id_to: i64,
) -> Result<()> {
    tx.execute(
        "UPDATE messages SET in_context = 0
         WHERE session_id = ?1 AND id >= ?2 AND id <= ?3 AND in_context = 1",
        params![session_id.0, message_id_from, message_id_to],
    )
    .map_err(|e| Error::Message(format!("mark out of context: {e}")))?;
    Ok(())
}

/// Update persisted message body (used after emergency trim / in-place shrink).
///
/// Author: gz
pub fn update_message_content(conn: &Connection, message_id: i64, content: &str) -> Result<()> {
    conn.execute(
        "UPDATE messages SET content = ?1 WHERE id = ?2",
        params![content, message_id],
    )
    .map_err(|e| Error::Message(format!("update message {message_id}: {e}")))?;
    Ok(())
}

/// Mark all non-system rows out of LLM context (`/reset`); transcript remains in DB.
///
/// Author: gz
pub fn reset_context(conn: &Connection, session_id: &SessionId) -> Result<u64> {
    let updated = conn
        .execute(
            "UPDATE messages SET in_context = 0
             WHERE session_id = ?1 AND role != 'system' AND in_context = 1",
            params![session_id.0],
        )
        .map_err(|e| Error::Message(format!("reset session context: {e}")))?;
    Ok(updated as u64)
}

/// Mark a contiguous id range out of LLM context (failed turn rollback).
///
/// Author: gz
pub fn mark_ids_out_of_context(
    conn: &Connection,
    session_id: &SessionId,
    ids: &[i64],
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 2))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE messages SET in_context = 0
         WHERE session_id = ?1 AND id IN ({placeholders}) AND in_context = 1"
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(session_id.0.clone())];
    for id in ids {
        params.push(Box::new(*id));
    }
    conn.execute(
        &sql,
        rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
    )
    .map_err(|e| Error::Message(format!("mark ids out of context: {e}")))?;
    Ok(())
}

pub(super) fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}
