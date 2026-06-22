//! Session retention invariants: messages are append-only except explicit purge.

use std::fs;
use std::path::Path;

const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const CORE_SRC: &str = "core/src";

#[test]
fn messages_delete_only_in_purge_module() {
    let root = Path::new(WORKSPACE_ROOT).join(CORE_SRC);
    let mut offenders = Vec::new();

    scan_rs_files(&root, &mut |path, content| {
        let rel = path
            .strip_prefix(WORKSPACE_ROOT)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        if rel.contains("/target/") {
            return;
        }

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("DELETE FROM messages") {
                if !rel.ends_with("store/sessions.rs") {
                    offenders.push(format!("{}:{}  {}", rel, idx + 1, trimmed));
                }
            }
        }
    });

    if !offenders.is_empty() {
        panic!(
            "DELETE FROM messages must only appear in core/src/store/sessions.rs (purge):\n{}",
            offenders.join("\n")
        );
    }
}

#[test]
fn agent_and_context_do_not_call_replace_messages() {
    let forbidden = [
        "core/src/agent.rs",
        "core/src/context.rs",
    ];

    for rel in forbidden {
        let path = Path::new(WORKSPACE_ROOT).join(rel);
        let content = fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !content.contains("replace_messages"),
            "{rel} must not call replace_messages (use mark_out_of_context / update_system_message)"
        );
    }
}

fn scan_rs_files(dir: &Path, f: &mut dyn FnMut(&Path, &str)) {
    if !dir.is_dir() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_rs_files(&path, f);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let content = fs::read_to_string(&path).unwrap_or_default();
            f(&path, &content);
        }
    }
}
