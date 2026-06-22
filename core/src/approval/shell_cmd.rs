use std::path::Path;

use crate::approval::shell_deobfuscate::{
    extract_substitution_bodies, normalize_for_detection, normalize_with_status,
    simulate_substitution_body,
};

/// One simple command (`argv[0]` + args), after leading `VAR=val` assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCommand {
    pub words: Vec<String>,
}

impl SimpleCommand {
    pub fn name(&self) -> Option<&str> {
        self.words.first().map(|w| command_basename(w))
    }

    pub fn args(&self) -> &[String] {
        if self.words.len() <= 1 {
            &[]
        } else {
            &self.words[1..]
        }
    }
}

/// Commands connected by `|`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub commands: Vec<SimpleCommand>,
}

/// Full user command line (`;` / `&&` / `||` separated pipelines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub pipelines: Vec<Pipeline>,
}

/// Why a command needs approval — `grant_key` is stored in `[tools.approvals.commands].allow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DangerHit {
    pub grant_key: String,
    pub hardline: bool,
}

pub fn parse_command_line(input: &str) -> CommandLine {
    let mut pipelines = Vec::new();
    for unit in split_command_units(input) {
        let unit = unit.trim();
        if unit.is_empty() {
            continue;
        }
        let mut commands = Vec::new();
        for segment in split_pipelines(unit) {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }
            commands.push(parse_simple_command(segment));
        }
        if !commands.is_empty() {
            pipelines.push(Pipeline { commands });
        }
    }
    CommandLine { pipelines }
}

pub fn command_basename(word: &str) -> &str {
    Path::new(word)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(word)
}

/// Best-effort write targets for bash filesystem approval (`>`, `cp`, `mv`, `tee`, …).
pub fn bash_write_targets(input: &str) -> Vec<String> {
    let mut targets = Vec::new();
    collect_bash_write_targets(input, &mut targets);
    if targets.is_empty() {
        collect_bash_write_targets(&normalize_for_detection(input), &mut targets);
    }
    targets.sort();
    targets.dedup();
    targets
}

fn collect_bash_write_targets(input: &str, targets: &mut Vec<String>) {
    for unit in split_command_units(input) {
        let unit = unit.trim();
        if unit.is_empty() {
            continue;
        }
        for segment in split_pipelines(unit) {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }
            let words = tokenize_words(segment);
            targets.extend(redirect_targets_from_words(&words));
            targets.extend(command_write_targets(&parse_simple_command(segment)));
        }
    }
}

fn redirect_targets_from_words(words: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, token) in words.iter().enumerate() {
        if token == ">" || token == ">>" {
            if let Some(next) = words.get(i + 1) {
                push_write_path(&mut out, next);
            }
            continue;
        }
        if token.starts_with('>') && token != ">" && token != ">>" {
            let path = token.trim_start_matches('>').trim_start_matches('>');
            push_write_path(&mut out, path);
        }
    }
    out
}

fn command_write_targets(cmd: &SimpleCommand) -> Vec<String> {
    let Some(name) = cmd.name() else {
        return Vec::new();
    };
    let args = cmd.args();
    match name {
        "cp" | "mv" | "install" => last_path_arg(args).into_iter().collect(),
        "tee" => positional_path_args(args),
        "touch" => positional_path_args(args),
        "sed" => sed_inplace_targets(args),
        "dd" => dd_of_target(args).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn push_write_path(out: &mut Vec<String>, raw: &str) {
    let path = clean_path_token(raw);
    if is_meaningful_write_path(&path) {
        out.push(path);
    }
}

fn clean_path_token(raw: &str) -> String {
    raw.trim()
        .trim_matches(['"', '\''])
        .trim_end_matches([';', '|', '&'])
        .to_string()
}

fn is_meaningful_write_path(path: &str) -> bool {
    !path.is_empty() && path != "/dev/null"
}

fn positional_path_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|a| !a.starts_with('-'))
        .filter_map(|a| {
            let p = clean_path_token(a);
            is_meaningful_write_path(&p).then_some(p)
        })
        .collect()
}

fn last_path_arg(args: &[String]) -> Option<String> {
    positional_path_args(args).into_iter().next_back()
}

fn sed_inplace_targets(args: &[String]) -> Vec<String> {
    let mut saw_inplace = false;
    let mut skipped_script = false;
    let mut paths = Vec::new();
    for a in args {
        if *a == "-i" || *a == "--in-place" {
            saw_inplace = true;
            continue;
        }
        if a.starts_with("-i") && a.len() > 2 {
            saw_inplace = true;
            continue;
        }
        if a.starts_with("--") || a.starts_with('-') {
            continue;
        }
        if !saw_inplace {
            continue;
        }
        if !skipped_script {
            skipped_script = true;
            continue;
        }
        push_write_path(&mut paths, a);
    }
    paths
}

