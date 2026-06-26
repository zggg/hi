use clap::Subcommand;
use hi_core::{
    extract_knots, merge_extracted, KnotConfidence, KnotKind, KnotProvenance, KnotStatus,
    t, KnotVisibility, MemoryConfig, MessageId, NewKnot, OwnerId, SessionId, SessionStore,
};

use crate::bridge::ProviderBridge;
use crate::runtime::load_config;
use crate::services::build_provider;

#[derive(Subcommand, Debug)]
/// Author: gz
pub enum MemoryCommands {
    /// List active knots for the configured owner
    List {
        #[arg(long, help = "Include superseded and deleted knots")]
        all: bool,
        #[arg(long, help = "Owner id (default: from [memory].owner_id)")]
        owner: Option<String>,
    },
    /// Show knot details
    Show {
        id: i64,
    },
    /// Add a knot manually
    Add {
        text: String,
        #[arg(long, value_name = "KIND", default_value = "fact")]
        kind: String,
        #[arg(long, help = "Mark as confirmed (default: inferred)")]
        confirmed: bool,
        #[arg(long, help = "Do not decay (忘川 permanent)")]
        permanent: bool,
        #[arg(long, help = "Owner id override")]
        owner: Option<String>,
    },
    /// Soft-delete a knot (status = deleted)
    Forget {
        id: i64,
    },
    /// Set clarity to 1.0
    Reinforce {
        id: i64,
        #[arg(long, help = "Also mark permanent")]
        permanent: bool,
    },
    /// Run knot extraction on a session transcript (LLM)
    Extract {
        #[arg(long, default_value = "chat:main", help = "Session id")]
        session: String,
        #[arg(long, help = "Owner id override")]
        owner: Option<String>,
    },
}

