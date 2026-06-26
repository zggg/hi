use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode,
    DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::cursor::{MoveUp, MoveTo};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::ExecutableCommand;
use hi_core::{
    parse_session_command, t, AgentEvent, AgentSession, Locale, MessageId, ModelControl,
    SessionCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::approval::{ApprovalState, SharedApproval};
use crate::input::{InputAction, InputArea};
use crate::model_picker::{model_list_lines, model_picker_lines};
use crate::render::{
    busy_line, queued_for_next_turn_line, render_diff, render_notice, render_user,
    reply_logical_line, thinking_body_lines, thinking_header, thinking_preview, thinking_summary,
    tool_body_lines, tool_header_line, tool_line, tool_preview_lines, tool_status_line,
};
use crate::slash::{self, SlashCommand};
use crate::turn::{Block, Notice, ThinkingPhase, ToolPhase, Turn};
use crate::widgets::{
    approval_lines, banner_lines, input_row_count, input_viewport_lines, turn_elapsed,
    INPUT_ROWS_MIN,
};

const STREAM_TICK_MS: u64 = 16;
const KEY_POLL_MS: u64 = 50;

/// `/model` 二级菜单：一级选 provider 实例后进入，拉取并选择该实例可切换的模型。
#[derive(Debug, Clone)]
enum ModelStage {
    /// 正在为 `provider` 拉取可用模型列表。
    Loading { provider: String },
    /// 拉取完成，从 `models` 中选择目标模型。
    Pick {
        provider: String,
        models: Vec<String>,
        sel: usize,
    },
}
const STATUS_ROWS: u16 = 1;
/// 固定在输入区上方的 LLM 活动预览行（thinking / tool / 回复尾行）。
const ACTIVITY_ROWS: usize = 4;

type Term = Terminal<CrosstermBackend<Stdout>>;

/// 终端内联 TUI：完成的对话写入终端原生 scrollback（鼠标/滚动条原生可用），
/// 底部内联视口：活动预览 ×2 + 输入（默认 2 行，可增高）+ 状态。
///
/// Author: gz
pub struct TuiApp {
    session: Arc<Mutex<Box<dyn AgentSession>>>,
    approval: SharedApproval,
    model: String,
    workdir: String,
    session_id: String,
    turns: Vec<Turn>,
    input: InputArea,
    running: bool,
    /// 忙碌时提交的消息，当前回合结束后按 FIFO 处理。
    message_queue: Vec<String>,
    /// 待写入 scrollback 的排队提示（与 `message_queue` 同步追加）。
    pending_queue_notices: Vec<String>,
    event_rx: Option<mpsc::UnboundedReceiver<AgentEvent>>,
    turn_task: Option<JoinHandle<hi_core::Result<Vec<AgentEvent>>>>,
    quit: bool,
    turn_started: Option<std::time::Instant>,
    busy_hint: String,
    anim_tick: u8,
    dirty: bool,
    /// 已完整写入历史的回合数；其余游标针对正在写入的当前回合。
    committed_turns: usize,
    cur_user: bool,
    /// 当前回合已提交到历史缓冲的 block 数。
    cur_block: usize,
    cur_notices: usize,
    /// 当前内联视口总行数（输入 + 状态），随输入扩缩。
    viewport_rows: u16,
    /// 斜杠命令菜单高亮项（`/reset` 等）。
    slash_sel: usize,
    /// `/model` 一级（provider 实例）菜单高亮项。
    model_submenu_sel: usize,
    /// `/model` 二级（模型）菜单状态；`None` 表示未进入二级。
    model_stage: Option<ModelStage>,
    /// 二级菜单的异步模型拉取任务。
    model_fetch_task: Option<JoinHandle<hi_core::Result<Vec<String>>>>,
    model_control: Arc<dyn ModelControl>,
    locale: Locale,
    /// 详细模式：开启后 think 与工具 output 全文流式写入 scrollback（`/verbose` 或 `-v`）。
    verbose: bool,
}

impl TuiApp {
    pub fn new(
        session: Box<dyn AgentSession>,
        model: String,
        workdir: String,
        session_id: String,
        model_control: Arc<dyn ModelControl>,
        locale: Locale,
        verbose: bool,
    ) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
            approval: SharedApproval::new(),
            model,
            workdir,
            session_id,
            model_control,
            turns: Vec::new(),
            input: InputArea::default(),
            running: false,
            message_queue: Vec::new(),
            pending_queue_notices: Vec::new(),
            event_rx: None,
            turn_task: None,
            quit: false,
            turn_started: None,
            busy_hint: t(locale, MessageId::TuiBusyWaitingModel, &[]),
            anim_tick: 0,
            dirty: true,
            committed_turns: 0,
            cur_user: false,
            cur_block: 0,
            cur_notices: 0,
            viewport_rows: ACTIVITY_ROWS as u16 + INPUT_ROWS_MIN as u16 + STATUS_ROWS,
            slash_sel: 0,
            model_submenu_sel: 0,
            model_stage: None,
            model_fetch_task: None,
            locale,
            verbose,
        }
    }

    pub async fn run(mut self) -> hi_core::Result<()> {
        // 内联视口：底部固定多行，完成内容进终端原生 scrollback；不使用备用屏。
        enable_raw_mode().map_err(|e| hi_core::Error::Message(e.to_string()))?;
        let keyboard_enhanced = push_keyboard_enhancement()?;
        io::stdout()
            .execute(EnableBracketedPaste)
            .map_err(|e| hi_core::Error::Message(e.to_string()))?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(self.viewport_rows),
            },
        )
        .map_err(|e| hi_core::Error::Message(e.to_string()))?;

        // 启动横幅写入 scrollback，随后随对话向上滚动。
        commit(
            &mut terminal,
            banner_lines(&self.model, &self.workdir, &self.session_id),
        )?;
        commit(&mut terminal, vec![Line::from("")])?;

        let result = self.event_loop(&mut terminal).await;

        let _ = disable_raw_mode();
        if keyboard_enhanced {
            let _ = io::stdout().execute(PopKeyboardEnhancementFlags);
        }
        let _ = io::stdout().execute(DisableBracketedPaste);
        let _ = io::stdout().execute(crossterm::cursor::MoveToNextLine(1));
        println!(
            "{}",
            t(
                self.locale,
                MessageId::TuiExitResumeHint,
                std::slice::from_ref(&self.session_id),
            )
        );
        result
    }

    async fn event_loop(&mut self, terminal: &mut Term) -> hi_core::Result<()> {
        loop {
            self.poll_turn_finished().await?;
            self.poll_model_fetch().await;

            if self.agent_busy() {
                self.anim_tick = self.anim_tick.wrapping_add(1);
                self.dirty = true;
                // 回复结束后仍可能有结绳/压缩等后台事件，需持续 drain channel。
                self.wait_live_events(Duration::from_millis(STREAM_TICK_MS))
                    .await;
            } else {
                self.drain_live_events();
            }

            // 先 flush 把完成行写入 scrollback，再 draw 重绘底部视口。
            self.flush(terminal)?;
            self.sync_viewport(terminal)?;

            terminal
                .draw(|f| self.draw(f))
                .map_err(|e| hi_core::Error::Message(e.to_string()))?;
            self.dirty = false;

            if event::poll(Duration::from_millis(if self.agent_busy() {
                STREAM_TICK_MS
            } else {
                KEY_POLL_MS
            }))
            .map_err(|e| hi_core::Error::Message(e.to_string()))?
            {
                match event::read().map_err(|e| hi_core::Error::Message(e.to_string()))? {
                    Event::Key(key) => {
                        self.handle_key_event(key).await?;
                    }
                    Event::Paste(text) => {
                        let _ = self.input.handle_paste(&text);
                        self.mark_dirty();
                    }
                    Event::Resize(_, _) => {
                        terminal
                            .autoresize()
                            .map_err(|e| hi_core::Error::Message(e.to_string()))?;
                        self.dirty = true;
                    }
                    _ => {}
                }
            }

            if self.quit {
                break;
            }
        }
        Ok(())
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 把已完成的对话内容逐段写入终端原生 scrollback（`insert_before`）：
    /// 用户行、思考摘要、每个完成的工具、按完整逻辑行的回复、通知。
    /// 当前回合仍在产出时，尾部未完成行不提交，仅作为底部视口流式预览。
    fn flush(&mut self, term: &mut Term) -> hi_core::Result<()> {
        let width = term
            .size()
            .map_err(|e| hi_core::Error::Message(e.to_string()))?
            .width;

        // 每帧批量写入 scrollback，避免逐行 insert_before 拖慢流式输出。
        let mut batch: Vec<Line<'static>> = Vec::new();
        let verbose = self.verbose;

        for text in self.pending_queue_notices.drain(..).collect::<Vec<_>>() {
            batch.push(queued_for_next_turn_line(&text));
        }

        let mut streaming_hold = false;

        while self.committed_turns < self.turns.len() {
            let i = self.committed_turns;
            let is_last = i + 1 == self.turns.len();
            // 回复/工具仍在产出，尾部未完成内容留在底部预览。
            let streaming = is_last && self.running;
            // 事件流（含回合后的记忆抽取等迟到通知）是否已结束。
            let closed = !is_last || self.turn_task.is_none();

            if !self.cur_user {
                let mut lines = Vec::new();
                if i > 0 {
                    lines.push(Line::from(""));
                }
                lines.extend(render_user(&self.turns[i], width));
                batch.extend(lines);
                self.cur_user = true;
            }

            // 按事件顺序逐个 block 提交：思考摘要 / 工具行 / diff / 回复逻辑行。
            'blocks: while self.cur_block < self.turns[i].blocks.len() {
                match &mut self.turns[i].blocks[self.cur_block] {
                    Block::Thinking(t) => {
                        let thinking_streaming =
                            streaming && t.phase == ThinkingPhase::Streaming;
                        if verbose {
                            if !t.content.is_empty() && !t.summary_committed {
                                batch.push(thinking_header());
                                t.summary_committed = true;
                            }
                            let content = t.content.clone();
                            let logical: Vec<&str> = content.split('\n').collect();
                            let mut committable = logical.len();
                            if thinking_streaming {
                                committable = committable.saturating_sub(1);
                            } else if content.ends_with('\n') && committable > 0 {
                                committable -= 1;
                            }
                            while t.lines_committed < committable {
                                let idx = t.lines_committed;
                                batch.extend(thinking_body_lines(logical[idx], width));
                                t.lines_committed += 1;
                            }
                            if thinking_streaming && t.lines_committed < logical.len() {
                                streaming_hold = true;
                                break 'blocks;
                            }
                            self.cur_block += 1;
                        } else {
                            if thinking_streaming {
                                streaming_hold = true;
                                break 'blocks;
                            }
                            if !t.content.is_empty() && !t.summary_committed {
                                let summary = thinking_summary(&t.content);
                                t.summary_committed = true;
                                batch.push(summary);
                            }
                            self.cur_block += 1;
                        }
                    }
                    Block::Tool(tool) => {
                        if verbose {
                            let running = tool.phase == ToolPhase::Running;
                            let tool_streaming = running && streaming;
                            if !tool.header_committed {
                                batch.push(tool_header_line(tool, width));
                                tool.header_committed = true;
                            }
                            let output = tool.output.clone();
                            if !output.is_empty() {
                                let logical: Vec<&str> = output.split('\n').collect();
                                let mut committable = logical.len();
                                if tool_streaming {
                                    committable = committable.saturating_sub(1);
                                } else if output.ends_with('\n') && committable > 0 {
                                    committable -= 1;
                                }
                                while tool.lines_committed < committable {
                                    let idx = tool.lines_committed;
                                    batch.extend(tool_body_lines(logical[idx], width));
                                    tool.lines_committed += 1;
                                }
                                if tool_streaming && tool.lines_committed < logical.len() {
                                    streaming_hold = true;
                                    break 'blocks;
                                }
                            } else if tool_streaming {
                                streaming_hold = true;
                                break 'blocks;
                            }
                            if let ToolPhase::Done(success) = tool.phase {
                                batch.push(tool_status_line(success));
                            }
                            self.cur_block += 1;
                        } else {
                            if tool.phase == ToolPhase::Running && streaming {
                                streaming_hold = true;
                                break 'blocks;
                            }
                            let line = tool_line(tool, width);
                            batch.push(line);
                            self.cur_block += 1;
                        }
                    }
                    Block::Diff(diff) => {
                        let lines = render_diff(&diff.path, &diff.lines, width);
                        batch.extend(lines);
                        self.cur_block += 1;
                    }
                    Block::Reply(reply) => {
                        let content = reply.content.clone();
                        let logical: Vec<&str> = content.split('\n').collect();
                        // 以 block 自身 streaming 为准，避免后续插入 Tool 块时误判为非流式而重复 commit。
                        let reply_streaming = streaming && reply.streaming;
                        let mut committable = logical.len();
                        if reply_streaming {
                            committable = committable.saturating_sub(1);
                        } else if content.ends_with('\n') && committable > 0 {
                            committable -= 1;
                        }
                        while reply.lines_committed < committable {
                            let idx = reply.lines_committed;
                            let first = idx == 0;
                            let lines = reply_logical_line(logical[idx], width, first);
                            reply.lines_committed += 1;
                            batch.extend(lines);
                        }
                        if reply_streaming && reply.lines_committed < logical.len() {
                            streaming_hold = true;
                            break 'blocks;
                        }
                        self.cur_block += 1;
                    }
                }
            }

            if !streaming_hold {
                while let Some(notice) = self.turns[i].notices.get(self.cur_notices) {
                    batch.extend(render_notice(notice, width));
                    self.cur_notices += 1;
                }
            }

            // 末回合事件流未结束（仍在产出或等待迟到通知）→ 保留游标等待下一帧。
            if !closed {
                break;
            }
            self.committed_turns += 1;
            self.reset_cursors();
        }
        commit(term, batch)
    }

    fn reset_cursors(&mut self) {
        self.cur_user = false;
        self.cur_block = 0;
        self.cur_notices = 0;
    }

    fn drain_live_events(&mut self) {
        let mut pending = Vec::new();
        if let Some(rx) = self.event_rx.as_mut() {
            while let Ok(event) = rx.try_recv() {
                pending.push(event);
            }
        }
        for event in pending {
            self.apply_event(event);
        }
    }

    async fn wait_live_events(&mut self, timeout: Duration) {
        let Some(rx) = &mut self.event_rx else {
            tokio::time::sleep(timeout).await;
            return;
        };
        tokio::select! {
            ev = rx.recv() => {
                if let Some(ev) = ev {
                    self.apply_event(ev);
                }
            }
            _ = tokio::time::sleep(timeout) => {}
        }
        self.drain_live_events();
    }

    async fn poll_turn_finished(&mut self) -> hi_core::Result<()> {
        if let Some(task) = &self.turn_task {
            if task.is_finished() {
                if let Some(task) = self.turn_task.take() {
                    self.drain_live_events();
                    self.event_rx = None;
                    self.running = false;
                    self.turn_started = None;
                    match task.await {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => {
                            if let Some(turn) = self.turns.last_mut() {
                                turn.push_notice(Notice::Error(e.render(self.locale)));
                            }
                        }
                        Err(e) if e.is_cancelled() => {
                            if let Some(turn) = self.turns.last_mut() {
                                turn.push_notice(Notice::System(t(
                                    self.locale,
                                    MessageId::TuiInterrupted,
                                    &[],
                                )));
                                turn.finalize();
                            }
                        }
                        Err(e) => {
                            if let Some(turn) = self.turns.last_mut() {
                                turn.push_notice(Notice::Error(format!("agent task failed: {e}")));
                            }
                        }
                    }
                    if let Some(turn) = self.turns.last_mut() {
                        turn.finalize();
                    }
                    self.mark_dirty();
                }
            }
        }
        self.dispatch_next_queued().await?;
        Ok(())
    }

    fn agent_busy(&self) -> bool {
        self.turn_task.is_some()
    }

    fn enqueue_message(&mut self, text: String) {
        self.message_queue.push(text.clone());
        self.pending_queue_notices.push(text);
        self.mark_dirty();
    }

    async fn dispatch_next_queued(&mut self) -> hi_core::Result<()> {
        while !self.agent_busy() {
            if self.message_queue.is_empty() {
                break;
            }
            let next = self.message_queue.remove(0);
            self.dispatch_user_message(next).await?;
        }
        Ok(())
    }

    fn sync_viewport(&mut self, term: &mut Term) -> hi_core::Result<()> {
        let width = term
            .size()
            .map_err(|e| hi_core::Error::Message(e.to_string()))?
            .width;
        let input_rows = input_row_count(self.input.as_str(), self.input.cursor(), width);
        let target = ACTIVITY_ROWS as u16 + input_rows as u16 + STATUS_ROWS;
        if target == self.viewport_rows {
            return Ok(());
        }

        if target < self.viewport_rows {
            let delta = self.viewport_rows - target;
            for _ in 0..delta {
                io::stdout()
                    .execute(MoveUp(1))
                    .map_err(|e| hi_core::Error::Message(e.to_string()))?;
                io::stdout()
                    .execute(Clear(ClearType::CurrentLine))
                    .map_err(|e| hi_core::Error::Message(e.to_string()))?;
            }
            io::stdout()
                .execute(MoveTo(
                    0,
                    term.size()
                        .map_err(|e| hi_core::Error::Message(e.to_string()))?
                        .height
                        .saturating_sub(1),
                ))
                .map_err(|e| hi_core::Error::Message(e.to_string()))?;
        }

        self.viewport_rows = target;
        *term = Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            TerminalOptions {
                viewport: Viewport::Inline(target),
            },
        )
        .map_err(|e| hi_core::Error::Message(e.to_string()))?;
        term.clear()
            .map_err(|e| hi_core::Error::Message(e.to_string()))?;
        self.dirty = true;
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> hi_core::Result<()> {
        if let ApprovalState::Waiting { .. } = self.approval.state() {
            if key.kind == KeyEventKind::Press {
                return self.handle_approval_key(key.code);
            }
            return Ok(());
        }

        if let KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) =
            key.code
        {
            self.input.set_shift_held(key.kind != KeyEventKind::Release);
            self.mark_dirty();
            return Ok(());
        }

        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        if self.model_stage.is_some() {
            return self.handle_model_stage_key(key).await;
        }

        if self.model_submenu_open() {
            return self.handle_model_submenu_key(key).await;
        }

        if self.slash_menu_matches().is_some() {
            match key.code {
                KeyCode::Up => {
                    self.slash_sel = self.slash_sel.saturating_sub(1);
                    self.mark_dirty();
                    return Ok(());
                }
                KeyCode::Down => {
                    if let Some(n) = self.slash_menu_matches().map(|m| m.len()) {
                        self.slash_sel = (self.slash_sel + 1).min(n.saturating_sub(1));
                    }
                    self.mark_dirty();
                    return Ok(());
                }
                KeyCode::Tab | KeyCode::Enter
                    if key.code == KeyCode::Tab
                        || (!key.modifiers.contains(KeyModifiers::SHIFT) && !self.input.shift_held()) =>
                {
                    self.apply_slash_selection();
                    self.slash_sel = 0;
                    self.mark_dirty();
                    return Ok(());
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.agent_busy() {
                    self.abort_turn();
                } else {
                    self.quit = true;
                }
            }
            KeyCode::Enter => {
                match self.input.handle(key.code, key.modifiers) {
                    InputAction::Submit(text) => self.submit_message(text).await?,
                    InputAction::None => {}
                }
                self.mark_dirty();
            }
            _ => {
                let _ = self.input.handle(key.code, key.modifiers);
                self.slash_sel = 0;
                if self.model_submenu_open() {
                    self.sync_model_submenu_sel();
                }
                self.mark_dirty();
            }
        }
        Ok(())
    }

    fn slash_menu_matches(&self) -> Option<Vec<&'static SlashCommand>> {
        if !slash::menu_visible(self.input.as_str(), self.input.cursor(), self.agent_busy()) {
            return None;
        }
        let token = slash::slash_token(self.input.as_str(), self.input.cursor())?;
        let query = token.strip_prefix('/').unwrap_or("");
        let matches = slash::filter_commands(query);
        if matches.is_empty() {
            return None;
        }
        Some(matches)
    }

    fn apply_slash_selection(&mut self) {
        let Some(matches) = self.slash_menu_matches() else {
            return;
        };
        let sel = slash::clamp_selection(self.slash_sel, matches.len());
        if let Some(cmd) = matches.get(sel) {
            self.input.replace_slash_token(cmd.name);
            if cmd.name == "/model" {
                self.sync_model_submenu_sel();
            }
        }
    }

    fn model_submenu_open(&self) -> bool {
        slash::model_submenu_filter(self.input.as_str(), self.input.cursor()).is_some()
    }

    fn filtered_model_profiles(&self) -> (Vec<hi_core::ModelProfile>, Vec<hi_core::ModelProfile>) {
        let all = self.model_control.profiles();
        let filter = slash::model_submenu_filter(self.input.as_str(), self.input.cursor())
            .unwrap_or("");
        let refs = slash::filter_model_profiles(&all, filter);
        let owned: Vec<_> = refs.into_iter().cloned().collect();
        (all, owned)
    }

    fn sync_model_submenu_sel(&mut self) {
        let (_, filtered) = self.filtered_model_profiles();
        if filtered.is_empty() {
            self.model_submenu_sel = 0;
            return;
        }
        let active = filtered.iter().position(|p| p.active).unwrap_or(0);
        self.model_submenu_sel = active.min(filtered.len().saturating_sub(1));
    }

    async fn submit_message(&mut self, text: String) -> hi_core::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        if self.agent_busy() {
            self.enqueue_message(text);
            return Ok(());
        }
        self.dispatch_user_message(text).await
    }

    async fn dispatch_user_message(&mut self, text: String) -> hi_core::Result<()> {
        self.turns.push(Turn {
            user: text.clone(),
            ..Turn::default()
        });
        self.mark_dirty();

        if text == "/verbose" {
            self.verbose = !self.verbose;
            if let Some(turn) = self.turns.last_mut() {
                let msg = if self.verbose {
                    MessageId::TuiVerboseOn
                } else {
                    MessageId::TuiVerboseOff
                };
                turn.push_notice(Notice::System(t(self.locale, msg, &[])));
                turn.finalize();
            }
            self.mark_dirty();
            return Ok(());
        }

        if let Some(cmd) = parse_session_command(&text) {
            match cmd {
                SessionCommand::Reset => {
                    let mut guard = self.session.lock().await;
                    guard.reset_context()?;
                    if let Some(turn) = self.turns.last_mut() {
                        turn.push_notice(Notice::System(
                            t(self.locale, MessageId::TuiContextReset, &[]),
                        ));
                        turn.finalize();
                    }
                }
                SessionCommand::Compact => {
                    let mut guard = self.session.lock().await;
                    match guard.compact_context(true).await {
                        Ok(events) => {
                            if let Some(turn) = self.turns.last_mut() {
                                for event in events {
                                    turn.apply_localized(event, self.locale);
                                }
                                turn.finalize();
                            }
                        }
                        Err(e) => {
                            if let Some(turn) = self.turns.last_mut() {
                                turn.push_notice(Notice::Error(e.render(self.locale)));
                                turn.finalize();
                            }
                        }
                    }
                }
            }
            self.mark_dirty();
            return Ok(());
        }

        self.start_agent_turn(text).await
    }

    /// 推入一条仅含通知的回合（菜单态下没有进行中的 turn 可挂载通知）。
    fn emit_notice(&mut self, user: String, notice: Notice) {
        self.turns.push(Turn {
            user,
            ..Turn::default()
        });
        if let Some(turn) = self.turns.last_mut() {
            turn.push_notice(notice);
            turn.finalize();
        }
        self.mark_dirty();
    }

    /// 某 provider 实例当前绑定的模型（用于二级菜单标注「当前」）。
    fn current_model_of(&self, provider: &str) -> Option<String> {
        self.model_control
            .profiles()
            .into_iter()
            .find(|p| p.name == provider)
            .map(|p| p.model)
    }

    /// 用指定 model 激活某 provider 实例，重建 session 并刷新状态行模型名。
    async fn activate_model(&mut self, name: &str, model: &str) -> hi_core::Result<()> {
        if self.agent_busy() {
            if let Some(turn) = self.turns.last_mut() {
                turn.push_notice(Notice::Error(t(self.locale, MessageId::GatewayBusy, &[])));
                turn.finalize();
            }
            return Ok(());
        }
        match self.model_control.activate(name, model) {
            Ok((active_model, session)) => {
                *self.session.lock().await = session;
                self.model = active_model.clone();
                if let Some(turn) = self.turns.last_mut() {
                    turn.push_notice(Notice::System(t(
                        self.locale,
                        MessageId::TuiModelActivated,
                        &[name.to_string(), active_model],
                    )));
                    turn.finalize();
                }
            }
            Err(e) => {
                if let Some(turn) = self.turns.last_mut() {
                    turn.push_notice(Notice::Error(e.render(self.locale)));
                    turn.finalize();
                }
            }
        }
        self.mark_dirty();
        Ok(())
    }

    /// 一级菜单选中 provider 实例后进入二级：异步拉取该实例可切换的模型列表。
    async fn enter_model_stage(&mut self) -> hi_core::Result<()> {
        let (_, filtered) = self.filtered_model_profiles();
        if filtered.is_empty() {
            let typed = self.input.as_str().trim().to_string();
            self.emit_notice(
                typed.clone(),
                Notice::Error(t(self.locale, MessageId::TuiModelUnknown, &[typed])),
            );
            self.input.clear();
            return Ok(());
        }
        if self.agent_busy() {
            self.emit_notice(
                "/model".to_string(),
                Notice::Error(t(self.locale, MessageId::GatewayBusy, &[])),
            );
            return Ok(());
        }
        let sel = slash::clamp_selection(self.model_submenu_sel, filtered.len());
        let provider = filtered[sel].name.clone();
        self.model_stage = Some(ModelStage::Loading {
            provider: provider.clone(),
        });
        let mc = Arc::clone(&self.model_control);
        self.model_fetch_task = Some(tokio::spawn(async move { mc.list_models(&provider).await }));
        self.mark_dirty();
        Ok(())
    }

    /// 二级菜单确认选中模型：激活 provider + model。
    async fn confirm_model_pick(
        &mut self,
        provider: String,
        models: Vec<String>,
        sel: usize,
    ) -> hi_core::Result<()> {
        if models.is_empty() {
            return Ok(());
        }
        let model = models[slash::clamp_selection(sel, models.len())].clone();
        self.model_stage = None;
        self.input.clear();
        self.turns.push(Turn {
            user: format!("/model {provider} {model}"),
            ..Turn::default()
        });
        self.activate_model(&provider, &model).await
    }

    /// 轮询二级菜单的模型拉取任务：完成后填充选择列表或报错回退。
    async fn poll_model_fetch(&mut self) {
        let finished = self
            .model_fetch_task
            .as_ref()
            .map(|t| t.is_finished())
            .unwrap_or(false);
        if !finished {
            return;
        }
        let Some(task) = self.model_fetch_task.take() else {
            return;
        };
        // 仍处于 Loading 才接受结果；用户中途 Esc 取消则丢弃。
        let provider = match &self.model_stage {
            Some(ModelStage::Loading { provider }) => provider.clone(),
            _ => return,
        };
        match task.await {
            Ok(Ok(models)) => {
                if models.is_empty() {
                    self.model_stage = None;
                    self.emit_notice(
                        format!("/model {provider}"),
                        Notice::Error(t(self.locale, MessageId::TuiModelNoModels, &[])),
                    );
                } else {
                    let sel = self
                        .current_model_of(&provider)
                        .and_then(|cur| models.iter().position(|m| *m == cur))
                        .unwrap_or(0);
                    self.model_stage = Some(ModelStage::Pick {
                        provider,
                        models,
                        sel,
                    });
                    self.mark_dirty();
                }
            }
            Ok(Err(e)) => {
                self.model_stage = None;
                self.emit_notice(
                    format!("/model {provider}"),
                    Notice::Error(t(
                        self.locale,
                        MessageId::TuiModelFetchFailed,
                        &[e.render(self.locale)],
                    )),
                );
            }
            Err(_) => {
                self.model_stage = None;
                self.mark_dirty();
            }
        }
    }

    fn set_model_pick_sel(&mut self, next: usize) {
        if let Some(ModelStage::Pick { sel, .. }) = &mut self.model_stage {
            *sel = next;
            self.mark_dirty();
        }
    }

    /// 二级菜单（Loading / Pick）键处理。
    async fn handle_model_stage_key(&mut self, key: KeyEvent) -> hi_core::Result<()> {
        match self.model_stage.clone() {
            Some(ModelStage::Loading { .. }) => {
                if key.code == KeyCode::Esc {
                    if let Some(task) = self.model_fetch_task.take() {
                        task.abort();
                    }
                    self.model_stage = None;
                    self.mark_dirty();
                }
            }
            Some(ModelStage::Pick { provider, models, sel }) => match key.code {
                KeyCode::Up => self.set_model_pick_sel(sel.saturating_sub(1)),
                KeyCode::Down => {
                    self.set_model_pick_sel((sel + 1).min(models.len().saturating_sub(1)))
                }
                KeyCode::Enter
                    if !key.modifiers.contains(KeyModifiers::SHIFT) && !self.input.shift_held() =>
                {
                    self.confirm_model_pick(provider, models, sel).await?;
                }
                KeyCode::Esc => {
                    // 退回一级（provider）菜单，输入框仍保留 `/model`。
                    self.model_stage = None;
                    self.mark_dirty();
                }
                _ => {}
            },
            None => {}
        }
        Ok(())
    }

    async fn handle_model_submenu_key(&mut self, key: KeyEvent) -> hi_core::Result<()> {
        let (_, filtered) = self.filtered_model_profiles();
        if filtered.is_empty() {
            match key.code {
                KeyCode::Enter
                    if !key.modifiers.contains(KeyModifiers::SHIFT) && !self.input.shift_held() =>
                {
                    self.enter_model_stage().await?;
                }
                KeyCode::Esc => {
                    self.input.clear();
                    self.mark_dirty();
                }
                _ => {}
            }
            return Ok(());
        }
        match key.code {
            KeyCode::Up => {
                self.model_submenu_sel = self.model_submenu_sel.saturating_sub(1);
                self.mark_dirty();
            }
            KeyCode::Down => {
                self.model_submenu_sel = (self.model_submenu_sel + 1)
                    .min(filtered.len().saturating_sub(1));
                self.mark_dirty();
            }
            KeyCode::Enter
                if !key.modifiers.contains(KeyModifiers::SHIFT) && !self.input.shift_held() =>
            {
                self.enter_model_stage().await?;
            }
            KeyCode::Esc => {
                self.input.clear();
                self.mark_dirty();
            }
            _ => {}
        }
        Ok(())
    }

    async fn start_agent_turn(&mut self, text: String) -> hi_core::Result<()> {
        self.running = true;
        self.turn_started = Some(std::time::Instant::now());
        self.busy_hint = t(self.locale, MessageId::TuiBusyWaitingModel, &[]);
        self.mark_dirty();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.event_rx = Some(event_rx);

        let session = Arc::clone(&self.session);
        let approval = self.approval.clone();
        self.turn_task = Some(tokio::spawn(async move {
            let mut guard = session.lock().await;
            guard.run_turn(&text, &approval, Some(event_tx)).await
        }));
        Ok(())
    }

    fn abort_turn(&mut self) {
        if let Some(task) = self.turn_task.take() {
            task.abort();
        }
        self.event_rx = None;
        self.running = false;
        self.turn_started = None;
        if let Some(turn) = self.turns.last_mut() {
            turn.finalize();
        }
        self.mark_dirty();
    }

    fn handle_approval_key(&mut self, code: KeyCode) -> hi_core::Result<()> {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => self.approval.respond(true),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => self.approval.respond(false),
            _ => {}
        }
        self.mark_dirty();
        Ok(())
    }

    fn apply_event(&mut self, event: AgentEvent) {
        match &event {
            AgentEvent::AssistantDelta { .. } => {
                self.busy_hint = t(self.locale, MessageId::TuiBusyGenerating, &[])
            }
            AgentEvent::ReasoningDelta { .. } => {
                self.busy_hint = t(self.locale, MessageId::TuiBusyThinking, &[])
            }
            AgentEvent::ToolCallStarted { name, .. }
            | AgentEvent::ToolOutputDelta { name, .. }
            | AgentEvent::ToolCallFinished { name, .. } => {
                self.busy_hint = format!("tool {name}");
            }
            AgentEvent::TurnCompleted => {
                self.running = false;
            }
            AgentEvent::KnotsExtracting => {
                self.busy_hint = t(self.locale, MessageId::TuiBusyMemoryExtract, &[]);
            }
            AgentEvent::ContextCompressed { .. } if !self.running => {
                self.busy_hint = t(self.locale, MessageId::TuiBusyCompress, &[]);
            }
            _ => {}
        }
        if let Some(turn) = self.turns.last_mut() {
            turn.apply_localized(event, self.locale);
        }
        self.mark_dirty();
    }

    fn activity_lines(&self, width: u16) -> Vec<Line<'static>> {
        if !self.running {
            return Vec::new();
        }
        self.streaming_output_lines(width).unwrap_or_default()
    }

    /// 流式活动预览：think / 运行中工具 / 未落历史的回复尾行。
    fn streaming_output_lines(&self, width: u16) -> Option<Vec<Line<'static>>> {
        let turn = self.turns.last()?;
        let block = turn.blocks.last()?;
        let lines = match block {
            Block::Reply(reply) if reply.streaming => {
                let logical: Vec<&str> = reply.content.split('\n').collect();
                logical
                    .iter()
                    .skip(reply.lines_committed)
                    .enumerate()
                    .flat_map(|(off, seg)| {
                        reply_logical_line(seg, width, reply.lines_committed + off == 0)
                    })
                    .collect()
            }
            Block::Tool(tool) if tool.phase == ToolPhase::Running => {
                if self.verbose {
                    let logical: Vec<&str> = tool.output.split('\n').collect();
                    logical
                        .iter()
                        .skip(tool.lines_committed)
                        .flat_map(|seg| tool_body_lines(seg, width))
                        .collect()
                } else {
                    tool_preview_lines(tool, width, ACTIVITY_ROWS)
                }
            }
            Block::Thinking(t)
                if t.phase == ThinkingPhase::Streaming && !t.content.is_empty() =>
            {
                if self.verbose {
                    let logical: Vec<&str> = t.content.split('\n').collect();
                    logical
                        .iter()
                        .skip(t.lines_committed)
                        .flat_map(|seg| thinking_body_lines(seg, width))
                        .collect()
                } else {
                    vec![thinking_preview(&t.content, width)]
                }
            }
            _ => return None,
        };
        let start = lines.len().saturating_sub(ACTIVITY_ROWS);
        Some(lines[start..].to_vec())
    }

    fn status_line(&self) -> Line<'static> {
        if let Some(stage) = &self.model_stage {
            let id = match stage {
                ModelStage::Loading { .. } => MessageId::TuiStatusModelLoading,
                ModelStage::Pick { .. } => MessageId::TuiStatusModelPick,
            };
            return Line::from(Span::styled(
                t(self.locale, id, &[]),
                crate::theme::UiTheme::MUTED,
            ));
        }
        if self.model_submenu_open() {
            return Line::from(Span::styled(
                t(self.locale, MessageId::TuiStatusModelMenu, &[]),
                crate::theme::UiTheme::MUTED,
            ));
        }
        if self.slash_menu_matches().is_some() {
            return Line::from(Span::styled(
                t(self.locale, MessageId::TuiStatusSlashMenu, &[]),
                crate::theme::UiTheme::MUTED,
            ));
        }
        if self.agent_busy() {
            let elapsed = turn_elapsed(self.turn_started)
                .map(|d| d.as_secs_f32())
                .unwrap_or(0.0);
            busy_line(&self.busy_hint, self.anim_tick, elapsed)
        } else {
            Line::from(Span::styled(
                t(
                    self.locale,
                    MessageId::TuiStatusDefault,
                    std::slice::from_ref(&self.model),
                ),
                crate::theme::UiTheme::MUTED,
            ))
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let width = area.width;
        let input_rows_count = input_row_count(self.input.as_str(), self.input.cursor(), width);
        let mut constraints = vec![Constraint::Length(1); ACTIVITY_ROWS];
        constraints.extend((0..input_rows_count).map(|_| Constraint::Length(1)));
        constraints.push(Constraint::Length(1));
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let activity_rows = &rows[..ACTIVITY_ROWS];
        let input_rows = &rows[ACTIVITY_ROWS..ACTIVITY_ROWS + input_rows_count];
        let status_row = rows[ACTIVITY_ROWS + input_rows_count];

        if let ApprovalState::Waiting { command, .. } = self.approval.state() {
            for row in activity_rows {
                frame.render_widget(Paragraph::new(Line::from("")), *row);
            }
            let lines = approval_lines(&command, self.locale);
            for (idx, row) in input_rows.iter().enumerate() {
                let line = lines.get(idx).cloned().unwrap_or_else(|| Line::from(""));
                frame.render_widget(Paragraph::new(line), *row);
            }
            frame.render_widget(Paragraph::new(Line::from("")), status_row);
            return;
        }

        frame.render_widget(Paragraph::new(self.status_line()), status_row);

        let inp = input_viewport_lines(
            self.input.as_str(),
            self.input.cursor(),
            input_rows_count,
            width,
        );
        for (line, row) in inp.into_iter().zip(input_rows.iter()) {
            frame.render_widget(Paragraph::new(line), *row);
        }

        if let Some(matches) = self.slash_menu_matches() {
            let sel = slash::clamp_selection(self.slash_sel, matches.len());
            let menu = slash::menu_lines(&matches, sel, width, ACTIVITY_ROWS, self.locale);
            let menu_top = ACTIVITY_ROWS.saturating_sub(menu.len());
            for (slot, row) in activity_rows.iter().enumerate() {
                let line = slot
                    .checked_sub(menu_top)
                    .and_then(|idx| menu.get(idx))
                    .cloned()
                    .unwrap_or_else(|| Line::from(""));
                frame.render_widget(Paragraph::new(line), *row);
            }
            return;
        }

        if let Some(stage) = &self.model_stage {
            let menu = match stage {
                ModelStage::Loading { .. } => vec![Line::from(Span::styled(
                    t(self.locale, MessageId::TuiModelFetching, &[]),
                    crate::theme::UiTheme::MUTED,
                ))],
                ModelStage::Pick { provider, models, sel } => {
                    let current = self.current_model_of(provider);
                    let sel = slash::clamp_selection(*sel, models.len());
                    model_list_lines(models, current.as_deref(), sel, width, ACTIVITY_ROWS)
                }
            };
            let menu_top = ACTIVITY_ROWS.saturating_sub(menu.len());
            for (slot, row) in activity_rows.iter().enumerate() {
                let line = slot
                    .checked_sub(menu_top)
                    .and_then(|idx| menu.get(idx))
                    .cloned()
                    .unwrap_or_else(|| Line::from(""));
                frame.render_widget(Paragraph::new(line), *row);
            }
            return;
        }

        if self.model_submenu_open() {
            let (_, filtered) = self.filtered_model_profiles();
            let sel = slash::clamp_selection(self.model_submenu_sel, filtered.len());
            let menu = model_picker_lines(&filtered, sel, width, ACTIVITY_ROWS);
            let menu_top = ACTIVITY_ROWS.saturating_sub(menu.len());
            for (slot, row) in activity_rows.iter().enumerate() {
                let line = slot
                    .checked_sub(menu_top)
                    .and_then(|idx| menu.get(idx))
                    .cloned()
                    .unwrap_or_else(|| Line::from(""));
                frame.render_widget(Paragraph::new(line), *row);
            }
            return;
        }

        let output_lines = self.activity_lines(width);
        let out_top = ACTIVITY_ROWS.saturating_sub(output_lines.len());
        for (slot, row) in activity_rows.iter().enumerate() {
            let line = slot
                .checked_sub(out_top)
                .and_then(|idx| output_lines.get(idx))
                .cloned()
                .unwrap_or_else(|| Line::from(""));
            frame.render_widget(Paragraph::new(line), *row);
        }
    }
}

/// macOS 上 CSI-u 全键码协议会与中文 IME 组字/上屏冲突；默认不启用。
/// 设置 `HI_TUI_KEYBOARD_ENHANCE=1` 可强制开启（Shift 跟踪等高级键位）。
fn push_keyboard_enhancement() -> hi_core::Result<bool> {
    #[cfg(target_os = "macos")]
    {
        let force = std::env::var_os("HI_TUI_KEYBOARD_ENHANCE").is_some_and(|v| {
            v.eq_ignore_ascii_case("1") || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        });
        if !force {
            return Ok(false);
        }
    }
    io::stdout()
        .execute(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
        ))
        .map_err(|e| hi_core::Error::Message(e.to_string()))?;
    Ok(true)
}

/// 把一组行写入终端原生 scrollback（位于内联视口上方）。
fn commit(term: &mut Term, lines: Vec<Line<'static>>) -> hi_core::Result<()> {
    let height = lines.len() as u16;
    if height == 0 {
        return Ok(());
    }
    term.insert_before(height, move |buf| {
        Paragraph::new(lines).render(buf.area, buf);
    })
    .map_err(|e| hi_core::Error::Message(e.to_string()))
}
