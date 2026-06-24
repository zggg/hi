use crossterm::event::{KeyCode, KeyModifiers};
use std::time::{Duration, Instant};

/// 粘贴结束后短暂忽略 Enter，避免终端在 Bracketed Paste 后额外投递的回车误触发发送。
const PASTE_SUBMIT_GUARD: Duration = Duration::from_millis(200);

/// Multi-line input：Enter 发送；Ctrl+J 换行（全终端可靠），Shift+Enter 换行（需 CSI-u）。
///
/// Author: gz
#[derive(Default)]
pub struct InputArea {
    text: String,
    /// Byte index into `text`, always on a UTF-8 char boundary.
    cursor: usize,
    suppress_submit_until: Option<Instant>,
    /// Kitty / CSI-u 键盘协议下 Shift 修饰键状态（Enter 本身可能不带 SHIFT 位）。
    shift_held: bool,
}

/// Author: gz
#[derive(PartialEq, Eq, Debug)]
pub enum InputAction {
    None,
    Submit(String),
}

impl InputArea {
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn replace_slash_token(&mut self, replacement: &str) {
        let first_nl = self.text.find('\n').unwrap_or(self.text.len());
        let rest = if first_nl < self.text.len() {
            self.text[first_nl..].to_string()
        } else {
            String::new()
        };
        self.text = format!("{replacement}{rest}");
        self.cursor = replacement.len();
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn set_shift_held(&mut self, held: bool) {
        self.shift_held = held;
    }

    pub fn shift_held(&self) -> bool {
        self.shift_held
    }

    pub fn handle_paste(&mut self, pasted: &str) -> InputAction {
        let normalized = pasted.replace("\r\n", "\n").replace('\r', "\n");
        self.insert_str(&normalized);
        self.suppress_submit_until = Some(Instant::now() + PASTE_SUBMIT_GUARD);
        InputAction::None
    }

    pub fn handle(&mut self, code: KeyCode, modifiers: KeyModifiers) -> InputAction {
        match code {
            // Ctrl+J 在裸模式下被 crossterm 解为 Char('j')+CONTROL（字节 0x0A），
            // 不依赖 CSI-u、不与中文 IME 冲突，是全终端可靠的换行键。
            KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.insert_char('\n');
                InputAction::None
            }
            KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) || self.shift_held => {
                self.insert_char('\n');
                InputAction::None
            }
            KeyCode::Enter => {
                if self.submit_suppressed() {
                    return InputAction::None;
                }
                let trimmed = self.text.trim();
                if trimmed.is_empty() {
                    return InputAction::None;
                }
                let out = trimmed.to_string();
                self.text.clear();
                self.cursor = 0;
                InputAction::Submit(out)
            }
            KeyCode::Backspace => {
                self.backspace();
                InputAction::None
            }
            KeyCode::Delete => {
                self.delete_forward();
                InputAction::None
            }
            KeyCode::Left => {
                self.cursor = prev_char_boundary(&self.text, self.cursor);
                InputAction::None
            }
            KeyCode::Right => {
                self.cursor = next_char_boundary(&self.text, self.cursor);
                InputAction::None
            }
            KeyCode::Up => {
                self.cursor = cursor_up(&self.text, self.cursor);
                InputAction::None
            }
            KeyCode::Down => {
                self.cursor = cursor_down(&self.text, self.cursor);
                InputAction::None
            }
            KeyCode::Home => {
                self.cursor = line_start(&self.text, self.cursor);
                InputAction::None
            }
            KeyCode::End => {
                self.cursor = line_end(&self.text, self.cursor);
                InputAction::None
            }
            KeyCode::Esc => {
                self.text.clear();
                self.cursor = 0;
                InputAction::None
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
                InputAction::None
            }
            _ => InputAction::None,
        }
    }

    fn insert_char(&mut self, ch: char) {
        let pos = snap_boundary(&self.text, self.cursor);
        self.text.insert(pos, ch);
        self.cursor = pos + ch.len_utf8();
    }

    fn insert_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let pos = snap_boundary(&self.text, self.cursor);
        self.text.insert_str(pos, s);
        self.cursor = pos + s.len();
    }

    fn backspace(&mut self) {
        let pos = snap_boundary(&self.text, self.cursor);
        if pos == 0 {
            return;
        }
        let prev = prev_char_boundary(&self.text, pos);
        self.text.drain(prev..pos);
        self.cursor = prev;
    }

    fn delete_forward(&mut self) {
        let pos = snap_boundary(&self.text, self.cursor);
        if pos >= self.text.len() {
            return;
        }
        let next = next_char_boundary(&self.text, pos);
        self.text.drain(pos..next);
    }

    fn submit_suppressed(&self) -> bool {
        self.suppress_submit_until
            .is_some_and(|until| Instant::now() < until)
    }
}

fn snap_boundary(text: &str, pos: usize) -> usize {
    if pos >= text.len() {
        return text.len();
    }
    if text.is_char_boundary(pos) {
        return pos;
    }
    text[..pos]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn prev_char_boundary(text: &str, pos: usize) -> usize {
    let pos = snap_boundary(text, pos);
    if pos == 0 {
        return 0;
    }
    text[..pos]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn next_char_boundary(text: &str, pos: usize) -> usize {
    let pos = snap_boundary(text, pos);
    if pos >= text.len() {
        return text.len();
    }
    text[pos..]
        .char_indices()
        .nth(1)
        .map(|(i, _)| pos + i)
        .unwrap_or(text.len())
}

fn line_start(text: &str, pos: usize) -> usize {
    let pos = snap_boundary(text, pos);
    text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

fn line_end(text: &str, pos: usize) -> usize {
    let pos = snap_boundary(text, pos);
    text[pos..]
        .find('\n')
        .map(|i| pos + i)
        .unwrap_or(text.len())
}

fn cursor_up(text: &str, cursor: usize) -> usize {
    let pos = snap_boundary(text, cursor);
    let start = line_start(text, pos);
    if start == 0 {
        return pos;
    }
    let col = pos - start;
    let prev_start = line_start(text, start - 1);
    let prev_end = start - 1;
    let target = prev_start + col.min(prev_end.saturating_sub(prev_start));
    snap_boundary(text, target)
}

fn cursor_down(text: &str, cursor: usize) -> usize {
    let pos = snap_boundary(text, cursor);
    let start = line_start(text, pos);
    let end = line_end(text, pos);
    if end >= text.len() {
        return pos;
    }
    let col = pos - start;
    let next_start = end + 1;
    let next_end = line_end(text, next_start);
    let target = next_start + col.min(next_end.saturating_sub(next_start));
    snap_boundary(text, target)
}

#[cfg(test)]
#[path = "../test/unit/input.rs"]
mod tests;
