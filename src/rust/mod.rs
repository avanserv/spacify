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
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("the bundled Rust grammar matches the tree-sitter ABI");
        parser
    });
}

const DIRECTIVES: Directives = Directives {
    prefix: "//",
    off: &["spacify:off"],
    on: &["spacify:on"],
};

/// Rustfmt keeps at most one blank line between statements, in every block.
const MAX: usize = 1;

/// Formats Rust `source`, returning `None` when it is already formatted.
pub(crate) fn format(source: &str, rules: &Rules) -> Result<Option<String>, FormatError> {
    let tree = PARSER
        .with_borrow_mut(|parser| parser.parse(source, None))
        .ok_or(FormatError::Parse)?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(FormatError::Syntax(first_error_row(root) + 1));
    }

    let (blocks, comments) = scan(root);
    let context = Context {
        source,
        rows: Source::new(source, &DIRECTIVES, comments),
        rules,
    };
    let mut edits = Vec::new();
    for block in blocks {
        context.block(block, &mut edits);
    }

    if edits.is_empty() {
        return Ok(None);
    }
    Ok(Some(edit::apply(source, &mut edits)))
}

/// Every `block` in the tree, and the first and last row of every comment spanning several rows.
/// Only blocks hold statements: a file, an `impl` or a `mod` holds items, whose gaps are rustfmt's.
fn scan(root: Node) -> (Vec<Node>, Vec<(usize, usize)>) {
    let mut blocks = Vec::new();
    let mut comments = Vec::new();
    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        match node.kind() {
            "block" => blocks.push(node),
            "block_comment" if node.end_position().row > node.start_position().row => {
                comments.push((node.start_position().row, node.end_position().row));
            }
            _ => {}
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return (blocks, comments);
            }
        }
    }
}

struct Context<'a> {
    source: &'a str,
    rows: Source<'a>,
    rules: &'a Rules,
}

impl Context<'_> {
    fn block(&self, block: Node, edits: &mut Vec<Edit>) {
        let mut cursor = block.walk();
        let children: Vec<Node> = block
            .named_children(&mut cursor)
            .filter(|child| !is_comment(child.kind()))
            .collect();
        // A function body's tail expression is what it returns. Elsewhere it is a value.
        let returns = block.parent().is_some_and(|parent| parent.kind() == "function_item");

        let mut statements = Vec::with_capacity(children.len());
        // An outer attribute belongs to the statement after it: the gap between them is not one.
        let mut attributes: Option<(usize, usize, bool)> = None;
        for (index, &child) in children.iter().enumerate() {
            if child.kind() == "attribute_item" {
                let skip = self.is_rustfmt_skip(child);
                let start = attributes.map_or((child.start_position().row, child.start_position().column), |a| {
                    (a.0, a.1)
                });
                attributes = Some((start.0, start.1, skip || attributes.is_some_and(|a| a.2)));
                continue;
            }

            let tail = index == children.len() - 1 && !is_statement(child.kind());
            let (start_row, column, skip) =
                attributes
                    .take()
                    .unwrap_or((child.start_position().row, child.start_position().column, false));
            statements.push(Statement {
                kind: classify(child, tail && returns),
                start_row,
                end_row: last_code_row(child, is_comment),
                column,
                skip,
            });
        }

        for pair in statements.windows(2) {
            if let Some(edit) = self.rows.gap(pair[0], pair[1], statements.len(), MAX, self.rules) {
                edits.push(edit);
            }
        }
    }

    fn is_rustfmt_skip(&self, attribute: Node) -> bool {
        let text: String = self.source[attribute.byte_range()]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        text.contains("rustfmt::skip")
    }
}

fn is_comment(kind: &str) -> bool {
    matches!(kind, "line_comment" | "block_comment")
}

/// The kinds a block holds as statements. Anything else in last place is the tail expression.
fn is_statement(kind: &str) -> bool {
    is_item(kind) || matches!(kind, "let_declaration" | "expression_statement" | "empty_statement")
}

/// Items, `use` and attributes on the block itself, whose gaps are rustfmt's alone.
fn is_item(kind: &str) -> bool {
    kind.ends_with("_item")
        || matches!(
            kind,
            "use_declaration" | "macro_definition" | "extern_crate_declaration" | "inner_attribute_item"
        )
}

/// What `node`, a child of a block, is. `returns` marks a function body's tail expression.
fn classify(node: Node, returns: bool) -> Kind {
    match node.kind() {
        kind if is_item(kind) => Kind::Definition,
        "let_declaration" => Kind::Simple(Group::Assign),
        "expression_statement" => node
            .named_child(0)
            .map_or(Kind::Other, |expression| classify_expression(expression, false)),
        "empty_statement" => Kind::Other,
        _ => classify_expression(node, returns),
    }
}

fn classify_expression(expression: Node, returns: bool) -> Kind {
    match expression.kind() {
        "if_expression" | "match_expression" | "for_expression" | "while_expression" | "loop_expression" | "block"
        | "unsafe_block" | "async_block" | "const_block" | "try_block" | "gen_block" => Kind::Compound,
        _ if returns => Kind::Exit,
        "return_expression" | "break_expression" | "continue_expression" => Kind::Exit,
        "assignment_expression" => Kind::Simple(Group::Assign),
        "compound_assignment_expr" => Kind::Simple(Group::AugAssign),
        _ if is_call(expression) => Kind::Simple(Group::Call),
        _ => Kind::Other,
    }
}

/// A call, a macro call or an `.await`, through any `?` on it.
fn is_call(expression: Node) -> bool {
    match expression.kind() {
        "call_expression" | "macro_invocation" | "await_expression" => true,
        "try_expression" => expression.named_child(0).is_some_and(is_call),
        _ => false,
    }
}
