use std::time::Duration;

use hi_core::{
    available_gateway_channels, expand_path, gateway_channel, gateway_channel_default,
    resolve_locale, t, ChannelsConfig, Config, FeishuConfig, GatewayChannelKind, Locale,
    MessageId, WeComConfig, WeixinConfig,
};
use hi_gateway::weixin::login::{fetch_qr_code, wait_for_qr_login, QrStatusKind};

use super::wizard::{Session, SelectOption};

const CHANNEL_WECOM: &str = "wecom";
const CHANNEL_FEISHU: &str = "feishu";
const CHANNEL_WEIXIN: &str = "weixin";

/// Run async iLink calls from the sync wizard (`hi` already uses `#[tokio::main]`).
fn block_on_async<F: std::future::Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

fn print_terminal_qr(locale: Locale, content: &str) -> anyhow::Result<()> {
    use qrcode::render::unicode;
    use qrcode::QrCode;

    let code = QrCode::new(content.as_bytes()).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            t(locale, MessageId::QrGenerateFailed, &[e.to_string()])
        )
    })?;
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    println!("{image}");
    Ok(())
}

const GATE_SKIP: &str = "skip";
const GATE_CONFIGURE: &str = "configure";
const STEP_BACK: &str = "__back__";

/// Full wizard (`hi gateway setup`) vs minimal embedded steps (`hi setup`).
///
/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayWizardMode {
    Embedded,
    Standalone,
}

/// Result of the optional gateway section inside `hi setup`.
///
/// Author: gz
pub enum EmbeddedGatewayOutcome {
    Skipped {
        workspace: String,
    },
    Configured {
        workspace: String,
        channels: ChannelsConfig,
    },
}

struct OwnedChannelOption {
    value: &'static str,
    label: String,
    hint: String,
}

fn channel_label_id(id: &str) -> MessageId {
    match id {
        CHANNEL_FEISHU => MessageId::ChannelFeishuLabel,
        CHANNEL_WEIXIN => MessageId::ChannelWeixinLabel,
        _ => MessageId::ChannelWecomLabel,
    }
}

fn channel_hint_id(id: &str) -> MessageId {
    match id {
        CHANNEL_FEISHU => MessageId::ChannelFeishuHint,
        CHANNEL_WEIXIN => MessageId::ChannelWeixinHint,
        _ => MessageId::ChannelWecomHint,
    }
}

fn gateway_channel_options(locale: Locale) -> Vec<OwnedChannelOption> {
    available_gateway_channels()
        .map(|c| OwnedChannelOption {
            value: c.id,
            label: t(locale, channel_label_id(c.id), &[]),
            hint: t(locale, channel_hint_id(c.id), &[]),
        })
        .collect()
}

fn select_options<'a>(owned: &'a [OwnedChannelOption]) -> Vec<SelectOption<'a>> {
    owned
        .iter()
        .map(|o| SelectOption {
            value: o.value,
            label: &o.label,
            hint: &o.hint,
        })
        .collect()
}

fn back_option(locale: Locale) -> (String, String) {
    (
        t(locale, MessageId::WizardBack, &[]),
        String::new(),
    )
}

fn choose_channel(session: &Session, default: &str) -> anyhow::Result<&'static GatewayChannelKind> {
    let owned = gateway_channel_options(session.locale());
    let options = select_options(&owned);
    if options.is_empty() {
        anyhow::bail!(
            "{}",
            t(
                session.locale(),
                MessageId::GatewaySetupNoChannels,
                &[]
            )
        );
    }
    let picked = session.select_with(
        &t(session.locale(), MessageId::GatewaySetupChannelPrompt, &[]),
        &options,
        default,
        false,
    )?;
    Ok(gateway_channel(picked).expect("catalog and options stay in sync"))
}

