const MAX_DEOBFUSCATE_ITER: usize = 16;

/// Strip control noise, then run bounded fixed-point de-obfuscation.
pub fn normalize_for_detection(input: &str) -> String {
    normalize_with_status(input).0
}

/// Returns `(normalized, converged)`.
pub fn normalize_with_status(input: &str) -> (String, bool) {
    deobfuscate(&base_normalize(input))
}

/// Returns `(normalized, converged)`.
pub fn deobfuscate(input: &str) -> (String, bool) {
    let mut current = input.to_string();
    for _ in 0..MAX_DEOBFUSCATE_ITER {
        let next = apply_rules_once(&current);
        if next == current {
            return (current, true);
        }
        current = next;
    }
    (current, false)
}

/// Bodies of `` `...` `` / `$(...)` for secondary analysis (literal unwrap only).
pub fn extract_substitution_bodies(input: &str) -> Vec<String> {
    let mut bodies = Vec::new();
    collect_backtick_bodies(input, &mut bodies);
    collect_dollar_paren_bodies(input, &mut bodies);
    bodies
}

fn base_normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\0' {
            continue;
        }
        if c == '\u{001b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn apply_rules_once(input: &str) -> String {
    let mut s = input.to_string();
    s = unwrap_command_substitutions(&s);
    s = expand_ifs(&s);
    s = decode_ansi_c_quotes(&s);
    s = collapse_empty_quotes(&s);
    s = collapse_backslash_escapes(&s);
    s = strip_wrapper_prefixes(&s);
    s = collapse_adjacent_quote_splits(&s);
    s
}

fn expand_ifs(input: &str) -> String {
    input
        .replace("${IFS}", " ")
        .replace("$IFS", " ")
}

fn decode_ansi_c_quotes(input: &str) -> String {
    if !input.contains("$'") {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'\'') {
            chars.next();
            let mut decoded = String::new();
            while let Some(ch) = chars.next() {
                if ch == '\'' {
                    break;
                }
                if ch == '\\' {
                    let Some(next) = chars.next() else { break };
                    decoded.push(match next {
                        'a' => '\u{07}',
                        'b' => '\u{08}',
                        'e' | 'E' => '\u{1b}',
                        'f' => '\u{0c}',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'v' => '\u{0b}',
                        '\\' => '\\',
                        '\'' => '\'',
                        '"' => '"',
                        '?' => '\u{7f}',
                        'x' => {
                            let mut hex = String::new();
                            while matches!(chars.peek(), Some(c) if c.is_ascii_hexdigit()) {
                                hex.push(chars.next().unwrap());
                            }
                            u32::from_str_radix(if hex.is_empty() { "0" } else { &hex }, 16)
                                .ok()
                                .and_then(char::from_u32)
                                .unwrap_or('?')
                        }
                        '0'..='7' => {
                            let mut oct = String::from(next);
                            for _ in 0..2 {
                                if matches!(chars.peek(), Some(c) if c.is_ascii_digit() && *c <= '7') {
                                    oct.push(chars.next().unwrap());
                                } else {
                                    break;
                                }
                            }
                            u32::from_str_radix(&oct, 8)
                                .ok()
                                .and_then(char::from_u32)
                                .unwrap_or('?')
                        }
                        other => other,
                    });
                    continue;
                }
                decoded.push(ch);
            }
            out.push_str(&decoded);
            continue;
        }
        out.push(c);
    }
    out
}

fn collapse_empty_quotes(input: &str) -> String {
    let mut s = input.to_string();
    for _ in 0..32 {
        let next = s.replace("''", "").replace("\"\"", "");
        if next == s {
            break;
        }
        s = next;
    }
    s
}

fn collapse_adjacent_quote_splits(input: &str) -> String {
    let mut s = input.to_string();
    for _ in 0..32 {
        let next = s
            .replace("' '", "")
            .replace("\" \"", "")
            .replace("'\"", "")
            .replace("\"'", "");
        if next == s {
            break;
        }
        s = next;
    }
    s
}

