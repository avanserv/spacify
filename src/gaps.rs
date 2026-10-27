//! The language-free half of formatting: where the gap between two sibling statements runs, which
//! comments above the next statement belong to it, and the edit that gives the gap the blank lines a
//! rule wants. Each language finds its statements and classifies them, then hands the pair here.

use std::fmt;

use tree_sitter::Node;

use crate::config::Rules;
use crate::edit::Edit;
use crate::rules::{self, Gap, Kind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// One-based line of the first syntax error.
    Syntax(usize),
    /// A line ending that is a `\r` alone, which Python accepts and the parser does not.
    LoneCarriageReturn,
    Parse,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(line) => write!(f, "syntax error on line {line}"),
            Self::LoneCarriageReturn => f.write_str(
                "line endings that are a bare carriage return are not supported: convert the file to \\n or \\r\\n",
            ),
            Self::Parse => f.write_str("the parser gave up on this file"),
        }
    }
}

impl std::error::Error for FormatError {}

/// A statement, as far as the gaps around it care.
#[derive(Debug, Clone, Copy)]
pub struct Statement {
    pub kind: Kind,
    pub start_row: usize,
    /// The row its code ends on, leaving out a trailing comment the parser filed inside it.
    pub end_row: usize,
    pub column: usize,
    /// Marked to be left as written, which leaves the gaps on both sides of it alone too.
    pub skip: bool,
}

/// How a language's comments switch spacify off and on for a region.
pub struct Directives {
    /// What starts a comment line: `#` or `//`.
    pub prefix: &'static str,
    /// The markers, with every space removed: `fmt:off`, `spacify:off`.
    pub off: &'static [&'static str],
    pub on: &'static [&'static str],
}

/// The source split into rows, with what the gap logic needs to know about each one.
pub struct Source<'a> {
    pub lines: Vec<&'a str>,
    /// `disabled[r]` counts the rows before `r` inside a region switched off by a directive.
    disabled: Vec<u32>,
    /// For each row, the row its comment starts on: the row itself unless a comment spanning
    /// several rows covers it, so that no blank line ever lands inside such a comment.
    comment_start: Vec<usize>,
}

impl<'a> Source<'a> {
    /// `multiline_comments` holds the first and last row of every comment that spans several.
    pub fn new(
        source: &'a str,
        directives: &Directives,
        multiline_comments: impl IntoIterator<Item = (usize, usize)>,
    ) -> Self {
        let lines: Vec<&str> = source.split_inclusive('\n').collect();
        let mut comment_start: Vec<usize> = (0..lines.len()).collect();
        for (first, last) in multiline_comments {
            let last = last.min(lines.len().saturating_sub(1));
            comment_start[first..=last].fill(first);
        }

        Self {
            disabled: disabled_prefix(&lines, directives),
            lines,
            comment_start,
        }
    }

    /// The edit that gives the gap between `prev` and `next` the blank lines the rules want, or
    /// `None` when it already has them or nothing may touch it.
    /// `max` is the formatter's ceiling for the block, and `block_len` its statement count.
    pub fn gap(&self, prev: Statement, next: Statement, block_len: usize, max: usize, rules: &Rules) -> Option<Edit> {
        if prev.kind == Kind::Definition || next.kind == Kind::Definition {
            return None;
        }

        let prev_end = prev.end_row;
        let next_start = next.start_row;
        if next_start <= prev_end
            || prev.skip
            || next.skip
            || self.comment_start[next_start] != next_start
            || self.disabled[next_start + 1] > self.disabled[prev_end]
        {
            return None;
        }

        // Comments directly above the next statement, at its indentation or less, belong to it:
        // the blank line goes above them, never between them and their statement.
        let mut row = next_start;
        while row > prev_end + 1 && !is_blank(self.lines[row - 1]) {
            let start = self.comment_start[row - 1];
            if start <= prev_end || indent(self.lines[start]) > next.column {
                break;
            }
            row = start;
        }

        let mut blank = 0;
        while row - blank > prev_end + 1 && is_blank(self.lines[row - blank - 1]) {
            blank += 1;
        }

        let gap = Gap {
            prev: prev.kind,
            next: next.kind,
            blank,
            // Only comments sit between two statements, so any line in the gap with text is one.
            has_comment: (prev_end + 1..next_start).any(|r| !is_blank(self.lines[r])),
            both_single_line: prev.start_row == prev_end && next_start == next.end_row,
            block_len,
            max,
        };

        rules::desired(&gap, rules)
            .filter(|&desired| desired != blank)
            .map(|desired| Edit {
                row,
                current: blank,
                desired,
            })
    }
}

