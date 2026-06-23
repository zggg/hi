use hi_core::error::Error;
use hi_core::{t, Locale, MessageId};

/// Replace all-empty reply chunks with a localized placeholder.
///
/// Author: gz
pub fn normalize_reply_parts(locale: Locale, parts: Vec<String>) -> Vec<String> {
    if parts.iter().all(|p| p.trim().is_empty()) {
        vec![t(locale, MessageId::EmptyChannelReply, &[])]
    } else {
        parts
    }
}

/// Truncate or rewrite errors before showing them in chat.
///
/// Author: gz
pub fn user_visible_error(locale: Locale, err: &Error) -> String {
    let raw = err.to_string();
    if raw.contains("error sending request") || raw.contains("cannot reach LLM service") {
        t(locale, MessageId::LlmTransportError, &[raw])
    } else if raw.len() > 400 {
        format!("{}…", &raw[..400])
    } else {
        raw
    }
}
