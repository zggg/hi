//! Architecture boundary tests for the hi workspace.
//!
//! Enforces crate dependency rules from docs/architecture/LAYERS.md.
//! Baseline: architecture-tests/known-violations.json (ratchet — entries only shrink).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const KNOWN_VIOLATIONS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/known-violations.json");

/// Crate name → allowed hi workspace dependency names.
fn allowed_deps() -> HashMap<&'static str, HashSet<&'static str>> {
    HashMap::from([
        ("hi-core", HashSet::new()),
        ("hi-ai", HashSet::new()),
        ("hi-tui", HashSet::from(["hi-core"])),
        ("hi-gateway", HashSet::from(["hi-core"])),
        ("hi", HashSet::from(["hi-core", "hi-ai", "hi-tui", "hi-gateway"])),
    ])
}

/// Directory name → package name (from Cargo.toml).
fn crate_dirs() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("core", "hi-core"),
        ("ai", "hi-ai"),
        ("tui", "hi-tui"),
        ("gateway", "hi-gateway"),
        ("app", "hi"),
    ])
}

fn layer_of(crate_name: &str) -> &'static str {
    match crate_name {
        "hi-core" | "hi-ai" => "foundation",
        "hi-tui" | "hi-gateway" => "adapters",
        "hi" => "entry",
        _ => "unknown",
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq, Hash)]
/// Author: gz
struct Violation {
    file: String,
    line: usize,
    imports: String,
    from_layer: String,
    to_layer: String,
    #[serde(default)]
    reason: String,
}

fn violation_key(v: &Violation) -> String {
    format!("{}:{}", v.file, v.imports)
}

fn format_violation(v: &Violation) -> String {
    format!(
        "VIOLATION: {}:{} imports {} — {} cannot import {}. See docs/architecture/LAYERS.md",
        v.file, v.line, v.imports, v.from_layer, v.to_layer
    )
}

fn scan_cargo_deps(root: &Path, rel_dir: &str, package: &str) -> Vec<Violation> {
    let allowed = allowed_deps();
    let Some(permitted) = allowed.get(package) else {
        return vec![];
    };

    let cargo_path = root.join(rel_dir).join("Cargo.toml");
    let content = match fs::read_to_string(&cargo_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut violations = Vec::new();
    let mut in_deps = false;

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]" || trimmed == "[dev-dependencies]";
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((dep, _)) = trimmed.split_once('=') {
            let dep = dep.trim();
            if dep.starts_with("hi") {
                let dep_name = dep.replace('_', "-");
                if !permitted.contains(dep_name.as_str()) {
                    violations.push(Violation {
                        file: format!("{rel_dir}/Cargo.toml"),
                        line: idx + 1,
                        imports: dep_name.clone(),
                        from_layer: layer_of(package).to_string(),
                        to_layer: layer_of(&dep_name).to_string(),
                        reason: String::new(),
                    });
                }
            }
        }
    }
    violations
}

fn hi_crate_from_use(path: &str) -> Option<&'static str> {
    if path.starts_with("hi_core") {
        Some("hi-core")
    } else if path.starts_with("hi_ai") {
        Some("hi-ai")
    } else if path.starts_with("hi_tui") {
        Some("hi-tui")
    } else if path.starts_with("hi_gateway") {
        Some("hi-gateway")
    } else if path.starts_with("hi::") || path == "hi" {
        Some("hi")
    } else {
        None
    }
}

fn scan_rust_sources(root: &Path, rel_dir: &str, package: &str) -> Vec<Violation> {
    let allowed = allowed_deps();
    let Some(permitted) = allowed.get(package) else {
        return vec![];
    };

    let src_dir = root.join(rel_dir).join("src");
    let mut violations = Vec::new();
    collect_rs_files(&src_dir, &mut |file_path| {
        let rel_file = file_path
            .strip_prefix(root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(file_path).unwrap_or_default();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("use ") {
                let import_path = rest.split(';').next().unwrap_or(rest).trim();
                if let Some(target) = hi_crate_from_use(import_path) {
                    if !permitted.contains(target) {
                        violations.push(Violation {
                            file: rel_file.clone(),
                            line: idx + 1,
                            imports: import_path.to_string(),
                            from_layer: layer_of(package).to_string(),
                            to_layer: layer_of(target).to_string(),
                            reason: String::new(),
                        });
                    }
                }
            }
        }
    });
    violations
}

fn collect_rs_files(dir: &Path, f: &mut dyn FnMut(&Path)) {
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
            collect_rs_files(&path, f);
        } else if path.extension().is_some_and(|e| e == "rs") {
            f(&path);
        }
    }
}

fn collect_all_violations(root: &Path) -> Vec<Violation> {
    let dirs = crate_dirs();
    let mut all = Vec::new();
    for (rel_dir, package) in &dirs {
        all.extend(scan_cargo_deps(root, rel_dir, package));
        all.extend(scan_rust_sources(root, rel_dir, package));
    }
    all.sort_by(|a, b| violation_key(a).cmp(&violation_key(b)));
    all
}

fn load_baseline() -> Vec<Violation> {
    let content = fs::read_to_string(KNOWN_VIOLATIONS).unwrap_or_else(|_| "[]".to_string());
    serde_json::from_str(&content).unwrap_or_default()
}

#[test]
fn no_new_architecture_violations() {
    let root = Path::new(WORKSPACE_ROOT);
    let all = collect_all_violations(root);
    let baseline = load_baseline();
    let known: HashSet<String> = baseline.iter().map(violation_key).collect();

    let new_violations: Vec<_> = all
        .iter()
        .filter(|v| !known.contains(&violation_key(v)))
        .collect();

    if !new_violations.is_empty() {
        let msg: Vec<_> = new_violations.iter().map(|v| format_violation(v)).collect();
        panic!("New architecture violations found:\n{}", msg.join("\n"));
    }
}

#[test]
fn violation_count_only_shrinks_ratchet() {
    let root = Path::new(WORKSPACE_ROOT);
    let all = collect_all_violations(root);
    let baseline = load_baseline();
    assert!(
        all.len() <= baseline.len(),
        "Violation count increased: {} > baseline {}. Fix violations to reduce the count — never add new ones.",
        all.len(),
        baseline.len()
    );
}
