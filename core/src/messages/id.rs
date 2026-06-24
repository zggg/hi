/// User-visible message identifiers (locale-neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageId {
    // --- config / startup ---
    MissingApiKey,
    UnknownAiProvider,       // {0} provider
    UnknownModel,              // {0} name, {1} available
    ParseHiConfig,             // {0} detail
    SerializeHiConfig,
    ReadCurrentDir,            // {0} detail
    CreateWorkspace,           // {0} path, {1} detail
    InvalidWorkspace,          // {0} path, {1} detail
    ConfigLock,                // {0} detail
    ProviderLock,
    ApprovalPolicyLock,
    BuildAgent,                // {0} detail
    ConfigNotSetup,            // hi config when no ai
    HiTomlPath,                // {0} path — label only

    // --- channels ---
    ChannelsNotConfigured,
    UnknownChannelId,          // {0} id
    WecomAccountMissing,       // {0} account
    FeishuAccountMissing,
    WeixinAccountMissing,
    MissingWecomSecret,
    MissingWecomBotId,
    MissingFeishuAppId,
    MissingFeishuAppSecret,
    FeishuAllowlistEmpty,
    WecomAllowlistEmpty,
    WecomDmPolicyOpenWarn,
    FeishuDmPolicyOpenWarn,
    DefaultWelcome,

    // --- store / memory / agent ---
    SchemaIncompatible,        // {0} detail
    MemorySearchDisabled,
    MemoryQueryEmpty,
    ToolIterationLimit,
    MemoryRecorded,            // {0} id
    MemoryDuplicate,
    EmptyChannelReply,
    EmergencyTrimSummary,      // {0} count
    ExtractKnotsFailed,        // {0} detail

    // --- approval ---
    ApprovalPromptBash,        // {0} command, {1} grant hint
    ApprovalPromptFile,        // {0} op, {1} path, {2} grant hint
    ApprovalPromptGeneric,       // {0} kind, {1} detail, {2} grant

    // --- gateway replies ---
    GatewayThinking,
    GatewayBusy,
    GatewayTurnAck,
    GatewayTurnWallTimeout,    // {0} seconds
    GatewayProcessFailed,      // {0} detail
    GatewayUnsupportedMessage, // {0} kind
    GatewayCheckOkWecom,
    GatewayCheckOkFeishu,
    GatewayCheckOkWeixin,
    GatewayCheckOkGeneric,

    // --- gateway svc (CLI) ---
    GatewayStarted,            // {0} pid
    GatewayLogsDir,            // {0} path
    GatewayStopHint,
    GatewayNotRunning,
    GatewayStopped,            // {0} pid
    GatewayForceStopped,       // {0} pid
    GatewayPidFile,            // {0} path
    GatewayWorkspace,          // {0} path
    GatewayChannels,           // {0} names csv
    GatewayChannelsNone,
    GatewayStatusRunning,      // {0} pid
    GatewayStatusStopped,
    GatewayRecentLog,          // {0} path
    GatewayReloadSent,
    GatewayStartFailed,        // {0} detail
    GatewayPidParseFailed,     // {0} raw
    GatewayStopSignalFailed,   // {0} pid
    GatewayStopFailed,         // {0} pid
    GatewayReloadUnixOnly,
    GatewayReloadSignalFailed, // {0} pid
    GatewayRecentLogLine,        // {0} line

    // --- chat / session CLI ---
    ChatBanner,                // {0} session, {1} model, {2} cwd
    ContextReset,
    ModelSlashTuiOnly,
    UnknownChatArg,            // {0} arg
    SessionListHeader,         // {0} count
    SessionListRow,              // {0} id, {1} count, {2} preview
    SessionEmpty,
    SessionNotFound,           // {0} id
    SessionExported,           // {0} count, {1} path
    SessionDeleted,            // {0} id
    SessionPurgeNeedConfirm,
    CompressionListHeader,
    CompressionRow,
    CompressionDetailHeader,
    CompressionDetailSummary,

    // --- memory CLI ---
    MemoryListEmpty,           // {0} owner
    MemoryAdded,               // {0} id
    MemoryForgotten,           // {0} id
    MemoryReinforced,          // {0} id
    MemoryExtractNone,         // {0} session
    MemoryExtractDone,         // {0} added, {1} merged, {2} skipped
    MemoryListRow,
    MemoryDisabledInConfig,
    MemoryUnknownKind,         // {0} kind

    // --- LLM / provider (from ai layer) ---
    LlmHttpError,              // {0} code, {1} detail, {2} hint
    LlmTransportError,         // {0} detail
    LlmReadBodyError,          // {0} detail
    LlmParseError,             // {0} detail
    LlmStreamError,            // {0} detail
    AnthropicNeedsApiKey,

    // --- stdin approval ---
    StdinApprovalHeader,
    StdinApprovalPrompt,

    // --- wizard: shared ---
    WizardWritten,             // {0} path
    WizardFinishSteps,
    WizardConfigPathLine,      // {0} path
    WizardSelectNumberPrompt,  // {0} default index
    WizardWriteFailed,
    WizardFinishTitle,
    WizardCancelled,

    // --- wizard: setup ---
    SetupTitle,
    SetupUpdateTitle,
    SetupNotePath,             // {0} path
    SetupNoteRepeat,
    SetupSummaryProvider,      // {0} val
    SetupSummaryModel,
    SetupSummaryWorkspace,
    SetupSummaryApiKeySet,
    SetupSummaryApiKeyMissing,
    SetupWorkspaceNote,
    SetupWorkspacePrompt,
    SetupProviderPrompt,
    SetupOllamaUrlPrompt,
    SetupBaseUrlPrompt,
    SetupDeepseekKeyNote,
    SetupModelPrompt,
    SetupModelCustom,
    SetupModelFetching,
    SetupModelFetchDone,       // {0} count
    SetupModelFetchFailedTitle,
    SetupModelFetchFailedNote, // {0} reason
    SetupApiKeyPrompt,
    SetupSaving,
    SetupFinishNoGateway,
    SetupFinishWithGateway,
    NoteTitleSetup,
    NoteTitleLlm,
    NoteTitleDeepseekApiKey,
    SetupCurrentSummaryTitle,
    SetupSummaryChannels,      // {0} summary
    SetupSummaryMaskedKey,
    SetupLlmSummaryBody,         // {0} provider, {1} model, {2} api key line
    SetupCodexNotLoggedInTitle,
    SetupCodexSkippedTitle,
    SetupCodexSkippedNote,
    SetupCodexModelPrompt,
    SetupCodexLoginHint,         // {0} auth path
    ProviderDeepseekHint,
    ProviderOpenaiCompatLabel,
    ProviderOpenaiCompatHint,
    ProviderCodexHint,
    ProviderOllamaLabel,
    ProviderOllamaHint,
    ModelHintFlagship,
    ModelHintReasonerDeprecated,
    ModelHintDeprecatedJuly2026,
    ModelHintSonnet4Recommended,
    ModelBackToProviderLabel,
    ModelBackToProviderHint,
    ModelCustomLabel,
    ModelCustomHint,
    SetupModelNamePrompt,
    ModelSetupTitle,
    ModelSetupFinish,

    // --- wizard: gateway ---
    GatewaySetupTitle,
    GatewaySetupUpdateTitle,
    GatewaySetupNote,
    GatewaySetupChannelPrompt,
    GatewaySetupWecomCredsNote,
    GatewaySetupBotIdPrompt,
    GatewaySetupSecretPrompt,
    GatewaySetupAllowlistPrompt,
    GatewaySetupAllowlistUsersPrompt,
    GatewaySetupOpenModeNote,
    GatewaySetupWelcomePrompt,
    GatewaySetupSaving,
    GatewaySetupFinish,
    GatewaySetupNeedBaseConfig,
    GatewaySetupNoChannels,
    GatewaySetupFeishuCredsNote,
    GatewaySetupWeixinCredsNote,
    GatewaySetupGenericCredsNote,
    GatewaySetupFeishuAppIdPrompt,
    GatewaySetupFeishuSecretPrompt,
    GatewaySetupFeishuDomainPrompt,
    GatewaySetupFeishuMentionPrompt,
    GatewaySetupFeishuMentionOnTitle,
    GatewaySetupFeishuMentionOnNote,
    GatewaySetupFeishuMentionOffTitle,
    GatewaySetupFeishuMentionOffNote,
    GatewaySetupWeixinRiskTitle,
    GatewaySetupWeixinRiskNote,
    GatewaySetupWeixinReloginPrompt,
    GatewaySetupWeixinLoginSuccessTitle,
    GatewaySetupWeixinLoginSuccessNote, // {0} ilink_user_id
    GatewaySetupAllowlistMissingTitle,
    GatewaySetupAllowlistMissingNote, // {0} id_label
    GatewaySetupCredentialNoteTitle,
    GatewaySetupExistingChannelsTitle,
    GatewaySetupExistingChannelsNote, // {0} summary
    GatewaySetupGatePrompt,
    GatewaySetupGateConfigureLabel,
    GatewaySetupGateConfigureHint,
    GatewaySetupGateSkipLabel,
    GatewaySetupGateSkipHint,
    ChannelSummaryWecom,
    ChannelSummaryFeishu,
    ChannelSummaryWeixin,
    ChannelSummaryNone,
    QrGenerateFailed, // {0} detail
    WizardBack,
    WizardSelectEmpty, // {0} message
    WizardPasswordKeepSuffix,
    NoteTitleWorkspace,
    NoteTitleGatewaySetup,
    NoteTitleDmPolicy,
    NoteTitleQr,
    WeixinQrPrompt,
    WeixinQrWaitingNoteTitle,
    WeixinQrWaiting,
    WeixinQrScanned,
    WeixinQrExpired,

    // --- gateway channel labels (setup menu) ---
    ChannelWecomLabel,
    ChannelWecomHint,
    ChannelFeishuLabel,
    ChannelFeishuHint,
    ChannelWeixinLabel,
    ChannelWeixinHint,

    // --- model presets hints (optional short) ---
    ModelHintRecommended,
    ModelHintLegacy,

    // --- TUI ---
    TuiBusyWaitingModel,
    TuiBusyGenerating,
    TuiBusyThinking,
    TuiBusyMemoryExtract,
    TuiBusyCompress,
    TuiInterrupted,
    TuiContextReset,
    TuiModelActivated,         // {0} name
    TuiModelUnknown,           // {0} name
    TuiStatusDefault,          // {0} model
    TuiStatusSlashMenu,
    TuiStatusModelMenu,
    TuiSlashModel,
    TuiSlashModelDesc,
    TuiSlashReset,
    TuiSlashResetDesc,
    TuiSlashCompact,
    TuiSlashCompactDesc,
    TuiSlashVerbose,
    TuiSlashVerboseDesc,
    TuiVerboseOn,
    TuiVerboseOff,
    TuiExitResumeHint,         // {0} session id
    TuiModelCurrentSuffix,
    TuiApprovalTitle,
    TuiApprovalApprove,
    TuiApprovalReject,
    TuiQueueWaiting,           // {0} n

    // --- tool budget (LLM nudge — follows UI locale) ---
    ToolBudgetReminder,
    ToolBudgetForceSummary,
    ToolBudgetSummaryPrefix,
}
