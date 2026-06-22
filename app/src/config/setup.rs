use std::collections::BTreeMap;

use hi_core::{
    expand_path, t, AiConfig, AiProviderEntry, ChannelsConfig, Config, Locale, MessageId,
};

use super::codex;
use super::gateway::{self, EmbeddedGatewayOutcome, summarize_configured_channels};
use super::model_presets::{self, ModelPick};
use super::wizard::{self, Session, SelectOption};

/// Author: gz
struct ProviderPreset {
    id: &'static str,
    provider: &'static str,
    model: &'static str,
    base_url: Option<&'static str>,
    /// When true, base_url is fixed to the preset value (no prompt).
    fixed_base_url: bool,
}

const PRESETS: [ProviderPreset; 5] = [
    ProviderPreset {
        id: "deepseek",
        provider: "openai-compat",
        model: "deepseek-v4-flash",
        base_url: Some("https://api.deepseek.com"),
        fixed_base_url: true,
    },
    ProviderPreset {
        id: "openai-compat",
        provider: "openai-compat",
        model: "gpt-4o",
        base_url: None,
        fixed_base_url: false,
    },
    ProviderPreset {
        id: "codex",
        provider: "codex",
        model: "gpt-5.5",
        base_url: Some(codex::DEFAULT_CODEX_BASE_URL),
        fixed_base_url: true,
    },
    ProviderPreset {
        id: "anthropic",
        provider: "anthropic",
        model: "claude-sonnet-4-20250514",
        base_url: None,
        fixed_base_url: false,
    },
    ProviderPreset {
        id: "ollama",
        provider: "ollama",
        model: "llama3.2",
        base_url: Some("http://localhost:11434"),
        fixed_base_url: false,
    },
];

fn preset_by_id(id: &str) -> &'static ProviderPreset {
    PRESETS
        .iter()
        .find(|p| p.id == id)
        .unwrap_or(&PRESETS[0])
}

fn preset_default_id(ai: &AiConfig) -> &'static str {
    if ai.default.is_empty() && ai.providers.is_empty() && ai.api_key.is_empty() {
        return "deepseek";
    }
    if ai.default == "deepseek" || ai.provider_entry("deepseek").is_some() {
        return "deepseek";
    }
    if ai.base_url
        .as_deref()
        .is_some_and(|u| u.contains("deepseek.com"))
    {
        return "deepseek";
    }
    match ai.provider.as_str() {
        "codex" => "codex",
        "anthropic" | "claude" => "anthropic",
        "ollama" => "ollama",
        _ => "openai-compat",
    }
}

struct OwnedPresetOption {
    value: &'static str,
    label: String,
    hint: String,
}

fn preset_label(locale: Locale, id: &str) -> String {
    match id {
        "openai-compat" => t(locale, MessageId::ProviderOpenaiCompatLabel, &[]),
        "ollama" => t(locale, MessageId::ProviderOllamaLabel, &[]),
        "deepseek" => "DeepSeek".into(),
        "codex" => "OpenAI Codex".into(),
        "anthropic" => "Anthropic Claude".into(),
        _ => id.to_string(),
    }
}

fn preset_hint(locale: Locale, id: &str) -> String {
    match id {
        "deepseek" => t(locale, MessageId::ProviderDeepseekHint, &[]),
        "openai-compat" => t(locale, MessageId::ProviderOpenaiCompatHint, &[]),
        "codex" => t(locale, MessageId::ProviderCodexHint, &[]),
        "ollama" => t(locale, MessageId::ProviderOllamaHint, &[]),
        "anthropic" => "Anthropic Messages API".into(),
        _ => String::new(),
    }
}

fn provider_options(locale: Locale) -> Vec<OwnedPresetOption> {
    PRESETS
        .iter()
        .map(|p| OwnedPresetOption {
            value: p.id,
            label: preset_label(locale, p.id),
            hint: preset_hint(locale, p.id),
        })
        .collect()
}

fn select_options<'a>(owned: &'a [OwnedPresetOption]) -> Vec<SelectOption<'a>> {
    owned
        .iter()
        .map(|o| SelectOption {
            value: o.value,
            label: &o.label,
            hint: &o.hint,
        })
        .collect()
}

fn choose_provider(session: &Session, current: &AiConfig) -> anyhow::Result<&'static ProviderPreset> {
    let default = preset_default_id(current);
    let owned = provider_options(session.locale());
    let options = select_options(&owned);
    let id = session.select(
        &t(session.locale(), MessageId::SetupProviderPrompt, &[]),
        &options,
        default,
    )?;
    Ok(preset_by_id(id))
}

