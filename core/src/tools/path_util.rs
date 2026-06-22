use std::path::{Path, PathBuf};

use crate::config::expand_path;
use crate::error::{Error, Result};

/// Workspace anchor for relative file paths.
#[derive(Debug, Clone)]
pub struct FileAccess {
    pub workspace: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ResolvedPath {
    pub path: PathBuf,
}

fn candidate_path(access: &FileAccess, path: &str) -> Result<(PathBuf, PathBuf)> {
    let workspace = access
        .workspace
        .canonicalize()
        .map_err(|e| Error::Message(format!("invalid workspace: {e}")))?;

    let candidate = expand_path(path);
    let candidate = if candidate.is_absolute() {
        candidate
    } else {
        workspace.join(&candidate)
    };
    Ok((workspace, candidate))
}

fn ensure_under_workspace(workspace: &Path, resolved: &Path) -> Result<()> {
    if resolved.starts_with(workspace) {
        return Ok(());
    }
    Err(Error::Message(format!(
        "path escapes workspace: {}",
        resolved.display()
    )))
}

/// Resolve `path` for read/edit (target must already exist).
pub fn resolve_path(access: &FileAccess, path: &str) -> Result<ResolvedPath> {
    let (workspace, candidate) = candidate_path(access, path)?;

    let resolved = candidate
        .canonicalize()
        .map_err(|e| Error::Message(format!("path not found: {path} ({e})")))?;

    if !candidate.is_absolute() {
        ensure_under_workspace(&workspace, &resolved)?;
    }

    Ok(ResolvedPath { path: resolved })
}

/// Resolve `path` for write (creates are allowed; parent directory must exist or be under workspace).
pub fn resolve_path_for_write(access: &FileAccess, path: &str) -> Result<ResolvedPath> {
    let (workspace, candidate) = candidate_path(access, path)?;

    if candidate.exists() {
        let resolved = candidate
            .canonicalize()
            .map_err(|e| Error::Message(format!("path not found: {path} ({e})")))?;
        if !candidate.is_absolute() {
            ensure_under_workspace(&workspace, &resolved)?;
        }
        return Ok(ResolvedPath { path: resolved });
    }

    if let Some(parent) = candidate.parent() {
        if parent.exists() {
            let canon_parent = parent
                .canonicalize()
                .map_err(|e| Error::Message(format!("invalid parent for {path}: {e}")))?;
            let file_name = candidate.file_name().ok_or_else(|| {
                Error::Message(format!("write: invalid path (no file name): {path}"))
            })?;
            let resolved = canon_parent.join(file_name);
            if !candidate.is_absolute() {
                ensure_under_workspace(&workspace, &resolved)?;
            }
            return Ok(ResolvedPath { path: resolved });
        }
    }

    if candidate.is_absolute() {
        return Ok(ResolvedPath { path: candidate });
    }

    ensure_under_workspace(&workspace, &candidate)?;
    Ok(ResolvedPath { path: candidate })
}

#[cfg(test)]
#[path = "../../test/unit/tools/path_util.rs"]
mod tests;
