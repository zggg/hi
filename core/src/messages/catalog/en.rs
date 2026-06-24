use super::super::id::MessageId;

fn arg(args: &[String], i: usize) -> &str {
    args.get(i).map(String::as_str).unwrap_or("")
}

pub(super) fn format_en(id: MessageId, args: &[String]) -> String {
    match id {
        // --- config / startup ---
        MessageId::MissingApiKey => {
            "missing LLM api_key in ~/.hi/hi.toml — set [ai.providers.<name>] or run `hi setup`".into()
        }
        MessageId::UnknownAiProvider => format!(
            "unknown ai.provider {:?} — use openai-compat | codex | anthropic | ollama",
            arg(args, 0)
        ),
        MessageId::UnknownModel => format!(
            "unknown model {:?} — available: {}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::ParseHiConfig => format!("parse hi config: {}", arg(args, 0)),
        MessageId::SerializeHiConfig => "serialize hi config failed".into(),
        MessageId::ReadCurrentDir => format!("read current directory: {}", arg(args, 0)),
        MessageId::CreateWorkspace => format!(
            "create workspace {}: {}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::InvalidWorkspace => format!(
            "invalid workspace {}: {}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::ConfigLock => format!("config lock: {}", arg(args, 0)),
        MessageId::ProviderLock => "provider lock failed".into(),
        MessageId::ApprovalPolicyLock => "approval policy lock failed".into(),
        MessageId::BuildAgent => format!("build agent: {}", arg(args, 0)),
        MessageId::ConfigNotSetup => {
            "not configured yet — run `hi setup` first".into()
        }
        MessageId::HiTomlPath => format!("hi.toml: {}", arg(args, 0)),

        // --- channels ---
        MessageId::ChannelsNotConfigured => {
            "no message channels configured — run `hi gateway setup`, then `hi gateway`".into()
        }
        MessageId::UnknownChannelId => format!(
            "unknown channel {:?} — available: wecom, feishu, weixin and :<account> forms",
            arg(args, 0)
        ),
        MessageId::WecomAccountMissing => format!(
            "wecom account {:?} not configured — add [channels.wecom] or [channels.wecom.{}] in hi.toml",
            arg(args, 0),
            arg(args, 0)
        ),
        MessageId::FeishuAccountMissing => format!(
            "feishu account {:?} not configured — add [channels.feishu] or [channels.feishu.{}] in hi.toml",
            arg(args, 0),
            arg(args, 0)
        ),
        MessageId::WeixinAccountMissing => format!(
            "weixin account {:?} not configured — add [channels.weixin] or [channels.weixin.{}] in hi.toml",
            arg(args, 0),
            arg(args, 0)
        ),
        MessageId::MissingWecomSecret => {
            "missing wecom secret — run `hi gateway setup` to fill secret".into()
        }
        MessageId::MissingWecomBotId => {
            "wecom.bot_id is empty — create a WeCom smart bot and set bot_id".into()
        }
        MessageId::MissingFeishuAppId => {
            "feishu.app_id is empty — run `hi gateway setup`".into()
        }
        MessageId::MissingFeishuAppSecret => {
            "feishu.app_secret is empty — run `hi gateway setup`".into()
        }
        MessageId::FeishuAllowlistEmpty => {
            "feishu allowlist is empty: no one can message the bot — set allow_from in hi.toml or run `hi gateway setup`".into()
        }
        MessageId::WecomAllowlistEmpty => {
            "wecom allowlist is empty: no one can message the bot — set allow_from in hi.toml or run `hi gateway setup`".into()
        }
        MessageId::WecomDmPolicyOpenWarn => {
            "wecom dm_policy=open: all users can trigger the agent; use allowlist in production".into()
        }
        MessageId::FeishuDmPolicyOpenWarn => {
            "feishu dm_policy=open: all users can trigger the agent; use allowlist in production".into()
        }
        MessageId::DefaultWelcome => {
            "Hi, I'm hi — your ultra-lightweight personal AI assistant. How can I help?".into()
        }

        // --- store / memory / agent ---
        MessageId::SchemaIncompatible => arg(args, 0).to_string(),
        MessageId::MemorySearchDisabled => {
            "memory_search: memory disabled or not persisted".into()
        }
        MessageId::MemoryQueryEmpty => "memory_search: query must not be empty".into(),
        MessageId::ToolIterationLimit => format!(
            "tool-call limit reached ({}) and no summary could be generated — narrow the task or raise context.max_tool_iterations",
            arg(args, 0)
        ),
        MessageId::MemoryRecorded => format!("memory recorded #{id}", id = arg(args, 0)),
        MessageId::MemoryDuplicate => "memory already exists (duplicate skipped)".into(),
        MessageId::EmptyChannelReply => "empty reply from agent".into(),
        MessageId::EmergencyTrimSummary => format!(
            "emergency trim: shortened {} message(s)",
            arg(args, 0)
        ),
        MessageId::ExtractKnotsFailed => format!("knot extraction failed: {}", arg(args, 0)),

        // --- approval ---
        MessageId::ApprovalPromptBash => format!(
            "⚠️ Confirmation required — reply approve/confirm or cancel:\nbash: {}\n→ add {}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::ApprovalPromptFile => format!(
            "⚠️ Confirmation required — reply approve/confirm or cancel:\n{}: {}\n→ add {}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),
        MessageId::ApprovalPromptGeneric => format!(
            "⚠️ Confirmation required — reply approve/confirm or cancel:\n{}: {}\n→ add {}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),

        // --- gateway replies ---
        MessageId::GatewayThinking => "Thinking…".into(),
        MessageId::GatewayBusy => "Still processing your last message — please wait…".into(),
        MessageId::GatewayTurnAck => "Got it — thinking…".into(),
        MessageId::GatewayProcessFailed => format!(
            "Something went wrong: {}\nPlease try again later.",
            arg(args, 0)
        ),
        MessageId::GatewayUnsupportedMessage => format!(
            "Unsupported {kind} message — please send text.",
            kind = arg(args, 0)
        ),
        MessageId::GatewayCheckOkWecom => "WeCom gateway check OK".into(),
        MessageId::GatewayCheckOkFeishu => "Feishu gateway check OK".into(),
        MessageId::GatewayCheckOkWeixin => "Weixin iLink gateway check OK".into(),
        MessageId::GatewayCheckOkGeneric => "Gateway check OK".into(),

        // --- gateway svc (CLI) ---
        MessageId::GatewayStarted => format!("gateway started (pid {})", arg(args, 0)),
        MessageId::GatewayLogsDir => format!("logs: {}", arg(args, 0)),
        MessageId::GatewayStopHint => "stop: hi gateway stop".into(),
        MessageId::GatewayNotRunning => "gateway is not running".into(),
        MessageId::GatewayStopped => format!("gateway stopped (pid {})", arg(args, 0)),
        MessageId::GatewayForceStopped => format!("gateway force-stopped (pid {})", arg(args, 0)),
        MessageId::GatewayPidFile => format!("pid file: {}", arg(args, 0)),
        MessageId::GatewayWorkspace => format!("workspace: {}", arg(args, 0)),
        MessageId::GatewayChannels => format!("channels: {}", arg(args, 0)),
        MessageId::GatewayChannelsNone => {
            "channels: (none — run `hi gateway setup`)".into()
        }
        MessageId::GatewayStatusRunning => format!("status: running (pid {})", arg(args, 0)),
        MessageId::GatewayStatusStopped => "status: stopped".into(),
        MessageId::GatewayRecentLog => format!("recent log: {}", arg(args, 0)),
        MessageId::GatewayReloadSent => format!(
            "notified gateway to reload hi.toml ([ai], [tools.approvals]) (pid {})",
            arg(args, 0)
        ),
        MessageId::GatewayStartFailed => format!("failed to start gateway: {}", arg(args, 0)),
        MessageId::GatewayPidParseFailed => format!("cannot parse gateway pid: {}", arg(args, 0)),
        MessageId::GatewayStopSignalFailed => format!("failed to send stop signal (pid {})", arg(args, 0)),
        MessageId::GatewayStopFailed => format!("failed to stop gateway (pid {})", arg(args, 0)),
        MessageId::GatewayReloadUnixOnly => {
            "gateway reload is Unix-only for now — use `hi gateway restart`".into()
        }
        MessageId::GatewayReloadSignalFailed => format!("failed to send reload signal (pid {})", arg(args, 0)),
        MessageId::GatewayRecentLogLine => format!("recent log: {}", arg(args, 0)),

        // --- chat / session CLI ---
        MessageId::ChatBanner => format!(
            "hi chat — Enter to send (/quit /reset /compact) | session={} | model={} | cwd={}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),
        MessageId::ContextReset => {
            "context cleared (full transcript kept in DB — use `hi session show` to view)".into()
        }
        MessageId::ModelSlashTuiOnly => "/model is TUI-only — use `hi` or `hi tui`".into(),
        MessageId::UnknownChatArg => format!(
            "unknown argument `{}`. Single turn: `hi chat word1 word2 …`; REPL: `hi chat`",
            arg(args, 0)
        ),
        MessageId::SessionListHeader => format!("{} session(s)", arg(args, 0)),
        MessageId::SessionListRow => format!(
            "{id:<24} {total:>8} {ctx:>8}  {wd}",
            id = arg(args, 0),
            total = arg(args, 1),
            ctx = arg(args, 2),
            wd = arg(args, 3)
        ),
        MessageId::SessionEmpty => "(no sessions)".into(),
        MessageId::SessionNotFound => format!("empty session or not found: {}", arg(args, 0)),
        MessageId::SessionExported => format!(
            "exported {} message(s) → {}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::SessionDeleted => format!("deleted session {} and all messages", arg(args, 0)),
        MessageId::SessionPurgeNeedConfirm => format!(
            "refused: add --confirm to permanently delete session {}",
            arg(args, 0)
        ),
        MessageId::CompressionListHeader => "compression events".into(),
        MessageId::CompressionRow => format!(
            "#{}  msgs {}..{}  ({} rows)  {}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2),
            arg(args, 3),
            arg(args, 4)
        ),
        MessageId::CompressionDetailHeader => format!("compression #{}", arg(args, 0)),
        MessageId::CompressionDetailSummary => arg(args, 0).to_string(),

        // --- memory CLI ---
        MessageId::MemoryListEmpty => format!("(no knots, owner={})", arg(args, 0)),
        MessageId::MemoryAdded => format!("knot added #{id}", id = arg(args, 0)),
        MessageId::MemoryForgotten => format!("knot forgotten #{id}", id = arg(args, 0)),
        MessageId::MemoryReinforced => format!("knot reinforced #{id}", id = arg(args, 0)),
        MessageId::MemoryExtractNone => format!(
            "session {} has no in_context messages to extract",
            arg(args, 0)
        ),
        MessageId::MemoryExtractDone => format!(
            "extraction done: added {}, skipped {}, superseded {}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),
        MessageId::MemoryListRow => format!(
            "{id:>6}  {kind:<12} {conf:<10} {clr:>5}  {content}",
            id = arg(args, 0),
            kind = arg(args, 1),
            conf = arg(args, 2),
            clr = arg(args, 3),
            content = arg(args, 4)
        ),
        MessageId::MemoryDisabledInConfig => {
            "[memory].enabled = false — enable memory in hi.toml".into()
        }
        MessageId::MemoryUnknownKind => format!(
            "unknown kind: {} (preference|fact|decision|task|procedure)",
            arg(args, 0)
        ),

        // --- LLM / provider ---
        MessageId::LlmHttpError => {
            let code = arg(args, 0);
            let detail = arg(args, 1);
            let hint = arg(args, 2);
            if detail.is_empty() {
                format!("LLM API request failed (HTTP {code}). {hint}")
            } else {
                format!("LLM API request failed (HTTP {code}): {detail}\n{hint}")
            }
        }
        MessageId::LlmTransportError => format!(
            "cannot reach LLM service — check network, proxy, and base_url in hi.toml.\nDetails: {}",
            arg(args, 0)
        ),
        MessageId::LlmReadBodyError => format!(
            "failed to read LLM response — connection may have dropped.\nDetails: {}",
            arg(args, 0)
        ),
        MessageId::LlmParseError => format!(
            "LLM returned an unparseable response — service or model name may be wrong.\nDetails: {}",
            arg(args, 0)
        ),
        MessageId::LlmStreamError => format!(
            "LLM stream read error — connection may have dropped.\nDetails: {}",
            arg(args, 0)
        ),
        MessageId::AnthropicNeedsApiKey => {
            "Anthropic requires an API key — run `hi setup`".into()
        }

        // --- stdin approval ---
        MessageId::StdinApprovalHeader => "⚠️  Confirmation required:".into(),
        MessageId::StdinApprovalPrompt => "Approve? [y/N]: ".into(),

        // --- wizard: shared ---
        MessageId::WizardWritten => format!("written to {}", arg(args, 0)),
        MessageId::WizardFinishSteps => arg(args, 0).to_string(),
        MessageId::WizardConfigPathLine => format!("Config file: {}", arg(args, 0)),
        MessageId::WizardSelectNumberPrompt => format!("Choose [{}]", arg(args, 0)),
        MessageId::WizardWriteFailed => "write failed".into(),
        MessageId::WizardFinishTitle => "Done".into(),
        MessageId::WizardCancelled => "setup wizard cancelled".into(),

        // --- wizard: setup ---
        MessageId::SetupTitle => "hi · setup wizard".into(),
        MessageId::SetupUpdateTitle => "hi · update configuration".into(),
        MessageId::SetupNotePath => format!(
            "Writes to {}.\nLLM is required; message channels are optional (hi gateway setup later).\nSecrets stay on this machine — do not commit to git.\nPress Enter to keep current values.",
            arg(args, 0)
        ),
        MessageId::SetupNoteRepeat => {
            "Press Enter to keep the current value; menus with «back» let you return.".into()
        }
        MessageId::SetupSummaryProvider => format!("Provider: {}", arg(args, 0)),
        MessageId::SetupSummaryModel => format!("Model: {}", arg(args, 0)),
        MessageId::SetupSummaryWorkspace => format!("Gateway workspace: {}", arg(args, 0)),
        MessageId::SetupSummaryApiKeySet => "API Key: set".into(),
        MessageId::SetupSummaryApiKeyMissing => "API Key: not set".into(),
        MessageId::SetupWorkspaceNote => {
            "Gateway workspace is for remote message channels.\nhi / hi tui / hi chat use the shell cwd, not this path.".into()
        }
        MessageId::SetupWorkspacePrompt => "Gateway workspace".into(),
        MessageId::SetupProviderPrompt => "Choose LLM provider".into(),
        MessageId::SetupOllamaUrlPrompt => "Ollama URL (without /v1 suffix)".into(),
        MessageId::SetupBaseUrlPrompt => "API base_url (empty = provider default)".into(),
        MessageId::SetupDeepseekKeyNote => {
            "DeepSeek API Key\nGet one at https://platform.deepseek.com/api_keys\nDocs: https://api-docs.deepseek.com/".into()
        }
        MessageId::SetupModelPrompt => "Choose model".into(),
        MessageId::SetupModelCustom => "Custom model name".into(),
        MessageId::SetupModelFetching => "Fetching available models…".into(),
        MessageId::SetupModelFetchDone => format!("Fetched {} models", arg(args, 0)),
        MessageId::SetupModelFetchFailedTitle => "Failed to fetch model list".into(),
        MessageId::SetupModelFetchFailedNote => format!(
            "Falling back to built-in list / manual input.\nDetail: {}",
            arg(args, 0)
        ),
        MessageId::SetupApiKeyPrompt => "API Key".into(),
        MessageId::SetupSaving => "Saving configuration…".into(),
        MessageId::SetupFinishNoGateway => "\
Configuration saved.

  hi chat hello          try a chat
  hi                     terminal UI
  hi gateway setup       add channels later
  hi config              view config (secrets redacted)

Tool approval rules were written to hi.toml; runtime grants append automatically."
            .into(),
        MessageId::SetupFinishWithGateway => "\
Configuration saved.

  hi chat hello          local chat
  hi gateway --check     connection check
  hi gateway             start gateway
  hi gateway setup       add or update channels
  hi config              view config (secrets redacted)

Gateway tools run in the workspace directory; local chat/tui use the shell cwd.
Tool approval rules were written to hi.toml; runtime grants append automatically."
            .into(),
        MessageId::NoteTitleSetup => "Setup".into(),
        MessageId::NoteTitleLlm => "LLM".into(),
        MessageId::NoteTitleDeepseekApiKey => "DeepSeek API Key".into(),
        MessageId::SetupCurrentSummaryTitle => "Current configuration".into(),
        MessageId::SetupSummaryChannels => format!("Channels: {}", arg(args, 0)),
        MessageId::SetupSummaryMaskedKey => "API Key: set (****)".into(),
        MessageId::SetupLlmSummaryBody => format!(
            "Provider: {p}\nModel: {m}\n{k}\n\nContinue to message channels →",
            p = arg(args, 0),
            m = arg(args, 1),
            k = arg(args, 2),
        ),
        MessageId::SetupCodexNotLoggedInTitle => "OpenAI Codex not signed in".into(),
        MessageId::SetupCodexSkippedTitle => "Codex skipped".into(),
        MessageId::SetupCodexSkippedNote => {
            "Did not switch to Codex this time — pick another provider.\nAfter `codex login`, choose OpenAI Codex again.".into()
        }
        MessageId::SetupCodexModelPrompt => "Choose Codex model".into(),
        MessageId::SetupCodexLoginHint => format!(
            "No local Codex credentials found:\n  {}\n\n\
             Sign in with the OpenAI Codex CLI (ChatGPT account):\n  codex login\n\n\
             Then re-run `hi setup` and choose OpenAI Codex.",
            arg(args, 0)
        ),
        MessageId::ProviderDeepseekHint => "Official API (platform.deepseek.com)".into(),
        MessageId::ProviderOpenaiCompatLabel => "OpenAI-compatible API".into(),
        MessageId::ProviderOpenaiCompatHint => {
            "OpenAI, Moonshot, self-hosted OpenAI-compatible endpoints".into()
        }
        MessageId::ProviderCodexHint => {
            "Reuse local Codex CLI login (ChatGPT) — no API key".into()
        }
        MessageId::ProviderOllamaLabel => "Ollama (local)".into(),
        MessageId::ProviderOllamaHint => "Runs on this machine — no API key".into(),
        MessageId::ModelHintFlagship => "Flagship".into(),
        MessageId::ModelHintReasonerDeprecated => "Reasoning mode, deprecated 2026-07".into(),
        MessageId::ModelHintDeprecatedJuly2026 => "Legacy, deprecated 2026-07".into(),
        MessageId::ModelHintSonnet4Recommended => "Sonnet 4 (recommended)".into(),
        MessageId::ModelBackToProviderLabel => "← Back to provider list".into(),
        MessageId::ModelBackToProviderHint => "Return to provider menu".into(),
        MessageId::ModelCustomLabel => "Custom model name…".into(),
        MessageId::ModelCustomHint => "Type model name manually".into(),
        MessageId::SetupModelNamePrompt => "Model name (model)".into(),

        // --- wizard: gateway ---
        MessageId::GatewaySetupTitle => "hi · message channel setup".into(),
        MessageId::GatewaySetupUpdateTitle => "hi · update message channels".into(),
        MessageId::GatewaySetupNote => format!(
            "Writes channel settings to {}.\nUse `hi setup` for LLM; this wizard adds or updates channels.\nPress Enter to keep current values.",
            arg(args, 0)
        ),
        MessageId::GatewaySetupChannelPrompt => "Choose message channel".into(),
        MessageId::GatewaySetupWecomCredsNote => {
            "WeCom: get Bot ID and Secret from Admin Console → Smart Bot → long-connection mode.\nSecret is shown once — keep it safe and out of git.".into()
        }
        MessageId::GatewaySetupBotIdPrompt => "Bot ID (wecom.bot_id)".into(),
        MessageId::GatewaySetupSecretPrompt => "Bot Secret (wecom.secret)".into(),
        MessageId::GatewaySetupAllowlistPrompt => "Enable DM allowlist?".into(),
        MessageId::GatewaySetupAllowlistUsersPrompt => format!(
            "Allowed user IDs ({label}, comma-separated, at least one)",
            label = arg(args, 0)
        ),
        MessageId::GatewaySetupOpenModeNote => {
            "Open mode: anyone can trigger the agent.\nUse only for dev; switch back to allowlist for production.".into()
        }
        MessageId::GatewaySetupWelcomePrompt => {
            "Welcome message (empty = system default)".into()
        }
        MessageId::GatewaySetupSaving => "Saving configuration…".into(),
        MessageId::GatewaySetupFinish => "\
Channel configuration saved.

Next steps:
  hi gateway --check    connection check
  hi gateway            start gateway
  hi config             view config (secrets redacted)"
            .into(),
        MessageId::GatewaySetupNeedBaseConfig => format!(
            "Complete base setup first:\n  hi setup\n\nConfig file: {}",
            arg(args, 0)
        ),
        MessageId::GatewaySetupNoChannels => "No message channels available in this build".into(),
        MessageId::GatewaySetupFeishuCredsNote => {
            "Feishu: create an app at https://open.feishu.cn/app,\n\
             enable bot, subscribe to im.message.receive_v1, use long-connection events.\n\
             Permissions: DM, group @ messages (or all group messages), send messages.\n\
             Add the bot to target groups; keep App Secret out of git.".into()
        }
        MessageId::GatewaySetupWeixinCredsNote => {
            "Personal WeChat iLink (experimental):\n\
             1. WeChat 8.0.70+ (iOS) / 8.0.69+ (Android)\n\
             2. Me → Settings → Plugins → enable iLink\n\
             3. This wizard shows a QR code for authorization\n\
             4. Prefer a secondary account; keep bot_token out of git\n\
             See docs/guides/weixin-ilink-integration.md".into()
        }
        MessageId::GatewaySetupGenericCredsNote => {
            "Prepare credentials per your platform documentation.".into()
        }
        MessageId::GatewaySetupFeishuAppIdPrompt => "App ID (feishu.app_id)".into(),
        MessageId::GatewaySetupFeishuSecretPrompt => "App Secret (feishu.app_secret)".into(),
        MessageId::GatewaySetupFeishuDomainPrompt => {
            "API domain (empty = open.feishu.cn; Lark international = open.larksuite.com)".into()
        }
        MessageId::GatewaySetupFeishuMentionPrompt => {
            "Group chat: reply only when @mentioned (mention_enabled, default true)?".into()
        }
        MessageId::GatewaySetupFeishuMentionOnTitle => "@mention".into(),
        MessageId::GatewaySetupFeishuMentionOnNote => {
            "When enabled, the bot replies in groups only when @mentioned \
             (permission: im:message.group_at_msg:readonly).\n\
             Set mention_enabled = false in hi.toml to reply to all group messages.".into()
        }
        MessageId::GatewaySetupFeishuMentionOffTitle => "Open group chat".into(),
        MessageId::GatewaySetupFeishuMentionOffNote => {
            "With @ restriction off, the bot may reply to all group text messages.\n\
             Grant im:message.group_msg on Feishu Open Platform and republish the app.".into()
        }
        MessageId::GatewaySetupWeixinRiskTitle => "Risk notice".into(),
        MessageId::GatewaySetupWeixinRiskNote => {
            "Personal WeChat iLink is in gray release; Tencent may change or end the service.\n\
             Prefer a secondary account; read the iLink terms of use.".into()
        }
        MessageId::GatewaySetupWeixinReloginPrompt => {
            "bot_token already set — scan QR again to re-login?".into()
        }
        MessageId::GatewaySetupWeixinLoginSuccessTitle => "Login successful".into(),
        MessageId::GatewaySetupWeixinLoginSuccessNote => format!(
            "Bound WeChat user: {}\nAfter `hi gateway` starts, DM via the iLink plugin on your phone.",
            arg(args, 0)
        ),
        MessageId::GatewaySetupAllowlistMissingTitle => "User ID required".into(),
        MessageId::GatewaySetupAllowlistMissingNote => format!(
            "Allowlist mode requires at least one {}.\n\
             Feishu open_id: event logs or API debugger; WeCom userid: admin console.",
            arg(args, 0)
        ),
        MessageId::GatewaySetupCredentialNoteTitle => "Credentials".into(),
        MessageId::GatewaySetupExistingChannelsTitle => "Configured channels".into(),
        MessageId::GatewaySetupExistingChannelsNote => format!(
            "Current: {}\nChoose configure now to update the default instance; \
             add other instances with `hi gateway setup`.",
            arg(args, 0)
        ),
        MessageId::GatewaySetupGatePrompt => "Configure message channels?".into(),
        MessageId::GatewaySetupGateConfigureLabel => "Configure now".into(),
        MessageId::GatewaySetupGateConfigureHint => "WeCom, Feishu, or personal WeChat".into(),
        MessageId::GatewaySetupGateSkipLabel => "Skip — local hi chat / tui only".into(),
        MessageId::GatewaySetupGateSkipHint => "Run hi gateway setup later".into(),
        MessageId::ChannelSummaryWecom => "WeCom".into(),
        MessageId::ChannelSummaryFeishu => "Feishu".into(),
        MessageId::ChannelSummaryWeixin => "Personal WeChat".into(),
        MessageId::ChannelSummaryNone => "Not configured".into(),
        MessageId::QrGenerateFailed => format!("Failed to render terminal QR code: {}", arg(args, 0)),
        MessageId::WizardBack => "← Back".into(),
        MessageId::WizardSelectEmpty => format!("{}: no options", arg(args, 0)),
        MessageId::WizardPasswordKeepSuffix => " (Enter to keep current)".into(),
        MessageId::NoteTitleWorkspace => "Workspace".into(),
        MessageId::NoteTitleGatewaySetup => "Gateway setup".into(),
        MessageId::NoteTitleDmPolicy => "DM policy".into(),
        MessageId::NoteTitleQr => "QR code".into(),
        MessageId::WeixinQrPrompt => "Scan the QR code below with WeChat:".into(),
        MessageId::WeixinQrWaitingNoteTitle => "QR code".into(),
        MessageId::WeixinQrWaiting => "Waiting for scan (up to 3 minutes)…".into(),
        MessageId::WeixinQrScanned => "Scanned — confirm on your phone…".into(),
        MessageId::WeixinQrExpired => "QR code expired".into(),

        // --- gateway channel labels ---
        MessageId::ChannelWecomLabel => "WeCom smart bot".into(),
        MessageId::ChannelWecomHint => "WebSocket long connection · supported".into(),
        MessageId::ChannelFeishuLabel => "Feishu bot".into(),
        MessageId::ChannelFeishuHint => "WebSocket long connection · supported".into(),
        MessageId::ChannelWeixinLabel => "Personal WeChat (iLink)".into(),
        MessageId::ChannelWeixinHint => "iLink polling · experimental · requires phone plugin".into(),

        // --- model preset hints ---
        MessageId::ModelHintRecommended => "recommended".into(),
        MessageId::ModelHintLegacy => "legacy".into(),

        // --- TUI ---
        MessageId::TuiBusyWaitingModel => "Waiting for model…".into(),
        MessageId::TuiBusyGenerating => "Generating reply".into(),
        MessageId::TuiBusyThinking => "Thinking".into(),
        MessageId::TuiBusyMemoryExtract => "Extracting memory".into(),
        MessageId::TuiBusyCompress => "Compressing context".into(),
        MessageId::TuiInterrupted => "Interrupted".into(),
        MessageId::TuiContextReset => {
            "Context cleared (transcript kept in database)".into()
        }
        MessageId::TuiModelActivated => format!(
            "Model switched: {name} · {model} (saved to hi.toml)",
            name = arg(args, 0),
            model = arg(args, 1)
        ),
        MessageId::TuiModelUnknown => format!(
            "No matching provider instance — check hi.toml [ai.providers] ({})",
            arg(args, 0)
        ),
        MessageId::TuiStatusDefault => format!(
            "←→ move cursor · Enter send · Ctrl+J newline · Ctrl+C exit · model {}",
            arg(args, 0)
        ),
        MessageId::TuiStatusSlashMenu => "↑↓ select · Tab/Enter fill input".into(),
        MessageId::TuiStatusModelMenu => "↑↓ select model · Enter activate · Esc cancel".into(),
        MessageId::TuiSlashModel => "/model".into(),
        MessageId::TuiSlashModelDesc => "Pick and switch a configured model instance".into(),
        MessageId::TuiSlashReset => "/reset".into(),
        MessageId::TuiSlashResetDesc => "Clear agent-visible context (alias /clear)".into(),
        MessageId::TuiSlashCompact => "/compact".into(),
        MessageId::TuiSlashCompactDesc => "Force trim and LLM-summarize context".into(),
        MessageId::TuiSlashVerbose => "/verbose".into(),
        MessageId::TuiSlashVerboseDesc => "Toggle verbose: stream full think & tool output".into(),
        MessageId::TuiVerboseOn => "Verbose: on · think & tool output stream in full".into(),
        MessageId::TuiVerboseOff => "Verbose: off · summary lines only".into(),
        MessageId::TuiExitResumeHint => format!(
            "Session saved · session={s} · resume: hi -s {s}",
            s = arg(args, 0)
        ),
        MessageId::TuiModelCurrentSuffix => " (current)".into(),
        MessageId::TuiApprovalTitle => "Approval".into(),
        MessageId::TuiApprovalApprove => "y approve".into(),
        MessageId::TuiApprovalReject => "n/Esc reject".into(),
        MessageId::TuiQueueWaiting => format!(
            "Queued for the next turn: {}",
            arg(args, 0)
        ),

        // --- tool budget ---
        MessageId::ToolBudgetReminder => format!(
            "[Budget: {}/{} — ~{} turn(s) left; wrap up and reply soon.]",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),
        MessageId::ToolBudgetForceSummary => {
            "(System: tool-call limit reached for this turn. Summarize progress, findings, and remaining work without calling any tools. Reply in English.)".into()
        }
        MessageId::ToolBudgetSummaryPrefix => format!(
            "(Tool-call limit reached ({max}/{max}); summary below)",
            max = arg(args, 0)
        ),
    }
}
