use std::cell::RefCell;

use tree_sitter::{Node, Parser};

use crate::config::Rules;
use crate::edit::{self, Edit};
use crate::gaps::{Directives, FormatError, Source, Statement, first_error_row, last_code_row};
use crate::rules::{Group, Kind};

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new({
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("the bundled Python grammar matches the tree-sitter ABI");
        parser
    });
}

const DIRECTIVES: Directives = Directives {
    prefix: "#",
    off: &["fmt:off", "spacify:off"],
    on: &["fmt:on", "spacify:on"],
};

/// Formats Python `source`, returning `None` when it is already formatted. A `stub` (`.pyi`) gets
/// ruff's stub ceiling of one blank line at module level.
pub(crate) fn format(source: &str, rules: &Rules, stub: bool) -> Result<Option<String>, FormatError> {
    let tree = PARSER
        .with_borrow_mut(|parser| parser.parse(source, None))
        .ok_or(FormatError::Parse)?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(FormatError::Syntax(first_error_row(root) + 1));
    }

    let context = Context {
        source,
        // Python comments never span several rows.
        rows: Source::new(source, &DIRECTIVES, []),
        rules,
        module_max: if stub { 1 } else { 2 },
    };
    let mut edits = Vec::new();
    context.container(root, true, &mut edits);

    if edits.is_empty() {
        return Ok(None);
    }
    Ok(Some(edit::apply(source, &mut edits)))
}

struct Context<'a> {
    source: &'a str,
    rows: Source<'a>,
    rules: &'a Rules,
    module_max: usize,
}

impl Context<'_> {
    /// Visits a `module` or a `block`: the gaps between its statements, then its nested blocks.
    /// `module_scope` holds outside every function and class, even inside a top-level `if`.
    fn container(&self, node: Node, module_scope: bool, edits: &mut Vec<Edit>) {
        let mut cursor = node.walk();
        let statements: Vec<Node> = node
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment")
            .collect();

        // A `match` body holds `case` clauses, which are not statements and own no gaps.
        if !statements.iter().any(|child| child.kind() == "case_clause") {
            let module = node.kind() == "module";
            let has_docstring = module
                || node
                    .parent()
                    .is_some_and(|parent| matches!(parent.kind(), "function_definition" | "class_definition"));

            for (index, pair) in statements.windows(2).enumerate() {
                let docstring = index == 0 && has_docstring && is_docstring(pair[0]);
                if !docstring && let Some(edit) = self.gap(pair[0], pair[1], statements.len(), module, module_scope) {
                    edits.push(edit);
                }
            }
        }

        for statement in statements {
            self.descend(statement, module_scope, edits);
        }
    }

    /// Finds the blocks inside a compound statement, through its clauses and decorators.
    fn descend(&self, node: Node, module_scope: bool, edits: &mut Vec<Edit>) {
        let module_scope = module_scope && !node.kind().ends_with("_definition");
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            let kind = child.kind();
            if kind == "block" {
                self.container(child, module_scope, edits);
            } else if kind.ends_with("_clause") || kind.ends_with("_definition") {
                self.descend(child, module_scope, edits);
            }
        }
    }

    fn gap(&self, prev: Node, next: Node, block_len: usize, module: bool, module_scope: bool) -> Option<Edit> {
        let max = if module { self.module_max } else { 1 };
        self.rows.gap(
            self.statement(prev, module_scope),
            self.statement(next, module_scope),
            block_len,
            max,
            self.rules,
        )
    }

    fn statement(&self, node: Node, module_scope: bool) -> Statement {
        let start_row = node.start_position().row;
        let end_row = last_code_row(node, |kind| kind == "comment");
        Statement {
            kind: self.classify(node, module_scope),
            start_row,
            end_row,
            column: node.start_position().column,
            skip: has_fmt_skip(self.rows.lines[start_row]) || has_fmt_skip(self.rows.lines[end_row]),
        }
    }

    fn classify(&self, node: Node, module_scope: bool) -> Kind {
        match node.kind() {
            "if_statement" | "for_statement" | "while_statement" | "try_statement" | "with_statement"
            | "match_statement" => Kind::Compound,
            "function_definition" | "class_definition" | "decorated_definition" => Kind::Definition,
            "return_statement" | "raise_statement" | "break_statement" | "continue_statement" => Kind::Exit,
            "import_statement" | "import_from_statement" | "future_import_statement" => Kind::Import,
            "delete_statement" => Kind::Simple(Group::Del),
            "assert_statement" => Kind::Simple(Group::Assert),
            "global_statement" | "nonlocal_statement" => Kind::Simple(Group::Scope),
            "expression_statement" if node.named_child_count() == 1 => {
                let Some(child) = node.named_child(0) else {
                    return Kind::Other;
                };
                match child.kind() {
                    "assignment" if module_scope && self.assigns_constants(child) => Kind::Simple(Group::Constant),
                    "assignment" => Kind::Simple(Group::Assign),
                    "augmented_assignment" => Kind::Simple(Group::AugAssign),
                    "call" | "await" => Kind::Simple(Group::Call),
                    "yield" => Kind::Exit,
                    _ => Kind::Other,
                }
            }
            _ => Kind::Other,
        }
    }

    /// Whether every name `assignment` binds, through a chain such as `A = B = 1`, is a constant
    /// name: `A = 1`, `A: int = 1`, `A, *B = items`, `[A, (B, C)] = rows`.
    fn assigns_constants(&self, assignment: Node) -> bool {
        assignment
            .child_by_field_name("left")
            .is_some_and(|left| self.binds_constants(left))
            && assignment
                .child_by_field_name("right")
                .is_none_or(|right| right.kind() != "assignment" || self.assigns_constants(right))
    }

    fn binds_constants(&self, target: Node) -> bool {
        match target.kind() {
            "identifier" => is_constant_name(&self.source[target.byte_range()]),
            "pattern_list" | "tuple_pattern" | "list_pattern" => {
                let mut cursor = target.walk();
                target.named_child_count() > 0
                    && target
                        .named_children(&mut cursor)
                        .all(|child| self.binds_constants(child))
            }
            "list_splat_pattern" => target.named_child(0).is_some_and(|name| self.binds_constants(name)),
            _ => false,
        }
    }
}

