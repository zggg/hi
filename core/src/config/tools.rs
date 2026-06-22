use serde::{Deserialize, Serialize};

/// Tool-related settings under `[tools]` in `hi.toml`.
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ToolsConfig {
    #[serde(default)]
    pub approvals: ApprovalsConfig,
}

/// Unified approval policy under `[tools.approvals]`.
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalsConfig {
    #[serde(default = "default_approval_mode_on")]
    pub mode: ApprovalMode,
    #[serde(default)]
    pub workspace: WorkspaceApprovalConfig,
    #[serde(default)]
    pub filesystem: FilesystemApprovalConfig,
    #[serde(default)]
    pub commands: CommandsApprovalConfig,
}

impl Default for ApprovalsConfig {
    fn default() -> Self {
        Self {
            mode: ApprovalMode::On,
            workspace: WorkspaceApprovalConfig::default(),
            filesystem: FilesystemApprovalConfig::default(),
            commands: CommandsApprovalConfig::default(),
        }
    }
}

impl ApprovalsConfig {
    pub fn has_runtime_grants(&self) -> bool {
        !self.commands.allow.is_empty() || !self.filesystem.allow_write.is_empty()
    }

    pub fn is_default(&self) -> bool {
        self.mode == ApprovalMode::On
            && self.workspace.trust
            && self.filesystem.is_empty()
            && self.commands.allow.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalMode {
    #[default]
    On,
    Off,
}

fn default_approval_mode_on() -> ApprovalMode {
    ApprovalMode::On
}

/// `[tools.approvals.workspace]` — workspace 内 read/write/edit/bash 免审（hardline 与 deny 除外）。
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceApprovalConfig {
    #[serde(default = "default_true")]
    pub trust: bool,
}

impl Default for WorkspaceApprovalConfig {
    fn default() -> Self {
        Self { trust: true }
    }
}

fn default_true() -> bool {
    true
}

/// `[tools.approvals.filesystem]` — 路径前缀 allow/deny；bash 重定向写入同样适用。
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FilesystemApprovalConfig {
    #[serde(default)]
    pub allow_read: Vec<String>,
    #[serde(default)]
    pub allow_write: Vec<String>,
    #[serde(default)]
    pub deny_read: Vec<String>,
    #[serde(default)]
    pub deny_write: Vec<String>,
}

impl FilesystemApprovalConfig {
    pub fn is_empty(&self) -> bool {
        self.allow_read.is_empty()
            && self.allow_write.is_empty()
            && self.deny_read.is_empty()
            && self.deny_write.is_empty()
    }
}

/// `[tools.approvals.commands]` — bash 危险命令免审（按真实命令名 / grant_key 匹配）。
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CommandsApprovalConfig {
    #[serde(default)]
    pub allow: Vec<String>,
}
