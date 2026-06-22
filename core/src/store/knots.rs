use rusqlite::{params, Connection, Transaction};

use crate::error::{Error, Result};
use crate::memory::{
    content_hash, Knot, KnotConfidence, NewKnot, OwnerId,
};

use super::time::now_unix;

use crate::SessionId;

/// Provenance for knot extraction (compression or turn).
///
/// Author: gz
#[derive(Debug, Clone, Default)]
pub struct KnotProvenance {
    pub session_id: Option<SessionId>,
    pub compression_id: Option<i64>,
    pub message_id_from: Option<i64>,
    pub message_id_to: Option<i64>,
}

/// Outcome of merging one extracted knot.
///
/// Author: gz
pub enum MergeKnotResult {
    Added,
    Superseded,
    Skipped,
}

/// Author: gz
pub enum KnotEventType {
    Created,
    Reinforced,
    Forgot,
    Injected,
    Superseded,
    Extracted,
}

impl KnotEventType {
    const fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Reinforced => "reinforced",
            Self::Forgot => "forgot",
            Self::Injected => "injected",
            Self::Superseded => "superseded",
            Self::Extracted => "extracted",
        }
    }
}

/// Author: gz
pub fn ensure_owner(conn: &Connection, owner_id: &OwnerId) -> Result<()> {
    let now = now_unix();
    conn.execute(
        "INSERT OR IGNORE INTO memory_owners (id, display_name, created_at) VALUES (?1, ?1, ?2)",
        params![owner_id.0, now],
    )
    .map_err(|e| Error::Message(format!("ensure memory_owner: {e}")))?;
    Ok(())
}

const KNOT_SELECT: &str = "SELECT id, owner_id, kind, content, status, task_status, confidence,
    clarity, permanent, visibility, content_hash, access_count, last_accessed_at, created_at, updated_at
    FROM knots";

/// Author: gz
pub fn list_active(conn: &Connection, owner_id: &OwnerId) -> Result<Vec<Knot>> {
    list(conn, owner_id, false)
}

/// Author: gz
pub fn list_all(conn: &Connection, owner_id: &OwnerId) -> Result<Vec<Knot>> {
    list(conn, owner_id, true)
}

fn list(conn: &Connection, owner_id: &OwnerId, include_inactive: bool) -> Result<Vec<Knot>> {
    let sql = if include_inactive {
        format!("{KNOT_SELECT} WHERE owner_id = ?1 ORDER BY updated_at DESC")
    } else {
        format!("{KNOT_SELECT} WHERE owner_id = ?1 AND status = 'active' ORDER BY clarity DESC, updated_at DESC")
    };
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| Error::Message(format!("prepare list knots: {e}")))?;
    let rows = stmt
        .query_map(params![owner_id.0], Knot::from_row)
        .map_err(|e| Error::Message(format!("list knots: {e}")))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Message(format!("knot row: {e}")))
}

/// Author: gz
pub fn get(conn: &Connection, knot_id: i64) -> Result<Knot> {
    conn.query_row(
        &format!("{KNOT_SELECT} WHERE id = ?1"),
        params![knot_id],
        Knot::from_row,
    )
    .map_err(|e| Error::Message(format!("get knot {knot_id}: {e}")))
}

/// Author: gz
pub fn insert(conn: &Connection, knot: &NewKnot) -> Result<i64> {
    ensure_owner(conn, &knot.owner_id)?;
    let now = now_unix();
    let hash = content_hash(&knot.content);

    if let Some(existing) = find_active_by_hash(conn, &knot.owner_id, &hash)? {
        bump_access(conn, existing, now)?;
        return Ok(existing);
    }

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| Error::Message(format!("begin knot insert: {e}")))?;
    let clarity = if knot.confidence == KnotConfidence::Dream {
        knot.clarity.min(0.4)
    } else {
        knot.clarity
    };
    tx.execute(
        "INSERT INTO knots (
            owner_id, scope, session_id, kind, content, status, task_status,
            confidence, clarity, permanent, visibility, content_hash,
            access_count, created_at, updated_at
         ) VALUES (?1, 'owner', NULL, ?2, ?3, 'active', ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?10)",
        params![
            knot.owner_id.0,
            knot.kind.as_str(),
            knot.content,
            knot.task_status.map(|t| t.as_str()),
            knot.confidence.as_str(),
            clarity,
            i64::from(knot.permanent),
            knot.visibility.as_str(),
            hash,
            now,
        ],
    )
    .map_err(|e| Error::Message(format!("insert knot: {e}")))?;
    let id = tx.last_insert_rowid();
    append_event(&tx, id, &knot.owner_id, KnotEventType::Created, None, now)?;
    tx.commit()
        .map_err(|e| Error::Message(format!("commit knot insert: {e}")))?;
    Ok(id)
}

