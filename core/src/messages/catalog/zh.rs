use super::super::id::MessageId;

fn arg(args: &[String], i: usize) -> &str {
    args.get(i).map(String::as_str).unwrap_or("")
}

pub(super) fn format_zh(id: MessageId, args: &[String]) -> String {
    match id {
        // --- config / startup ---
        MessageId::MissingApiKey => {
            "missing LLM api_key in ~/.hi/hi.toml — 在 [ai.providers.<name>] 填写或运行 `hi setup`".into()
        }
        MessageId::UnknownAiProvider => format!(
            "unknown ai.provider {:?} — 请使用 openai-compat | codex | anthropic | ollama",
            arg(args, 0)
        ),
        MessageId::UnknownModel => format!(
            "未知模型 {:?} — 可选：{}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::ParseHiConfig => format!("解析 hi 配置失败：{}", arg(args, 0)),
        MessageId::SerializeHiConfig => "序列化 hi 配置失败".into(),
        MessageId::ReadCurrentDir => format!("读取当前目录失败：{}", arg(args, 0)),
        MessageId::CreateWorkspace => format!(
            "创建工作目录 {} 失败：{}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::InvalidWorkspace => format!(
            "无效工作目录 {}：{}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::ConfigLock => format!("配置锁失败：{}", arg(args, 0)),
        MessageId::ProviderLock => "Provider 锁失败".into(),
        MessageId::ApprovalPolicyLock => "审批策略锁失败".into(),
        MessageId::BuildAgent => format!("构建 Agent 失败：{}", arg(args, 0)),
        MessageId::ConfigNotSetup => "尚未配置 — 请先运行 `hi setup`".into(),
        MessageId::HiTomlPath => format!("hi.toml: {}", arg(args, 0)),

        // --- channels ---
        MessageId::ChannelsNotConfigured => {
            "未配置消息渠道：运行 `hi gateway setup` 后再执行 `hi gateway`".into()
        }
        MessageId::UnknownChannelId => format!(
            "未知渠道 {:?} — 当前可用：wecom、feishu、weixin 及对应 :<账户名> 形式",
            arg(args, 0)
        ),
        MessageId::WecomAccountMissing => format!(
            "未配置 wecom 账户 {:?} — 在 hi.toml 添加 [channels.wecom] 或 [channels.wecom.{}]",
            arg(args, 0),
            arg(args, 0)
        ),
        MessageId::FeishuAccountMissing => format!(
            "未配置 feishu 账户 {:?} — 在 hi.toml 添加 [channels.feishu] 或 [channels.feishu.{}]",
            arg(args, 0),
            arg(args, 0)
        ),
        MessageId::WeixinAccountMissing => format!(
            "未配置 weixin 账户 {:?} — 在 hi.toml 添加 [channels.weixin] 或 [channels.weixin.{}]",
            arg(args, 0),
            arg(args, 0)
        ),
        MessageId::MissingWecomSecret => {
            "missing wecom secret — 运行 `hi gateway setup` 填写 secret".into()
        }
        MessageId::MissingWecomBotId => {
            "wecom.bot_id 为空 — 在企微管理后台创建「智能机器人」并填写 bot_id".into()
        }
        MessageId::MissingFeishuAppId => {
            "feishu.app_id 为空 — 运行 `hi gateway setup`".into()
        }
        MessageId::MissingFeishuAppSecret => {
            "feishu.app_secret 为空 — 运行 `hi gateway setup`".into()
        }
        MessageId::FeishuAllowlistEmpty => {
            "feishu allowlist 为空：无人可发消息，请在 hi.toml 配置 allow_from 或运行 `hi gateway setup`".into()
        }
        MessageId::WecomAllowlistEmpty => {
            "wecom allowlist 为空：无人可发消息，请在 hi.toml 配置 allow_from 或运行 `hi gateway setup`".into()
        }
        MessageId::WecomDmPolicyOpenWarn => {
            "wecom dm_policy=open：所有用户可触发 Agent；生产环境请改用 allowlist".into()
        }
        MessageId::FeishuDmPolicyOpenWarn => {
            "feishu dm_policy=open：所有用户可触发 Agent；生产环境请改用 allowlist".into()
        }
        MessageId::DefaultWelcome => {
            "你好，我是 hi，极致轻量化的个人 AI 助手，有什么可以帮您？".into()
        }

        // --- store / memory / agent ---
        MessageId::SchemaIncompatible => arg(args, 0).to_string(),
        MessageId::MemorySearchDisabled => "memory_search: 记忆未启用或未持久化".into(),
        MessageId::MemoryQueryEmpty => "memory_search: query 不能为空".into(),
        MessageId::ToolIterationLimit => format!(
            "本轮工具调用已达上限（{} 轮），且无法生成总结；请缩小任务或调高 context.max_tool_iterations 后重试",
            arg(args, 0)
        ),
        MessageId::MemoryRecorded => format!("已记录记忆 #{id}", id = arg(args, 0)),
        MessageId::MemoryDuplicate => "记忆已存在（重复已跳过）".into(),
        MessageId::EmptyChannelReply => "Agent 返回空回复".into(),
        MessageId::EmergencyTrimSummary => format!("紧急裁剪：截短 {} 条消息", arg(args, 0)),
        MessageId::ExtractKnotsFailed => format!("结绳抽取失败：{}", arg(args, 0)),

        // --- approval ---
        MessageId::ApprovalPromptBash => format!(
            "⚠️ 需确认，请回复「确认」执行或「取消」放弃:\nbash: {}\n→ 加入 {}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::ApprovalPromptFile => format!(
            "⚠️ 需确认，请回复「确认」执行或「取消」放弃:\n{}: {}\n→ 加入 {}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),
        MessageId::ApprovalPromptGeneric => format!(
            "⚠️ 需确认，请回复「确认」执行或「取消」放弃:\n{}: {}\n→ 加入 {}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),

        // --- gateway replies ---
        MessageId::GatewayThinking => "思考中…".into(),
        MessageId::GatewayBusy => "上一条还在处理，请稍候…".into(),
        MessageId::GatewayTurnAck => "收到，思考中…".into(),
        MessageId::GatewayProcessFailed => format!(
            "处理失败：{}\n请稍后重试。",
            arg(args, 0)
        ),
        MessageId::GatewayUnsupportedMessage => format!(
            "暂不支持 {kind} 消息，请发送文本。",
            kind = arg(args, 0)
        ),
        MessageId::GatewayCheckOkWecom => "企业微信网关预检通过".into(),
        MessageId::GatewayCheckOkFeishu => "飞书网关预检通过".into(),
        MessageId::GatewayCheckOkWeixin => "个人微信 iLink 网关预检通过".into(),
        MessageId::GatewayCheckOkGeneric => "网关预检通过".into(),

        // --- gateway svc (CLI) ---
        MessageId::GatewayStarted => format!("gateway 已启动 (pid {})", arg(args, 0)),
        MessageId::GatewayLogsDir => format!("日志目录: {}", arg(args, 0)),
        MessageId::GatewayStopHint => "停止: hi gateway stop".into(),
        MessageId::GatewayNotRunning => "gateway 未运行".into(),
        MessageId::GatewayStopped => format!("gateway 已停止 (pid {})", arg(args, 0)),
        MessageId::GatewayForceStopped => format!("gateway 已强制停止 (pid {})", arg(args, 0)),
        MessageId::GatewayPidFile => format!("pid 文件: {}", arg(args, 0)),
        MessageId::GatewayWorkspace => format!("workspace: {}", arg(args, 0)),
        MessageId::GatewayChannels => format!("渠道: {}", arg(args, 0)),
        MessageId::GatewayChannelsNone => "渠道: （未配置 — 运行 hi gateway setup）".into(),
        MessageId::GatewayStatusRunning => format!("状态: 运行中 (pid {})", arg(args, 0)),
        MessageId::GatewayStatusStopped => "状态: 未运行".into(),
        MessageId::GatewayRecentLog => format!("最近日志: {}", arg(args, 0)),
        MessageId::GatewayReloadSent => format!(
            "已通知 gateway 重新加载 hi.toml（[ai]、[tools.approvals]）(pid {})",
            arg(args, 0)
        ),
        MessageId::GatewayStartFailed => format!("启动 gateway 失败: {}", arg(args, 0)),
        MessageId::GatewayPidParseFailed => format!("无法解析 gateway pid: {}", arg(args, 0)),
        MessageId::GatewayStopSignalFailed => format!("发送停止信号失败 (pid {})", arg(args, 0)),
        MessageId::GatewayStopFailed => format!("停止 gateway 失败 (pid {})", arg(args, 0)),
        MessageId::GatewayReloadUnixOnly => {
            "gateway reload 当前仅支持 Unix，请使用 `hi gateway restart`".into()
        }
        MessageId::GatewayReloadSignalFailed => format!("发送 reload 信号失败 (pid {})", arg(args, 0)),
        MessageId::GatewayRecentLogLine => format!("最近日志: {}", arg(args, 0)),

        // --- chat / session CLI ---
        MessageId::ChatBanner => format!(
            "hi chat — Enter 发送 (/quit /reset /compact) | session={} | model={} | cwd={}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),
        MessageId::ContextReset => {
            "上下文已清空（完整 transcript 仍保留在数据库，可用 hi session show 查看）".into()
        }
        MessageId::ModelSlashTuiOnly => "/model 仅 TUI 可用，请使用 hi 或 hi tui".into(),
        MessageId::UnknownChatArg => format!(
            "未知参数 `{}`。单轮：`hi chat 词1 词2 …`；多轮：`hi chat`",
            arg(args, 0)
        ),
        MessageId::SessionListHeader => format!("{} 个会话", arg(args, 0)),
        MessageId::SessionListRow => format!(
            "{id:<24} {total:>8} {ctx:>8}  {preview}",
            id = arg(args, 0),
            total = arg(args, 1),
            ctx = arg(args, 2),
            preview = arg(args, 3)
        ),
        MessageId::SessionEmpty => "（无会话）".into(),
        MessageId::SessionNotFound => format!("（空会话或无此 session: {}）", arg(args, 0)),
        MessageId::SessionExported => format!(
            "已导出 {} 条消息 → {}",
            arg(args, 0),
            arg(args, 1)
        ),
        MessageId::SessionDeleted => format!("已删除 session {} 及其全部 messages", arg(args, 0)),
        MessageId::SessionPurgeNeedConfirm => {
            format!("拒绝执行：须加 --confirm 才会永久删除 session {}", arg(args, 0))
        }
        MessageId::CompressionListHeader => "压缩记录".into(),
        MessageId::CompressionRow => format!(
            "#{}  msgs {}..{}  ({} rows)  {}",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2),
            arg(args, 3),
            arg(args, 4)
        ),
        MessageId::CompressionDetailHeader => format!("压缩 #{}", arg(args, 0)),
        MessageId::CompressionDetailSummary => arg(args, 0).to_string(),

        // --- memory CLI ---
        MessageId::MemoryListEmpty => format!("（无结绳记忆，owner={}）", arg(args, 0)),
        MessageId::MemoryAdded => format!("已打结 #{id}", id = arg(args, 0)),
        MessageId::MemoryForgotten => format!("已遗忘结 #{id}", id = arg(args, 0)),
        MessageId::MemoryReinforced => format!("已强化结 #{id}", id = arg(args, 0)),
        MessageId::MemoryExtractNone => format!("会话 {} 无 in_context 消息可抽取", arg(args, 0)),
        MessageId::MemoryExtractDone => format!(
            "抽取完成：新增 {}，跳过 {}，取代 {}",
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
            "[memory].enabled = false，请在 hi.toml 中启用记忆".into()
        }
        MessageId::MemoryUnknownKind => format!(
            "未知 kind: {}（preference|fact|decision|task|procedure）",
            arg(args, 0)
        ),

        // --- LLM / provider ---
        MessageId::LlmHttpError => {
            let code = arg(args, 0);
            let detail = arg(args, 1);
            let hint = arg(args, 2);
            if detail.is_empty() {
                format!("大模型 API 请求失败（HTTP {code}）。{hint}")
            } else {
                format!("大模型 API 请求失败（HTTP {code}）：{detail}\n{hint}")
            }
        }
        MessageId::LlmTransportError => format!(
            "无法连接大模型服务，请检查网络、代理，以及 hi.toml 中的 base_url。\n详情：{}",
            arg(args, 0)
        ),
        MessageId::LlmReadBodyError => format!(
            "读取大模型响应失败，连接可能已中断。\n详情：{}",
            arg(args, 0)
        ),
        MessageId::LlmParseError => format!(
            "大模型返回了无法解析的响应，可能是服务异常或 model 名称不正确。\n详情：{}",
            arg(args, 0)
        ),
        MessageId::LlmStreamError => format!(
            "读取大模型流式响应时出错，连接可能已中断。\n详情：{}",
            arg(args, 0)
        ),
        MessageId::AnthropicNeedsApiKey => "Anthropic 需要 API Key，请运行 hi setup 配置。".into(),

        // --- stdin approval ---
        MessageId::StdinApprovalHeader => "⚠️  需要确认:".into(),
        MessageId::StdinApprovalPrompt => "Approve? [y/N]: ".into(),

        // --- wizard: shared ---
        MessageId::WizardWritten => format!("已写入 {}", arg(args, 0)),
        MessageId::WizardFinishSteps => arg(args, 0).to_string(),
        MessageId::WizardConfigPathLine => format!("配置文件: {}", arg(args, 0)),
        MessageId::WizardSelectNumberPrompt => format!("请选择 [{}]", arg(args, 0)),
        MessageId::WizardWriteFailed => "写入失败".into(),
        MessageId::WizardFinishTitle => "完成".into(),
        MessageId::WizardCancelled => "已取消配置向导".into(),

        // --- wizard: setup ---
        MessageId::SetupTitle => "hi · 配置向导".into(),
        MessageId::SetupUpdateTitle => "hi · 更新配置".into(),
        MessageId::SetupNotePath => format!(
            "写入 {}。\nLLM 必填；消息渠道可选，稍后也可 hi gateway setup 补配。\n密钥仅存本机，勿提交 Git。\n回车保留当前值；带「返回」的菜单可选上一项。",
            arg(args, 0)
        ),
        MessageId::SetupNoteRepeat => {
            "回车保留当前值；带「返回」的菜单可选上一项。".into()
        }
        MessageId::SetupSummaryProvider => format!("Provider：{}", arg(args, 0)),
        MessageId::SetupSummaryModel => format!("模型：{}", arg(args, 0)),
        MessageId::SetupSummaryWorkspace => format!("Gateway 工作目录：{}", arg(args, 0)),
        MessageId::SetupSummaryApiKeySet => "API Key：已设置".into(),
        MessageId::SetupSummaryApiKeyMissing => "API Key：未设置".into(),
        MessageId::SetupWorkspaceNote => {
            "此项为 hi gateway 远程消息渠道使用的工作目录。\nhi / hi tui / hi chat 使用命令行当前工作目录，与此项无关。".into()
        }
        MessageId::SetupWorkspacePrompt => "Gateway 工作目录（workspace）".into(),
        MessageId::SetupProviderPrompt => "请选择大模型 Provider".into(),
        MessageId::SetupOllamaUrlPrompt => "Ollama 服务地址（不含 /v1 后缀）".into(),
        MessageId::SetupBaseUrlPrompt => "API 基础地址 base_url（留空则使用 Provider 默认值）".into(),
        MessageId::SetupDeepseekKeyNote => {
            "DeepSeek API Key\n在 https://platform.deepseek.com/api_keys 申请 API Key。\n官方文档：https://api-docs.deepseek.com/zh-cn/".into()
        }
        MessageId::SetupModelPrompt => "请选择模型".into(),
        MessageId::SetupModelCustom => "自定义模型名称".into(),
        MessageId::SetupApiKeyPrompt => "API Key".into(),
        MessageId::SetupSaving => "正在保存配置…".into(),
        MessageId::SetupFinishNoGateway => "\
配置已保存。

  hi chat 你好           试聊验收
  hi                     终端 UI
  hi gateway setup       以后补配消息渠道
  hi config              查看配置（密钥脱敏）

工具确认规则已写入 hi.toml，运行时确认会自动追加。"
            .into(),
        MessageId::SetupFinishWithGateway => "\
配置已保存。

  hi chat 你好           本地试聊
  hi gateway --check     渠道连接预检
  hi gateway             启动网关
  hi gateway setup       新增或修改渠道
  hi config              查看配置（密钥脱敏）

Gateway 工具在 workspace 目录执行；本地 chat/tui 在当前目录。
工具确认规则已写入 hi.toml，运行时确认会自动追加。"
            .into(),
        MessageId::NoteTitleSetup => "配置向导".into(),
        MessageId::NoteTitleLlm => "大模型".into(),
        MessageId::NoteTitleDeepseekApiKey => "DeepSeek API Key".into(),
        MessageId::SetupCurrentSummaryTitle => "当前配置摘要".into(),
        MessageId::SetupSummaryChannels => format!("消息渠道：{}", arg(args, 0)),
        MessageId::SetupSummaryMaskedKey => "API Key：已设置（****）".into(),
        MessageId::SetupLlmSummaryBody => format!(
            "Provider：{p}\n模型：{m}\n{k}\n\n继续配置消息渠道 →",
            p = arg(args, 0),
            m = arg(args, 1),
            k = arg(args, 2),
        ),
        MessageId::SetupCodexNotLoggedInTitle => "OpenAI Codex 未登录".into(),
        MessageId::SetupCodexSkippedTitle => "已跳过 Codex".into(),
        MessageId::SetupCodexSkippedNote => {
            "本次未切换到 Codex，请重选 Provider。\n完成 `codex login` 后重新选择 OpenAI Codex 即可。".into()
        }
        MessageId::SetupCodexModelPrompt => "选择 Codex 模型".into(),
        MessageId::SetupCodexLoginHint => format!(
            "未检测到本地 Codex 登录凭证：\n  {}\n\n\
             请先用 OpenAI Codex CLI 登录（ChatGPT 账号）：\n  codex login\n\n\
             登录完成后重新运行 `hi setup`，选择 OpenAI Codex 即可进入模型选择。",
            arg(args, 0)
        ),
        MessageId::ProviderDeepseekHint => "官方 API（platform.deepseek.com）".into(),
        MessageId::ProviderOpenaiCompatLabel => "OpenAI 兼容接口".into(),
        MessageId::ProviderOpenaiCompatHint => {
            "OpenAI、Moonshot、自部署 OpenAI 兼容服务".into()
        }
        MessageId::ProviderCodexHint => "复用本地 Codex CLI 登录（ChatGPT），无需 API Key".into(),
        MessageId::ProviderOllamaLabel => "Ollama 本地推理".into(),
        MessageId::ProviderOllamaHint => "本机部署，无需 API Key".into(),
        MessageId::ModelHintFlagship => "旗舰".into(),
        MessageId::ModelHintReasonerDeprecated => "思考模式，2026/07 弃用".into(),
        MessageId::ModelHintDeprecatedJuly2026 => "旧版，2026/07 弃用".into(),
        MessageId::ModelHintSonnet4Recommended => "Sonnet 4（推荐）".into(),
        MessageId::ModelBackToProviderLabel => "← 返回重新选择 Provider".into(),
        MessageId::ModelBackToProviderHint => "回到厂商列表".into(),
        MessageId::ModelCustomLabel => "自定义模型名…".into(),
        MessageId::ModelCustomHint => "手动输入 model 名称".into(),
        MessageId::SetupModelNamePrompt => "模型名称（model）".into(),

        // --- wizard: gateway ---
        MessageId::GatewaySetupTitle => "hi · 消息渠道配置".into(),
        MessageId::GatewaySetupUpdateTitle => "hi · 更新消息渠道配置".into(),
        MessageId::GatewaySetupNote => format!(
            "本向导将写入 {} 中的消息渠道相关配置。\n\n大模型请用 `hi setup`；此处用于新增或更新消息渠道。\n带默认值的项直接按回车即可保留当前值。",
            arg(args, 0)
        ),
        MessageId::GatewaySetupChannelPrompt => "请选择消息渠道".into(),
        MessageId::GatewaySetupWecomCredsNote => {
            "请在企业微信管理后台「智能机器人 → 长连接模式」中获取 Bot ID 与 Secret。\nSecret 仅展示一次，请妥善保管，勿提交至版本库。".into()
        }
        MessageId::GatewaySetupBotIdPrompt => "Bot ID（wecom.bot_id）".into(),
        MessageId::GatewaySetupSecretPrompt => "Bot Secret（wecom.secret）".into(),
        MessageId::GatewaySetupAllowlistPrompt => "是否启用私信白名单？".into(),
        MessageId::GatewaySetupAllowlistUsersPrompt => format!(
            "允许发消息的用户 ID（{}，逗号分隔，至少一个）",
            arg(args, 0)
        ),
        MessageId::GatewaySetupOpenModeNote => {
            "开放模式：所有用户均可触发 Agent。\n仅建议在开发联调时使用；生产环境请改回 allowlist。".into()
        }
        MessageId::GatewaySetupWelcomePrompt => "欢迎语（留空则使用系统默认）".into(),
        MessageId::GatewaySetupSaving => "正在保存配置…".into(),
        MessageId::GatewaySetupFinish => "\
消息渠道配置已保存。

后续步骤：
  hi gateway --check    连接预检
  hi gateway            启动网关
  hi config             查看配置（密钥脱敏）"
            .into(),
        MessageId::GatewaySetupNeedBaseConfig => format!(
            "请先完成基础配置：\n  hi setup\n\n配置文件：{}",
            arg(args, 0)
        ),
        MessageId::GatewaySetupNoChannels => "当前版本未提供可用的消息渠道".into(),
        MessageId::GatewaySetupFeishuCredsNote => {
            "请在飞书开放平台（https://open.feishu.cn/app）创建企业自建应用，\n\
             开启机器人能力，订阅 im.message.receive_v1，事件订阅方式选「使用长连接接收事件」。\n\
             权限：单聊消息、群聊 @ 消息（或群聊全部消息）、发送消息。\n\
             将机器人加入目标群聊后即可在群内对话；App Secret 勿提交至版本库。".into()
        }
        MessageId::GatewaySetupWeixinCredsNote => {
            "个人微信 iLink（实验性）：\n\
             1. 手机微信升级到 8.0.70+（iOS）/ 8.0.69+（Android）\n\
             2. 我 → 设置 → 插件 → 启用 iLink 插件\n\
             3. 本向导将生成二维码，用微信扫码授权\n\
             4. 建议用小号绑定；bot_token 勿提交至版本库\n\
             详见 docs/guides/weixin-ilink-integration.md".into()
        }
        MessageId::GatewaySetupGenericCredsNote => "请按平台文档准备连接凭证。".into(),
        MessageId::GatewaySetupFeishuAppIdPrompt => "App ID（feishu.app_id）".into(),
        MessageId::GatewaySetupFeishuSecretPrompt => "App Secret（feishu.app_secret）".into(),
        MessageId::GatewaySetupFeishuDomainPrompt => {
            "API 域名（留空默认 open.feishu.cn；国际版 Lark 填 open.larksuite.com）".into()
        }
        MessageId::GatewaySetupFeishuMentionPrompt => {
            "群聊是否仅响应 @机器人（mention_enabled，默认 true）".into()
        }
        MessageId::GatewaySetupFeishuMentionOnTitle => "@mention 说明".into(),
        MessageId::GatewaySetupFeishuMentionOnNote => {
            "启用后，群聊中需 @机器人 才会回复（权限：im:message.group_at_msg:readonly）。\n\
             可随时在 hi.toml 将 mention_enabled 改为 false 以响应群内所有消息。".into()
        }
        MessageId::GatewaySetupFeishuMentionOffTitle => "群聊自由对话".into(),
        MessageId::GatewaySetupFeishuMentionOffNote => {
            "关闭 @ 限制后，机器人在群内可响应所有文本消息。\n\
             请在飞书开放平台开通 im:message.group_msg 权限并重新发版。".into()
        }
        MessageId::GatewaySetupWeixinRiskTitle => "风险提示".into(),
        MessageId::GatewaySetupWeixinRiskNote => {
            "个人微信 iLink 处于灰度测试，腾讯可随时调整或终止服务。\n\
             建议用小号绑定；请阅读微信 iLink 功能使用条款。".into()
        }
        MessageId::GatewaySetupWeixinReloginPrompt => "已存在 bot_token，是否重新扫码登录？".into(),
        MessageId::GatewaySetupWeixinLoginSuccessTitle => "登录成功".into(),
        MessageId::GatewaySetupWeixinLoginSuccessNote => format!(
            "已绑定微信用户：{}\n启动 `hi gateway` 后，在手机微信 iLink 插件中私聊即可。",
            arg(args, 0)
        ),
        MessageId::GatewaySetupAllowlistMissingTitle => "需要用户 ID".into(),
        MessageId::GatewaySetupAllowlistMissingNote => format!(
            "allowlist 模式下至少填写一个{}。\n飞书 open_id 可在事件日志或 API 调试中查看；企微 userid 见管理后台。",
            arg(args, 0)
        ),
        MessageId::GatewaySetupCredentialNoteTitle => "凭证说明".into(),
        MessageId::GatewaySetupExistingChannelsTitle => "已配置的消息渠道".into(),
        MessageId::GatewaySetupExistingChannelsNote => format!(
            "当前：{}\n选择「现在配置」将更新 default 实例；新增其它实例请用 `hi gateway setup`。",
            arg(args, 0)
        ),
        MessageId::GatewaySetupGatePrompt => "是否配置消息渠道？".into(),
        MessageId::GatewaySetupGateConfigureLabel => "现在配置".into(),
        MessageId::GatewaySetupGateConfigureHint => "企业微信、飞书或个人微信".into(),
        MessageId::GatewaySetupGateSkipLabel => "暂不配置，仅本地使用 hi chat / tui".into(),
        MessageId::GatewaySetupGateSkipHint => "稍后可运行 hi gateway setup".into(),
        MessageId::ChannelSummaryWecom => "企业微信".into(),
        MessageId::ChannelSummaryFeishu => "飞书".into(),
        MessageId::ChannelSummaryWeixin => "个人微信".into(),
        MessageId::ChannelSummaryNone => "未配置".into(),
        MessageId::QrGenerateFailed => format!("生成终端二维码失败: {}", arg(args, 0)),
        MessageId::WizardBack => "← 返回".into(),
        MessageId::WizardSelectEmpty => format!("{}: 没有可选项", arg(args, 0)),
        MessageId::WizardPasswordKeepSuffix => "（留空表示保持不变）".into(),
        MessageId::NoteTitleWorkspace => "工作目录".into(),
        MessageId::NoteTitleGatewaySetup => "消息渠道配置".into(),
        MessageId::NoteTitleDmPolicy => "私信策略".into(),
        MessageId::NoteTitleQr => "二维码".into(),
        MessageId::WeixinQrPrompt => "请用微信扫描下方二维码：".into(),
        MessageId::WeixinQrWaitingNoteTitle => "二维码".into(),
        MessageId::WeixinQrWaiting => "等待扫码（最长 3 分钟）…".into(),
        MessageId::WeixinQrScanned => "已扫码，请在手机上确认…".into(),
        MessageId::WeixinQrExpired => "二维码已过期".into(),

        // --- gateway channel labels ---
        MessageId::ChannelWecomLabel => "企业微信智能机器人".into(),
        MessageId::ChannelWecomHint => "WebSocket 长连接 · 已接入".into(),
        MessageId::ChannelFeishuLabel => "飞书机器人".into(),
        MessageId::ChannelFeishuHint => "长连接 WebSocket · 已接入".into(),
        MessageId::ChannelWeixinLabel => "个人微信（iLink）".into(),
        MessageId::ChannelWeixinHint => "iLink 长轮询 · 实验性 · 需手机插件灰度".into(),

        // --- model preset hints ---
        MessageId::ModelHintRecommended => "推荐".into(),
        MessageId::ModelHintLegacy => "旧版".into(),

        // --- TUI ---
        MessageId::TuiBusyWaitingModel => "等待模型…".into(),
        MessageId::TuiBusyGenerating => "生成回复".into(),
        MessageId::TuiBusyThinking => "思考".into(),
        MessageId::TuiBusyMemoryExtract => "结绳记事".into(),
        MessageId::TuiBusyCompress => "压缩上下文".into(),
        MessageId::TuiInterrupted => "已中断".into(),
        MessageId::TuiContextReset => "上下文已清空（transcript 仍保留在数据库）".into(),
        MessageId::TuiModelActivated => format!(
            "已切换模型：{name} · {model}（已写入 hi.toml）",
            name = arg(args, 0),
            model = arg(args, 1)
        ),
        MessageId::TuiModelUnknown => format!(
            "未匹配到模型实例 — 检查 hi.toml [ai.providers]（{}）",
            arg(args, 0)
        ),
        MessageId::TuiStatusDefault => format!(
            "←→ 移动光标 · Enter 发送 · Shift+Enter 换行 · Ctrl+C 退出 · model {}",
            arg(args, 0)
        ),
        MessageId::TuiStatusSlashMenu => "↑↓ 选择 · Tab/Enter 填入输入框".into(),
        MessageId::TuiStatusModelMenu => "↑↓ 选择模型 · Enter 激活 · Esc 取消".into(),
        MessageId::TuiSlashModel => "/model".into(),
        MessageId::TuiSlashModelDesc => "选择并切换已配置的模型实例".into(),
        MessageId::TuiSlashReset => "/reset".into(),
        MessageId::TuiSlashResetDesc => "清空 Agent 可见上下文（别名 /clear）".into(),
        MessageId::TuiSlashCompact => "/compact".into(),
        MessageId::TuiSlashCompactDesc => "强制裁剪并 LLM 摘要上下文".into(),
        MessageId::TuiModelCurrentSuffix => " (当前)".into(),
        MessageId::TuiApprovalTitle => "审批".into(),
        MessageId::TuiApprovalApprove => "y 批准".into(),
        MessageId::TuiApprovalReject => "n/Esc 拒绝".into(),
        MessageId::TuiQueueWaiting => format!("下一条排队等待: {}", arg(args, 0)),

        // --- tool budget ---
        MessageId::ToolBudgetReminder => format!(
            "[预算提醒: {}/{}，剩余约 {} 轮，请开始收束并准备回复。]",
            arg(args, 0),
            arg(args, 1),
            arg(args, 2)
        ),
        MessageId::ToolBudgetForceSummary => {
            "（系统：本轮工具调用次数已达上限。请根据以上对话与工具结果，不再调用任何工具，向用户总结目前已完成的工作、关键发现和尚未完成的部分。用简体中文回复。）".into()
        }
        MessageId::ToolBudgetSummaryPrefix => format!(
            "（本轮工具调用已达上限（{max}/{max}），以下为阶段性总结）",
            max = arg(args, 0)
        ),
    }
}
