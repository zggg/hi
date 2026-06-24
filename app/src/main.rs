mod i18n;
mod approval;
mod bridge;
mod chat_output;
mod config;
mod gateway_svc;
mod logging;
mod memory;
mod model_control;
mod runtime;
mod services;
mod session;

use std::io::{self, Write};
use std::sync::Arc;

use approval::StdinApproval;
use chat_output::{print_chat_error, print_chat_events, print_chat_final};
use clap::{Parser, Subcommand};
use hi_core::{parse_session_command, t, Channel, MessageId, SessionCommand, SessionId};
use memory::MemoryCommands;
use session::SessionCommands;
use runtime::{load_channels, load_config, resolve_config_workspace, AgentRuntime, HiServices};

#[derive(Parser)]
#[command(
    name = "hi",
    version,
    about = "Ultra-lightweight personal AI assistant with TUI and channel gateway",
    after_help = "Run `hi` with no subcommand to start the TUI (same as `hi tui`)."
)]
/// Author: gz
struct Cli {
    /// Session id for transcript/context isolation (default: tui:main)
    #[arg(short = 's', long, value_name = "SESSION_ID")]
    session: Option<String>,
    /// Verbose output (TUI/chat: stream think & tools; session show: full debug transcript)
    #[arg(short = 'v', long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
/// Author: gz
enum Commands {
    /// Interactive LLM + workspace setup (re-run uses current values as defaults)
    Setup,
    /// Configure the LLM model only (add/switch model, keep workspace & channels)
    Model,
    /// Start interactive terminal UI
    Tui {
        /// Session id for transcript/context isolation (default: tui:main)
        #[arg(long, value_name = "SESSION_ID")]
        session: Option<String>,
    },
    /// Message-channel gateway (`cargo run`: foreground; release install: background start)
    Gateway {
        #[command(subcommand)]
        action: Option<GatewayAction>,
        /// Validate bot_id + secret and subscribe once, then exit
        #[arg(long)]
        check: bool,
    },
    /// Chat with the agent (stdin REPL or `hi chat 词1 词2 …` single turn)
    Chat {
        /// Session id for transcript/context isolation (default: chat:main)
        #[arg(long, value_name = "SESSION_ID")]
        session: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        message: Vec<String>,
    },
    /// Print effective configuration (`~/.hi/hi.toml`, secrets redacted)
    Config,
    /// Session transcript (append-only) and compression history
    Session {
        #[command(subcommand)]
        sub: SessionCommands,
    },
    /// Long-term knot memory (结绳记事)
    Memory {
        #[command(subcommand)]
        sub: MemoryCommands,
    },
}

#[derive(Subcommand)]
/// Author: gz
enum GatewayAction {
    /// Configure a message channel (current: wecom, feishu)
    Setup,
    /// Start gateway in background (default for release builds)
    Start,
    /// Stop background gateway
    Stop,
    /// Restart background gateway
    Restart,
    /// Show gateway process status
    Status,
    /// Reload hi.toml `[ai]` / `[tools.*]` without restarting (Unix only, SIGUSR1)
    #[cfg(unix)]
    Reload,
    /// Run gateway in foreground (`cargo run` default; logs to ~/.hi/logs/hi.log)
    Run,
}

/// Default when `hi gateway` has no subcommand: debug → foreground, release → background.
fn default_gateway_action() -> GatewayAction {
    if cfg!(debug_assertions) {
        GatewayAction::Run
    } else {
        GatewayAction::Start
    }
}

fn map_core_err(e: hi_core::Error, locale: hi_core::Locale) -> anyhow::Error {
    i18n::map_core_err(e, locale)
}

#[cfg(unix)]
fn spawn_gateway_reload_listener(services: Arc<HiServices>) {
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sig = match signal(SignalKind::user_defined1()) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "gateway: failed to install SIGUSR1 reload handler");
                return;
            }
        };
        while sig.recv().await.is_some() {
            match services.reload_from_disk() {
                Ok(()) => tracing::info!(
                    "gateway: reloaded hi.toml ([ai], [tools.approvals])"
                ),
                Err(e) => tracing::warn!(error = %e, "gateway: reload from hi.toml failed"),
            }
        }
    });
}

#[cfg(not(unix))]
fn spawn_gateway_reload_listener(_services: Arc<HiServices>) {}

fn chat_single_from_args(message: &[String], locale: hi_core::Locale) -> anyhow::Result<Option<String>> {
    if message.is_empty() {
        return Ok(None);
    }
    if message[0].starts_with('-') {
        anyhow::bail!(
            "{}",
            t(locale, MessageId::UnknownChatArg, &[message[0].clone()])
        );
    }
    Ok(Some(message.join(" ")))
}

