use std::time::Instant;

use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use hi_core::{t, Locale, MessageId};

use crate::render::short_path;
use crate::theme::{full_width_line, UiTheme};

/// Side-by-side ASCII "hi" (5 rows).
const HI_ASCII: &[&str] = &[
    " _   _   _ ",
    "| | | | (_)",
    "| |_| |  | ",
    "|  _  |  | ",
    "|_| |_| _| ",
];

/// 启动横幅：ASCII "hi" + 模型/目录/会话元信息，仅在启动时写入一次（随后随历史滚动）。
///
/// Author: gz
pub fn banner_lines(model: &str, workdir: &str, session_id: &str) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = HI_ASCII
        .iter()
        .map(|row| Line::from(Span::styled((*row).to_string(), UiTheme::BANNER)))
        .collect();
    out.push(Line::from(""));
    out.push(Line::from(vec![
        Span::styled("model     ", UiTheme::META_LABEL),
        Span::styled(model.to_string(), UiTheme::META_VALUE),
    ]));
    out.push(Line::from(vec![
        Span::styled("directory ", UiTheme::META_LABEL),
        Span::styled(short_path(workdir), UiTheme::META_VALUE),
    ]));
    out.push(Line::from(vec![
        Span::styled("session   ", UiTheme::META_LABEL),
        Span::styled(session_id.to_string(), UiTheme::META_VALUE),
    ]));
    out.push(Line::from(""));
    out
}

/// 输入区最少显示行数（空内容时 `›` + 空续行）。
pub const INPUT_ROWS_MIN: usize = 2;
/// 输入区最多扩至行数，避免占满屏幕。
pub const INPUT_ROWS_MAX: usize = 8;

/// 根据内容计算输入区应占行数（含最少/上限）。
///
/// Author: gz
pub fn input_row_count(text: &str, cursor: usize, width: u16) -> usize {
    let lines = input_lines(text, cursor, width).len();
    lines.clamp(INPUT_ROWS_MIN, INPUT_ROWS_MAX)
}

/// 输入视口渲染：至多 `visible_rows` 行，不足最少行时补空行；超出时随光标滚动。
///
/// Author: gz
pub fn input_viewport_lines(
    text: &str,
    cursor: usize,
    visible_rows: usize,
    width: u16,
) -> Vec<Line<'static>> {
    let visible_rows = visible_rows.clamp(INPUT_ROWS_MIN, INPUT_ROWS_MAX);
    let all = input_lines(text, cursor, width);
    let window = if all.len() <= visible_rows {
        all
    } else {
        let cur_line = cursor_line_index(text, cursor);
        let start = cur_line
            .saturating_sub(visible_rows - 1)
            .min(all.len().saturating_sub(visible_rows));
        all[start..start + visible_rows].to_vec()
    };
    pad_input_lines(window, visible_rows, width)
}

fn pad_input_lines(
    mut lines: Vec<Line<'static>>,
    max_rows: usize,
    width: u16,
) -> Vec<Line<'static>> {
    let line_width = width.max(1) as usize;
    while lines.len() < max_rows {
        lines.push(full_width_line("  ", "", line_width, UiTheme::USER_BG));
    }
    lines
}

fn cursor_line_index(text: &str, cursor: usize) -> usize {
    let cursor = snap_cursor(text, cursor);
    text[..cursor].matches('\n').count()
}

/// 输入框各行（带 `›` 提示与光标），供底部内联视口渲染。
///
/// Author: gz
pub fn input_lines(text: &str, cursor: usize, width: u16) -> Vec<Line<'static>> {
    let line_width = width.max(1) as usize;
    let cursor = snap_cursor(text, cursor);
    if text.is_empty() {
        return vec![input_row_with_cursor("> ", "", "", line_width)];
    }

    let mut out = Vec::new();
    let mut line_start = 0usize;
    let mut line_no = 0usize;

    loop {
        let line_end = text[line_start..]
            .find('\n')
            .map(|i| line_start + i)
            .unwrap_or(text.len());
        let mark = if line_no == 0 { "> " } else { "  " };

        if cursor >= line_start && cursor <= line_end {
            let local = cursor - line_start;
            let left = &text[line_start..line_start + local];
            let right = &text[line_start + local..line_end];
            out.push(input_row_with_cursor(mark, left, right, line_width));
        } else {
            out.push(full_width_line(
                mark,
                &text[line_start..line_end],
                line_width,
                UiTheme::USER_BG,
            ));
        }

        if line_end >= text.len() {
            break;
        }
        line_start = line_end + 1;
        line_no += 1;
    }
    out
}

fn input_row_with_cursor(mark: &str, left: &str, right: &str, line_width: usize) -> Line<'static> {
    let used = mark.width() + left.width() + 1 + right.width();
    let pad = line_width.saturating_sub(used);
    let mut spans = vec![
        Span::styled(mark.to_string(), UiTheme::USER_BG),
        Span::styled(left.to_string(), UiTheme::USER_BG),
        Span::styled("▌", UiTheme::INPUT_CURSOR),
        Span::styled(right.to_string(), UiTheme::USER_BG),
    ];
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), UiTheme::USER_BG));
    }
    Line::from(spans)
}

fn snap_cursor(text: &str, cursor: usize) -> usize {
    if cursor >= text.len() {
        return text.len();
    }
    if text.is_char_boundary(cursor) {
        return cursor;
    }
    text[..cursor]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// 内联审批提示（命令审批挤入底部视口两行）。
///
/// Author: gz
pub fn approval_lines(command: &str, locale: Locale) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", t(locale, MessageId::TuiApprovalTitle, &[])),
                UiTheme::WARN.add_modifier(Modifier::BOLD),
            ),
            Span::styled(command.to_string(), UiTheme::META_VALUE),
        ]),
        Line::from(vec![
            Span::styled(
                t(locale, MessageId::TuiApprovalApprove, &[]),
                UiTheme::WARN.add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", UiTheme::MUTED),
            Span::styled(
                t(locale, MessageId::TuiApprovalReject, &[]),
                UiTheme::WARN.add_modifier(Modifier::BOLD),
            ),
        ]),
    ]
}

/// Author: gz
pub fn turn_elapsed(since: Option<Instant>) -> Option<std::time::Duration> {
    since.map(|t| t.elapsed())
}

#[cfg(test)]
#[path = "../test/unit/widgets.rs"]
mod tests;
