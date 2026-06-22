use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Serialize, Serializer};

use super::endpoint::ChannelEndpoint;
use super::feishu::FeishuConfig;
use super::hi_toml;
use super::paths;
use super::wecom::WeComConfig;
use super::weixin::WeixinConfig;
use crate::error::{Error, Result};
use crate::messages::MessageId;

const WECOM_SCALAR_KEYS: &[&str] = &[
    "enabled",
    "bot_id",
    "secret",
    "websocket_url",
    "dm_policy",
    "allow_from",
    "welcome_message",
];

const FEISHU_SCALAR_KEYS: &[&str] = &[
    "enabled",
    "app_id",
    "app_secret",
    "domain",
    "dm_policy",
    "allow_from",
    "mention_enabled",
    "welcome_message",
];

const WEIXIN_SCALAR_KEYS: &[&str] = &[
    "enabled",
    "bot_token",
    "ilink_bot_id",
    "ilink_user_id",
    "base_url",
    "welcome_message",
    "bot_type",
    "poll_timeout_secs",
];

/// Message-channel settings (`~/.hi/hi.toml` `[channels.*]` sections).
///
/// Author: gz
#[derive(Debug, Clone, Default)]
pub struct ChannelsConfig {
    wecom_accounts: BTreeMap<String, WeComConfig>,
    feishu_accounts: BTreeMap<String, FeishuConfig>,
    weixin_accounts: BTreeMap<String, WeixinConfig>,
}

impl ChannelsConfig {
    pub fn load() -> Result<Self> {
        Self::load_from_document(&hi_toml::read_document()?)
    }

    pub fn channels_path() -> PathBuf {
        paths::hi_config_path()
    }

    pub fn save(&self) -> Result<()> {
        let mut doc = hi_toml::read_document()?;
        let table = doc
            .as_table_mut()
            .ok_or_else(|| Error::Message("hi config root must be a table".into()))?;

        table.remove("enabled");
        table.remove("default");
        table.remove("wecom");

        let mut to_save = self.clone();
        to_save.normalize();

        if to_save.wecom_accounts.is_empty()
            && to_save.feishu_accounts.is_empty()
            && to_save.weixin_accounts.is_empty()
        {
            table.remove("channels");
        } else {
            let mut channels = toml::Table::new();
            if !to_save.wecom_accounts.is_empty() {
                channels.insert(
                    "wecom".into(),
                    accounts_to_toml(&to_save.wecom_accounts),
                );
            }
            if !to_save.feishu_accounts.is_empty() {
                channels.insert(
                    "feishu".into(),
                    accounts_to_toml(&to_save.feishu_accounts),
                );
            }
            if !to_save.weixin_accounts.is_empty() {
                channels.insert(
                    "weixin".into(),
                    accounts_to_toml(&to_save.weixin_accounts),
                );
            }
            table.insert("channels".into(), toml::Value::Table(channels));
        }

        hi_toml::write_document(&doc)
    }

    pub fn redacted(&self) -> Self {
        let mut c = self.clone();
        for cfg in c.wecom_accounts.values_mut() {
            cfg.secret = super::mask_secret(&cfg.secret);
        }
        for cfg in c.feishu_accounts.values_mut() {
            cfg.app_secret = super::mask_secret(&cfg.app_secret);
        }
        for cfg in c.weixin_accounts.values_mut() {
            cfg.bot_token = super::mask_secret(&cfg.bot_token);
        }
        c
    }

    pub fn wecom_accounts(&self) -> &BTreeMap<String, WeComConfig> {
        &self.wecom_accounts
    }

    pub fn feishu_accounts(&self) -> &BTreeMap<String, FeishuConfig> {
        &self.feishu_accounts
    }

    pub fn weixin_accounts(&self) -> &BTreeMap<String, WeixinConfig> {
        &self.weixin_accounts
    }

    pub fn set_wecom_account(&mut self, account: impl Into<String>, config: WeComConfig) {
        let account = account.into();
        if config.is_empty() {
            self.wecom_accounts.remove(&account);
        } else {
            self.wecom_accounts.insert(account, config);
        }
    }