/// 为待配置的 Provider 计算默认值：优先当前激活项，其次 `[ai.providers.<name>]`，最后 preset。
fn provider_defaults(active: &AiConfig, preset: &ProviderPreset) -> AiConfig {
    if active.default == preset.id {
        return active.clone();
    }
    if let Some(entry) = active.provider_entry(preset.id) {
        return AiConfig {
            default: preset.id.to_string(),
            provider: entry.provider.clone(),
            model: if entry.model.is_empty() {
                preset.model.to_string()
            } else {
                entry.model.clone()
            },
            base_url: entry
                .base_url
                .clone()
                .or_else(|| preset.base_url.map(str::to_string)),
            api_key: entry.api_key.clone(),
            providers: Default::default(),
        };
    }
    if active.provider == preset.provider && preset.id == preset.provider {
        return active.clone();
    }
    AiConfig {
        default: preset.id.to_string(),
        provider: preset.provider.to_string(),
        model: preset.model.to_string(),
        base_url: preset.base_url.map(str::to_string),
        api_key: String::new(),
        providers: Default::default(),
    }
}

fn prompt_ai(
    session: &Session,
    current: &AiConfig,
    preset: &ProviderPreset,
) -> anyhow::Result<Option<AiConfig>> {
    let provider = preset.provider.to_string();
    let instance = preset.id.to_string();
    let base_default = current
        .base_url
        .as_deref()
        .or(preset.base_url)
        .unwrap_or("");

    if preset.id == "deepseek" {
        session.note(
            &t(session.locale(), MessageId::NoteTitleDeepseekApiKey, &[]),
            &t(session.locale(), MessageId::SetupDeepseekKeyNote, &[]),
        )?;
    }

    let base_url = if provider == "ollama" {
        let url = session.input(&t(session.locale(), MessageId::SetupOllamaUrlPrompt, &[]), base_default)?;
        Some(url)
    } else if preset.fixed_base_url {
        preset.base_url.map(str::to_string)
    } else {
        session.input_optional(&t(session.locale(), MessageId::SetupBaseUrlPrompt, &[]), base_default)?
    };

    let model = match model_presets::choose_model(session, preset.id, &current.model)? {
        ModelPick::Back => return Ok(None),
        ModelPick::Model(m) => m,
    };

    let api_key = if provider == "ollama" {
        String::new()
    } else {
        session.password_keep(
            &t(session.locale(), MessageId::SetupApiKeyPrompt, &[]),
            !current.api_key.is_empty(),
            &current.api_key,
        )?
    };

    Ok(Some(AiConfig {
        default: instance,
        provider,
        model,
        base_url,
        api_key,
        providers: Default::default(),
    }))
}

/// OpenAI Codex 流程：检测本地 `~/.codex` 登录态；已登录则从本地模型列表选择，
/// 未登录则给出 `codex login` 引导并保持原配置不变。
fn prompt_codex(
    session: &Session,
    current: &AiConfig,
) -> anyhow::Result<Option<AiConfig>> {
    let status = codex::auth_status();
    if !status.logged_in {
        session.note(
            &t(session.locale(), MessageId::SetupCodexNotLoggedInTitle, &[]),
            &codex::login_hint(session.locale(), &status.auth_path),
        )?;
        session.note(
            &t(session.locale(), MessageId::SetupCodexSkippedTitle, &[]),
            &t(session.locale(), MessageId::SetupCodexSkippedNote, &[]),
        )?;
        return Ok(None);
    }

    let models = codex::model_ids();
    let default = if models.iter().any(|m| m == &current.model) {
        current.model.clone()
    } else {
        models.first().cloned().unwrap_or_else(|| "gpt-5.5".to_string())
    };
    let (back_label, back_hint) = model_presets::back_option(session.locale());
    let mut owned: Vec<(String, String, String)> = models
        .into_iter()
        .map(|m| (m.clone(), m.clone(), String::new()))
        .collect();
    owned.push((
        model_presets::BACK_VALUE.to_string(),
        back_label,
        back_hint,
    ));
    let options: Vec<SelectOption> = owned
        .iter()
        .map(|(value, label, hint)| SelectOption {
            value: value.as_str(),
            label: label.as_str(),
            hint: hint.as_str(),
        })
        .collect();
    let picked = session.select_with(
        &t(session.locale(), MessageId::SetupCodexModelPrompt, &[]),
        &options,
        &default,
        false,
    )?;
    if picked == model_presets::BACK_VALUE {
        return Ok(None);
    }

    Ok(Some(AiConfig {
        default: "codex".into(),
        provider: "codex".to_string(),
        model: picked.to_string(),
        base_url: Some(codex::DEFAULT_CODEX_BASE_URL.to_string()),
        api_key: String::new(),
        providers: Default::default(),
    }))
}

/// 把旧激活项与新选定项写入 `providers`，并更新 `default`。
/// 首次安装（`keep_previous = false`）只写入用户本次选择的实例。
fn merge_providers(previous: &AiConfig, mut chosen: AiConfig, keep_previous: bool) -> AiConfig {
    let mut providers = if keep_previous {
        previous.providers.clone()
    } else {
        BTreeMap::new()
    };
    if keep_previous && !previous.default.is_empty() {
        providers.insert(
            previous.default.clone(),
            AiProviderEntry {
                provider: previous.provider.clone(),
                model: previous.model.clone(),
                base_url: previous.base_url.clone(),
                api_key: previous.api_key.clone(),
            },
        );
    }
    let name = chosen.default.clone();
    providers.insert(
        name.clone(),
        AiProviderEntry {
            provider: chosen.provider.clone(),
            model: chosen.model.clone(),
            base_url: chosen.base_url.clone(),
            api_key: chosen.api_key.clone(),
        },
    );
    chosen.providers = providers;
    chosen.normalize();
    chosen
}

