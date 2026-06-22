use super::search::{select_baseline_knots, extract_keywords};
use super::{Knot, KnotConfidence, KnotKind, KnotVisibility, TaskStatus};
use crate::config::MemoryConfig;
use crate::memory::decay::effective_clarity;

pub const BASELINE_SEARCH_HINT: &str =
    "\n\nFor tasks, decisions, procedures, and other long-term memory not listed above, \
use the memory_search tool with keywords.";

/// Result of knot recall for system prompt injection.
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectResult {
    pub block: String,
    pub injected_ids: Vec<i64>,
}

/// Select knots and format the system prompt block.
///
/// Author: gz
pub fn build_injection(
    knots: &[Knot],
    config: &MemoryConfig,
    user_query: Option<&str>,
    now: i64,
) -> InjectResult {
    let baseline_mode = config.memory_search_enabled && config.inject_baseline_only;
    let selected = if baseline_mode {
        select_baseline_knots(knots, config, now)
    } else {
        select_knots(knots, config, user_query, now)
    };
    let max_chars = if baseline_mode {
        config.inject_baseline_max_chars
    } else {
        config.max_inject_chars
    };
    let mut block = if baseline_mode {
        format_baseline_block(&selected, max_chars)
    } else {
        format_full_block(&selected, max_chars)
    };
    if baseline_mode && config.memory_search_enabled {
        block.push_str(BASELINE_SEARCH_HINT);
    }
    let injected_ids: Vec<i64> = selected.iter().map(|k| k.id).collect();
    InjectResult {
        block,
        injected_ids,
    }
}

fn select_knots(
    knots: &[Knot],
    config: &MemoryConfig,
    user_query: Option<&str>,
    now: i64,
) -> Vec<Knot> {
    let keywords = user_query.map(extract_keywords).unwrap_or_default();
    let mut candidates: Vec<(Knot, f32)> = knots
        .iter()
        .filter(|k| k.visibility == KnotVisibility::Inject)
        .filter_map(|k| {
            let clarity = effective_clarity(k, config, now);
            if clarity < config.inject_clarity_threshold {
                return None;
            }
            if k.confidence == KnotConfidence::Dream && clarity < 0.6 {
                return None;
            }
            Some((k.clone(), clarity))
        })
        .collect();

    if !keywords.is_empty() {
        candidates.retain(|(k, _)| {
            always_include_kind(k.kind) || keywords.iter().any(|kw| k.content.contains(kw))
        });
    }

    candidates.sort_by(|(a, ca), (b, cb)| {
        kind_rank(b.kind)
            .cmp(&kind_rank(a.kind))
            .then_with(|| b.permanent.cmp(&a.permanent))
            .then_with(|| task_open_rank(b).cmp(&task_open_rank(a)))
            .then_with(|| cb.partial_cmp(ca).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });

    candidates.into_iter().map(|(k, _)| k).collect()
}

fn always_include_kind(kind: KnotKind) -> bool {
    matches!(kind, KnotKind::Preference | KnotKind::Fact)
}

fn kind_rank(kind: KnotKind) -> u8 {
    match kind {
        KnotKind::Preference => 5,
        KnotKind::Fact => 4,
        KnotKind::Task => 3,
        KnotKind::Decision => 2,
        KnotKind::Procedure => 1,
    }
}

fn task_open_rank(knot: &Knot) -> u8 {
    if knot.kind == KnotKind::Task && knot.task_status == Some(TaskStatus::Open) {
        1
    } else {
        0
    }
}

fn format_baseline_block(knots: &[Knot], max_chars: usize) -> String {
    if knots.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "## Long-term memory (baseline)\n\n\
         Stable preferences and facts about the user. If anything conflicts with the current \
         conversation, prefer the conversation.\n",
    );
    let mut used = out.chars().count();

    for kind in [KnotKind::Preference, KnotKind::Fact] {
        let section: Vec<_> = knots.iter().filter(|k| k.kind == kind).collect();
        if section.is_empty() {
            continue;
        }
        let heading = section_heading(kind);
        let header = format!("\n{heading}\n");
        if used + header.chars().count() > max_chars {
            break;
        }
        out.push_str(&header);
        used += header.chars().count();

        for knot in section {
            let line = format_knot_line(knot);
            if used + line.chars().count() > max_chars {
                return out;
            }
            out.push_str(&line);
            used += line.chars().count();
        }
    }
    out
}

fn format_full_block(knots: &[Knot], max_chars: usize) -> String {
    if knots.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "## Long-term memory\n\n\
         Facts about the user from prior sessions. If anything conflicts with the current \
         conversation, prefer the conversation; ask when unsure.\n",
    );
    let mut used = out.chars().count();

    for kind in [
        KnotKind::Preference,
        KnotKind::Fact,
        KnotKind::Task,
        KnotKind::Decision,
        KnotKind::Procedure,
    ] {
        let section: Vec<_> = knots.iter().filter(|k| k.kind == kind).collect();
        if section.is_empty() {
            continue;
        }
        let heading = section_heading(kind);
        let header = format!("\n{heading}\n");
        if used + header.chars().count() > max_chars {
            break;
        }
        out.push_str(&header);
        used += header.chars().count();

        for knot in section {
            let line = format_knot_line(knot);
            if used + line.chars().count() > max_chars {
                return out;
            }
            out.push_str(&line);
            used += line.chars().count();
        }
    }
    out
}

fn section_heading(kind: KnotKind) -> &'static str {
    match kind {
        KnotKind::Preference => "### Preferences",
        KnotKind::Fact => "### Facts",
        KnotKind::Task => "### Tasks",
        KnotKind::Decision => "### Decisions",
        KnotKind::Procedure => "### Procedures",
    }
}

fn format_knot_line(knot: &Knot) -> String {
    let tag = confidence_tag(knot.confidence);
    match knot.kind {
        KnotKind::Task if knot.task_status == Some(TaskStatus::Open) => {
            format!("- [ ] {} ({tag})\n", knot.content)
        }
        KnotKind::Task if knot.task_status == Some(TaskStatus::Done) => {
            format!("- [x] {} ({tag})\n", knot.content)
        }
        _ => format!("- {} ({tag})\n", knot.content),
    }
}

fn confidence_tag(confidence: KnotConfidence) -> &'static str {
    match confidence {
        KnotConfidence::Confirmed => "confirmed",
        KnotConfidence::Inferred => "inferred",
        KnotConfidence::Dream => "unverified",
    }
}

#[cfg(test)]
#[path = "../../test/unit/memory/inject.rs"]
mod tests;
