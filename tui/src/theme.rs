use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// Terminal palette — Codex-inspired, minimal chrome.
///
/// Author: gz
pub struct UiTheme;

impl UiTheme {
    pub const BANNER: Style = Style::new()
        .fg(Color::Rgb(160, 55, 55))
        .add_modifier(Modifier::BOLD);
    pub const META_LABEL: Style = Style::new().fg(Color::DarkGray);
    pub const META_VALUE: Style = Style::new().fg(Color::Gray);
    pub const MUTED: Style = Style::new().fg(Color::DarkGray);

    pub const STATUS_BUSY: Style = Style::new().fg(Color::Yellow);
    pub const WARN: Style = Style::new().fg(Color::Yellow);

    /// 用户输入：白字 + 深灰底（整行铺底，与模型回复的默认黑底区分）。
    pub const USER_BG: Style = Style::new()
        .fg(Color::White)
        .bg(Color::Rgb(38, 38, 38));
    /// 模型回复：略弱于用户的浅灰字，无背景。
    pub const ASSISTANT: Style = Style::new().fg(Color::Rgb(210, 210, 210));
    pub fn thinking() -> Style {
        Style::new()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC)
    }
    pub const TOOL: Style = Style::new().fg(Color::Rgb(180, 170, 140));
    pub const DIFF: Style = Style::new().fg(Color::Rgb(120, 150, 200));
    pub const ERROR: Style = Style::new().fg(Color::Red);

    pub const INPUT_CURSOR: Style = Style::new()
        .fg(Color::White)
        .bg(Color::Rgb(38, 38, 38))
        .add_modifier(Modifier::SLOW_BLINK);

    pub fn busy_frame(tick: u8) -> &'static str {
        const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        FRAMES[tick as usize % FRAMES.len()]
    }
}

/// 按终端列宽铺满背景的行（mark + text + 空格填充）。
///
/// Author: gz
pub fn full_width_line(mark: &str, text: &str, line_width: usize, style: Style) -> Line<'static> {
    let used = mark.width() + text.width();
    let pad = line_width.saturating_sub(used);
    let mut spans = vec![
        Span::styled(mark.to_string(), style),
        Span::styled(text.to_string(), style),
    ];
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), style));
    }
    Line::from(spans)
}
