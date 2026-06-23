use async_trait::async_trait;
use hi_core::error::{Error, Result};
use hi_core::Locale;

use super::user_error::{normalize_reply_parts, user_visible_error};

/// Per-turn lifecycle hooks; default implementations are no-ops.
///
/// Author: gz
pub struct TurnHookContext<'a> {
    pub locale: Locale,
    pub user_key: &'a str,
}

#[async_trait]
pub trait TurnHooks: Send + Sync {
    async fn on_turn_start(&self, _ctx: &TurnHookContext<'_>) -> Result<()> {
        Ok(())
    }

    async fn before_run_turn(&self, _ctx: &TurnHookContext<'_>) -> Result<()> {
        Ok(())
    }

    async fn wall_timeout(&self, _ctx: &TurnHookContext<'_>) -> Result<Option<std::time::Duration>> {
        Ok(None)
    }

    async fn after_run_turn(&self, _ctx: &TurnHookContext<'_>) -> Result<()> {
        Ok(())
    }

    fn normalize_parts(&self, locale: Locale, parts: Vec<String>) -> Vec<String> {
        normalize_reply_parts(locale, parts)
    }

    async fn before_deliver(
        &self,
        _ctx: &TurnHookContext<'_>,
        _parts: &mut Vec<String>,
    ) -> Result<()> {
        Ok(())
    }

    async fn on_delivery_failed(&self, _ctx: &TurnHookContext<'_>, _err: &Error) -> Result<()> {
        Ok(())
    }

    fn format_failure(&self, locale: Locale, err: &Error) -> String {
        user_visible_error(locale, err)
    }
}

/// Default hooks for channels without extra UX (Feishu, WeCom).
///
/// Author: gz
pub struct NoopTurnHooks;

#[async_trait]
impl TurnHooks for NoopTurnHooks {}
