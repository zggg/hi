use serde::{Deserialize, Serialize};

/// UI language preference (`~/.hi/hi.toml` `[locale]`).
///
/// Author: gz
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocaleConfig {
    /// `zh` or `en`. Omitted → follow system on every run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

impl LocaleConfig {
    pub fn set_lang(&mut self, locale: crate::messages::Locale) {
        self.lang = Some(locale.as_str().to_string());
    }
}
