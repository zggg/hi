//! Terminal UI (ratatui) — interactive interface for hi personal AI assistant.

mod app;
mod approval;
mod input;
mod menu_scroll;
mod model_picker;
mod render;
mod slash;
mod theme;
mod turn;
mod widgets;

use std::sync::Arc;

use hi_core::{AgentSession, Locale, ModelControl};

pub use approval::SharedApproval;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    session: Box<dyn AgentSession>,
    model: String,
    workdir: String,
    session_id: String,
    model_control: Arc<dyn ModelControl>,
    locale: Locale,
    verbose: bool,
) -> hi_core::Result<()> {
    app::TuiApp::new(session, model, workdir, session_id, model_control, locale, verbose)
        .run()
        .await
}