fn find_active_by_hash(
    conn: &Connection,
    owner_id: &OwnerId,
    hash: &str,
) -> Result<Option<i64>> {
    let id: Option<i64> = conn
        .query_row(
            "SELECT id FROM knots WHERE owner_id = ?1 AND content_hash = ?2 AND status = 'active'",
            params![owner_id.0, hash],
            |row| row.get(0),
        )
        .ok();
    Ok(id)
}

fn bump_access(conn: &Connection, knot_id: i64, now: i64) -> Result<()> {
    conn.execute(
        "UPDATE knots SET access_count = access_count + 1, last_accessed_at = ?1, updated_at = ?1
         WHERE id = ?2",
        params![now, knot_id],
    )
    .map_err(|e| Error::Message(format!("bump knot access: {e}")))?;
    Ok(())
}

/// Merge one LLM-extracted knot (dedup, supersede, provenance).
///
/// Author: gz
pub fn merge_knot(
    conn: &Connection,
    knot: &NewKnot,
    supersedes_hash: Option<&str>,
    provenance: &KnotProvenance,
) -> Result<(MergeKnotResult, i64)> {
    ensure_owner(conn, &knot.owner_id)?;
    let now = now_unix();
    let hash = content_hash(&knot.content);

    if let Some(existing) = find_active_by_hash(conn, &knot.owner_id, &hash)? {
        bump_access(conn, existing, now)?;
        return Ok((MergeKnotResult::Skipped, existing));
    }

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| Error::Message(format!("begin knot merge: {e}")))?;

    let mut superseded_old: Option<i64> = None;
    if let Some(old_hash) = supersedes_hash {
        if let Some(old_id) = find_active_by_hash(&tx, &knot.owner_id, old_hash)? {
            tx.execute(
                "UPDATE knots SET status = 'superseded', updated_at = ?1 WHERE id = ?2",
                params![now, old_id],
            )
            .map_err(|e| Error::Message(format!("supersede knot: {e}")))?;
            append_event_with_provenance(
                &tx,
                old_id,
                &knot.owner_id,
                KnotEventType::Superseded,
                None,
                provenance,
                now,
            )?;
            superseded_old = Some(old_id);
        }
    }

    let clarity = if knot.confidence == KnotConfidence::Dream {
        knot.clarity.min(0.4)
    } else {
        knot.clarity
    };
    tx.execute(
        "INSERT INTO knots (
            owner_id, scope, session_id, kind, content, status, task_status,
            confidence, clarity, permanent, visibility, content_hash,
            access_count, created_at, updated_at, superseded_by
         ) VALUES (?1, 'owner', ?2, ?3, ?4, 'active', ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?11, NULL)",
        params![
            knot.owner_id.0,
            provenance.session_id.as_ref().map(|s| s.0.as_str()),
            knot.kind.as_str(),
            knot.content,
            knot.task_status.map(|t| t.as_str()),
            knot.confidence.as_str(),
            clarity,
            i64::from(knot.permanent),
            knot.visibility.as_str(),
            hash,
            now,
        ],
    )
    .map_err(|e| Error::Message(format!("insert merged knot: {e}")))?;
    let new_id = tx.last_insert_rowid();

    if let Some(old_id) = superseded_old {
        tx.execute(
            "UPDATE knots SET superseded_by = ?1 WHERE id = ?2",
            params![new_id, old_id],
        )
        .map_err(|e| Error::Message(format!("link superseded_by: {e}")))?;
    }

    append_event_with_provenance(
        &tx,
        new_id,
        &knot.owner_id,
        KnotEventType::Extracted,
        None,
        provenance,
        now,
    )?;

    if let Some(compression_id) = provenance.compression_id {
        if let Some(session_id) = &provenance.session_id {
            tx.execute(
                "INSERT OR IGNORE INTO knot_provenance
                 (knot_id, compression_id, session_id, message_id_from, message_id_to)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    new_id,
                    compression_id,
                    session_id.0,
                    provenance.message_id_from,
                    provenance.message_id_to,
                ],
            )
            .map_err(|e| Error::Message(format!("insert knot provenance: {e}")))?;
        }
    }

    tx.commit()
        .map_err(|e| Error::Message(format!("commit knot merge: {e}")))?;

    let result = if superseded_old.is_some() {
        MergeKnotResult::Superseded
    } else {
        MergeKnotResult::Added
    };
    Ok((result, new_id))
}

