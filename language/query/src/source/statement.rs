use destack_dir as dir;
use destack_source::{EnclosingSpan, Span};

use super::{SourceQueryContext, span_owns_cursor};

/// The cursor's statement relationship to one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockStatementPosition {
    /// The cursor is on one statement head inside the block.
    StatementHead,
    /// The cursor is in a statement gap inside the block.
    StatementGap,
}

/// Check whether an expression can act as one statement head.
fn expression_is_statement_head_candidate(
    _parsed_tree: &dir::Tree,
    expr: &dir::Expression,
) -> bool {
    match expr {
        dir::Expression::Missing => false,
        _ => !expr.is_wide(),
    }
}

/// Check whether an expression still owns one trailing missing slot.
fn expression_has_trailing_missing_slot(
    parsed_tree: &dir::Tree,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expr = parsed_tree.get(expr_id);

    match expr {
        dir::Expression::Let { declarators, .. } | dir::Expression::Using { declarators, .. } => {
            let Some(last_declarator) = declarators.last() else {
                return false;
            };
            let declarator = parsed_tree.get(*last_declarator);
            let Some(value) = declarator.value else {
                return false;
            };

            matches!(parsed_tree.get(value), dir::Expression::Missing)
        }
        dir::Expression::Assign { right, .. } => {
            matches!(parsed_tree.get(*right), dir::Expression::Missing)
        }
        dir::Expression::Return { value } | dir::Expression::Yield { value, .. } => {
            value.is_some_and(|value| matches!(parsed_tree.get(value), dir::Expression::Missing))
        }
        dir::Expression::Throw { value } => {
            matches!(parsed_tree.get(*value), dir::Expression::Missing)
        }
        _ => false,
    }
}

/// Return true when the cursor is on the main span of one statement head.
fn cursor_is_on_statement_main_span(
    parsed_tree: &dir::Tree,
    expr_id: dir::LocalNodeId<dir::Expression>,
    span: Span,
    offset: u32,
) -> bool {
    let main_span = parsed_tree.source_map.get_main(expr_id.id).unwrap_or(span);
    span_owns_cursor(main_span, offset)
}

/// Resolve the cursor's statement position inside one block.
pub(crate) fn block_statement_position(
    parsed: SourceQueryContext<'_>,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<BlockStatementPosition> {
    // only block spans can expose statement positions
    if parsed.tree().get_node_type(enc.idx) != dir::NodeType::Block {
        return None;
    }

    let parsed_tree = parsed.tree();
    let block_id = dir::LocalNodeId::<dir::Block>::new(enc.idx);
    let block = parsed_tree.get(block_id);

    // empty blocks always expose one statement gap
    if block.is_empty() {
        return Some(BlockStatementPosition::StatementGap);
    }

    // track the last completed expression before the cursor
    let mut last_expression_before_cursor = None;

    for expr_id in block.iter_expressions() {
        let span = parsed_tree.source_map.get(expr_id.id);

        // statement heads only count when the cursor is on the owning statement span
        if span_owns_cursor(span, offset) {
            if !cursor_is_on_statement_main_span(parsed_tree, expr_id, span, offset) {
                return None;
            }

            let expr = parsed_tree.get(expr_id);
            if expression_is_statement_head_candidate(parsed_tree, expr) {
                return Some(BlockStatementPosition::StatementHead);
            }

            return None;
        }

        // remember the last expression before the cursor
        if span.end <= offset {
            last_expression_before_cursor = Some(expr_id);
        }
    }

    // trailing missing slots still belong to the current statement
    if let Some(expr_id) = last_expression_before_cursor
        && expression_has_trailing_missing_slot(parsed_tree, expr_id)
    {
        return None;
    }

    Some(BlockStatementPosition::StatementGap)
}