fn dd_of_target(args: &[String]) -> Option<String> {
    for a in args {
        if let Some(path) = a.strip_prefix("of=") {
            let p = clean_path_token(path);
            if is_meaningful_write_path(&p) {
                return Some(p);
            }
        }
    }
    None
}

pub fn analyze_dangers(input: &str) -> Vec<DangerHit> {
    let mut hits = Vec::new();
    let (_, converged) = normalize_with_status(input);
    if !converged {
        hits.push(DangerHit {
            grant_key: "obfuscated-shell".into(),
            hardline: false,
        });
    }
    for surface in analysis_surfaces(input) {
        hits.extend(analyze_surface(&surface));
    }
    hits.sort_by(|a, b| a.grant_key.cmp(&b.grant_key));
    hits.dedup_by(|a, b| a.grant_key == b.grant_key && a.hardline == b.hardline);
    hits
}

fn analysis_surfaces(input: &str) -> Vec<String> {
    let mut surfaces = vec![input.to_string()];
    let normalized = normalize_for_detection(input);
    if normalized != input {
        surfaces.push(normalized);
    }
    for body in extract_substitution_bodies(input) {
        if body.trim().is_empty() {
            continue;
        }
        surfaces.push(body.clone());
        let nested = normalize_for_detection(&body);
        if nested != body {
            surfaces.push(nested);
        }
        let simulated = simulate_substitution_body(&body);
        if !simulated.is_empty() && simulated != body {
            surfaces.push(simulated);
        }
    }
    surfaces.sort();
    surfaces.dedup();
    surfaces
}

fn analyze_surface(input: &str) -> Vec<DangerHit> {
    let line = parse_command_line(input);
    let mut hits = Vec::new();

    if is_fork_bomb(input) {
        hits.push(DangerHit {
            grant_key: "fork-bomb".into(),
            hardline: true,
        });
    }

    if redirects_to_dev(input) {
        hits.push(DangerHit {
            grant_key: "redirect-dev".into(),
            hardline: true,
        });
    }

    for pipeline in &line.pipelines {
        if let Some(key) = pipeline_danger(pipeline) {
            hits.push(DangerHit {
                grant_key: key.to_string(),
                hardline: false,
            });
        }
        for cmd in &pipeline.commands {
            hits.extend(analyze_simple_command(cmd));
        }
    }
    hits
}

pub fn is_hardline_command(input: &str) -> bool {
    analyze_dangers(input).iter().any(|h| h.hardline)
}

pub fn is_dangerous_command(input: &str) -> bool {
    !analyze_dangers(input).is_empty()
}

pub fn primary_grant_key(input: &str) -> String {
    analyze_dangers(input)
        .first()
        .map(|h| h.grant_key.clone())
        .unwrap_or_else(|| {
            parse_command_line(input)
                .pipelines
                .first()
                .and_then(|p| p.commands.first())
                .and_then(|c| c.name())
                .unwrap_or("bash")
                .to_string()
        })
}

pub fn is_allowlisted(input: &str, allow: &[String]) -> bool {
    let hits = analyze_dangers(input);
    if hits.is_empty() {
        return false;
    }
    hits.iter().all(|hit| {
        allow.iter().any(|entry| {
            let entry = entry.trim();
            !entry.is_empty() && entry.eq_ignore_ascii_case(&hit.grant_key)
        })
    })
}

fn analyze_simple_command(cmd: &SimpleCommand) -> Vec<DangerHit> {
    let Some(name) = cmd.name() else {
        return Vec::new();
    };
    let args = cmd.args();
    let mut hits = Vec::new();

    match name {
        "sudo" => hits.push(hit("sudo", false)),
        "curl" | "wget" => hits.push(hit(name, false)),
        "eval" => hits.push(hit("eval", false)),
        "mkfs" => hits.push(hit("mkfs", true)),
        "dd" => hits.push(hit("dd", true)),
        "rm" if rm_is_recursive(args) => {
            let hardline = rm_targets_root(args);
            hits.push(DangerHit {
                grant_key: "rm".into(),
                hardline,
            });
        }
        "chmod" if chmod_is_world_writable(args) => {
            hits.push(hit("chmod", true));
        }
        "bash" | "sh" | "dash" | "zsh" | "ksh" if shell_runs_inline_code(name, args) => {
            hits.push(hit(name, false));
        }
        "python" | "python3" | "perl" | "ruby" | "node" if script_runs_inline_code(args) => {
            hits.push(hit(name, false));
        }
        _ => {}
    }
    hits
}

fn hit(key: &str, hardline: bool) -> DangerHit {
    DangerHit {
        grant_key: key.to_string(),
        hardline,
    }
}

