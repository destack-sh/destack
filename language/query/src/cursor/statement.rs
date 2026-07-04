use destack_dir as dir;
use destack_source::EnclosingSpan;

use crate::ModuleQueryContext;

/// The cursor's statement relationship to one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockStatementPosition {
    /// The cursor is on one statement head inside the block.
    StatementHead,
    /// The cursor is in a statement gap inside the block.
    StatementGap,
}

/// Return whether one expression is a statement head.
fn is_statement_head_expression(expr: &dir::Expression) -> bool {
    match expr {
        dir::Expression::Missing => false,
        _ => !expr.is_wide(),
    }
}

/// Return whether one expression still owns a trailing expression hole.
fn has_trailing_expression_hole(
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

impl ModuleQueryContext<'_> {
    /// Resolve the cursor's statement position inside one block.
    pub(crate) fn block_statement_position(
        &self,
        enclosing: &EnclosingSpan,
        offset: u32,
    ) -> Option<BlockStatementPosition> {
        // only block spans can expose statement positions
        if self.tree().get_node_type(enclosing.source_id) != dir::NodeType::Block {
            return None;
        }

        let parsed_tree = self.tree();
        let block_id = dir::LocalNodeId::<dir::Block>::new(enclosing.source_id);
        let block = parsed_tree.get(block_id);

        // empty blocks always expose one statement gap
        if block.is_empty() {
            return Some(BlockStatementPosition::StatementGap);
        }

        // track the last completed expression before the cursor
        let mut last_expression_before_cursor = None;

        for expr_id in block.iter_expressions() {
            let span = parsed_tree.source_index.get(expr_id.id);

            // statement heads only count when the cursor is on the owning statement span
            if span.owns_cursor(offset) {
                let main_span = parsed_tree
                    .source_index
                    .get_main(expr_id.id)
                    .unwrap_or_else(|| {
                        panic!(
                            "missing main source span for statement expression {}",
                            expr_id.id
                        )
                    });
                if !main_span.owns_cursor(offset) {
                    return None;
                }

                let expr = parsed_tree.get(expr_id);
                if is_statement_head_expression(expr) {
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
        if let Some(expr_id) = last_expression_before_cursor {
            if has_trailing_expression_hole(parsed_tree, expr_id) {
                return None;
            }
        }

        Some(BlockStatementPosition::StatementGap)
    }
}
