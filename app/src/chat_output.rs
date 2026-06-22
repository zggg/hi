use std::io::{self, Write};

use hi_core::{AgentEvent, SerializableDiffKind};

fn normalize_text(text: &str) -> String {
    text.replace('\r', "")
}

fn write_body(out: &mut impl Write, text: &str) -> io::Result<()> {
    let text = normalize_text(text);
    if text.is_empty() {
        return Ok(());
    }
    for line in text.split('\n') {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

/// Interactive `hi chat` REPL: tools, diffs, then assistant reply.
pub fn print_chat_events(events: &[AgentEvent]) {
    print_chat_events_with_options(events, false);
}

/// Single-shot `hi chat …`: only the final assistant text (and errors).
pub fn print_chat_final(events: &[AgentEvent]) {
    print_chat_events_with_options(events, true);
}

pub fn print_chat_error(message: &str) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{message}");
    let _ = out.flush();
}

fn print_chat_events_with_options(events: &[AgentEvent], reply_only: bool) {
    let mut out = io::stdout().lock();
    let mut assistant = String::new();
    let mut reasoning = String::new();
    let mut had_tools = false;

    for event in events {
        match event {
            AgentEvent::ToolCallStarted { name, .. } if !reply_only => {
                let _ = writeln!(out, "[tool] {name} …");
                had_tools = true;
            }
            AgentEvent::ToolCallFinished { name, success, .. } if !reply_only => {
                let status = if *success { "ok" } else { "failed" };
                let _ = writeln!(out, "[tool] {name} {status}");
            }
            AgentEvent::FileDiff { path, lines } if !reply_only => {
                let _ = writeln!(out, "--- {path}");
                for line in lines {
                    let prefix = match line.kind {
                        SerializableDiffKind::Remove => '-',
                        SerializableDiffKind::Add => '+',
                        SerializableDiffKind::Context => ' ',
                    };
                    let _ = writeln!(out, "{prefix} {}", line.text);
                }
            }
            AgentEvent::ApprovalRequired { command } if !reply_only => {
                let _ = writeln!(out);
                let _ = writeln!(out, "[approval required] {command}");
            }
            AgentEvent::ContextCompressed { summary } if !reply_only => {
                let _ = writeln!(out);
                let _ = writeln!(out, "[context compressed] {} chars summary", summary.len());
            }
            AgentEvent::AssistantDelta { text } => assistant.push_str(text),
            AgentEvent::ReasoningDelta { text } => reasoning.push_str(text),
            AgentEvent::Error { message } => {
                let _ = writeln!(out, "{message}");
            }
            AgentEvent::TurnCompleted => {}
            _ => {}
        }
    }

    if reply_only && assistant.is_empty() && !reasoning.is_empty() {
        assistant = reasoning;
    }
    if !reply_only && had_tools && !assistant.is_empty() {
        let _ = writeln!(out);
    }
    let _ = write_body(&mut out, &assistant);
    let _ = out.flush();
}

#[cfg(test)]
#[path = "../test/unit/chat_output.rs"]
mod tests;
