use crate::error::Result;
use crate::memory::{
    ExtractedKnot, KnotConfidence, KnotKind, KnotVisibility, NewKnot, OwnerId,
};
use crate::store::{KnotProvenance, SessionStore};

/// Result of merging extracted knots into the store.
///
/// Author: gz
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeOutcome {
    pub added: usize,
    pub superseded: usize,
    pub skipped: usize,
}

fn initial_clarity(confidence: KnotConfidence) -> f32 {
    match confidence {
        KnotConfidence::Confirmed => 1.0,
        KnotConfidence::Inferred => 0.7,
        KnotConfidence::Dream => 0.4,
    }
}

fn task_status_for(kind: KnotKind, task_status: Option<crate::memory::TaskStatus>) -> Option<crate::memory::TaskStatus> {
    if kind == KnotKind::Task {
        Some(task_status.unwrap_or(crate::memory::TaskStatus::Open))
    } else {
        None
    }
}

/// Persist extracted knots (dedup, supersede, provenance).
///
/// Author: gz
pub fn merge_extracted(
    store: &SessionStore,
    owner: &OwnerId,
    extracted: &[ExtractedKnot],
    provenance: &KnotProvenance,
) -> Result<MergeOutcome> {
    let mut outcome = MergeOutcome::default();
    for item in extracted {
        let content = item.content.trim();
        if content.is_empty() {
            continue;
        }
        let clarity = initial_clarity(item.confidence);
        let new_knot = NewKnot {
            owner_id: owner.clone(),
            kind: item.kind,
            content: content.to_string(),
            confidence: item.confidence,
            clarity,
            permanent: item.confidence == KnotConfidence::Confirmed,
            visibility: KnotVisibility::Inject,
            task_status: task_status_for(item.kind, item.task_status),
        };

        match store.merge_knot(&new_knot, item.supersedes_content_hash.as_deref(), provenance)? {
            crate::store::MergeKnotResult::Added => outcome.added += 1,
            crate::store::MergeKnotResult::Superseded => {
                outcome.added += 1;
                outcome.superseded += 1;
            }
            crate::store::MergeKnotResult::Skipped => outcome.skipped += 1,
        }
    }
    Ok(outcome)
}

#[cfg(test)]
#[path = "../../test/unit/memory/merge.rs"]
mod tests;
