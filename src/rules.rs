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
    /// An assignment to `UPPER_CASE` names outside every function and class: configuration, which
    /// reads apart from the statements setting the module up, such as `logger = getLogger(__name__)`.
    Constant,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Two single-line assignments in a nested block, two blank lines apart: a gap each rule can
    /// be pointed at by changing a field or two.
    fn gap() -> Gap {
        Gap {
            prev: Kind::Simple(Group::Assign),
            next: Kind::Simple(Group::Assign),
            blank: 2,
            has_comment: false,
            both_single_line: true,
            block_len: 5,
            max: 1,
        }
    }

    fn desired_default(gap: &Gap) -> Option<usize> {
        desired(gap, &Rules::default())
    }

    #[test]
    fn added_lines_stay_between_one_and_the_ceiling() {
        let before_block = |blank, max| {
            desired_default(&Gap {
                next: Kind::Compound,
                blank,
                max,
                ..gap()
            })
        };
        assert_eq!(before_block(0, 1), Some(1));
        assert_eq!(before_block(3, 1), Some(1));
        assert_eq!(before_block(0, 2), Some(1));
        assert_eq!(before_block(2, 2), Some(2));
        assert_eq!(before_block(5, 2), Some(2));
    }

    #[test]
    fn blank_after_block_follows_any_compound() {
        let after = Gap {
            prev: Kind::Compound,
            next: Kind::Other,
            blank: 0,
            ..gap()
        };
        assert_eq!(desired_default(&after), Some(1));
    }

    #[test]
    fn blank_before_exit_starts_at_the_threshold() {
        let exit = |block_len| {
            desired_default(&Gap {
                next: Kind::Exit,
                blank: 0,
                block_len,
                ..gap()
            })
        };
        assert_eq!(exit(2), None);
        assert_eq!(exit(3), Some(1));
    }

    #[test]
    fn collapse_needs_every_condition() {
        assert_eq!(desired_default(&gap()), Some(0));
        assert_eq!(desired_default(&Gap { blank: 0, ..gap() }), None);
        assert_eq!(
            desired_default(&Gap {
                has_comment: true,
                ..gap()
            }),
            None
        );
        assert_eq!(
            desired_default(&Gap {
                both_single_line: false,
                ..gap()
            }),
            None
        );
        assert_eq!(
            desired_default(&Gap {
                next: Kind::Simple(Group::Call),
                ..gap()
            }),
            None
        );
    }

    #[test]
    fn constants_do_not_collapse_into_assignments() {
        let constant_then_assign = Gap {
            prev: Kind::Simple(Group::Constant),
            ..gap()
        };
        assert_eq!(desired_default(&constant_then_assign), None);
        let constants = Gap {
            prev: Kind::Simple(Group::Constant),
            next: Kind::Simple(Group::Constant),
            ..gap()
        };
        assert_eq!(desired_default(&constants), Some(0));
    }

    #[test]
    fn definitions_imports_and_others_get_no_opinion() {
        for kind in [Kind::Definition, Kind::Import, Kind::Other] {
            assert_eq!(
                desired_default(&Gap {
                    prev: kind,
                    next: kind,
                    ..gap()
                }),
                None,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn each_toggle_disables_only_its_rule() {
        let before_block = Gap {
            next: Kind::Compound,
            blank: 0,
            ..gap()
        };
        let after_block = Gap {
            prev: Kind::Compound,
            blank: 0,
            ..gap()
        };
        let before_exit = Gap {
            next: Kind::Exit,
            blank: 0,
            ..gap()
        };
        let collapse = gap();
        let all = [before_block, after_block, before_exit, collapse];
        let toggles: [fn(&mut Rules); 4] = [
            |rules| rules.blank_before_block = false,
            |rules| rules.blank_after_block = false,
            |rules| rules.blank_before_exit = false,
            |rules| rules.collapse_groups = false,
        ];

        for (off, toggle) in toggles.iter().enumerate() {
            let mut rules = Rules::default();
            toggle(&mut rules);
            for (index, gap) in all.iter().enumerate() {
                assert_eq!(
                    desired(gap, &rules).is_none(),
                    index == off,
                    "toggle {off} on gap {index}"
                );
            }
        }
    }
}