/// The row a statement's code ends on. Tree-sitter can file a trailing comment inside the node it
/// follows, and such a comment belongs to the gap after the statement instead.
pub fn last_code_row(node: Node, is_comment: impl Fn(&str) -> bool) -> usize {
    let mut node = node;
    loop {
        let mut cursor = node.walk();
        let last = node
            .children(&mut cursor)
            .filter(|child| !is_comment(child.kind()))
            .last();
        match last {
            Some(child) => node = child,
            None => return node.end_position().row,
        }
    }
}

pub fn first_error_row(root: Node) -> usize {
    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        if node.is_error() || node.is_missing() {
            return node.start_position().row;
        }
        // Only a subtree that holds an error is worth entering.
        if node.has_error() && cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return root.start_position().row;
            }
        }
    }
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// Leading whitespace, in bytes.
fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn directive(line: &str, prefix: &str) -> Option<String> {
    let comment = line.trim().strip_prefix(prefix)?;
    Some(comment.chars().filter(|c| !c.is_whitespace()).collect())
}

/// Prefix counts of the rows inside switched-off regions, the markers included.
fn disabled_prefix(lines: &[&str], directives: &Directives) -> Vec<u32> {
    let mut prefix = Vec::with_capacity(lines.len() + 2);
    prefix.push(0);
    let mut off = false;
    let mut total = 0;
    for line in lines {
        if let Some(marker) = directive(line, directives.prefix) {
            if directives.off.contains(&marker.as_str()) {
                off = true;
            } else if directives.on.contains(&marker.as_str()) {
                total += 1;
                prefix.push(total);
                off = false;
                continue;
            }
        }
        total += u32::from(off);
        prefix.push(total);
    }
    // The row past the end, for a gap that ends on the last line.
    prefix.push(total);
    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: Directives = Directives {
        prefix: "#",
        off: &["fmt:off", "spacify:off"],
        on: &["fmt:on", "spacify:on"],
    };

    #[test]
    fn directives_ignore_whitespace() {
        assert_eq!(directive("  # fmt: off\n", "#").as_deref(), Some("fmt:off"));
        assert_eq!(directive("//spacify:on", "//").as_deref(), Some("spacify:on"));
        assert_eq!(directive("x = 1  # fmt: off\n", "#"), None);
    }

    #[test]
    fn disabled_regions_count_their_markers() {
        let lines = ["a\n", "# fmt: off\n", "b\n", "# fmt: on\n", "c\n"];
        assert_eq!(disabled_prefix(&lines, &HASH), [0, 0, 1, 2, 3, 3, 3]);
    }

    #[test]
    fn an_unterminated_region_runs_to_the_end() {
        let lines = ["a\n", "# spacify: off\n", "b\n"];
        assert_eq!(disabled_prefix(&lines, &HASH), [0, 0, 1, 2, 2]);
    }

    #[test]
    fn an_on_marker_alone_disables_only_itself() {
        let lines = ["a\n", "# fmt: on\n", "b\n"];
        assert_eq!(disabled_prefix(&lines, &HASH), [0, 0, 1, 1, 1]);
    }

    #[test]
    fn a_marker_for_another_language_is_a_plain_comment() {
        let lines = ["a\n", "// spacify: off\n", "b\n"];
        assert_eq!(disabled_prefix(&lines, &HASH), [0, 0, 0, 0, 0]);
    }

    #[test]
    fn rows_inside_a_multiline_comment_point_at_its_start() {
        let source = Source::new("a\n/* x\n\n*/\nb\n", &HASH, [(1, 3)]);
        assert_eq!(source.comment_start, [0, 1, 1, 1, 4]);
    }
}
