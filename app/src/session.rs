use std::path::PathBuf;

use clap::Subcommand;
use hi_core::{t, MessageId, Role, SessionId, SessionStore, StoredMessage};

use crate::runtime::load_config;

#[derive(Subcommand, Debug)]
/// Author: gz
pub enum SessionCommands {
    /// List all sessions (message counts and last activity)
    List,
    /// Show transcript (`--context` for agent-visible rows only)
    Show {
        #[arg(long, default_value = "chat:main")]
        session: String,
        #[arg(long, help = "Only rows with in_context = 1")]
        context: bool,
    },
    /// Export full transcript as JSON
    Export {
        #[arg(long, default_value = "chat:main")]
        session: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List compression events for a session
    Compressions {
        #[arg(long, default_value = "chat:main")]
        session: String,
    },
    /// Show one compression event
    CompressionShow {
        id: i64,
    },
    /// Permanently delete a session and all messages (requires --confirm)
    Purge {
        #[arg(long)]
        session: String,
        #[arg(long)]
        confirm: bool,
    },
}

pub fn run(cmd: SessionCommands) -> anyhow::Result<()> {
    let config = load_config().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let locale = config.resolved_locale();
    let store = SessionStore::open(config.sessions_db_path()).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    match cmd {
        SessionCommands::List => cmd_list(&store, locale),
        SessionCommands::Show { session, context } => cmd_show(&store, &session, context, locale),
        SessionCommands::Export { session, output } => {
            cmd_export(&store, &session, output.as_ref(), locale)
        }
        SessionCommands::Compressions { session } => cmd_compressions(&store, &session, locale),
        SessionCommands::CompressionShow { id } => cmd_compression_show(&store, id, locale),
        SessionCommands::Purge { session, confirm } => cmd_purge(&store, &session, confirm, locale),
    }
}

fn cmd_list(store: &SessionStore, locale: hi_core::Locale) -> anyhow::Result<()> {
    let sessions = store.list_sessions().map_err(map_err)?;
    if sessions.is_empty() {
        println!("{}", t(locale, MessageId::SessionEmpty, &[]));
        return Ok(());
    }
    println!(
        "{:<24} {:>8} {:>8}  WORKDIR",
        "SESSION", "TOTAL", "CONTEXT"
    );
    for s in sessions {
        println!(
            "{:<24} {:>8} {:>8}  {}",
            s.session_id.0,
            s.message_total,
            s.message_in_context,
            s.working_directory,
        );
    }
    Ok(())
}

fn cmd_show(
    store: &SessionStore,
    session: &str,
    context_only: bool,
    locale: hi_core::Locale,
) -> anyhow::Result<()> {
    let session_id = SessionId(session.to_string());
    let rows = if context_only {
        store.load_context_messages(&session_id).map_err(map_err)?
    } else {
        store.load_all_messages(&session_id).map_err(map_err)?
    };
    if rows.is_empty() {
        println!(
            "{}",
            t(locale, MessageId::SessionNotFound, &[session.to_string()])
        );
        return Ok(());
    }
    for row in rows {
        print_message_line(&row);
    }
    Ok(())
}

fn cmd_export(
    store: &SessionStore,
    session: &str,
    output: Option<&PathBuf>,
    locale: hi_core::Locale,
) -> anyhow::Result<()> {
    let session_id = SessionId(session.to_string());
    let rows = store.load_all_messages(&session_id).map_err(map_err)?;
    let json = serde_json::to_string_pretty(&export_rows(&rows))?;
    match output {
        Some(path) => {
            std::fs::write(path, json)?;
            println!(
                "{}",
                t(
                    locale,
                    MessageId::SessionExported,
                    &[rows.len().to_string(), path.display().to_string()],
                )
            );
        }
        None => println!("{json}"),
    }
    Ok(())
}

fn cmd_compressions(
    store: &SessionStore,
    session: &str,
    locale: hi_core::Locale,
) -> anyhow::Result<()> {
    let session_id = SessionId(session.to_string());
    let list = store.list_compressions(&session_id).map_err(map_err)?;
    if list.is_empty() {
        println!("{}", t(locale, MessageId::CompressionListHeader, &[]));
        return Ok(());
    }
    for c in list {
        println!(
            "#{}  msgs {}..{}  ({} rows)  {}",
            c.id,
            c.message_id_from,
            c.message_id_to,
            c.message_count,
            c.summary_text.as_deref().unwrap_or("—"),
        );
    }
    Ok(())
}

fn cmd_compression_show(
    store: &SessionStore,
    id: i64,
    _locale: hi_core::Locale,
) -> anyhow::Result<()> {
    let c = store.get_compression(id).map_err(map_err)?;
    println!("session:     {}", c.session_id.0);
    println!("messages:    {}..{} ({} rows)", c.message_id_from, c.message_id_to, c.message_count);
    println!("tokens:      {:?}", c.token_estimate);
    println!("created_at:  {}", c.created_at);
    if let Some(summary) = &c.summary_text {
        println!("\n--- summary ---\n{summary}");
    }
    Ok(())
}

fn cmd_purge(
    store: &SessionStore,
    session: &str,
    confirm: bool,
    locale: hi_core::Locale,
) -> anyhow::Result<()> {
    if !confirm {
        anyhow::bail!(
            "{}",
            t(
                locale,
                MessageId::SessionPurgeNeedConfirm,
                &[session.to_string()]
            )
        );
    }
    let session_id = SessionId(session.to_string());
    store.purge_session(&session_id).map_err(map_err)?;
    println!(
        "{}",
        t(locale, MessageId::SessionDeleted, &[session.to_string()])
    );
    Ok(())
}

const PREVIEW_MAX_CHARS: usize = 120;

fn truncate_preview(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

fn print_message_line(row: &StoredMessage) {
    let role = match row.message.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };
    let ctx = if row.in_context { "" } else { " [archived]" };
    print!("#{:<5} {role}{ctx}: ", row.id);
    let content = row.message.content.lines().next().unwrap_or("");
    println!("{}", truncate_preview(content, PREVIEW_MAX_CHARS));
}

#[derive(serde::Serialize)]
struct ExportRow {
    id: i64,
    in_context: bool,
    role: String,
    content: String,
}

fn export_rows(rows: &[StoredMessage]) -> Vec<ExportRow> {
    rows.iter()
        .map(|r| {
            let role = match r.message.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            ExportRow {
                id: r.id,
                in_context: r.in_context,
                role: role.into(),
                content: r.message.content.clone(),
            }
        })
        .collect()
}

fn map_err(e: hi_core::Error) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

#[cfg(test)]
#[path = "../test/unit/session.rs"]
mod tests;