    pub fn set_feishu_account(&mut self, account: impl Into<String>, config: FeishuConfig) {
        let account = account.into();
        if config.is_empty() {
            self.feishu_accounts.remove(&account);
        } else {
            self.feishu_accounts.insert(account, config);
        }
    }

    pub fn set_weixin_account(&mut self, account: impl Into<String>, config: WeixinConfig) {
        let account = account.into();
        if config.is_empty() {
            self.weixin_accounts.remove(&account);
        } else {
            self.weixin_accounts.insert(account, config);
        }
    }

    pub fn normalize(&mut self) {
        self.wecom_accounts.retain(|_, cfg| !cfg.is_empty());
        self.feishu_accounts.retain(|_, cfg| !cfg.is_empty());
        self.weixin_accounts.retain(|_, cfg| !cfg.is_empty());
    }

    pub fn active_channel(&self) -> Result<String> {
        self.enabled_endpoints()?
            .first()
            .map(|e| e.id.clone())
            .ok_or_else(|| Error::localized(MessageId::ChannelsNotConfigured))
    }

    pub fn enabled_endpoints(&self) -> Result<Vec<ChannelEndpoint>> {
        let names = self.all_enabled_endpoint_ids();
        if names.is_empty() {
            return Err(Error::localized(MessageId::ChannelsNotConfigured));
        }
        let mut out = Vec::with_capacity(names.len());
        for name in names {
            out.push(self.endpoint_by_id(&name)?);
        }
        Ok(out)
    }

    fn all_enabled_endpoint_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .wecom_accounts
            .iter()
            .filter(|(_, cfg)| !cfg.is_empty() && cfg.enabled)
            .map(|(name, _)| endpoint_id_for_wecom_account(name))
            .chain(
                self.feishu_accounts
                    .iter()
                    .filter(|(_, cfg)| !cfg.is_empty() && cfg.enabled)
                    .map(|(name, _)| endpoint_id_for_feishu_account(name)),
            )
            .chain(
                self.weixin_accounts
                    .iter()
                    .filter(|(_, cfg)| !cfg.is_empty() && cfg.enabled)
                    .map(|(name, _)| endpoint_id_for_weixin_account(name)),
            )
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    fn endpoint_by_id(&self, id: &str) -> Result<ChannelEndpoint> {
        if id == "wecom" {
            return self.wecom_endpoint("default");
        }
        if let Some(account) = id.strip_prefix("wecom:") {
            return self.wecom_endpoint(account);
        }
        if id == "feishu" {
            return self.feishu_endpoint("default");
        }
        if let Some(account) = id.strip_prefix("feishu:") {
            return self.feishu_endpoint(account);
        }
        if id == "weixin" {
            return self.weixin_endpoint("default");
        }
        if let Some(account) = id.strip_prefix("weixin:") {
            return self.weixin_endpoint(account);
        }
        Err(Error::with_arg(
            MessageId::UnknownChannelId,
            id.to_string(),
        ))
    }

    fn wecom_endpoint(&self, account: &str) -> Result<ChannelEndpoint> {
        let config = self.wecom_account(account)?.clone();
        Ok(ChannelEndpoint::wecom(account, config))
    }

    fn feishu_endpoint(&self, account: &str) -> Result<ChannelEndpoint> {
        let config = self.feishu_account(account)?.clone();
        Ok(ChannelEndpoint::feishu(account, config))
    }

    fn weixin_endpoint(&self, account: &str) -> Result<ChannelEndpoint> {
        let config = self.weixin_account(account)?.clone();
        Ok(ChannelEndpoint::weixin(account, config))
    }

    pub fn wecom_account(&self, account: &str) -> Result<&WeComConfig> {
        self.wecom_accounts
            .get(account)
            .filter(|w| !w.is_empty())
            .ok_or_else(|| {
                Error::with_arg(MessageId::WecomAccountMissing, account.to_string())
            })
    }

    pub fn require_wecom(&self) -> Result<&WeComConfig> {
        self.wecom_account("default")
    }

    pub fn wecom_secret(&self) -> Result<String> {
        let wecom = self.require_wecom()?;
        wecom_secret_from(wecom)
    }

