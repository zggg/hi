use std::path::Path;

use hi_core::{DiffKind, DiffLine};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::theme::{full_width_line, UiTheme};
use crate::turn::{Notice, ToolBlock, ToolPhase, Turn};

const DIFF_MAX: usize = 24;
const USER_MARK: &str = "> ";
const CONT_MARK: &str = "  ";
const BULLET: &str = "● ";
/// verbose 展开时 think / 工具正文的缩进（对齐到引导符之后）。
const VERBOSE_INDENT: &str = "    ";

/// 底部忙碌指示（旋转动画 + 提示 + 已耗时）。
///
/// Author: gz
pub fn busy_line(hint: &str, tick: u8, elapsed_secs: f32) -> Line<'static> {
    Line::from(vec![
        Span::styled(UiTheme::busy_frame(tick).to_string(), UiTheme::STATUS_BUSY),
        Span::raw(" "),
        Span::styled(hint.to_string(), UiTheme::STATUS_BUSY),
        Span::styled(format!(" · {elapsed_secs:.0}s"), UiTheme::MUTED),
    ])
}

/// 用户消息行（提交时写入终端原生历史）。
///
/// Author: gz
pub fn render_user(turn: &Turn, width: u16) -> Vec<Line<'static>> {
    let w = width.max(1) as usize;
    let body = w.saturating_sub(USER_MARK.len()).max(1);
    user_lines(&turn.user, w, body)
}

/// 思考摘要行（一行，思考结束后写入 scrollback）。
///
/// Author: gz
pub fn thinking_summary(content: &str) -> Line<'static> {
    let n = content.chars().count();
    cont_line(Line::from(vec![
        Span::styled("▸ think", UiTheme::thinking()),
        Span::styled(format!(" · {n} 字"), UiTheme::MUTED),
    ]))
}

/// 思考流式预览行（活动区，取尾部一行）。
///
/// Author: gz
pub fn thinking_preview(text: &str, width: u16) -> Line<'static> {
    let body = (width.max(1) as usize).saturating_sub(USER_MARK.len()).max(1);
    let tail = wrap_text(text, body).into_iter().last().unwrap_or_default();
    Line::from(vec![
        Span::styled("▸ ", UiTheme::thinking()),
        Span::styled(tail, UiTheme::thinking().add_modifier(Modifier::ITALIC)),
    ])
}

/// verbose：think 全文起始头行 `▸ think`（写入 scrollback，正文逐行跟随其后）。
///
/// Author: gz
pub fn thinking_header() -> Line<'static> {
    cont_line(Line::from(Span::styled("▸ think", UiTheme::thinking())))
}

/// verbose：think 正文逻辑行（暗色斜体，按宽度换行；全文不截断）。
///
/// Author: gz
pub fn thinking_body_lines(text: &str, width: u16) -> Vec<Line<'static>> {
    let body = (width.max(1) as usize)
        .saturating_sub(VERBOSE_INDENT.len())
        .max(1);
    let style = UiTheme::thinking().add_modifier(Modifier::ITALIC);
    wrap_text(text, body)
        .into_iter()
        .map(|l| Line::from(vec![Span::raw(VERBOSE_INDENT), Span::styled(l, style)]))
        .collect()
}

/// 单个工具调用行（完成时实时写入历史）：`✓ bash · ls ~`，
/// 后半段展示「执行了什么」（命令 / 路径 / 模式等关键参数）。
///
/// Author: gz
pub fn tool_line(tool: &ToolBlock, width: u16) -> Line<'static> {
    let (mark, color) = match tool.phase {
        ToolPhase::Running => ("·", UiTheme::MUTED),
        ToolPhase::Done(true) => ("✓", Style::default().fg(Color::Green)),
        ToolPhase::Done(false) => ("✗", Style::default().fg(Color::Red)),
    };
    tool_line_with_mark(mark, color, tool, width)
}

/// verbose：工具调用头行（始终用中性 `·` 标记，最终成败由后续状态行给出），
/// 这样流式 output 可以先于「完成态」写入 scrollback 而不丢失结果标记。
///
/// Author: gz
pub fn tool_header_line(tool: &ToolBlock, width: u16) -> Line<'static> {
    tool_line_with_mark("·", UiTheme::MUTED, tool, width)
}

/// verbose：工具完成状态行（`✓` / `✗`，紧随 output 之后写入 scrollback）。
///
/// Author: gz
pub fn tool_status_line(success: bool) -> Line<'static> {
    let (mark, color) = if success {
        ("✓", Style::default().fg(Color::Green))
    } else {
        ("✗", Style::default().fg(Color::Red))
    };
    Line::from(vec![
        Span::raw(VERBOSE_INDENT),
        Span::styled(mark.to_string(), color),
    ])
}

