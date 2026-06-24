//! Curated model lists per provider preset (setup wizard).
//!
//! Author: gz

use hi_core::{t, Locale, MessageId};

use super::wizard::SelectOption;

/// Author: gz
pub struct ModelOption {
    pub id: &'static str,
    pub label: &'static str,
}

const DEEPSEEK_MODELS: [ModelOption; 4] = [
    ModelOption {
        id: "deepseek-v4-flash",
        label: "deepseek-v4-flash",
    },
    ModelOption {
        id: "deepseek-v4-pro",
        label: "deepseek-v4-pro",
    },
    ModelOption {
        id: "deepseek-chat",
        label: "deepseek-chat",
    },
    ModelOption {
        id: "deepseek-reasoner",
        label: "deepseek-reasoner",
    },
];

const ANTHROPIC_MODELS: [ModelOption; 4] = [
    ModelOption {
        id: "claude-sonnet-4-20250514",
        label: "claude-sonnet-4-20250514",
    },
    ModelOption {
        id: "claude-opus-4-20250514",
        label: "claude-opus-4-20250514",
    },
    ModelOption {
        id: "claude-3-5-sonnet-20241022",
        label: "claude-3-5-sonnet-20241022",
    },
    ModelOption {
        id: "claude-3-5-haiku-20241022",
        label: "claude-3-5-haiku-20241022",
    },
];

const OLLAMA_MODELS: [ModelOption; 4] = [
    ModelOption {
        id: "llama3.2",
        label: "llama3.2",
    },
    ModelOption {
        id: "qwen2.5",
        label: "qwen2.5",
    },
    ModelOption {
        id: "deepseek-r1",
        label: "deepseek-r1",
    },
    ModelOption {
        id: "gemma2",
        label: "gemma2",
    },
];

const CUSTOM_MODEL_VALUE: &str = "__custom__";

/// Value for the model menu «back» item (return to provider selection).
pub const BACK_VALUE: &str = "__back__";

/// Model selection outcome.
pub enum ModelPick {
    Model(String),
    Back,
}

fn model_hint(locale: Locale, model_id: &str) -> String {
    match model_id {
        "deepseek-v4-flash" => t(locale, MessageId::ModelHintRecommended, &[]),
        "deepseek-v4-pro" => t(locale, MessageId::ModelHintFlagship, &[]),
        "deepseek-chat" => t(locale, MessageId::ModelHintDeprecatedJuly2026, &[]),
        "deepseek-reasoner" => t(locale, MessageId::ModelHintReasonerDeprecated, &[]),
        "claude-sonnet-4-20250514" => t(locale, MessageId::ModelHintSonnet4Recommended, &[]),
        "claude-opus-4-20250514" => "Opus 4".into(),
        "claude-3-5-sonnet-20241022" => "Sonnet 3.5".into(),
        "claude-3-5-haiku-20241022" => "Haiku 3.5".into(),
        "llama3.2" => "Meta Llama 3.2".into(),
        "qwen2.5" => "Qwen 2.5".into(),
        "deepseek-r1" => "DeepSeek R1".into(),
        "gemma2" => "Google Gemma 2".into(),
        _ => String::new(),
    }
}

/// «Back to provider list» option at the end of the model menu.
pub fn back_option(locale: Locale) -> (String, String) {
    (
        t(locale, MessageId::ModelBackToProviderLabel, &[]),
        t(locale, MessageId::ModelBackToProviderHint, &[]),
    )
}

/// Curated models for a preset id; empty slice means free-text input.
pub fn models_for(preset_id: &str) -> &'static [ModelOption] {
    match preset_id {
        "deepseek" => &DEEPSEEK_MODELS,
        "anthropic" => &ANTHROPIC_MODELS,
        "ollama" => &OLLAMA_MODELS,
        _ => &[],
    }
}

/// 从动态拉取的模型 id 列表构建选择菜单（末尾追加「自定义输入 / 返回」）。
///
/// Author: gz
pub fn pick_from_ids(
    session: &super::wizard::Session,
    ids: &[String],
    current: &str,
) -> anyhow::Result<ModelPick> {
    let locale = session.locale();
    let default = if ids.iter().any(|m| m == current) {
        current.to_string()
    } else {
        ids.first().cloned().unwrap_or_default()
    };

    let custom_label = t(locale, MessageId::ModelCustomLabel, &[]);
    let custom_hint = t(locale, MessageId::ModelCustomHint, &[]);
    let (back_label, back_hint) = back_option(locale);

    let mut owned: Vec<(String, String, String)> = ids
        .iter()
        .map(|m| (m.clone(), m.clone(), String::new()))
        .collect();
    owned.push((CUSTOM_MODEL_VALUE.to_string(), custom_label, custom_hint));
    owned.push((BACK_VALUE.to_string(), back_label, back_hint));

    let options: Vec<SelectOption> = owned
        .iter()
        .map(|(value, label, hint)| SelectOption {
            value: value.as_str(),
            label: label.as_str(),
            hint: hint.as_str(),
        })
        .collect();

    let picked = session.select_with(
        &t(locale, MessageId::SetupModelPrompt, &[]),
        &options,
        &default,
        false,
    )?;
    if picked == BACK_VALUE {
        return Ok(ModelPick::Back);
    }
    if picked == CUSTOM_MODEL_VALUE {
        return Ok(ModelPick::Model(session.input(
            &t(locale, MessageId::SetupModelNamePrompt, &[]),
            current,
        )?));
    }
    Ok(ModelPick::Model(picked.to_string()))
}

/// Interactive model pick: menu when curated list exists; custom + back at the end.
pub fn choose_model(
    session: &super::wizard::Session,
    preset_id: &str,
    current: &str,
) -> anyhow::Result<ModelPick> {
    let locale = session.locale();
    let models = models_for(preset_id);
    if models.is_empty() {
        return Ok(ModelPick::Model(
            session.input(
                &t(locale, MessageId::SetupModelNamePrompt, &[]),
                current,
            )?,
        ));
    }

    let default = if models.iter().any(|m| m.id == current) {
        current
    } else {
        models[0].id
    };

    let custom_label = t(locale, MessageId::ModelCustomLabel, &[]);
    let custom_hint = t(locale, MessageId::ModelCustomHint, &[]);
    let (back_label, back_hint) = back_option(locale);

    let mut owned: Vec<(String, String, String)> = models
        .iter()
        .map(|m| {
            (
                m.id.to_string(),
                m.label.to_string(),
                model_hint(locale, m.id),
            )
        })
        .collect();
    owned.push((
        CUSTOM_MODEL_VALUE.to_string(),
        custom_label,
        custom_hint,
    ));
    owned.push((BACK_VALUE.to_string(), back_label, back_hint));

    let options: Vec<SelectOption> = owned
        .iter()
        .map(|(value, label, hint)| SelectOption {
            value: value.as_str(),
            label: label.as_str(),
            hint: hint.as_str(),
        })
        .collect();

    let picked = session.select_with(
        &t(locale, MessageId::SetupModelPrompt, &[]),
        &options,
        default,
        false,
    )?;
    if picked == BACK_VALUE {
        return Ok(ModelPick::Back);
    }
    if picked == CUSTOM_MODEL_VALUE {
        return Ok(ModelPick::Model(
            session.input(
                &t(locale, MessageId::SetupModelNamePrompt, &[]),
                current,
            )?,
        ));
    }
    Ok(ModelPick::Model(picked.to_string()))
}

#[cfg(test)]
#[path = "../../test/unit/config/model_presets.rs"]
mod tests;
