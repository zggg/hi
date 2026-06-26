use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use hi_core::ModelProfile;

use crate::menu_scroll::menu_window_start;
use crate::theme::UiTheme;

const CONT: &str = "  ";

/// 模型选择器行（输入框上方）。
pub fn model_picker_lines(
    profiles: &[ModelProfile],
    selected: usize,
    width: u16,
    max_rows: usize,
) -> Vec<Line<'static>> {
    let w = width.max(1) as usize;
    let show = profiles.len().min(max_rows);
    let start = menu_window_start(profiles.len(), selected, max_rows);
    let mut out = Vec::with_capacity(show);
    for (slot, profile) in profiles.iter().skip(start).take(show).enumerate() {
        let i = start + slot;
        let active_row = i == selected;
        let name_style = if active_row {
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(55, 55, 95))
                .add_modifier(Modifier::BOLD)
        } else if profile.active {
            Style::default().fg(Color::Rgb(140, 200, 140))
        } else {
            UiTheme::ASSISTANT
        };
        let detail = format!("{} · {}", profile.adapter, profile.model);
        let suffix = if profile.active { " (当前)" } else { "" };
        let label = format!("{detail}{suffix}");
        let gap = w.saturating_sub(profile.name.width() + label.width() + CONT.len() + 2);
        let detail_style = if active_row {
            Style::default().fg(Color::Rgb(200, 200, 220)).bg(Color::Rgb(55, 55, 95))
        } else {
            UiTheme::MUTED
        };
        out.push(Line::from(vec![
            Span::raw(CONT),
            Span::styled(profile.name.clone(), name_style),
            Span::raw(" ".repeat(gap.max(1))),
            Span::styled(label, detail_style),
        ]));
    }
    out
}

/// 二级菜单：某 provider 实例下可切换的模型 id 列表（输入框上方）。
///
/// `current` 为该实例当前绑定模型，命中则标注 `(当前)`。
pub fn model_list_lines(
    models: &[String],
    current: Option<&str>,
    selected: usize,
    width: u16,
    max_rows: usize,
) -> Vec<Line<'static>> {
    let w = width.max(1) as usize;
    let show = models.len().min(max_rows);
    let start = menu_window_start(models.len(), selected, max_rows);
    let mut out = Vec::with_capacity(show);
    for (slot, model) in models.iter().skip(start).take(show).enumerate() {
        let i = start + slot;
        let active_row = i == selected;
        let is_current = current == Some(model.as_str());
        let name_style = if active_row {
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(55, 55, 95))
                .add_modifier(Modifier::BOLD)
        } else if is_current {
            Style::default().fg(Color::Rgb(140, 200, 140))
        } else {
            UiTheme::ASSISTANT
        };
        let suffix = if is_current { " (当前)" } else { "" };
        let gap = w.saturating_sub(model.width() + suffix.width() + CONT.len() + 1);
        let suffix_style = if active_row {
            Style::default().fg(Color::Rgb(200, 200, 220)).bg(Color::Rgb(55, 55, 95))
        } else {
            UiTheme::MUTED
        };
        out.push(Line::from(vec![
            Span::raw(CONT),
            Span::styled(model.clone(), name_style),
            Span::raw(" ".repeat(gap.max(1))),
            Span::styled(suffix.to_string(), suffix_style),
        ]));
    }
    out
}

#[cfg(test)]
#[path = "../test/unit/model_picker.rs"]
mod tests;