    pub fn wecom_secret_for(&self, account: &str) -> Result<String> {
        let wecom = self.wecom_account(account)?;
        wecom_secret_from(wecom)
    }

    pub fn feishu_account(&self, account: &str) -> Result<&FeishuConfig> {
        self.feishu_accounts
            .get(account)
            .filter(|f| !f.is_empty())
            .ok_or_else(|| {
                Error::with_arg(MessageId::FeishuAccountMissing, account.to_string())
            })
    }

    pub fn require_feishu(&self) -> Result<&FeishuConfig> {
        self.feishu_account("default")
    }

    pub fn weixin_account(&self, account: &str) -> Result<&WeixinConfig> {
        self.weixin_accounts
            .get(account)
            .filter(|w| !w.is_empty())
            .ok_or_else(|| {
                Error::with_arg(MessageId::WeixinAccountMissing, account.to_string())
            })
    }

    pub fn require_weixin(&self) -> Result<&WeixinConfig> {
        self.weixin_account("default")
    }
}

impl Serialize for ChannelsConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        if !self.wecom_accounts.is_empty()
            || !self.feishu_accounts.is_empty()
            || !self.weixin_accounts.is_empty()
        {
            let mut channels = serde_json::Map::new();
            if !self.wecom_accounts.is_empty() {
                channels.insert(
                    "wecom".into(),
                    serde_json::to_value(accounts_to_toml(&self.wecom_accounts))
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                );
            }
            if !self.feishu_accounts.is_empty() {
                channels.insert(
                    "feishu".into(),
                    serde_json::to_value(accounts_to_toml(&self.feishu_accounts))
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                );
            }
            if !self.weixin_accounts.is_empty() {
                channels.insert(
                    "weixin".into(),
                    serde_json::to_value(accounts_to_toml(&self.weixin_accounts))
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                );
            }
            map.serialize_entry("channels", &channels)?;
        }
        map.end()
    }
}

fn wecom_secret_from(wecom: &WeComConfig) -> Result<String> {
    let secret = wecom.secret.trim().to_string();
    if secret.is_empty() {
        return Err(Error::localized(MessageId::MissingWecomSecret));
    }
    Ok(secret)
}

fn endpoint_id_for_wecom_account(account: &str) -> String {
    endpoint_id_for_platform_account("wecom", account)
}

fn endpoint_id_for_feishu_account(account: &str) -> String {
    endpoint_id_for_platform_account("feishu", account)
}

fn endpoint_id_for_weixin_account(account: &str) -> String {
    endpoint_id_for_platform_account("weixin", account)
}

fn endpoint_id_for_platform_account(platform: &str, account: &str) -> String {
    if account == "default" {
        platform.into()
    } else {
        format!("{platform}:{account}")
    }
}

fn account_name_for_wecom_endpoint_id(id: &str) -> Option<String> {
    account_name_for_platform_endpoint_id("wecom", id)
}

fn accounts_to_toml<T: Serialize>(accounts: &BTreeMap<String, T>) -> toml::Value {
    let mut table = toml::Table::new();
    for (name, cfg) in accounts {
        let value = toml::Value::try_from(cfg)
            .unwrap_or(toml::Value::Table(toml::Table::new()));
        if name == "default" {
            if let toml::Value::Table(flat) = value {
                for (k, v) in flat {
                    table.insert(k, v);
                }
            }
        } else {
            table.insert(name.clone(), value);
        }
    }
    toml::Value::Table(table)
}

fn parse_wecom_accounts(value: Option<&toml::Value>) -> Result<BTreeMap<String, WeComConfig>> {
    let mut accounts = parse_platform_accounts::<WeComConfig>(value, WECOM_SCALAR_KEYS, "wecom")?;
    accounts.retain(|_, cfg| !cfg.is_empty());
    Ok(accounts)
}

fn parse_feishu_accounts(value: Option<&toml::Value>) -> Result<BTreeMap<String, FeishuConfig>> {
    let mut accounts =
        parse_platform_accounts::<FeishuConfig>(value, FEISHU_SCALAR_KEYS, "feishu")?;
    accounts.retain(|_, cfg| !cfg.is_empty());
    Ok(accounts)
}

