use hi_core::{t, AgentEvent, DiffLine, Locale, MessageId, SerializableDiffLine};

/// One user message and everything the agent did in response.
///
/// 回合内的思考 / 工具 / diff / 回复按**事件到达顺序**排成 `blocks`，
/// 因此「思考₁ → 工具₁ → 思考₂ → 工具₂ → 回复」这种多轮交错能逐段独立渲染，
/// 而不是把所有思考挤进一个块（旧固定槽位模型会丢失中途思考）。
///
/// Author: gz
#[derive(Clone, Default)]
pub struct Turn {
    pub user: String,
    pub blocks: Vec<Block>,
    pub notices: Vec<Notice>,
}

/// 回合内按顺序排列的一段内容。
///
/// Author: gz
#[derive(Clone)]
pub enum Block {
    Thinking(ThinkingBlock),
    Tool(ToolBlock),
    Diff(DiffBlock),
    Reply(ReplyBlock),
}

/// Author: gz
#[derive(Clone, Default)]
pub struct ReplyBlock {
    pub content: String,
    pub streaming: bool,
    /// 已写入 scrollback 的逻辑行数（按 block 追踪，避免重复 commit）。
    pub lines_committed: usize,
}

/// Author: gz
#[derive(Clone)]
pub struct ToolBlock {
    pub name: String,
    /// 调用参数原文（JSON），用于在行内显示「执行了什么」。
    pub arguments: String,
    pub phase: ToolPhase,
    /// 运行中 stdout/stderr 流式片段（bash 等）。
    pub output: String,
    /// verbose：调用头行 `· name · args` 是否已写入 scrollback（每 block 一次）。
    pub header_committed: bool,
    /// verbose：已写入 scrollback 的 output 逻辑行数（流式追踪，避免重复 commit）。
    pub lines_committed: usize,
}

impl Default for ToolBlock {
    fn default() -> Self {
        Self {
            name: String::new(),
            arguments: String::new(),
            phase: ToolPhase::Running,
            output: String::new(),
            header_committed: false,
            lines_committed: 0,
        }
    }
}

/// Author: gz
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolPhase {
    Running,
    Done(bool),
}

/// Author: gz
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThinkingPhase {
    Streaming,
    Collapsed { expanded: bool },
}

/// Author: gz
#[derive(Clone, Default)]
pub struct ThinkingBlock {
    pub content: String,
    pub phase: ThinkingPhase,
    /// 非 verbose：摘要行 `▸ think · N 字`；verbose：起始头行 `▸ think`。
    /// 两种模式下都表示「该 block 的引导行已写入 scrollback」，每 block 只 commit 一次。
    pub summary_committed: bool,
    /// verbose：已写入 scrollback 的 think 正文逻辑行数（流式追踪）。
    pub lines_committed: usize,
}

/// Author: gz
#[derive(Clone)]
pub struct DiffBlock {
    pub path: String,
    pub lines: Vec<DiffLine>,
}

/// Author: gz
#[derive(Clone)]
pub enum Notice {
    System(String),
    Error(String),
}

impl Default for ThinkingPhase {
    fn default() -> Self {
        Self::Collapsed { expanded: false }
    }
}

impl Turn {
    pub fn apply_localized(&mut self, event: AgentEvent, locale: Locale) {
        match event {
            AgentEvent::ContextCompressed { summary } => {
                let msg = t(
                    locale,
                    MessageId::CompressionDetailSummary,
                    &[summary],
                );
                let dup = self
                    .notices
                    .iter()
                    .any(|n| matches!(n, Notice::System(s) if s == &msg));
                if !dup {
                    self.notices.push(Notice::System(msg));
                }
            }
            AgentEvent::KnotsExtracted { count } => {
                self.notices.push(Notice::System(t(
                    locale,
                    MessageId::MemoryExtractDone,
                    &[count.to_string(), "0".into(), "0".into()],
                )));
            }
            other => self.apply(other),
        }
    }

