/// Line-oriented diff for TUI / chat previews.
///
/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Remove,
    Add,
    Context,
}

/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}

/// Diff two text blobs line-by-line (LCS), suitable for edit hunks and file overwrites.
///
/// Author: gz
pub fn line_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    if a.is_empty() && b.is_empty() {
        return Vec::new();
    }
    if a.is_empty() {
        return b
            .into_iter()
            .map(|line| DiffLine {
                kind: DiffKind::Add,
                text: line.to_string(),
            })
            .collect();
    }
    if b.is_empty() {
        return a
            .into_iter()
            .map(|line| DiffLine {
                kind: DiffKind::Remove,
                text: line.to_string(),
            })
            .collect();
    }

    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in a.iter().enumerate() {
        for (j, col) in b.iter().enumerate() {
            if row == col {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i][j + 1].max(dp[i + 1][j]);
            }
        }
    }

    let mut out = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            out.push(DiffLine {
                kind: DiffKind::Context,
                text: a[i - 1].to_string(),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            out.push(DiffLine {
                kind: DiffKind::Add,
                text: b[j - 1].to_string(),
            });
            j -= 1;
        } else {
            out.push(DiffLine {
                kind: DiffKind::Remove,
                text: a[i - 1].to_string(),
            });
            i -= 1;
        }
    }
    out.reverse();
    out
}

#[cfg(test)]
#[path = "../test/unit/diff.rs"]
mod tests;