fn parse_weixin_accounts(value: Option<&toml::Value>) -> Result<BTreeMap<String, WeixinConfig>> {
    let mut accounts =
        parse_platform_accounts::<WeixinConfig>(value, WEIXIN_SCALAR_KEYS, "weixin")?;
    accounts.retain(|_, cfg| !cfg.is_empty());
    Ok(accounts)
}

fn parse_platform_accounts<T: for<'de> serde::Deserialize<'de>>(
    value: Option<&toml::Value>,
    scalar_keys: &[&str],
    label: &str,
) -> Result<BTreeMap<String, T>> {
    let Some(toml::Value::Table(table)) = value else {
        return Ok(BTreeMap::new());
    };

    let mut root = toml::Table::new();
    let mut nested = BTreeMap::new();
    for (key, val) in table {
        if val.is_table() && !scalar_keys.contains(&key.as_str()) {
            nested.insert(key.clone(), val.clone());
        } else {
            root.insert(key.clone(), val.clone());
        }
    }

    let mut accounts = BTreeMap::new();
    if !root.is_empty() {
        let cfg: T = root.try_into().map_err(|e| {
            Error::Message(format!("parse {label} channel: {e}"))
        })?;
        accounts.insert("default".into(), cfg);
    }
    for (name, val) in nested {
        let cfg: T = val.try_into().map_err(|e| {
            Error::Message(format!("parse {label} channel {name}: {e}"))
        })?;
        accounts.insert(name, cfg);
    }
    Ok(accounts)
}

fn apply_legacy_channel_selection(
    accounts: &mut BTreeMap<String, WeComConfig>,
    legacy_enabled: Option<&toml::Value>,
    legacy_default: Option<&toml::Value>,
) {
    let enabled_list = legacy_enabled.and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect::<Vec<_>>()
    });

    if let Some(list) = enabled_list {
        for cfg in accounts.values_mut() {
            cfg.enabled = false;
        }
        for id in list {
            if let Some(name) = account_name_for_wecom_endpoint_id(&id) {
                if let Some(cfg) = accounts.get_mut(&name) {
                    cfg.enabled = true;
                }
            }
        }
        return;
    }

    if let Some(default) = legacy_default.and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        for cfg in accounts.values_mut() {
            cfg.enabled = false;
        }
        if let Some(name) = account_name_for_wecom_endpoint_id(default) {
            if let Some(cfg) = accounts.get_mut(&name) {
                cfg.enabled = true;
            }
        }
    }
}

fn account_name_for_platform_endpoint_id(platform: &str, id: &str) -> Option<String> {
    if id == platform {
        return Some("default".into());
    }
    id.strip_prefix(&format!("{platform}:")).map(str::to_string)
}

impl ChannelsConfig {
    fn load_from_document(doc: &toml::Value) -> Result<Self> {
        if doc.as_table().is_none_or(|t| t.is_empty()) {
            return Ok(Self::default());
        }

        let root = doc.as_table().unwrap();
        let legacy_enabled = root.get("enabled");
        let legacy_default = root.get("default");

        let channels_table = root.get("channels").and_then(|v| v.as_table());

        let mut wecom_accounts = if let Some(channels) = channels_table {
            parse_wecom_accounts(channels.get("wecom"))?
        } else {
            BTreeMap::new()
        };

        let feishu_accounts = if let Some(channels) = channels_table {
            parse_feishu_accounts(channels.get("feishu"))?
        } else {
            BTreeMap::new()
        };

        let weixin_accounts = if let Some(channels) = channels_table {
            parse_weixin_accounts(channels.get("weixin"))?
        } else {
            BTreeMap::new()
        };

        if wecom_accounts.is_empty() {
            wecom_accounts = parse_wecom_accounts(root.get("wecom"))?;
        }

        if legacy_enabled.is_some() || legacy_default.is_some() {
            apply_legacy_channel_selection(
                &mut wecom_accounts,
                legacy_enabled,
                legacy_default,
            );
        }

        let mut channels = Self {
            wecom_accounts,
            feishu_accounts,
            weixin_accounts,
        };
        channels.normalize();
        Ok(channels)
    }
}

#[cfg(test)]
#[path = "../../test/unit/config/channels.rs"]
mod tests;
