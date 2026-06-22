use super::{
    decay::effective_clarity, Knot, KnotConfidence, KnotKind, KnotVisibility, TaskStatus,
};
use crate::config::MemoryConfig;
use crate::error::{Error, Result};
use crate::memory::resolve_owner;
use crate::store::{SessionStore, now_unix};
use crate::SessionId;

/// Keyword tokens for matching (CJK 2+, Latin 3+).
///
/// Author: gz
pub fn extract_keywords(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut latin = String::new();
    let mut cjk = String::new();

    let flush_latin = |buf: &mut String, acc: &mut Vec<String>| {
        if buf.len() >= 3 {
            acc.push(buf.to_lowercase());
        }
        buf.clear();
    };

    let flush_cjk = |buf: &mut String, acc: &mut Vec<String>| {
        let chars: Vec<char> = buf.chars().collect();
        if chars.len() >= 2 {
            acc.push(buf.clone());
        }
        buf.clear();
    };

    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            flush_cjk(&mut cjk, &mut out);
            latin.push(ch.to_ascii_lowercase());
        } else if ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            flush_latin(&mut latin, &mut out);
            cjk.push(ch);
        } else {
            flush_latin(&mut latin, &mut out);
            flush_cjk(&mut cjk, &mut out);
        }
    }
    flush_latin(&mut latin, &mut out);
    flush_cjk(&mut cjk, &mut out);
    out.sort_unstable();
    out.dedup();
    out
}

fn passes_clarity(k: &Knot, config: &MemoryConfig, now: i64) -> bool {
    if k.visibility != KnotVisibility::Inject {
        return false;
    }
    let clarity = effective_clarity(k, config, now);
    if clarity < config.inject_clarity_threshold {
        return false;
    }
    !(k.confidence == KnotConfidence::Dream && clarity < 0.6)
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

fn sort_knots(candidates: &mut [(Knot, f32)]) {
    candidates.sort_by(|(a, ca), (b, cb)| {
        kind_rank(b.kind)
            .cmp(&kind_rank(a.kind))
            .then_with(|| b.permanent.cmp(&a.permanent))
            .then_with(|| task_open_rank(b).cmp(&task_open_rank(a)))
            .then_with(|| cb.partial_cmp(ca).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
}

/// Baseline knots for system prompt: stable preference + fact only.
///
/// Author: gz
pub fn select_baseline_knots(knots: &[Knot], config: &MemoryConfig, now: i64) -> Vec<Knot> {
    let mut candidates: Vec<(Knot, f32)> = knots
        .iter()
        .filter(|k| matches!(k.kind, KnotKind::Preference | KnotKind::Fact))
        .filter_map(|k| {
            if !passes_clarity(k, config, now) {
                return None;
            }
            Some((k.clone(), effective_clarity(k, config, now)))
        })
        .collect();
    sort_knots(&mut candidates);
    candidates.into_iter().map(|(k, _)| k).collect()
}

/// Query-driven knot search (for `memory_search` tool).
///
/// Author: gz
pub fn search_knots(
    knots: &[Knot],
    config: &MemoryConfig,
    query: &str,
    kind_filter: Option<KnotKind>,
    limit: usize,
    now: i64,
) -> Vec<Knot> {
    let keywords = extract_keywords(query);
    if keywords.is_empty() {
        return vec![];
    }

    let mut candidates: Vec<(Knot, f32)> = knots
        .iter()
        .filter(|k| kind_filter.is_none_or(|kind| k.kind == kind))
        .filter_map(|k| {
            if !passes_clarity(k, config, now) {
                return None;
            }
            if !keywords.iter().any(|kw| k.content.to_lowercase().contains(kw)) {
                return None;
            }
            Some((k.clone(), effective_clarity(k, config, now)))
        })
        .collect();

    sort_knots(&mut candidates);
    candidates
        .into_iter()
        .take(limit)
        .map(|(k, _)| k)
        .collect()
}

fn confidence_tag(confidence: KnotConfidence) -> &'static str {
    match confidence {
        KnotConfidence::Confirmed => "confirmed",
        KnotConfidence::Inferred => "inferred",
        KnotConfidence::Dream => "unverified",
    }
}

fn kind_label(kind: KnotKind) -> &'static str {
    match kind {
        KnotKind::Preference => "preference",
        KnotKind::Fact => "fact",
        KnotKind::Task => "task",
        KnotKind::Decision => "decision",
        KnotKind::Procedure => "procedure",
    }
}

fn format_line(knot: &Knot) -> String {
    let tag = confidence_tag(knot.confidence);
    let prefix = match knot.kind {
        KnotKind::Task if knot.task_status == Some(TaskStatus::Open) => "[ ] ",
        KnotKind::Task if knot.task_status == Some(TaskStatus::Done) => "[x] ",
        _ => "",
    };
    format!(
        "- #{id} [{kind}] {prefix}{content} ({tag})",
        id = knot.id,
        kind = kind_label(knot.kind),
        prefix = prefix,
        content = knot.content,
        tag = tag,
    )
}

/// Format search hits for tool output.
///
/// Author: gz
pub fn format_search_results(knots: &[Knot], max_chars: usize) -> String {
    if knots.is_empty() {
        return "No matching long-term memory found.".into();
    }
    let mut out = format!("Found {} memories:\n", knots.len());
    let mut used = out.chars().count();
    for knot in knots {
        let line = format!("{}\n", format_line(knot));
        if used + line.chars().count() > max_chars {
            out.push_str("\n(Results truncated; narrow the query or specify kind.)\n");
            break;
        }
        out.push_str(&line);
        used += line.chars().count();
    }
    out
}

/// Run memory search against the store (decay + query + record access).
///
/// Author: gz
pub fn run_memory_search(
    store: &SessionStore,
    session_id: &SessionId,
    config: &MemoryConfig,
    query: &str,
    kind_filter: Option<KnotKind>,
    limit: Option<usize>,
) -> Result<String> {
    let query = query.trim();
    if query.is_empty() {
        return Err(Error::Message("memory_search: query 不能为空".into()));
    }

    let owner = resolve_owner(session_id, config);
    store.ensure_memory_owner(&owner)?;
    store.apply_knot_decay(&owner, config)?;

    let limit = limit.unwrap_or(config.max_search_results).min(50);
    let now = now_unix();
    let knots = store.list_knots(&owner)?;
    let hits = search_knots(&knots, config, query, kind_filter, limit, now);

    let ids: Vec<i64> = hits.iter().map(|k| k.id).collect();
    if !ids.is_empty() {
        store.record_knot_injection(&ids)?;
    }

    Ok(format_search_results(&hits, config.max_search_chars))
}

#[cfg(test)]
#[path = "../../test/unit/memory/search.rs"]
mod tests;
