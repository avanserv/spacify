use crate::config::Rules;

/// What a statement is, as far as vertical rhythm cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `if`, `for`, `while`, `try`, `with`, `match`, in their `async` forms too.
    Compound,
    /// `def` and `class`, decorated or not. Ruff owns every gap around these.
    Definition,
    /// `return`, `raise`, `break`, `continue` and a bare `yield`.
    Exit,
    /// Never collapsed: ruff's isort rules separate import sections with blank lines.
    Import,
    Simple(Group),
    Other,
}

/// Simple statements that read as one paragraph when they follow each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Assign,
    AugAssign,
    Call,
    Del,
    Assert,
    Scope,
}

/// The space between two sibling statements of one block.
#[derive(Debug, Clone, Copy)]
pub struct Gap {
    pub prev: Kind,
    pub next: Kind,
    /// Blank lines directly above the next statement and the comments attached to it.
    pub blank: usize,
    pub has_comment: bool,
    pub both_single_line: bool,
    pub block_len: usize,
    /// Ruff's ceiling for this level: 2 at module level (1 in a stub), 1 in a nested block.
    pub max: usize,
}

/// The blank-line count `gap` should have, or `None` when no rule has an opinion on it. Rules that
/// add a line win over the one that removes lines, and nothing ever exceeds ruff's ceiling.
pub fn desired(gap: &Gap, rules: &Rules) -> Option<usize> {
    if blank_before_block(gap, rules) || blank_after_block(gap, rules) || blank_before_exit(gap, rules) {
        return Some(gap.blank.clamp(1, gap.max));
    }

    if collapse_groups(gap, rules) {
        return Some(0);
    }

    None
}

fn blank_before_block(gap: &Gap, rules: &Rules) -> bool {
    rules.blank_before_block && gap.next == Kind::Compound
}

fn blank_after_block(gap: &Gap, rules: &Rules) -> bool {
    rules.blank_after_block && gap.prev == Kind::Compound
}

fn blank_before_exit(gap: &Gap, rules: &Rules) -> bool {
    rules.blank_before_exit && gap.next == Kind::Exit && gap.block_len >= rules.exit_min_block_statements
}

/// A comment in the gap marks a paragraph the author meant, and a multi-line statement reads
/// better with air around it, so both keep their blank line.
fn collapse_groups(gap: &Gap, rules: &Rules) -> bool {
    rules.collapse_groups
        && gap.blank > 0
        && !gap.has_comment
        && gap.both_single_line
        && matches!((gap.prev, gap.next), (Kind::Simple(a), Kind::Simple(b)) if a == b)
}
