use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::approval::shell_cmd;
use crate::config::{expand_path, ApprovalsConfig, ApprovalMode, Config};
use crate::error::{Error, Result};

/// Shared unified approval policy (gateway reload updates in place).
pub type SharedApprovalPolicy = Arc<RwLock<ApprovalPolicy>>;

const CODE_SUBDIRS: &[&str] = &[
    "src", "lib", "test", "tests", "app", "apps", "cmd", "internal", "pkg",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOp {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantKind {
    Command,
    Path(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalNeed {
    pub prompt: String,
    pub grant: GrantKind,
}

/// Unified runtime approval policy for bash + file tools.
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalPolicy {
    mode: ApprovalMode,
    workspace_trust: bool,
    allow_read: Vec<String>,
    allow_write: Vec<String>,
    deny_read: Vec<String>,
    deny_write: Vec<String>,
    command_allow: Vec<String>,
    locale: crate::messages::Locale,
}

pub fn shared_approval_policy(
    config: &ApprovalsConfig,
    locale: crate::messages::Locale,
) -> SharedApprovalPolicy {
    Arc::new(RwLock::new(ApprovalPolicy::from_config(config, locale)))
}

impl ApprovalPolicy {
    pub fn from_config(config: &ApprovalsConfig, locale: crate::messages::Locale) -> Self {
        Self {
            mode: config.mode,
            workspace_trust: config.workspace.trust,
            allow_read: config.filesystem.allow_read.clone(),
            allow_write: config.filesystem.allow_write.clone(),
            deny_read: config.filesystem.deny_read.clone(),
            deny_write: config.filesystem.deny_write.clone(),
            command_allow: config.commands.allow.clone(),
            locale,
        }
    }

    pub fn to_config(&self) -> ApprovalsConfig {
        ApprovalsConfig {
            mode: self.mode,
            workspace: crate::config::WorkspaceApprovalConfig {
                trust: self.workspace_trust,
            },
            filesystem: crate::config::FilesystemApprovalConfig {
                allow_read: self.allow_read.clone(),
                allow_write: self.allow_write.clone(),
                deny_read: self.deny_read.clone(),
                deny_write: self.deny_write.clone(),
            },
            commands: crate::config::CommandsApprovalConfig {
                allow: self.command_allow.clone(),
            },
        }
    }

    pub fn mode_off(&self) -> bool {
        self.mode == ApprovalMode::Off
    }

    pub fn is_hardline_blocked(&self, command: &str) -> bool {
        is_hardline(command)
    }

    /// Whether builtin dangerous patterns apply (ignores workspace/filesystem).
    pub fn requires_approval(&self, command: &str) -> bool {
        if self.mode_off() || self.is_command_exempt(command) {
            return false;
        }
        is_builtin_dangerous(command)
    }

    pub fn requires_bash_approval(
        &self,
        workspace: &Path,
        command: &str,
    ) -> Result<Option<ApprovalNeed>> {
        if self.mode_off() {
            return Ok(None);
        }
        if is_hardline(command) {
            return Ok(None);
        }
        if let Some(need) = self.bash_filesystem_approval(workspace, command)? {
            return Ok(Some(need));
        }
        if self.workspace_trust && self.command_stays_in_workspace(workspace, command)? {
            return Ok(None);
        }
        if self.command_needs_approval(command) {
            return Ok(Some(self.bash_approval_need(command)));
        }
        Ok(None)
    }

    pub fn requires_file_approval(
        &self,
        workspace: &Path,
        resolved: &Path,
        op: FileOp,
    ) -> Result<Option<ApprovalNeed>> {
        if self.mode_off() {
            return Ok(None);
        }
        if self.path_denied(resolved, op)? {
            return Err(Error::Message(format!(
                "file access denied by policy: {}",
                resolved.display()
            )));
        }
        if self.workspace_trust && is_in_workspace(workspace, resolved)? {
            return Ok(None);
        }
        let allow = match op {
            FileOp::Read => &self.allow_read,
            FileOp::Write => &self.allow_write,
        };
        if prefix_covers(resolved, allow) {
            return Ok(None);
        }
        Ok(Some(self.file_approval_need(resolved, op)))
    }

    pub fn grant_command(&mut self, command: &str) -> Result<String> {
        if self.mode_off() || self.is_command_exempt(command) {
            return Ok(Self::grant_pattern(command));
        }
        let entry = Self::grant_pattern(command);
        if !self
            .command_allow
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&entry))
        {
            self.command_allow.push(entry.clone());
        }
        Ok(entry)
    }

    pub fn grant_for_command(&mut self, command: &str) -> Result<String> {
        self.grant_command(command)
    }

    pub fn grant_path(&mut self, resolved: &Path) -> Result<String> {
        let grant = permission_dir_for(resolved)?;
        let entry = grant.display().to_string();
        for list in [&mut self.allow_read, &mut self.allow_write] {
            if !list.iter().any(|p| p == &entry) {
                list.push(entry.clone());
            }
        }
        Ok(entry)
    }

    pub fn grant_for_path(&mut self, resolved: &Path) -> Result<String> {
        self.grant_path(resolved)
    }

    pub fn persist(&self) -> Result<()> {
        Config::sync_tools_approvals(self)
    }

    fn bash_filesystem_approval(
        &self,
        workspace: &Path,
        command: &str,
    ) -> Result<Option<ApprovalNeed>> {
        for raw in bash_write_targets(command) {
            let resolved = resolve_bash_path(workspace, &raw)?;
            if self.path_denied(&resolved, FileOp::Write)? {
                return Err(Error::Message(format!(
                    "bash write denied by policy: {}",
                    resolved.display()
                )));
            }
            if self.workspace_trust && is_in_workspace(workspace, &resolved)? {
                continue;
            }
            if prefix_covers(&resolved, &self.allow_write) {
                continue;
            }
            return Ok(Some(self.file_approval_need(&resolved, FileOp::Write)));
        }
        Ok(None)
    }

    fn command_stays_in_workspace(&self, workspace: &Path, command: &str) -> Result<bool> {
        for raw in bash_write_targets(command) {
            let resolved = resolve_bash_path(workspace, &raw)?;
            if !is_in_workspace(workspace, &resolved)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn command_needs_approval(&self, command: &str) -> bool {
        if self.is_command_exempt(command) {
            return false;
        }
        is_builtin_dangerous(command)
    }

    fn is_command_exempt(&self, command: &str) -> bool {
        shell_cmd::is_allowlisted(command, &self.command_allow)
    }

    fn path_denied(&self, path: &Path, op: FileOp) -> Result<bool> {
        let deny = match op {
            FileOp::Read => &self.deny_read,
            FileOp::Write => &self.deny_write,
        };
        Ok(prefix_covers(path, deny))
    }

    fn bash_approval_need(&self, command: &str) -> ApprovalNeed {
        let grant = Self::grant_pattern(command);
        ApprovalNeed {
            prompt: format_approval_prompt(
                self.locale,
                "bash",
                command,
                &format!("commands.allow = \"{grant}\""),
            ),
            grant: GrantKind::Command,
        }
    }

    fn file_approval_need(&self, resolved: &Path, op: FileOp) -> ApprovalNeed {
        let grant = permission_dir_for(resolved)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| resolved.display().to_string());
        let kind = match op {
            FileOp::Read => "read",
            FileOp::Write => "write",
        };
        ApprovalNeed {
            prompt: format_approval_prompt(
                self.locale,
                kind,
                &resolved.display().to_string(),
                &format!("filesystem.allow_{kind} = \"{grant}\""),
            ),
            grant: GrantKind::Path(resolved.to_path_buf()),
        }
    }

    pub fn grant_pattern(command: &str) -> String {
        shell_cmd::primary_grant_key(command)
    }
}

pub fn format_approval_prompt(
    locale: crate::messages::Locale,
    kind: &str,
    detail: &str,
    grant_label: &str,
) -> String {
    use crate::messages::{t, MessageId};
    if kind == "bash" {
        t(
            locale,
            MessageId::ApprovalPromptBash,
            &[detail.to_string(), grant_label.to_string()],
        )
    } else {
        t(
            locale,
            MessageId::ApprovalPromptFile,
            &[kind.to_string(), detail.to_string(), grant_label.to_string()],
        )
    }
}

/// Gateway / stdin: accept confirm/deny in zh and en.
pub fn is_approval_confirm(text: &str) -> bool {
    matches!(
        text.trim(),
        "确认" | "y" | "Y" | "yes" | "Yes" | "YES" | "approve" | "Approve" | "confirm" | "Confirm"
    )
}

pub fn is_approval_deny(text: &str) -> bool {
    matches!(
        text.trim(),
        "取消" | "n" | "N" | "no" | "No" | "NO" | "deny" | "Deny" | "cancel" | "Cancel"
    )
}

pub fn is_hardline(command: &str) -> bool {
    shell_cmd::is_hardline_command(command)
}

pub fn is_builtin_dangerous(command: &str) -> bool {
    shell_cmd::is_dangerous_command(command)
}

pub use shell_cmd::bash_write_targets;

fn resolve_bash_path(workspace: &Path, raw: &str) -> Result<PathBuf> {
    let path = PathBuf::from(raw);
    let resolved = if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    };
    if resolved.exists() {
        resolved
            .canonicalize()
            .map_err(|e| Error::Message(format!("invalid bash path {}: {e}", resolved.display())))
    } else {
        Ok(resolved)
    }
}

fn is_in_workspace(workspace: &Path, path: &Path) -> Result<bool> {
    let workspace = workspace
        .canonicalize()
        .map_err(|e| Error::Message(format!("invalid workspace: {e}")))?;
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };
    if resolved.exists() {
        return Ok(resolved
            .canonicalize()
            .map_err(|e| Error::Message(format!("invalid path {}: {e}", resolved.display())))?
            .starts_with(&workspace));
    }
    let Some(parent) = resolved.parent() else {
        return Ok(false);
    };
    if parent.exists() {
        return Ok(parent
            .canonicalize()
            .map_err(|e| Error::Message(format!("invalid parent {}: {e}", parent.display())))?
            .starts_with(&workspace));
    }
    Ok(false)
}

fn prefix_covers(path: &Path, permissions: &[String]) -> bool {
    permissions.iter().any(|p| {
        let p = p.trim();
        if p.is_empty() || p == "*" {
            return false;
        }
        prefix_matches(path, p)
    })
}

fn prefix_matches(path: &Path, permission: &str) -> bool {
    let root = expand_path(permission);
    if permission.contains('*') {
        return glob_match(path, permission);
    }
    root.canonicalize()
        .ok()
        .is_some_and(|root| path.starts_with(&root))
}

fn glob_match(path: &Path, pattern: &str) -> bool {
    let path_str = path.to_string_lossy();
    let expanded = expand_path(pattern);
    let pat = expanded.to_string_lossy();
    if let Some(prefix) = pat.strip_suffix('*') {
        return path_str.starts_with(prefix.trim_end_matches('/'));
    }
    path_str == pat
}

/// Directory to grant after user approval (git root or sensible parent).
pub fn permission_dir_for(resolved: &Path) -> Result<PathBuf> {
    if let Some(git_root) = git_repo_root(resolved) {
        return git_root.canonicalize().map_err(|e| {
            Error::Message(format!("invalid git root {}: {e}", git_root.display()))
        });
    }

    let parent = resolved
        .parent()
        .ok_or_else(|| Error::Message(format!("no parent directory for {}", resolved.display())))?;

    let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let chosen = if CODE_SUBDIRS.contains(&parent_name) {
        parent.parent().unwrap_or(parent)
    } else {
        parent
    };

    chosen
        .canonicalize()
        .map_err(|e| Error::Message(format!("invalid permission dir {}: {e}", chosen.display())))
}

fn git_repo_root(path: &Path) -> Option<PathBuf> {
    let mut dir = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
    }
}

pub fn file_approval_prompt(
    locale: crate::messages::Locale,
    name: &str,
    resolved: &Path,
) -> Result<String> {
    let grant = permission_dir_for(resolved)?;
    Ok(format_approval_prompt(
        locale,
        name,
        &resolved.display().to_string(),
        &format!(
            "filesystem.allow_write = \"{}\"",
            grant.display()
        ),
    ))
}

#[cfg(test)]
#[path = "../../test/unit/approval/policy.rs"]
mod tests;