fn note_llm_summary(session: &Session, ai: &AiConfig) -> anyhow::Result<()> {
    let locale = session.locale();
    let key = if ai.api_key.trim().is_empty() {
        t(locale, MessageId::SetupSummaryApiKeyMissing, &[])
    } else {
        t(locale, MessageId::SetupSummaryApiKeySet, &[])
    };
    session.note(
        &t(locale, MessageId::NoteTitleLlm, &[]),
        &t(
            locale,
            MessageId::SetupLlmSummaryBody,
            &[ai.provider.clone(), ai.model.clone(), key],
        ),
    )
}

fn finish_message(locale: Locale, configured_gateway: bool) -> String {
    t(
        locale,
        if configured_gateway {
            MessageId::SetupFinishWithGateway
        } else {
            MessageId::SetupFinishNoGateway
        },
        &[],
    )
}

fn prompt_llm(
    session: &Session,
    baseline_ai: &AiConfig,
    updating: bool,
) -> anyhow::Result<AiConfig> {
    loop {
        let preset = choose_provider(session, baseline_ai)?;
        let defaults = provider_defaults(baseline_ai, preset);
        let chosen = if preset.provider == "codex" {
            prompt_codex(session, &defaults)?
        } else {
            prompt_ai(session, &defaults, preset)?
        };
        match chosen {
            Some(c) => return Ok(merge_providers(baseline_ai, c, updating)),
            None => continue,
        }
    }
}

fn empty_ai_baseline() -> AiConfig {
    AiConfig {
        default: String::new(),
        providers: BTreeMap::new(),
        provider: String::new(),
        model: String::new(),
        base_url: None,
        api_key: String::new(),
    }
}

fn load_baseline(updating: bool) -> Config {
    if updating {
        Config::load().unwrap_or_default()
    } else {
        Config {
            ai: empty_ai_baseline(),
            ..Default::default()
        }
    }
}

fn save_setup(
    session: &Session,
    baseline: &Config,
    workspace: String,
    ai: AiConfig,
    channels: Option<ChannelsConfig>,
) -> anyhow::Result<()> {
    let workspace = expand_path(&workspace).display().to_string();
    std::fs::create_dir_all(&workspace)?;

    let config = Config {
        workspace,
        data_directory: baseline.data_directory.clone(),
        ai,
        context: baseline.context.clone(),
        memory: baseline.memory.clone(),
        logging: baseline.logging.clone(),
        tools: baseline.tools.clone(),
        locale: baseline.locale.clone(),
    };

    session.save(&t(session.locale(), MessageId::SetupSaving, &[]), || {
        config.save()?;
        if let Some(channels) = channels {
            channels.save()?;
        }
        Ok(())
    })?;
    Ok(())
}

/// LLM + optional gateway wizard (`hi setup`).
///
/// Author: gz
pub fn run() -> anyhow::Result<()> {
    let path = Config::config_path();
    let updating = path.exists();
    let baseline = load_baseline(updating);
    let channels_baseline = ChannelsConfig::load().unwrap_or_default();
    let locale = baseline.resolved_locale();
    let session = Session::new(path.clone(), locale);

    session.start(&if updating {
        t(locale, MessageId::SetupUpdateTitle, &[])
    } else {
        t(locale, MessageId::SetupTitle, &[])
    })?;

    session.note(
        &t(locale, MessageId::NoteTitleSetup, &[]),
        &t(
            locale,
            MessageId::SetupNotePath,
            &[path.display().to_string()],
        ),
    )?;

    if updating {
        let summary = wizard::summarize_setup(
            locale,
            &baseline,
            &summarize_configured_channels(&channels_baseline, locale),
        );
        session.note(
            &t(locale, MessageId::SetupCurrentSummaryTitle, &[]),
            &summary,
        )?;
    }

    let llm_baseline = if updating {
        baseline.ai.clone()
    } else {
        empty_ai_baseline()
    };

    let ai = prompt_llm(&session, &llm_baseline, updating)?;
    note_llm_summary(&session, &ai)?;

    let gateway_workspace = baseline.workspace.clone();
    match gateway::prompt_embedded_gateway(
        &session,
        &gateway_workspace,
        channels_baseline.clone(),
    )? {
        EmbeddedGatewayOutcome::Skipped { workspace } => {
            save_setup(&session, &baseline, workspace, ai, None)?;
            session.finish(&finish_message(locale, false))?;
        }
        EmbeddedGatewayOutcome::Configured {
            workspace,
            channels,
        } => {
            save_setup(&session, &baseline, workspace, ai, Some(channels))?;
            session.finish(&finish_message(locale, true))?;
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "../../test/unit/config/setup.rs"]
mod tests;