    pub fn apply(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::AssistantDelta { text } => {
                self.collapse_thinking();
                match self.blocks.last_mut() {
                    Some(Block::Reply(r)) if r.streaming => r.content.push_str(&text),
                    _ => self.blocks.push(Block::Reply(ReplyBlock {
                        content: text,
                        streaming: true,
                        lines_committed: 0,
                    })),
                }
            }
            AgentEvent::ReasoningDelta { text } => {
                self.close_reply();
                match self.blocks.last_mut() {
                    Some(Block::Thinking(t)) if t.phase == ThinkingPhase::Streaming => {
                        t.content.push_str(&text);
                    }
                    _ => self.blocks.push(Block::Thinking(ThinkingBlock {
                        content: text,
                        phase: ThinkingPhase::Streaming,
                        summary_committed: false,
                        lines_committed: 0,
                    })),
                }
            }
            AgentEvent::ToolCallStarted { name, arguments } => {
                self.collapse_thinking();
                self.close_reply();
                if let Some(Block::Tool(t)) = self.blocks.last() {
                    if t.name == name && t.phase == ToolPhase::Running {
                        return;
                    }
                }
                self.blocks.push(Block::Tool(ToolBlock {
                    name,
                    arguments,
                    phase: ToolPhase::Running,
                    ..ToolBlock::default()
                }));
            }
            AgentEvent::ToolOutputDelta { name, text } => {
                if let Some(t) = self.find_running_tool(&name) {
                    t.output.push_str(&text);
                }
            }
            AgentEvent::ToolCallFinished { name, success, .. } => {
                if let Some(t) = self.find_running_tool(&name) {
                    t.phase = ToolPhase::Done(success);
                    return;
                }
                self.blocks.push(Block::Tool(ToolBlock {
                    name,
                    arguments: String::new(),
                    phase: ToolPhase::Done(success),
                    ..ToolBlock::default()
                }));
            }
            AgentEvent::FileDiff { path, lines } => {
                let diff_lines: Vec<DiffLine> =
                    lines.iter().map(SerializableDiffLine::to_diff_line).collect();
                self.blocks.push(Block::Diff(DiffBlock {
                    path,
                    lines: diff_lines,
                }));
            }
            AgentEvent::ApprovalRequired { .. } => {}
            AgentEvent::ContextCompressed { summary } => {
                let msg = t(
                    Locale::Zh,
                    MessageId::CompressionDetailSummary,
                    &[summary],
                );
                let dup = self
                    .notices
                    .iter()
                    .any(|n| matches!(n, Notice::System(s) if s == &msg));
                if !dup {
                    self.notices.push(Notice::System(msg));
                }
            }
            // 记忆注入是后台静默行为，不在每条回复后播报。
            AgentEvent::KnotsInjected { .. } => {}
            AgentEvent::KnotsExtracting => {}
            AgentEvent::KnotsExtracted { count } => {
                self.notices.push(Notice::System(t(
                    Locale::Zh,
                    MessageId::MemoryExtractDone,
                    &[count.to_string(), "0".into(), "0".into()],
                )));
            }
            AgentEvent::Error { message } => {
                self.notices.push(Notice::Error(message));
            }
            AgentEvent::TurnCompleted => self.finalize(),
        }
    }

    pub fn finalize(&mut self) {
        self.collapse_thinking();
        self.close_reply();
    }

    pub fn push_notice(&mut self, notice: Notice) {
        self.notices.push(notice);
    }

    fn collapse_thinking(&mut self) {
        if let Some(Block::Thinking(t)) = self.blocks.last_mut() {
            if t.phase == ThinkingPhase::Streaming {
                t.phase = ThinkingPhase::Collapsed { expanded: false };
            }
        }
    }

    fn close_reply(&mut self) {
        if let Some(Block::Reply(r)) = self.blocks.last_mut() {
            r.streaming = false;
        }
    }

    fn find_running_tool(&mut self, name: &str) -> Option<&mut ToolBlock> {
        self.blocks.iter_mut().rev().find_map(|b| match b {
            Block::Tool(t) if t.name == name && t.phase == ToolPhase::Running => Some(t),
            _ => None,
        })
    }
}

#[cfg(test)]
#[path = "../test/unit/turn.rs"]
mod tests;
