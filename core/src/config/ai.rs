use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::Error;

/// 已配置的 LLM 实例摘要（`[ai.providers.<name>]`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProfile {
    pub name: String,
    pub adapter: String,
    pub model: String,
    pub active: bool,
}

/// 单个 LLM 实例（`[ai.providers.<name>]`）。
///
/// Author: gz
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiProviderEntry {
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// LLM API Key（Ollama / Codex 可留空）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api_key: String,
}

/// LLM 配置（`[ai]`）：`default` 指向 `[ai.providers.<name>]` 中的激活实例。
///
/// `provider` / `model` / `base_url` / `api_key` 为解析后的激活项，供运行时只读使用。
///
/// Author: gz
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// 当前激活的 provider 实例名（`[ai].default`）
    pub default: String,
    pub providers: BTreeMap<String, AiProviderEntry>,
    /// 激活实例的 adapter 类型（openai-compat / codex / …）
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
struct AiConfigWire {
    #[serde(default)]
    default: String,
    #[serde(default)]
    providers: BTreeMap<String, AiProviderEntry>,
    #[serde(default)]
    profiles: BTreeMap<String, LegacyProviderProfile>,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct LegacyProviderProfile {
    #[serde(default)]
    model: String,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    api_key: String,
}

#[derive(Serialize)]
struct AiConfigWireOut {
    default: String,
    providers: BTreeMap<String, AiProviderEntry>,
}

impl Default for AiConfig {
    fn default() -> Self {
        let mut config = Self {
            default: "openai-compat".into(),
            providers: BTreeMap::new(),
            provider: "openai-compat".into(),
            model: "gpt-4o".into(),
            base_url: None,
            api_key: String::new(),
        };
        config.snapshot_active();
        config
    }
}

impl AiConfig {
    /// 把当前激活项写入 `providers[default]`。
    pub fn snapshot_active(&mut self) {
        if self.default.is_empty() && !self.provider.is_empty() {
            self.default = self.provider.clone();
        }
        if self.default.is_empty() {
            return;
        }
        self.providers.insert(
            self.default.clone(),
            AiProviderEntry {
                provider: self.provider.clone(),
                model: self.model.clone(),
                base_url: self.base_url.clone(),
                api_key: self.api_key.clone(),
            },
        );
    }

    /// 切换激活实例（`[ai].default`），并刷新解析后的 provider/model 字段。
    pub fn activate_provider(&mut self, name: &str) -> crate::error::Result<()> {
        if !self.providers.contains_key(name) {
            return Err(Error::Message(format!(
                "未知模型配置 {name:?} — 可用: {}",
                self.providers.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        self.default = name.to_string();
        self.normalize();
        Ok(())
    }

    /// 列出 `[ai.providers]` 中全部实例。
    pub fn profiles(&self) -> Vec<ModelProfile> {
        let mut names: Vec<_> = self.providers.keys().cloned().collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|name| {
                let entry = self.providers.get(&name)?;
                Some(ModelProfile {
                    name: name.clone(),
                    adapter: entry.provider.clone(),
                    model: entry.model.clone(),
                    active: name == self.default,
                })
            })
            .collect()
    }

    /// 取某命名实例（`[ai.providers.<name>]`）。
    pub fn provider_entry(&self, name: &str) -> Option<&AiProviderEntry> {
        self.providers.get(name)
    }

    pub fn normalize(&mut self) {
        if self.default.is_empty() {
            if !self.provider.is_empty() {
                self.default = self.provider.clone();
            } else if let Some(key) = self.providers.keys().next().cloned() {
                self.default = key;
            }
        }

        if let Some(entry) = self.providers.get(&self.default).cloned() {
            self.provider = entry.provider;
            self.model = entry.model;
            self.base_url = entry.base_url;
            self.api_key = entry.api_key;
        } else if !self.provider.is_empty() || !self.model.is_empty() {
            let name = self.default.clone();
            self.providers.insert(
                name.clone(),
                AiProviderEntry {
                    provider: self.provider.clone(),
                    model: self.model.clone(),
                    base_url: self.base_url.clone(),
                    api_key: self.api_key.clone(),
                },
            );
            if self.default.is_empty() {
                self.default = name;
            }
        }
    }
}

impl Serialize for AiConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut providers = self.providers.clone();
        if !self.default.is_empty() {
            providers.insert(
                self.default.clone(),
                AiProviderEntry {
                    provider: self.provider.clone(),
                    model: self.model.clone(),
                    base_url: self.base_url.clone(),
                    api_key: self.api_key.clone(),
                },
            );
        }
        AiConfigWireOut {
            default: self.default.clone(),
            providers,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AiConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AiConfigWire::deserialize(deserializer)?;
        let mut providers = wire.providers;
        for (name, legacy) in wire.profiles {
            providers.entry(name.clone()).or_insert_with(|| AiProviderEntry {
                provider: name,
                model: legacy.model,
                base_url: legacy.base_url,
                api_key: legacy.api_key,
            });
        }

        let default = if wire.default.is_empty() && !wire.provider.is_empty() {
            wire.provider.clone()
        } else {
            wire.default
        };

        if !wire.provider.is_empty() || !wire.model.is_empty() {
            let active_name = default.clone();
            if !active_name.is_empty() {
                providers
                    .entry(active_name)
                    .or_insert_with(|| AiProviderEntry {
                        provider: wire.provider.clone(),
                        model: wire.model.clone(),
                        base_url: wire.base_url.clone(),
                        api_key: wire.api_key.clone(),
                    });
            }
        }

        let mut config = Self {
            default,
            providers,
            provider: wire.provider,
            model: wire.model,
            base_url: wire.base_url,
            api_key: wire.api_key,
        };
        config.normalize();
        Ok(config)
    }
}