fn choose_channel_with_back(
    session: &Session,
    default: &str,
) -> anyhow::Result<Result<&'static GatewayChannelKind, ()>> {
    let mut owned = gateway_channel_options(session.locale());
    let (back_label, back_hint) = back_option(session.locale());
    owned.push(OwnedChannelOption {
        value: STEP_BACK,
        label: back_label,
        hint: back_hint,
    });
    let options = select_options(&owned);
    let picked = session.select_with(
        &t(session.locale(), MessageId::GatewaySetupChannelPrompt, &[]),
        &options,
        default,
        false,
    )?;
    if picked == STEP_BACK {
        return Ok(Err(()));
    }
    Ok(Ok(gateway_channel(picked).expect("catalog and options stay in sync")))
}

fn credential_note(locale: Locale, channel: &GatewayChannelKind) -> String {
    let id = match channel.id {
        CHANNEL_WECOM => MessageId::GatewaySetupWecomCredsNote,
        CHANNEL_FEISHU => MessageId::GatewaySetupFeishuCredsNote,
        CHANNEL_WEIXIN => MessageId::GatewaySetupWeixinCredsNote,
        _ => MessageId::GatewaySetupGenericCredsNote,
    };
    t(locale, id, &[])
}

fn configure_wecom(
    session: &Session,
    existing: &WeComConfig,
    mode: GatewayWizardMode,
) -> anyhow::Result<WeComConfig> {
    let mut wecom = existing.clone();

    wecom.bot_id = session.input(
        &t(session.locale(), MessageId::GatewaySetupBotIdPrompt, &[]),
        &wecom.bot_id,
    )?;
    wecom.secret = session.password_keep(
        &t(session.locale(), MessageId::GatewaySetupSecretPrompt, &[]),
        !wecom.secret.is_empty(),
        &wecom.secret,
    )?;

    configure_dm_policy(session, &mut wecom.dm_policy, &mut wecom.allow_from, "userid")?;

    if mode == GatewayWizardMode::Standalone {
        let welcome_default = wecom.welcome_message.clone().unwrap_or_default();
        let welcome = session.input(
            &t(session.locale(), MessageId::GatewaySetupWelcomePrompt, &[]),
            &welcome_default,
        )?;
        wecom.welcome_message = if welcome.is_empty() {
            None
        } else {
            Some(welcome)
        };
    }

    Ok(wecom)
}

fn configure_feishu(
    session: &Session,
    existing: &FeishuConfig,
    mode: GatewayWizardMode,
) -> anyhow::Result<FeishuConfig> {
    let mut feishu = existing.clone();

    feishu.app_id = session.input(
        &t(session.locale(), MessageId::GatewaySetupFeishuAppIdPrompt, &[]),
        &feishu.app_id,
    )?;
    feishu.app_secret = session.password_keep(
        &t(session.locale(), MessageId::GatewaySetupFeishuSecretPrompt, &[]),
        !feishu.app_secret.is_empty(),
        &feishu.app_secret,
    )?;

    let domain_default = feishu.domain.clone().unwrap_or_default();
    let domain = session.input(
        &t(session.locale(), MessageId::GatewaySetupFeishuDomainPrompt, &[]),
        &domain_default,
    )?;
    feishu.domain = if domain.trim().is_empty() {
        None
    } else {
        Some(domain.trim().to_string())
    };

    configure_dm_policy(
        session,
        &mut feishu.dm_policy,
        &mut feishu.allow_from,
        "open_id",
    )?;

    feishu.mention_enabled = session.confirm(
        &t(session.locale(), MessageId::GatewaySetupFeishuMentionPrompt, &[]),
        feishu.mention_enabled,
    )?;
    if feishu.mention_enabled {
        session.note(
            &t(session.locale(), MessageId::GatewaySetupFeishuMentionOnTitle, &[]),
            &t(session.locale(), MessageId::GatewaySetupFeishuMentionOnNote, &[]),
        )?;
    } else {
        session.note(
            &t(session.locale(), MessageId::GatewaySetupFeishuMentionOffTitle, &[]),
            &t(session.locale(), MessageId::GatewaySetupFeishuMentionOffNote, &[]),
        )?;
    }

    if mode == GatewayWizardMode::Standalone {
        let welcome_default = feishu.welcome_message.clone().unwrap_or_default();
        let welcome = session.input(
            &t(session.locale(), MessageId::GatewaySetupWelcomePrompt, &[]),
            &welcome_default,
        )?;
        feishu.welcome_message = if welcome.is_empty() {
            None
        } else {
            Some(welcome)
        };
    }

    Ok(feishu)
}

