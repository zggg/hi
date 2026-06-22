use std::io::{self, Write};

use async_trait::async_trait;
use hi_core::approval::ApprovalHandler;
use hi_core::{resolve_locale, t, MessageId};
use hi_core::Result;

/// Author: gz
pub struct StdinApproval;

#[async_trait]
impl ApprovalHandler for StdinApproval {
    async fn approve_bash(&self, command: &str) -> Result<bool> {
        let locale = resolve_locale(None);
        eprintln!("\n{}", t(locale, MessageId::StdinApprovalHeader, &[]));
        eprintln!("   {command}");
        print!("{}", t(locale, MessageId::StdinApprovalPrompt, &[]));
        io::stdout()
            .flush()
            .map_err(|e| hi_core::Error::Message(e.to_string()))?;

        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| hi_core::Error::Message(e.to_string()))?;
        let line = line.trim().to_lowercase();
        Ok(line == "y" || line == "yes")
    }
}