/// An uppercase letter and no lowercase one, in any script: `MAX`, `T`, `_A1`, `ÉTÉ`.
fn is_constant_name(name: &str) -> bool {
    name.chars().any(char::is_uppercase) && !name.chars().any(char::is_lowercase)
}

fn is_docstring(node: Node) -> bool {
    node.kind() == "expression_statement"
        && node.named_child_count() == 1
        && node
            .named_child(0)
            .is_some_and(|child| matches!(child.kind(), "string" | "concatenated_string"))
}

fn has_fmt_skip(line: &str) -> bool {
    line.find('#').is_some_and(|hash| {
        let comment: String = line[hash..].chars().filter(|c| !c.is_whitespace()).collect();
        comment.contains("fmt:skip")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_names_have_an_uppercase_letter_and_no_lowercase_one() {
        for name in ["MAX", "_A1", "ÉTÉ", "T", "__ALL__"] {
            assert!(is_constant_name(name), "{name}");
        }
        for name in ["_", "_1", "Max", "__all__", "maxValue"] {
            assert!(!is_constant_name(name), "{name}");
        }
    }

    #[test]
    fn fmt_skip_is_found_in_any_spacing_and_after_other_comments() {
        assert!(has_fmt_skip("x = 1  # fmt: skip\n"));
        assert!(has_fmt_skip("x = 1  #fmt:skip\n"));
        assert!(has_fmt_skip("x = 1  # noqa: E501 # fmt: skip\n"));
        assert!(!has_fmt_skip("x = 1  # fmt: off\n"));
        assert!(!has_fmt_skip("x = 1\n"));
    }
}