fn configure_weixin(
    session: &Session,
    existing: &WeixinConfig,
    mode: GatewayWizardMode,
) -> anyhow::Result<WeixinConfig> {
    let mut weixin = existing.clone();

    session.note(
        &t(session.locale(), MessageId::GatewaySetupWeixinRiskTitle, &[]),
        &t(session.locale(), MessageId::GatewaySetupWeixinRiskNote, &[]),
    )?;

    let relogin = if !weixin.bot_token.is_empty() {
        session.confirm(
            &t(session.locale(), MessageId::GatewaySetupWeixinReloginPrompt, &[]),
            false,
        )?
    } else {
        true
    };

    if relogin {
        let base_url = weixin.base_url().to_string();
        session.note(
            &t(session.locale(), MessageId::WeixinQrWaitingNoteTitle, &[]),
            &t(session.locale(), MessageId::WeixinQrWaiting, &[]),
        )?;
        let qr = block_on_async(fetch_qr_code(&base_url, weixin.bot_type))?;

        println!();
        println!("{}", t(session.locale(), MessageId::WeixinQrPrompt, &[]));
        print_terminal_qr(session.locale(), &qr.qrcode_img_content)?;
        println!("{}", t(session.locale(), MessageId::WeixinQrWaiting, &[]));

        let qrcode = qr.qrcode.clone();
        let login = block_on_async(wait_for_qr_login(
            &base_url,
            &qrcode,
            Duration::from_secs(180),
            |status| match status {
                QrStatusKind::Scanned => {
                    println!("{}", t(session.locale(), MessageId::WeixinQrScanned, &[]))
                }
                QrStatusKind::Wait => {}
                QrStatusKind::Expired => {
                    println!("{}", t(session.locale(), MessageId::WeixinQrExpired, &[]))
                }
                QrStatusKind::Confirmed => {}
            },
        ))?;

        weixin.bot_token = login.bot_token;
        weixin.ilink_bot_id = login.ilink_bot_id;
        weixin.ilink_user_id = login.ilink_user_id;
        if !login.base_url.is_empty() {
            weixin.base_url = Some(login.base_url);
        }
        session.note(
            &t(session.locale(), MessageId::GatewaySetupWeixinLoginSuccessTitle, &[]),
            &t(
                session.locale(),
                MessageId::GatewaySetupWeixinLoginSuccessNote,
                &[weixin.ilink_user_id.clone()],
            ),
        )?;
    }

    if mode == GatewayWizardMode::Standalone {
        let welcome_default = weixin.welcome_message.clone().unwrap_or_default();
        let welcome = session.input(
            &t(session.locale(), MessageId::GatewaySetupWelcomePrompt, &[]),
            &welcome_default,
        )?;
        weixin.welcome_message = if welcome.is_empty() {
            None
        } else {
            Some(welcome)
        };
    }

    Ok(weixin)
}

fn configure_dm_policy(
    session: &Session,
    dm_policy: &mut String,
    allow_from: &mut Vec<String>,
    id_label: &str,
) -> anyhow::Result<()> {
    if session.confirm(&t(session.locale(), MessageId::GatewaySetupAllowlistPrompt, &[]), false)? {
        *dm_policy = "allowlist".into();
        loop {
            let ids = session.input(
                &t(
                    session.locale(),
                    MessageId::GatewaySetupAllowlistUsersPrompt,
                    &[id_label.to_string()],
                ),
                &allow_from.join(","),
            )?;
            *allow_from = ids
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            if !allow_from.is_empty() {
                break;
            }
            session.note(
                &t(session.locale(), MessageId::GatewaySetupAllowlistMissingTitle, &[]),
                &t(
                    session.locale(),
                    MessageId::GatewaySetupAllowlistMissingNote,
                    &[id_label.to_string()],
                ),
            )?;
        }
    } else {
        *dm_policy = "open".into();
        allow_from.clear();
        session.note(
            &t(session.locale(), MessageId::NoteTitleDmPolicy, &[]),
            &t(session.locale(), MessageId::GatewaySetupOpenModeNote, &[]),
        )?;
    }
    Ok(())
}

