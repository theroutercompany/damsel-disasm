//! Pseudocode diffing (plan feature 16, slice 2): for matched function
//! pairs, decompile both sides and compare the rendered lines — a
//! classic LCS line diff over pseudocode, so "what changed inside this
//! function between versions" reads like a source diff.
//!
//! The line text is the comparison key (addresses are provenance, not
//! identity): two lines are equal when they render identically, so
//! pure address moves don't pollute the diff.

use crate::ir::IrFunction;
use crate::render::render_pseudocode;

/// One pseudocode diff line.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PseudocodeDiffLine {
    pub kind: &'static str, // "same" | "add" | "remove"
    pub text: String,
}

/// Diff result for one matched pair.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PseudocodeDiff {
    pub function: String,
    pub left_lines: usize,
    pub right_lines: usize,
    pub added: usize,
    pub removed: usize,
    pub lines: Vec<PseudocodeDiffLine>,
}

/// Compute the LCS line diff between two lifted functions'
/// pseudocode.
pub fn diff_pseudocode(name: &str, left: &IrFunction, right: &IrFunction) -> PseudocodeDiff {
    let left_text: Vec<String> = render_pseudocode(left, None)
        .lines
        .into_iter()
        .map(|line| line.text)
        .collect();
    let right_text: Vec<String> = render_pseudocode(right, None)
        .lines
        .into_iter()
        .map(|line| line.text)
        .collect();
    diff_lines(name, &left_text, &right_text)
}

/// LCS diff over string slices (public for callers with pre-rendered
/// text).
pub fn diff_lines(name: &str, left: &[String], right: &[String]) -> PseudocodeDiff {
    // Classic LCS table.
    let mut table = vec![vec![0usize; right.len() + 1]; left.len() + 1];
    for (i, l) in left.iter().enumerate().rev() {
        for (j, r) in right.iter().enumerate().rev() {
            table[i][j] = if l == r {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }

    let mut lines = Vec::new();
    let mut added = 0;
    let mut removed = 0;
    let (mut i, mut j) = (0usize, 0usize);
    while i < left.len() && j < right.len() {
        if left[i] == right[j] {
            lines.push(PseudocodeDiffLine {
                kind: "same",
                text: left[i].clone(),
            });
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            lines.push(PseudocodeDiffLine {
                kind: "remove",
                text: left[i].clone(),
            });
            removed += 1;
            i += 1;
        } else {
            lines.push(PseudocodeDiffLine {
                kind: "add",
                text: right[j].clone(),
            });
            added += 1;
            j += 1;
        }
    }
    while i < left.len() {
        lines.push(PseudocodeDiffLine {
            kind: "remove",
            text: left[i].clone(),
        });
        removed += 1;
        i += 1;
    }
    while j < right.len() {
        lines.push(PseudocodeDiffLine {
            kind: "add",
            text: right[j].clone(),
        });
        added += 1;
        j += 1;
    }

    PseudocodeDiff {
        function: name.to_string(),
        left_lines: left.len(),
        right_lines: right.len(),
        added,
        removed,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn identical_lines_diff_clean() {
        let left = text(&["fn f() {", "  a = 1;", "}"]);
        let right = left.clone();
        let diff = diff_lines("f", &left, &right);
        assert_eq!(diff.added, 0);
        assert_eq!(diff.removed, 0);
        assert!(diff.lines.iter().all(|l| l.kind == "same"));
    }

    #[test]
    fn change_is_remove_plus_add() {
        let left = text(&["fn f() {", "  a = 1;", "}"]);
        let right = text(&["fn f() {", "  a = 2;", "}"]);
        let diff = diff_lines("f", &left, &right);
        assert_eq!(diff.removed, 1);
        assert_eq!(diff.added, 1);
        assert_eq!(diff.lines[0].kind, "same");
        assert_eq!(diff.lines[1].text, "  a = 1;");
        assert_eq!(diff.lines[1].kind, "remove");
        assert_eq!(diff.lines[2].text, "  a = 2;");
        assert_eq!(diff.lines[2].kind, "add");
    }

    #[test]
    fn pure_insertion_counts_adds_only() {
        let left = text(&["fn f() {", "}"]);
        let right = text(&["fn f() {", "  b = 9;", "}"]);
        let diff = diff_lines("f", &left, &right);
        assert_eq!(diff.added, 1);
        assert_eq!(diff.removed, 0);
    }
}