/// verbose：工具 output 正文逻辑行（暗色，按宽度换行；全文不截断）。
///
/// Author: gz
pub fn tool_body_lines(text: &str, width: u16) -> Vec<Line<'static>> {
    let body = (width.max(1) as usize)
        .saturating_sub(VERBOSE_INDENT.len())
        .max(1);
    wrap_text(text, body)
        .into_iter()
        .map(|l| Line::from(vec![Span::raw(VERBOSE_INDENT), Span::styled(l, UiTheme::MUTED)]))
        .collect()
}

fn tool_line_with_mark(
    mark: &str,
    color: Style,
    tool: &ToolBlock,
    width: u16,
) -> Line<'static> {
    let mut spans = vec![
        Span::styled(format!("{mark} "), color),
        Span::styled(tool.name.clone(), UiTheme::TOOL),
    ];
    let summary = tool_arg_summary(&tool.arguments);
    if !summary.is_empty() {
        let budget = (width.max(1) as usize)
            .saturating_sub(USER_MARK.len() + tool.name.width() + 5)
            .max(8);
        spans.push(Span::styled(
            format!(" · {}", truncate_cols(&summary, budget)),
            UiTheme::MUTED,
        ));
    }
    cont_line(Line::from(spans))
}

/// 运行中工具的流式预览（活动区最多 2 行：调用摘要 + 输出尾部）。
///
/// Author: gz
pub fn tool_preview_lines(tool: &ToolBlock, width: u16, max_lines: usize) -> Vec<Line<'static>> {
    debug_assert_eq!(tool.phase, ToolPhase::Running);
    let header = tool_line(tool, width);
    if max_lines <= 1 {
        return vec![header];
    }
    let body = (width.max(1) as usize).saturating_sub(USER_MARK.len()).max(1);
    let second = if tool.output.trim().is_empty() {
        cont_line(Line::from(Span::styled("…", UiTheme::MUTED)))
    } else {
        let tail = wrap_text(&tool.output, body)
            .into_iter()
            .last()
            .unwrap_or_default();
        cont_line(Line::from(Span::styled(tail, UiTheme::MUTED)))
    };
    vec![header, second]
}

/// 排队提示行（写入 scrollback）。
///
/// Author: gz
pub fn queued_for_next_turn_line(text: &str) -> Line<'static> {
    cont_line(Line::from(vec![
        Span::styled("Queued for the next turn: ", UiTheme::MUTED),
        Span::styled(text.to_string(), UiTheme::MUTED),
    ]))
}

/// 从工具参数 JSON 中提取最能说明「执行了什么」的单行摘要。
fn tool_arg_summary(arguments: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(arguments) else {
        return String::new();
    };
    let Some(obj) = value.as_object() else {
        return String::new();
    };
    for key in [
        "command", "cmd", "script", "path", "file", "filename", "pattern", "query", "url",
    ] {
        if let Some(s) = obj.get(key).and_then(|v| v.as_str()) {
            return s.split('\n').collect::<Vec<_>>().join(" ⏎ ");
        }
    }
    obj.values()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 按显示列宽截断字符串，超出加省略号。
fn truncate_cols(text: &str, max: usize) -> String {
    if text.width() <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > max.saturating_sub(1) {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.push('…');
    out
}

/// diff 块的行集合。
///
/// Author: gz
pub fn render_diff(path: &str, lines: &[DiffLine], width: u16) -> Vec<Line<'static>> {
    let body = (width.max(1) as usize).saturating_sub(USER_MARK.len()).max(1);
    diff_lines(path, lines, body)
}

/// 回复中一条完整逻辑行渲染成显示行（`first` 决定首段用 `•` 项目符）。
///
/// Author: gz
pub fn reply_logical_line(text: &str, width: u16, first: bool) -> Vec<Line<'static>> {
    let body = (width.max(1) as usize).saturating_sub(USER_MARK.len()).max(1);
    let mut out = Vec::new();
    for (i, line) in wrap_text(text, body).into_iter().enumerate() {
        let mark = if first && i == 0 { BULLET } else { CONT_MARK };
        out.push(bullet_row_prefixed(mark, &line, UiTheme::ASSISTANT));
    }
    out
}

/// 系统/错误通知行（宽度不足时自动换行）。
///
/// Author: gz
pub fn render_notice(notice: &Notice, width: u16) -> Vec<Line<'static>> {
    let line_width = width.max(1) as usize;
    match notice {
        Notice::System(text) => notice_lines(text, line_width, "• ", UiTheme::MUTED),
        Notice::Error(text) => notice_lines(text, line_width, "⚠ ", UiTheme::ERROR),
    }
}

/// 续行缩进，与首行 `  ⚠ ` / `  • ` 后的正文对齐。
const NOTICE_CONT: &str = "    ";

