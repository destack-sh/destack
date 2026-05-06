use destack_ast as ast;
use destack_source::{EnclosingSpan, Span};

use super::{AstQueryContext, span_owns_cursor};

/// The cursor's statement relationship to one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockStatementPosition {
    /// The cursor is on one statement head inside the block.
    StatementHead,
    /// The cursor is in a statement gap inside the block.
    StatementGap,
}

/// Check whether an expression can act as one statement head.
fn expression_is_statement_head_candidate(_ast_tree: &ast::Tree, expr: &ast::Expression) -> bool {
    match expr {
        ast::Expression::Missing => false,
        _ => !expr.is_wide(),
    }
}

/// Check whether an expression still owns one trailing missing slot.
fn expression_has_trailing_missing_slot(
    ast_tree: &ast::Tree,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ast_tree.get(expr_id);

    match expr {
        ast::Expression::Let { declarators, .. } | ast::Expression::Using { declarators, .. } => {
            let Some(last_declarator) = declarators.last() else {
                return false;
            };
            let declarator = ast_tree.get(*last_declarator);
            let Some(value) = declarator.value else {
                return false;
            };

            matches!(ast_tree.get(value), ast::Expression::Missing)
        }
        ast::Expression::Assign { right, .. } => {
            matches!(ast_tree.get(*right), ast::Expression::Missing)
        }
        ast::Expression::Return { value } | ast::Expression::Yield { value, .. } => {
            value.is_some_and(|value| matches!(ast_tree.get(value), ast::Expression::Missing))
        }
        ast::Expression::Throw { value } => {
            matches!(ast_tree.get(*value), ast::Expression::Missing)
        }
        _ => false,
    }
}

/// Return true when the cursor is on the main span of one statement head.
fn cursor_is_on_statement_main_span(
    ast_tree: &ast::Tree,
    expr_id: ast::LocalNodeId<ast::Expression>,
    span: Span,
    offset: u32,
) -> bool {
    let main_span = ast_tree.source_map.get_main(expr_id.id).unwrap_or(span);
    span_owns_cursor(main_span, offset)
}

/// Resolve the cursor's statement position inside one block.
pub(crate) fn block_statement_position(
    ast: AstQueryContext<'_>,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<BlockStatementPosition> {
    // only block spans can expose statement positions
    if ast.tree().get_node_type(enc.idx) != ast::NodeType::Block {
        return None;
    }

    let ast_tree = ast.tree();
    let block_id = ast::LocalNodeId::<ast::Block>::new(enc.idx);
    let block = ast_tree.get(block_id);

    // empty blocks always expose one statement gap
    if block.is_empty() {
        return Some(BlockStatementPosition::StatementGap);
    }

    // track the last completed expression before the cursor
    let mut last_expression_before_cursor = None;

    for expr_id in block.iter_expressions() {
        let span = ast_tree.source_map.get(expr_id.id);

        // statement heads only count when the cursor is on the owning statement span
        if span_owns_cursor(span, offset) {
            if !cursor_is_on_statement_main_span(ast_tree, expr_id, span, offset) {
                return None;
            }

            let expr = ast_tree.get(expr_id);
            if expression_is_statement_head_candidate(ast_tree, expr) {
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
        && expression_has_trailing_missing_slot(ast_tree, expr_id)
    {
        return None;
    }

    Some(BlockStatementPosition::StatementGap)
}
