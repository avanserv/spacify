mod rules;

use std::cell::RefCell;
use std::fmt;

use tree_sitter::{Node, Parser};

use crate::config::Rules;
use crate::edit::{self, Edit};
use rules::{Gap, Group, Kind};

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new({
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("the bundled Python grammar matches the tree-sitter ABI");
        parser
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// One-based line of the first syntax error.
    Syntax(usize),
    Parse,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(line) => write!(f, "syntax error on line {line}"),
            Self::Parse => f.write_str("the parser gave up on this file"),
        }
    }
}

impl std::error::Error for FormatError {}

/// Formats Python `source`, returning `None` when it is already formatted. A `stub` (`.pyi`) gets
/// ruff's stub ceiling of one blank line at module level.
pub fn format_source(source: &str, rules: &Rules, stub: bool) -> Result<Option<String>, FormatError> {
    let tree = PARSER
        .with_borrow_mut(|parser| parser.parse(source, None))
        .ok_or(FormatError::Parse)?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(FormatError::Syntax(first_error_row(root) + 1));
    }

    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let context = Context {
        source,
        disabled: disabled_prefix(&lines),
        lines,
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
    lines: Vec<&'a str>,
    /// `disabled[r]` counts the rows before `r` inside a `fmt: off` or `spacify: off` region.
    disabled: Vec<u32>,
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
        let prev_kind = self.classify(prev, module_scope);
        let next_kind = self.classify(next, module_scope);
        if prev_kind == Kind::Definition || next_kind == Kind::Definition {
            return None;
        }

        let prev_start = prev.start_position().row;
        let prev_end = last_code_row(prev);
        let next_start = next.start_position().row;
        if next_start <= prev_end || self.is_disabled(prev_start, prev_end, next_start) {
            return None;
        }

        // Comments directly above the next statement, at its indentation or less, belong to it:
        // the blank line goes above them, never between them and their statement.
        let indent = next.start_position().column;
        let mut row = next_start;
        while row > prev_end + 1 && comment_indent(self.lines[row - 1]).is_some_and(|width| width <= indent) {
            row -= 1;
        }

        let mut blank = 0;
        while row - blank > prev_end + 1 && is_blank(self.lines[row - blank - 1]) {
            blank += 1;
        }

        let gap = Gap {
            prev: prev_kind,
            next: next_kind,
            blank,
            has_comment: (prev_end + 1..next_start).any(|r| comment_indent(self.lines[r]).is_some()),
            both_single_line: prev_start == prev_end && next_start == last_code_row(next),
            block_len,
            max: if module { self.module_max } else { 1 },
        };

        rules::desired(&gap, self.rules)
            .filter(|&desired| desired != blank)
            .map(|desired| Edit {
                row,
                current: blank,
                desired,
            })
    }

    fn is_disabled(&self, prev_start: usize, prev_end: usize, next_start: usize) -> bool {
        self.disabled[next_start + 1] > self.disabled[prev_end]
            || [prev_start, prev_end, next_start]
                .iter()
                .any(|&row| has_fmt_skip(self.lines[row]))
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

/// The row a statement's code ends on. Tree-sitter can file a trailing comment inside the block it
/// follows, and such a comment belongs to the gap after the statement instead.
fn last_code_row(node: Node) -> usize {
    let mut node = node;
    loop {
        let mut cursor = node.walk();
        let last = node
            .children(&mut cursor)
            .filter(|child| child.kind() != "comment")
            .last();
        match last {
            Some(child) => node = child,
            None => return node.end_position().row,
        }
    }
}

fn first_error_row(root: Node) -> usize {
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

/// The indentation of a comment-only line, in bytes, or `None` for any other line.
fn comment_indent(line: &str) -> Option<usize> {
    let code = line.trim_start();
    code.starts_with('#').then(|| line.len() - code.len())
}

fn directive(line: &str) -> Option<String> {
    let comment = line.trim().strip_prefix('#')?;
    Some(comment.chars().filter(|c| !c.is_whitespace()).collect())
}

fn has_fmt_skip(line: &str) -> bool {
    line.find('#').is_some_and(|hash| {
        let comment: String = line[hash..].chars().filter(|c| !c.is_whitespace()).collect();
        comment.contains("fmt:skip")
    })
}

/// Prefix counts of the rows inside `fmt: off` / `spacify: off` regions, the markers included.
fn disabled_prefix(lines: &[&str]) -> Vec<u32> {
    let mut prefix = Vec::with_capacity(lines.len() + 2);
    prefix.push(0);
    let mut off = false;
    let mut total = 0;
    for line in lines {
        match directive(line).as_deref() {
            Some("fmt:off" | "spacify:off") => off = true,
            Some("fmt:on" | "spacify:on") => {
                total += 1;
                prefix.push(total);
                off = false;
                continue;
            }
            _ => {}
        }
        total += u32::from(off);
        prefix.push(total);
    }
    // The row past the end, for a gap that ends on the last line.
    prefix.push(total);
    prefix
}