fn collapse_backslash_escapes(input: &str) -> String {
    let mut out = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if !in_double && c == '\'' {
            in_single = !in_single;
            out.push('\'');
            continue;
        }
        if !in_single && c == '"' {
            in_double = !in_double;
            out.push('"');
            continue;
        }
        if !in_single && !in_double && c == '\\' {
            if let Some(next) = chars.next() {
                if next != '\n' && next != '\r' {
                    out.push(next);
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn strip_wrapper_prefixes(input: &str) -> String {
    let mut s = input.to_string();
    for _ in 0..8 {
        let next = strip_wrappers_once(&s);
        if next == s {
            return s;
        }
        s = next;
    }
    s
}

fn strip_wrappers_once(input: &str) -> String {
    const WRAPPERS: &[&str] = &["eval ", "command ", "builtin "];
    split_command_segments(input)
        .into_iter()
        .map(|segment| {
            let mut seg = segment.trim().to_string();
            loop {
                let lower = seg.to_ascii_lowercase();
                let Some(prefix) = WRAPPERS.iter().find(|p| lower.starts_with(**p)) else {
                    break;
                };
                seg = seg[prefix.len()..].trim_start().to_string();
            }
            seg
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Split on `;` / `&&` / `||` / `|` only (not whitespace).
fn split_command_segments(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if !in_double && c == '\'' {
            in_single = !in_single;
            current.push(c);
            continue;
        }
        if !in_single && c == '"' {
            in_double = !in_double;
            current.push(c);
            continue;
        }
        if !in_single && !in_double {
            if c == '&' && chars.peek() == Some(&'&') {
                chars.next();
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
                continue;
            }
            if c == '|' {
                if chars.peek() == Some(&'|') {
                    chars.next();
                }
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
                continue;
            }
            if c == ';' {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
                continue;
            }
        }
        current.push(c);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn collect_backtick_bodies(input: &str, out: &mut Vec<String>) {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'`' {
                i += 1;
            }
            out.push(String::from_utf8_lossy(&bytes[start..i]).into_owned());
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        i += 1;
    }
}

fn collect_dollar_paren_bodies(input: &str, out: &mut Vec<String>) {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'$' && bytes[i + 1] == b'(' {
            i += 2;
            let start = i;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    i += 1;
                }
            }
            if depth == 0 {
                out.push(String::from_utf8_lossy(&bytes[start..i]).into_owned());
            }
            i += 1;
            continue;
        }
        i += 1;
    }
}

fn unwrap_command_substitutions(input: &str) -> String {
    let mut s = input.to_string();
    for _ in 0..8 {
        let next = unwrap_command_substitutions_once(&s);
        if next == s {
            return s;
        }
        s = next;
    }
    s
}

fn unwrap_command_substitutions_once(input: &str) -> String {
    let mut out = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if !in_double && c == '\'' {
            in_single = !in_single;
            out.push(c);
            continue;
        }
        if !in_single && c == '"' {
            in_double = !in_double;
            out.push(c);
            continue;
        }
        if !in_single && !in_double && c == '`' {
            let mut body = String::new();
            for ch in chars.by_ref() {
                if ch == '`' {
                    break;
                }
                body.push(ch);
            }
            out.push_str(&literal_substitution_value(&body));
            continue;
        }
        if !in_single && !in_double && c == '$' && chars.peek() == Some(&'(') {
            chars.next();
            let mut body = String::new();
            let mut depth = 1usize;
            for ch in chars.by_ref() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                if depth > 0 {
                    body.push(ch);
                }
            }
            out.push_str(&literal_substitution_value(&body));
            continue;
        }
        out.push(c);
    }
    out
}

/// Best-effort static expansion for common obfuscation: `echo rm` → `rm`.
pub fn simulate_substitution_body(body: &str) -> String {
    literal_substitution_value(body)
}

fn literal_substitution_value(body: &str) -> String {
    let words = split_words(body);
    if words.is_empty() {
        return body.trim().to_string();
    }
    if words[0].eq_ignore_ascii_case("echo") {
        let mut args = words[1..].iter().map(String::as_str);
        while let Some(arg) = args.next() {
            if !is_echo_flag(arg) {
                return std::iter::once(arg).chain(args).collect::<Vec<_>>().join(" ");
            }
        }
        return String::new();
    }
    body.trim().to_string()
}

fn is_echo_flag(word: &str) -> bool {
    word.starts_with('-')
        && word.len() > 1
        && word[1..].chars().all(|c| matches!(c, 'n' | 'e' | 'E'))
}

fn split_words(input: &str) -> Vec<String> {
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
#[path = "../../test/unit/approval/shell_deobfuscate.rs"]
mod tests;