fn rm_is_recursive(args: &[String]) -> bool {
    args.iter().any(|arg| {
        if arg == "--recursive" {
            return true;
        }
        if arg.starts_with('-') && !arg.starts_with("--") {
            return arg.contains('r');
        }
        false
    })
}

fn rm_targets_root(args: &[String]) -> bool {
    args.iter().any(|arg| {
        if arg.starts_with('-') {
            return false;
        }
        let path = arg.trim_matches(['"', '\'']);
        path == "/" || path == "/*"
    })
}

fn chmod_is_world_writable(args: &[String]) -> bool {
    args.iter().any(|arg| {
        let a = arg.trim();
        a == "777"
            || a == "666"
            || a == "0777"
            || a == "0666"
            || a.contains("o+w")
            || a.contains("a+w")
    })
}

fn shell_runs_inline_code(_name: &str, args: &[String]) -> bool {
    args.iter().any(|arg| arg == "-c" || arg == "--command")
        || args.iter().any(|arg| {
            arg.starts_with('-') && !arg.starts_with("--") && arg.contains('c')
        })
}

fn script_runs_inline_code(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "-e" || arg == "-c" || arg == "--command")
}

fn pipeline_danger(pipeline: &Pipeline) -> Option<&'static str> {
    if pipeline.commands.len() < 2 {
        return None;
    }
    for pair in pipeline.commands.windows(2) {
        let left = pair[0].name().unwrap_or("");
        let right = pair[1].name().unwrap_or("");
        if matches!(left, "curl" | "wget")
            && matches!(right, "sh" | "bash" | "dash" | "zsh" | "ksh")
        {
            return Some("pipe-to-shell");
        }
        if left == "base64"
            && pair[0]
                .args()
                .iter()
                .any(|a| matches!(a.as_str(), "-d" | "-D" | "--decode"))
            && matches!(right, "sh" | "bash" | "dash" | "zsh" | "ksh")
        {
            return Some("pipe-to-shell");
        }
    }
    None
}

fn is_fork_bomb(input: &str) -> bool {
    let compact: String = input.split_whitespace().collect();
    compact.contains(":(){") || compact.contains(":(){")
}

fn redirects_to_dev(input: &str) -> bool {
    let tokens = tokenize_words(input);
    for (i, token) in tokens.iter().enumerate() {
        if token == ">" || token == ">>" {
            if let Some(next) = tokens.get(i + 1) {
                let target = next.trim_matches(['"', '\'']);
                if target.starts_with("/dev/") {
                    return true;
                }
            }
            continue;
        }
        if token.starts_with('>') {
            let target = token.trim_start_matches('>').trim_start_matches('>');
            let target = target.trim_matches(['"', '\'']);
            if target.starts_with("/dev/") {
                return true;
            }
        }
    }
    false
}

fn parse_simple_command(segment: &str) -> SimpleCommand {
    let words = tokenize_words(segment);
    let mut start = 0;
    while start < words.len() && looks_like_env_assign(&words[start]) {
        start += 1;
    }
    SimpleCommand {
        words: words[start..].to_vec(),
    }
}

fn looks_like_env_assign(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && word.len() > name.len() + 1
}

fn split_command_units(input: &str) -> Vec<String> {
    split_on_operators(input, &[";", "&&", "||"])
}

fn split_pipelines(input: &str) -> Vec<String> {
    split_on_operators(input, &["|"])
}

fn split_on_operators(input: &str, operators: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;

    while i < bytes.len() {
        let b = bytes[i];
        if !in_double && b == b'\'' {
            in_single = !in_single;
            current.push(char::from(b));
            i += 1;
            continue;
        }
        if !in_single && b == b'"' {
            in_double = !in_double;
            current.push(char::from(b));
            i += 1;
            continue;
        }
        if !in_single && !in_double {
            if let Some(advance) = match_operator(&input[i..], operators) {
                out.push(std::mem::take(&mut current));
                i += advance;
                continue;
            }
        }
        current.push(char::from(b));
        i += 1;
    }
    out.push(current);
    out
}

fn match_operator(rest: &str, operators: &[&str]) -> Option<usize> {
    operators
        .iter()
        .find(|op| rest.starts_with(**op))
        .map(|op| op.len())
}

fn tokenize_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;

    while i < bytes.len() {
        let b = bytes[i];
        if !in_double && b == b'\'' {
            in_single = !in_single;
            i += 1;
            continue;
        }
        if !in_single && b == b'"' {
            in_double = !in_double;
            i += 1;
            continue;
        }
        if !in_single && !in_double && b.is_ascii_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            continue;
        }
        if !in_single && !in_double && b == b'\\' && i + 1 < bytes.len() {
            current.push(char::from(bytes[i + 1]));
            i += 2;
            continue;
        }
        current.push(char::from(b));
        i += 1;
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

#[cfg(test)]
#[path = "../../test/unit/approval/shell_cmd.rs"]
mod tests;