fn notice_lines(text: &str, line_width: usize, badge: &str, style: Style) -> Vec<Line<'static>> {
    let first_mark = format!("{CONT_MARK}{badge}");
    let first_body = line_width.saturating_sub(first_mark.width()).max(1);
    let cont_body = line_width.saturating_sub(NOTICE_CONT.len()).max(1);
    let wrapped = wrap_variable_width(text, first_body, cont_body);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                Line::from(vec![
                    Span::styled(first_mark.clone(), style),
                    Span::styled(line, style),
                ])
            } else {
                Line::from(vec![
                    Span::styled(NOTICE_CONT.to_string(), style),
                    Span::styled(line, style),
                ])
            }
        })
        .collect()
}

/// 首行与续行可用宽度不同时的分段换行。
fn wrap_variable_width(text: &str, first_width: usize, rest_width: usize) -> Vec<String> {
    let first_pass = wrap_text(text, first_width);
    if first_pass.len() <= 1 {
        return first_pass;
    }
    let mut out = vec![first_pass[0].clone()];
    let tail = first_pass[1..].join(" ");
    out.extend(wrap_text(&tail, rest_width));
    out
}

fn user_lines(text: &str, line_width: usize, body_width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    if text.trim().is_empty() {
        out.push(user_row("…", line_width, UiTheme::USER_BG.add_modifier(Modifier::ITALIC)));
        return out;
    }
    for (i, line) in wrap_text(text, body_width).into_iter().enumerate() {
        let mark = if i == 0 { USER_MARK } else { CONT_MARK };
        out.push(user_row_prefixed(mark, &line, line_width));
    }
    out
}

fn user_row_prefixed(mark: &str, text: &str, line_width: usize) -> Line<'static> {
    full_width_line(mark, text, line_width, UiTheme::USER_BG)
}

fn user_row(text: &str, line_width: usize, style: Style) -> Line<'static> {
    full_width_line(USER_MARK, text, line_width, style)
}

fn bullet_row_prefixed(mark: &str, text: &str, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(mark.to_string(), UiTheme::ASSISTANT),
        Span::styled(text.to_string(), style),
    ])
}

fn diff_lines(path: &str, lines: &[DiffLine], width: usize) -> Vec<Line<'static>> {
    let mut out = vec![cont_line(Line::from(Span::styled(
        format!("± {path}"),
        UiTheme::DIFF.add_modifier(Modifier::BOLD),
    )))];
    let show = lines.len().min(DIFF_MAX);
    for line in &lines[..show] {
        let (pfx, style) = match line.kind {
            DiffKind::Remove => ("-", Style::default().fg(Color::Red)),
            DiffKind::Add => ("+", Style::default().fg(Color::Green)),
            DiffKind::Context => (" ", UiTheme::MUTED),
        };
        for wrapped in wrap_text(&format!("{pfx} {}", line.text), width.saturating_sub(2)) {
            out.push(cont_span(&wrapped, style));
        }
    }
    if lines.len() > DIFF_MAX {
        out.push(cont_span(
            &format!("… +{} 行", lines.len() - DIFF_MAX),
            UiTheme::MUTED,
        ));
    }
    out
}

fn cont_line(mut line: Line<'static>) -> Line<'static> {
    let mut spans = vec![Span::raw(CONT_MARK)];
    spans.append(&mut line.spans);
    Line::from(spans)
}

fn cont_span(text: &str, style: Style) -> Line<'static> {
    cont_line(Line::from(Span::styled(text.to_string(), style)))
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    for para in text.split('\n') {
        out.extend(wrap_para(para, width));
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn wrap_para(text: &str, width: usize) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for word in trimmed.split_whitespace() {
        let ww = word.width();
        if ww > width {
            if !cur.is_empty() {
                lines.push(cur);
                cur = String::new();
                cur_w = 0;
            }
            lines.extend(hard_break(word, width));
            continue;
        }
        if cur_w == 0 {
            cur = word.to_string();
            cur_w = ww;
        } else if cur_w + 1 + ww <= width {
            cur.push(' ');
            cur.push_str(word);
            cur_w += 1 + ww;
        } else {
            lines.push(cur);
            cur = word.to_string();
            cur_w = ww;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn hard_break(word: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut buf = String::new();
    let mut w = 0usize;
    for ch in word.chars() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > width && !buf.is_empty() {
            lines.push(buf);
            buf = String::new();
            w = 0;
        }
        buf.push(ch);
        w += cw;
    }
    if !buf.is_empty() {
        lines.push(buf);
    }
    lines
}

/// Shorten paths for the status bar.
///
/// Author: gz
pub fn short_path(path: &str) -> String {
    let home = std::env::var("HOME").ok();
    if let Some(h) = home {
        let prefix = format!("{h}/");
        if let Some(rest) = path.strip_prefix(&prefix) {
            return format!("~/{rest}");
        }
    }
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| format!("…/{s}"))
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
#[path = "../test/unit/render.rs"]
mod tests;