fn prompt_workspace(session: &Session, baseline_workspace: &str) -> anyhow::Result<String> {
    session.note(
        &t(session.locale(), MessageId::NoteTitleWorkspace, &[]),
        &t(session.locale(), MessageId::SetupWorkspaceNote, &[]),
    )?;
    let raw = session.input(
        &t(session.locale(), MessageId::SetupWorkspacePrompt, &[]),
        baseline_workspace,
    )?;
    let workspace = expand_path(&raw).display().to_string();
    std::fs::create_dir_all(&workspace)?;
    Ok(workspace)
}

fn apply_channel_config(
    session: &Session,
    channels: &mut ChannelsConfig,
    channel: &GatewayChannelKind,
    mode: GatewayWizardMode,
) -> anyhow::Result<()> {
    session.note(
        &t(
            session.locale(),
            MessageId::GatewaySetupCredentialNoteTitle,
            &[],
        ),
        &credential_note(session.locale(), channel),
    )?;
    match channel.id {
        CHANNEL_WECOM => {
            let existing = channels
                .wecom_account("default")
                .cloned()
                .unwrap_or_default();
            let wecom = configure_wecom(session, &existing, mode)?;
            wecom
                .validate_dm_access()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            channels.set_wecom_account("default", wecom);
        }
        CHANNEL_FEISHU => {
            let existing = channels
                .feishu_account("default")
                .cloned()
                .unwrap_or_default();
            let feishu = configure_feishu(session, &existing, mode)?;
            feishu
                .validate_dm_access()
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            channels.set_feishu_account("default", feishu);
        }
        CHANNEL_WEIXIN => {
            let existing = channels
                .weixin_account("default")
                .cloned()
                .unwrap_or_default();
            let weixin = configure_weixin(session, &existing, mode)?;
            channels.set_weixin_account("default", weixin);
        }
        _ => unreachable!("wizard only configures available channels"),
    }
    Ok(())
}

fn default_channel_from_existing(channels: &ChannelsConfig) -> Option<&str> {
    if channels.wecom_accounts().contains_key("default") {
        Some(CHANNEL_WECOM)
    } else if channels.feishu_accounts().contains_key("default") {
        Some(CHANNEL_FEISHU)
    } else if channels.weixin_accounts().contains_key("default") {
        Some(CHANNEL_WEIXIN)
    } else {
        None
    }
}

pub fn summarize_configured_channels(channels: &ChannelsConfig, locale: Locale) -> String {
    let mut parts = Vec::new();
    if channels
        .wecom_accounts()
        .get("default")
        .is_some_and(|c| !c.is_empty())
    {
        parts.push(t(locale, MessageId::ChannelSummaryWecom, &[]));
    }
    if channels
        .feishu_accounts()
        .get("default")
        .is_some_and(|c| !c.is_empty())
    {
        parts.push(t(locale, MessageId::ChannelSummaryFeishu, &[]));
    }
    if channels
        .weixin_accounts()
        .get("default")
        .is_some_and(|c| !c.is_empty())
    {
        parts.push(t(locale, MessageId::ChannelSummaryWeixin, &[]));
    }
    if parts.is_empty() {
        t(locale, MessageId::ChannelSummaryNone, &[])
    } else {
        parts.join(" + ")
    }
}

