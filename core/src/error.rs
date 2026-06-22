use std::fmt;

use crate::messages::{self, Locale, MessageId};

pub type Result<T> = std::result::Result<T, Error>;

/// Author: gz
#[derive(Debug, Clone)]
pub enum Error {
    /// Legacy free-form message (prefer `Localized`).
    Message(String),
    /// Locale-aware user message resolved via [`Error::render`].
    Localized(MessageId, Vec<String>),
}

impl Error {
    pub fn localized(id: MessageId) -> Self {
        Self::Localized(id, Vec::new())
    }

    pub fn with_arg(id: MessageId, arg: impl Into<String>) -> Self {
        Self::Localized(id, vec![arg.into()])
    }

    pub fn with_args(id: MessageId, args: Vec<String>) -> Self {
        Self::Localized(id, args)
    }

    pub fn render(&self, locale: Locale) -> String {
        match self {
            Self::Message(s) => s.clone(),
            Self::Localized(id, args) => messages::t(locale, *id, args),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render(Locale::Zh))
    }
}

impl std::error::Error for Error {}
