use hi_core::{t, Locale, MessageId, ModelProfile};
use hi_core::parse_session_command;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::menu_scroll::menu_window_start;
use crate::theme::UiTheme;

/// 内置斜杠命令（与 `hi_core::parse_session_command` 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: MessageId,
}

const COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "/model",
        description: MessageId::TuiSlashModelDesc,
    },
    SlashCommand {
        name: "/reset",
        description: MessageId::TuiSlashResetDesc,
    },
    SlashCommand {
        name: "/compact",
        description: MessageId::TuiSlashCompactDesc,
    },
];

/// 首行以 `/` 开头且光标在斜杠 token 内时返回 token 文本（含 `/`）。
pub fn slash_token(text: &str, cursor: usize) -> Option<&str> {
    let first_line = text.split('\n').next().unwrap_or("");
    if !first_line.starts_with('/') {
        return None;
    }
    let token_end = first_line.find(' ').unwrap_or(first_line.len());
    if cursor > token_end {
        return None;
    }
    Some(&first_line[..token_end])
}

/// TUI 专属：`/model` 下级菜单；返回实例名前缀过滤（空串 = 显示全部）。
pub fn model_submenu_filter(text: &str, cursor: usize) -> Option<&str> {
    let first_line = text.split('\n').next().unwrap_or("");
    let first_line_end = text.find('\n').unwrap_or(text.len());
    if cursor > first_line_end {
        return None;
    }
    if first_line == "/model" {
        return Some("");
    }
    if let Some(rest) = first_line.strip_prefix("/model ") {
        return Some(rest.trim());
    }
    None
}

pub fn filter_model_profiles<'a>(
    profiles: &'a [ModelProfile],
    filter: &str,
) -> Vec<&'a ModelProfile> {
    if filter.is_empty() {
        return profiles.iter().collect();
    }
    let q = filter.to_ascii_lowercase();
    profiles
        .iter()
        .filter(|p| {
            p.name.to_ascii_lowercase().contains(&q)
                || p.model.to_ascii_lowercase().contains(&q)
                || p.adapter.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

/// 顶层斜杠菜单（不含 `/model` 下级）。
pub fn top_level_slash_visible(text: &str, cursor: usize, agent_busy: bool) -> bool {
    if agent_busy || model_submenu_filter(text, cursor).is_some() {
        return false;
    }
    let Some(token) = slash_token(text, cursor) else {
        return false;
    };
    parse_session_command(token).is_none()
}

/// 按前缀过滤（忽略大小写）；`query` 为 `/` 后的部分，空则返回全部。
pub fn filter_commands(query: &str) -> Vec<&'static SlashCommand> {
    let q = query.trim();
    if q.is_empty() {
        return COMMANDS.iter().collect();
    }
    let ql = q.to_ascii_lowercase();
    COMMANDS
        .iter()
        .filter(|cmd| {
            cmd.name[1..].to_ascii_lowercase().starts_with(&ql)
                || (cmd.name == "/reset" && "clear".starts_with(&ql))
        })
        .collect()
}

/// 菜单是否应显示（完整斜杠命令已填入时关闭，便于再次 Enter 发送）。
pub fn menu_visible(text: &str, cursor: usize, agent_busy: bool) -> bool {
    top_level_slash_visible(text, cursor, agent_busy)
}

pub fn clamp_selection(selected: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        selected.min(count - 1)
    }
}

/// 渲染斜杠菜单（最多 `max_rows` 行，紧贴输入框上方）。
pub fn menu_lines(
    matches: &[&SlashCommand],
    selected: usize,
    width: u16,
    max_rows: usize,
    locale: Locale,
) -> Vec<Line<'static>> {
    let w = width.max(1) as usize;
    let show = matches.len().min(max_rows);
    let start = menu_window_start(matches.len(), selected, max_rows);
    let mut out = Vec::with_capacity(show);
    for (slot, cmd) in matches.iter().skip(start).take(show).enumerate() {
        let i = start + slot;
        let active = i == selected;
        let name_style = if active {
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(55, 55, 95))
                .add_modifier(Modifier::BOLD)
        } else {
            UiTheme::ASSISTANT
        };
        let desc_style = if active {
            Style::default().fg(Color::Rgb(200, 200, 220)).bg(Color::Rgb(55, 55, 95))
        } else {
            UiTheme::MUTED
        };
        let name = cmd.name;
        let desc = t(locale, cmd.description, &[]);
        let gap = w.saturating_sub(name.width() + desc.width() + 2);
        out.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(name.to_string(), name_style),
            Span::raw(" ".repeat(gap.max(1))),
            Span::styled(desc, desc_style),
        ]));
    }
    out
}

#[cfg(test)]
#[path = "../test/unit/slash.rs"]
mod tests;
