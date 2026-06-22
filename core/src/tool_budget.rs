//! Per-turn tool-call iteration budget: pressure notices and summary nudge.

use crate::messages::{t, Locale, MessageId};

/// Injected as a synthetic user turn when the iteration cap is hit (not persisted).
pub fn summary_nudge(locale: Locale) -> String {
    t(locale, MessageId::ToolBudgetForceSummary, &[])
}

/// Warn the model as it approaches the per-turn tool iteration cap.
pub fn budget_pressure_notice(locale: Locale, iteration: usize, max_iters: usize) -> Option<String> {
    if max_iters == 0 || iteration == 0 || iteration > max_iters {
        return None;
    }
    let left = max_iters.saturating_sub(iteration);
    let pct = iteration.saturating_mul(100) / max_iters;
    if pct >= 70 {
        Some(t(
            locale,
            MessageId::ToolBudgetReminder,
            &[iteration.to_string(), max_iters.to_string(), left.to_string()],
        ))
    } else {
        None
    }
}

pub fn budget_summary_prefix(locale: Locale, max_iters: usize) -> String {
    t(
        locale,
        MessageId::ToolBudgetSummaryPrefix,
        &[max_iters.to_string()],
    )
}

fn append_notice_to_last_tool(history: &mut [crate::llm::ChatMessage], notice: &str) {
    let Some(last) = history.last_mut() else {
        return;
    };
    if last.role != crate::llm::Role::Tool {
        return;
    }
    if !last.content.is_empty() {
        last.content.push_str("\n\n");
    }
    last.content.push_str(notice);
}

pub fn apply_budget_pressure(
    locale: Locale,
    history: &mut [crate::llm::ChatMessage],
    iteration: usize,
    max_iters: usize,
) {
    if let Some(notice) = budget_pressure_notice(locale, iteration, max_iters) {
        append_notice_to_last_tool(history, &notice);
    }
}

#[cfg(test)]
#[path = "../test/unit/tool_budget.rs"]
mod tests;