fn prompt_gateway_gate(session: &Session, channels: &ChannelsConfig) -> anyhow::Result<String> {
    let locale = session.locale();
    let summary = summarize_configured_channels(channels, locale);
    let none = t(locale, MessageId::ChannelSummaryNone, &[]);
    if summary != none {
        session.note(
            &t(locale, MessageId::GatewaySetupExistingChannelsTitle, &[]),
            &t(
                locale,
                MessageId::GatewaySetupExistingChannelsNote,
                std::slice::from_ref(&summary),
            ),
        )?;
    }

    let configure_label = t(locale, MessageId::GatewaySetupGateConfigureLabel, &[]);
    let configure_hint = t(locale, MessageId::GatewaySetupGateConfigureHint, &[]);
    let skip_label = t(locale, MessageId::GatewaySetupGateSkipLabel, &[]);
    let skip_hint = t(locale, MessageId::GatewaySetupGateSkipHint, &[]);
    let options = [
        SelectOption {
            value: GATE_CONFIGURE,
            label: &configure_label,
            hint: &configure_hint,
        },
        SelectOption {
            value: GATE_SKIP,
            label: &skip_label,
            hint: &skip_hint,
        },
    ];
    session
        .select_with(
            &t(locale, MessageId::GatewaySetupGatePrompt, &[]),
            &options,
            GATE_SKIP,
            false,
        )
        .map(str::to_string)
}

/// Optional gateway section embedded in `hi setup`.
///
/// Author: gz
pub fn prompt_embedded_gateway(
    session: &Session,
    baseline_workspace: &str,
    mut channels: ChannelsConfig,
) -> anyhow::Result<EmbeddedGatewayOutcome> {
    'gate: loop {
        match prompt_gateway_gate(session, &channels)? {
            choice if choice == GATE_SKIP => {
                return Ok(EmbeddedGatewayOutcome::Skipped {
                    workspace: baseline_workspace.to_string(),
                });
            }
            choice if choice == GATE_CONFIGURE => {}
            choice => anyhow::bail!("unexpected gateway gate choice: {choice}"),
        }

        let default_channel = gateway_channel_default(default_channel_from_existing(&channels));

        let channel = match choose_channel_with_back(session, default_channel)? {
            Ok(ch) => ch,
            Err(()) => continue 'gate,
        };

        let workspace = prompt_workspace(session, baseline_workspace)?;

        apply_channel_config(session, &mut channels, channel, GatewayWizardMode::Embedded)?;
        return Ok(EmbeddedGatewayOutcome::Configured {
            workspace,
            channels,
        });
    }
}

/// Gateway channel wizard (`hi gateway setup`).
///
/// Author: gz
pub fn run() -> anyhow::Result<()> {
    let app_path = Config::config_path();
    let locale = Config::load()
        .map(|c| c.resolved_locale())
        .unwrap_or_else(|_| resolve_locale(None));
    if !app_path.exists() {
        anyhow::bail!(
            "{}",
            t(
                locale,
                MessageId::GatewaySetupNeedBaseConfig,
                &[app_path.display().to_string()],
            )
        );
    }

    let channels = ChannelsConfig::load().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let updating = !channels.wecom_accounts().is_empty()
        || !channels.feishu_accounts().is_empty()
        || !channels.weixin_accounts().is_empty();

    let session = Session::new(app_path.clone(), locale);
    session.start(&if updating {
        t(locale, MessageId::GatewaySetupUpdateTitle, &[])
    } else {
        t(locale, MessageId::GatewaySetupTitle, &[])
    })?;

    session.note(
        &t(locale, MessageId::NoteTitleGatewaySetup, &[]),
        &t(
            locale,
            MessageId::GatewaySetupNote,
            &[app_path.display().to_string()],
        ),
    )?;

    let default_channel = gateway_channel_default(default_channel_from_existing(&channels));
    let channel = choose_channel(&session, default_channel)?;

    let mut channels = channels;
    apply_channel_config(&session, &mut channels, channel, GatewayWizardMode::Standalone)?;

    session.save(&t(locale, MessageId::GatewaySetupSaving, &[]), || channels.save())?;
    session.finish(&t(locale, MessageId::GatewaySetupFinish, &[]))?;
    Ok(())
}

#[cfg(test)]
#[path = "../../test/unit/config/gateway.rs"]
mod tests;
