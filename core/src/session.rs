use serde::{Deserialize, Serialize};

/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

/// Built-in session commands shared by TUI / chat REPL（不含 TUI 专属 `/model`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCommand {
    Reset,
    Compact,
}

pub fn parse_session_command(input: &str) -> Option<SessionCommand> {
    match input.trim() {
        "/reset" | "/clear" => Some(SessionCommand::Reset),
        "/compact" => Some(SessionCommand::Compact),
        _ => None,
    }
}

/// Handle for an active or persisted conversation.
///
/// Author: gz
#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub user_id: UserId,
    pub session_id: SessionId,
}