pub async fn run(cmd: MemoryCommands) -> anyhow::Result<()> {
    let config = load_config().map_err(map_err)?;
    let locale = config.resolved_locale();
    let store = SessionStore::open_with_pool(
        config.sessions_db_path(),
        config.storage.effective_read_pool_size(),
    )
    .map_err(map_err)?;

    match cmd {
        MemoryCommands::List { all, owner } => {
            let owner = owner_id(&config.memory, owner.as_deref());
            store.ensure_memory_owner(&owner).map_err(map_err)?;
            let knots = if all {
                store.list_all_knots(&owner).map_err(map_err)?
            } else {
                store.list_knots(&owner).map_err(map_err)?
            };
            if knots.is_empty() {
                println!(
                    "{}",
                    t(locale, MessageId::MemoryListEmpty, std::slice::from_ref(&owner.0))
                );
                return Ok(());
            }
            print_knot_header();
            for k in knots {
                print_knot_line(&k);
            }
        }
        MemoryCommands::Show { id } => {
            let k = store.get_knot(id).map_err(map_err)?;
            println!("id:          {}", k.id);
            println!("owner:       {}", k.owner_id.0);
            println!("kind:        {}", k.kind.as_str());
            println!("status:      {}", k.status.as_str());
            println!("confidence:  {}", k.confidence.as_str());
            println!("clarity:     {:.2}", k.clarity);
            println!("permanent:   {}", k.permanent);
            println!("content:     {}", k.content);
            println!("hash:        {}", k.content_hash);
            println!("created_at:  {}", k.created_at);
            println!("updated_at:  {}", k.updated_at);
        }
        MemoryCommands::Add {
            text,
            kind,
            confirmed,
            permanent,
            owner,
        } => {
            let owner = owner_id(&config.memory, owner.as_deref());
            store.ensure_memory_owner(&owner).map_err(map_err)?;
            let kind = parse_kind(&kind, locale)?;
            let confidence = if confirmed {
                KnotConfidence::Confirmed
            } else {
                KnotConfidence::Inferred
            };
            let clarity = if confirmed { 1.0 } else { 0.7 };
            let task_status = if kind == KnotKind::Task {
                Some(hi_core::TaskStatus::Open)
            } else {
                None
            };
            let id = store
                .add_knot(&NewKnot {
                    owner_id: owner,
                    kind,
                    content: text,
                    confidence,
                    clarity,
                    permanent,
                    visibility: KnotVisibility::Inject,
                    task_status,
                })
                .map_err(map_err)?;
            println!("{}", t(locale, MessageId::MemoryAdded, &[id.to_string()]));
        }
        MemoryCommands::Forget { id } => {
            store.forget_knot(id).map_err(map_err)?;
            println!("{}", t(locale, MessageId::MemoryForgotten, &[id.to_string()]));
        }
        MemoryCommands::Reinforce { id, permanent } => {
            store.reinforce_knot(id, permanent).map_err(map_err)?;
            println!(
                "{}",
                t(locale, MessageId::MemoryReinforced, &[id.to_string()])
            );
        }
        MemoryCommands::Extract { session, owner } => {
            if !config.memory.enabled {
                anyhow::bail!("{}", t(locale, MessageId::MemoryDisabledInConfig, &[]));
            }
            let owner = owner_id(&config.memory, owner.as_deref());
            store.ensure_memory_owner(&owner).map_err(map_err)?;
            let session_id = SessionId(session.clone());
            let stored = store.load_context_messages(&session_id).map_err(map_err)?;
            if stored.is_empty() {
                println!(
                    "{}",
                    t(locale, MessageId::MemoryExtractNone, std::slice::from_ref(&session))
                );
                return Ok(());
            }
            let messages: Vec<_> = stored.into_iter().map(|r| r.message).collect();
            let provider = build_provider(&config).map_err(map_err)?;
            let bridge = ProviderBridge::new(provider, locale);
            let existing = store.list_knots(&owner).map_err(map_err)?;
            let outcome = extract_knots(
                &bridge,
                &config.ai.model,
                &messages,
                &existing,
                &config.memory,
            )
            .await
            .map_err(map_err)?;
            let provenance = KnotProvenance {
                session_id: Some(session_id.clone()),
                ..KnotProvenance::default()
            };
            let merged =
                merge_extracted(&store, &owner, &outcome.extracted, &provenance).map_err(map_err)?;
            println!(
                "{}",
                t(
                    locale,
                    MessageId::MemoryExtractDone,
                    &[
                        merged.added.to_string(),
                        merged.skipped.to_string(),
                        merged.superseded.to_string(),
                    ],
                )
            );
        }
    }
    Ok(())
}

fn owner_id(memory: &MemoryConfig, override_owner: Option<&str>) -> OwnerId {
    OwnerId(
        override_owner
            .unwrap_or(&memory.owner_id)
            .to_string(),
    )
}

fn parse_kind(s: &str, locale: hi_core::Locale) -> anyhow::Result<KnotKind> {
    KnotKind::parse(s).ok_or_else(|| {
        anyhow::anyhow!(
            "{}",
            t(locale, MessageId::MemoryUnknownKind, &[s.to_string()])
        )
    })
}

fn print_knot_header() {
    println!(
        "{:>6}  {:<12} {:<10} {:>5}  CONTENT",
        "ID", "KIND", "CONF", "CLR"
    );
}

fn print_knot_line(k: &hi_core::Knot) {
    let content = if k.content.chars().count() > 60 {
        format!("{}…", k.content.chars().take(60).collect::<String>())
    } else {
        k.content.clone()
    };
    let status = if k.status == KnotStatus::Active {
        String::new()
    } else {
        format!(" [{}]", k.status.as_str())
    };
    println!(
        "{:>6}  {:<12} {:<10} {:>5.2}  {}{status}",
        k.id,
        k.kind.as_str(),
        k.confidence.as_str(),
        k.clarity,
        content,
    );
}

fn map_err(e: hi_core::Error) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}