async fn run_chat(
    services: Arc<HiServices>,
    session: Option<String>,
    single: Option<String>,
) -> anyhow::Result<()> {
    let locale = services.locale();
    let session_id = session
        .map(SessionId)
        .unwrap_or_else(|| Channel::Chat.default_session_id());
    let runtime = AgentRuntime::for_session(Arc::clone(&services), session_id)
        .map_err(|e| map_core_err(e, locale))?;
    let mut agent = runtime.build_loop().map_err(|e| map_core_err(e, locale))?;
    let approval = StdinApproval;

    if let Some(msg) = single {
        match agent.run_turn(&msg, &approval, None).await {
            Ok(events) => print_chat_final(&events),
            Err(e) => {
                print_chat_error(&e.render(locale));
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    println!(
        "{}",
        i18n::msg3(
            locale,
            hi_core::MessageId::ChatBanner,
            runtime.session_id().0.as_str(),
            runtime.model().as_str(),
            &runtime.workdir().display().to_string(),
        )
    );
    let stdin = io::stdin();
    loop {
        print!("you> ");
        io::stdout().flush()?;
        let mut line = String::new();
        let n = stdin.read_line(&mut line)?;
        if n == 0 {
            println!();
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "/quit" || line == "/exit" {
            break;
        }

        if line.starts_with("/model") {
            print_chat_error(&i18n::msg(locale, hi_core::MessageId::ModelSlashTuiOnly));
            println!();
            continue;
        }

        if let Some(cmd) = parse_session_command(line) {
            match cmd {
                SessionCommand::Reset => {
                    agent.reset_context().map_err(|e| map_core_err(e, locale))?;
                    println!("{}", i18n::msg(locale, hi_core::MessageId::ContextReset));
                }
                SessionCommand::Compact => {
                    match agent.compact_context(true).await {
                        Ok(events) => print_chat_events(&events),
                        Err(e) => print_chat_error(&e.render(locale)),
                    }
                }
            }
            println!();
            continue;
        }

        match agent.run_turn(line, &approval, None).await {
            Ok(events) => print_chat_events(&events),
            Err(e) => print_chat_error(&e.render(locale)),
        }
        println!();
    }

    Ok(())
}

async fn run_tui(
    services: Arc<HiServices>,
    session: Option<String>,
    verbose: bool,
) -> anyhow::Result<()> {
    let locale = services.locale();
    let session_id = session
        .map(SessionId)
        .unwrap_or_else(|| Channel::Tui.default_session_id());
    let runtime = AgentRuntime::for_session(Arc::clone(&services), session_id.clone())
        .map_err(|e| map_core_err(e, locale))?;
    let workdir_display = runtime.workdir().display().to_string();
    let workdir = runtime.workdir().clone();
    let agent = runtime.build_session().map_err(|e| map_core_err(e, locale))?;
    let model_control = Arc::new(model_control::AppModelControl::new(
        services,
        session_id.clone(),
        workdir,
    ));
    hi_tui::run(
        agent,
        runtime.model(),
        workdir_display,
        session_id.0,
        model_control,
        locale,
        verbose,
    )
    .await
    .map_err(|e| map_core_err(e, locale))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();

    let cli = Cli::parse();
    match cli.command {
        None => {
            let config = load_config().map_err(|e| map_core_err(e, hi_core::resolve_locale(None)))?;
            let locale = config.resolved_locale();
            let services = HiServices::open(config).map_err(|e| map_core_err(e, locale))?;
            run_tui(services, cli.session, cli.verbose).await?;
        }
        Some(Commands::Tui { session }) => {
            let config = load_config().map_err(|e| map_core_err(e, hi_core::resolve_locale(None)))?;
            let locale = config.resolved_locale();
            let services = HiServices::open(config).map_err(|e| map_core_err(e, locale))?;
            run_tui(services, session.or(cli.session), cli.verbose).await?;
        }
        Some(Commands::Gateway { action, check }) => {
            if check {
                let config = load_config().map_err(|e| map_core_err(e, hi_core::resolve_locale(None)))?;
                let locale = config.resolved_locale();
                let channels = load_channels().map_err(|e| map_core_err(e, locale))?;
                let services = HiServices::open(config).map_err(|e| map_core_err(e, locale))?;
                let gateway_workdir =
                    resolve_config_workspace(&services.config()).map_err(|e| map_core_err(e, locale))?;
                hi_gateway::run_gateway(channels, true, services, gateway_workdir, locale)
                    .await
                    .map_err(|e| map_core_err(e, locale))?;
                return Ok(());
            }

            match action.unwrap_or(default_gateway_action()) {
                GatewayAction::Setup => config::run_gateway_setup()?,
                GatewayAction::Start => gateway_svc::start()?,
                GatewayAction::Stop => gateway_svc::stop()?,
                GatewayAction::Restart => gateway_svc::restart()?,
                GatewayAction::Status => gateway_svc::status()?,
                #[cfg(unix)]
                GatewayAction::Reload => gateway_svc::reload()?,
                GatewayAction::Run => {
                    let _pid_guard = gateway_svc::PidGuard::new();
                    let config = load_config().map_err(|e| map_core_err(e, hi_core::resolve_locale(None)))?;
                    let locale = config.resolved_locale();
                    let channels = load_channels().map_err(|e| map_core_err(e, locale))?;
                    let services = HiServices::open(config).map_err(|e| map_core_err(e, locale))?;
                    let gateway_workdir =
                        resolve_config_workspace(&services.config()).map_err(|e| map_core_err(e, locale))?;
                    spawn_gateway_reload_listener(Arc::clone(&services));
                    hi_gateway::run_gateway(channels, false, services, gateway_workdir, locale)
                        .await
                        .map_err(|e| map_core_err(e, locale))?;
                }
            }
        }
        Some(Commands::Chat { session, message }) => {
            let config = load_config().map_err(|e| map_core_err(e, hi_core::resolve_locale(None)))?;
            let locale = config.resolved_locale();
            let services = HiServices::open(config).map_err(|e| map_core_err(e, locale))?;
            let single = chat_single_from_args(&message, locale)?;
            run_chat(services, session, single).await?;
        }
        Some(Commands::Setup) => config::run_setup()?,
        Some(Commands::Model) => config::run_model()?,
        Some(Commands::Config) => config::show()?,
        Some(Commands::Session { sub }) => session::run(sub, cli.verbose)?,
        Some(Commands::Memory { sub }) => memory::run(sub).await?,
    }

    Ok(())
}

#[cfg(test)]
#[path = "../test/unit/main.rs"]
mod tests;
