use std::str::FromStr;

/// Supported UI locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    Zh,
    En,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }

    pub fn from_tag(tag: &str) -> Option<Self> {
        let t = tag.trim().to_ascii_lowercase();
        let primary = t.split(['-', '_', '.']).next().unwrap_or("");
        match primary {
            "zh" | "cn" => Some(Self::Zh),
            "en" => Some(Self::En),
            _ => None,
        }
    }
}

impl FromStr for Locale {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_tag(s).ok_or(())
    }
}

/// Read `LANG` / `LC_ALL` / `LC_MESSAGES` (first match).
pub fn detect_system_locale() -> Locale {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(key) {
            if let Some(loc) = Locale::from_tag(&val) {
                return loc;
            }
        }
    }
    Locale::En
}

/// Resolve locale: `HI_LOCALE` → `hi.toml [locale]` → system → `En`.
pub fn resolve_locale(config_lang: Option<&str>) -> Locale {
    if let Ok(env) = std::env::var("HI_LOCALE") {
        if let Some(loc) = Locale::from_tag(&env) {
            return loc;
        }
    }
    if let Some(lang) = config_lang {
        if let Some(loc) = Locale::from_tag(lang) {
            return loc;
        }
    }
    detect_system_locale()
}
