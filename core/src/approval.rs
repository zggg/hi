mod policy;
mod shell_deobfuscate;
mod shell_cmd;

use async_trait::async_trait;

pub use policy::{
    bash_write_targets, file_approval_prompt, format_approval_prompt, is_builtin_dangerous,
    is_hardline, is_approval_confirm, is_approval_deny, permission_dir_for,
    shared_approval_policy, ApprovalNeed, ApprovalPolicy,
    FileOp, GrantKind, SharedApprovalPolicy,
};
pub use shell_cmd::{analyze_dangers, parse_command_line, primary_grant_key, CommandLine, DangerHit};

use crate::error::Result;

/// Returns true when default policy (no grants) would require approval.
pub fn is_dangerous_command(command: &str) -> bool {
    use crate::config::ApprovalsConfig;
    ApprovalPolicy::from_config(&ApprovalsConfig::default(), crate::messages::Locale::Zh)
        .requires_approval(command)
}

#[async_trait]
/// Author: gz
pub trait ApprovalHandler: Send + Sync {
    async fn approve_bash(&self, command: &str) -> Result<bool>;
}

#[cfg(test)]
#[path = "../test/unit/approval.rs"]
mod tests;
