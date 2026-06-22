mod catalog;
mod id;
mod locale;

pub use id::MessageId;
pub use locale::{detect_system_locale, resolve_locale, Locale};

/// Format a user-visible message for `locale`.
pub fn t(locale: Locale, id: MessageId, args: &[String]) -> String {
    catalog::format(locale, id, args)
}

#[cfg(test)]
#[path = "../../test/unit/messages/mod.rs"]
mod tests;
