use serde::{Deserialize, Serialize};

use super::OwnerId;

/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnotKind {
    Preference,
    Fact,
    Decision,
    Task,
    Procedure,
}

impl KnotKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preference => "preference",
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Task => "task",
            Self::Procedure => "procedure",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "preference" => Some(Self::Preference),
            "fact" => Some(Self::Fact),
            "decision" => Some(Self::Decision),
            "task" => Some(Self::Task),
            "procedure" => Some(Self::Procedure),
            _ => None,
        }
    }
}

/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnotConfidence {
    Confirmed,
    Inferred,
    Dream,
}

impl KnotConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Inferred => "inferred",
            Self::Dream => "dream",
        }
    }

    fn from_db(s: &str) -> Self {
        match s {
            "confirmed" => Self::Confirmed,
            "dream" => Self::Dream,
            _ => Self::Inferred,
        }
    }
}

/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnotStatus {
    Active,
    Superseded,
    Deleted,
}

impl KnotStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::Deleted => "deleted",
        }
    }

    fn from_db(s: &str) -> Self {
        match s {
            "superseded" => Self::Superseded,
            "deleted" => Self::Deleted,
            _ => Self::Active,
        }
    }
}

/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    Done,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    fn from_db_opt(s: Option<String>) -> Option<Self> {
        s.and_then(|v| match v.as_str() {
            "open" => Some(Self::Open),
            "done" => Some(Self::Done),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        })
    }
}

/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnotVisibility {
    Inject,
    Private,
}

impl KnotVisibility {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Inject => "inject",
            Self::Private => "private",
        }
    }

    fn from_db(s: &str) -> Self {
        if s == "private" {
            Self::Private
        } else {
            Self::Inject
        }
    }
}

/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knot {
    pub id: i64,
    pub owner_id: OwnerId,
    pub kind: KnotKind,
    pub content: String,
    pub status: KnotStatus,
    pub task_status: Option<TaskStatus>,
    pub confidence: KnotConfidence,
    pub clarity: f32,
    pub permanent: bool,
    pub visibility: KnotVisibility,
    pub content_hash: String,
    pub access_count: u32,
    pub last_accessed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Author: gz
#[derive(Debug, Clone)]
pub struct NewKnot {
    pub owner_id: OwnerId,
    pub kind: KnotKind,
    pub content: String,
    pub confidence: KnotConfidence,
    pub clarity: f32,
    pub permanent: bool,
    pub visibility: KnotVisibility,
    pub task_status: Option<TaskStatus>,
}

impl Knot {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            owner_id: OwnerId(row.get(1)?),
            kind: KnotKind::parse(row.get::<_, String>(2)?.as_str()).unwrap_or(KnotKind::Fact),
            content: row.get(3)?,
            status: KnotStatus::from_db(row.get::<_, String>(4)?.as_str()),
            task_status: TaskStatus::from_db_opt(row.get(5)?),
            confidence: KnotConfidence::from_db(row.get::<_, String>(6)?.as_str()),
            clarity: row.get(7)?,
            permanent: row.get::<_, i64>(8)? != 0,
            visibility: KnotVisibility::from_db(row.get::<_, String>(9)?.as_str()),
            content_hash: row.get(10)?,
            access_count: row.get::<_, i64>(11)? as u32,
            last_accessed_at: row.get(12)?,
            created_at: row.get(13)?,
            updated_at: row.get(14)?,
        })
    }
}

    /// Normalize content and produce a stable dedup key.
pub fn content_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