/// Soft-delete: `status = deleted`.
///
/// Author: gz
pub fn forget(conn: &Connection, knot_id: i64) -> Result<()> {
    let knot = get(conn, knot_id)?;
    let now = now_unix();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| Error::Message(format!("begin knot forget: {e}")))?;
    tx.execute(
        "UPDATE knots SET status = 'deleted', updated_at = ?1 WHERE id = ?2",
        params![now, knot_id],
    )
    .map_err(|e| Error::Message(format!("forget knot: {e}")))?;
    append_event(
        &tx,
        knot_id,
        &knot.owner_id,
        KnotEventType::Forgot,
        None,
        now,
    )?;
    tx.commit()
        .map_err(|e| Error::Message(format!("commit knot forget: {e}")))?;
    Ok(())
}

/// Author: gz
pub fn reinforce(conn: &Connection, knot_id: i64, permanent: bool) -> Result<()> {
    let knot = get(conn, knot_id)?;
    let now = now_unix();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| Error::Message(format!("begin knot reinforce: {e}")))?;
    tx.execute(
        "UPDATE knots SET clarity = 1.0, permanent = ?1, updated_at = ?2, last_accessed_at = ?2
         WHERE id = ?3",
        params![i64::from(permanent), now, knot_id],
    )
    .map_err(|e| Error::Message(format!("reinforce knot: {e}")))?;
    append_event(
        &tx,
        knot_id,
        &knot.owner_id,
        KnotEventType::Reinforced,
        Some(format!(r#"{{"permanent":{permanent}}}"#)),
        now,
    )?;
    tx.commit()
        .map_err(|e| Error::Message(format!("commit knot reinforce: {e}")))?;
    Ok(())
}

/// Lazy 忘川 decay persisted to SQLite; supersede when clarity drops below 0.1.
///
/// Author: gz
pub fn apply_decay(
    conn: &Connection,
    owner_id: &OwnerId,
    config: &crate::config::MemoryConfig,
) -> Result<()> {
    if !config.decay_enabled {
        return Ok(());
    }
    let now = now_unix();
    for knot in list_active(conn, owner_id)? {
        if knot.permanent {
            continue;
        }
        let new_clarity = crate::memory::effective_clarity(&knot, config, now);
        if new_clarity < 0.1 {
            conn.execute(
                "UPDATE knots SET status = 'superseded', clarity = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_clarity, now, knot.id],
            )
            .map_err(|e| Error::Message(format!("supersede knot: {e}")))?;
        } else if (new_clarity - knot.clarity).abs() > f32::EPSILON {
            conn.execute(
                "UPDATE knots SET clarity = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_clarity, now, knot.id],
            )
            .map_err(|e| Error::Message(format!("decay knot: {e}")))?;
        }
    }
    Ok(())
}

/// Reinforce clarity for knots injected this turn.
///
/// Author: gz
pub fn record_injection(conn: &Connection, knot_ids: &[i64]) -> Result<()> {
    if knot_ids.is_empty() {
        return Ok(());
    }
    let now = now_unix();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| Error::Message(format!("begin knot injection tx: {e}")))?;
    for id in knot_ids {
        let owner_id: String = tx
            .query_row(
                "SELECT owner_id FROM knots WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| Error::Message(format!("knot owner for injection: {e}")))?;
        tx.execute(
            "UPDATE knots SET
                clarity = MIN(1.0, clarity + 0.15),
                access_count = access_count + 1,
                last_accessed_at = ?1,
                updated_at = ?1
             WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| Error::Message(format!("record knot injection: {e}")))?;
        append_event(
            &tx,
            *id,
            &OwnerId(owner_id),
            KnotEventType::Injected,
            None,
            now,
        )?;
    }
    tx.commit()
        .map_err(|e| Error::Message(format!("commit knot injection: {e}")))?;
    Ok(())
}

fn append_event(
    tx: &Transaction<'_>,
    knot_id: i64,
    owner_id: &OwnerId,
    event_type: KnotEventType,
    detail_json: Option<String>,
    now: i64,
) -> Result<()> {
    append_event_with_provenance(
        tx,
        knot_id,
        owner_id,
        event_type,
        detail_json,
        &KnotProvenance::default(),
        now,
    )
}

fn append_event_with_provenance(
    tx: &Transaction<'_>,
    knot_id: i64,
    owner_id: &OwnerId,
    event_type: KnotEventType,
    detail_json: Option<String>,
    provenance: &KnotProvenance,
    now: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO knot_events (
            knot_id, owner_id, event_type, detail_json,
            source_session_id, source_compression_id,
            source_message_id_from, source_message_id_to, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            knot_id,
            owner_id.0,
            event_type.label(),
            detail_json,
            provenance.session_id.as_ref().map(|s| s.0.as_str()),
            provenance.compression_id,
            provenance.message_id_from,
            provenance.message_id_to,
            now,
        ],
    )
    .map_err(|e| Error::Message(format!("insert knot_event: {e}")))?;
    Ok(())
}

#[cfg(test)]
#[path = "../../test/unit/store/knots.rs"]
mod tests;
