use destack_ast as ast;
use destack_source::{EnclosingSpan, NodeSpanType, Span};

use super::{QueryContext, enclosing_spans_at_offsets, enclosing_spans_with_previous};

/// The cursor's statement relationship to one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockStatementPosition {
    /// The cursor is on one statement head inside the block.
    StatementHead,
    /// The cursor is in a statement gap inside the block.
    StatementGap,
}

/// Check whether a token type is trivia.
pub(crate) fn is_trivia_token(token: ast::TokenType) -> bool {
    matches!(
        token,
        ast::TokenType::Whitespace
            | ast::TokenType::Newline
            | ast::TokenType::LineComment
            | ast::TokenType::BlockComment
            | ast::TokenType::DocLineComment
            | ast::TokenType::DocBlockComment
            | ast::TokenType::End
    )
}

/// Find the previous significant token before or at the cursor.
pub(crate) fn previous_significant_token(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<ast::TokenSpan> {
    let mut candidate = None;

    // scan tokens in order for the latest significant token before the offset
    for token in ctx.source_context().tokens() {
        if token.span.file != ctx.file_id {
            continue;
        }

        if is_trivia_token(token.token.ty) {
            continue;
        }

        if token.span.end <= offset {
            candidate = Some(*token);
            continue;
        }

        if token.span.start > offset {
            break;
        }
    }

    candidate
}

/// Find the next significant token after or at the cursor.
pub(crate) fn next_significant_token(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<ast::TokenSpan> {
    for token in ctx.source_context().tokens() {
        if token.span.file != ctx.file_id {
            continue;
        }

        if is_trivia_token(token.token.ty) {
            continue;
        }

        if token.span.start >= offset {
            return Some(*token);
        }
    }

    None
}

/// Check whether a span owns the cursor boundary.
pub(crate) fn span_contains_cursor_boundary(span: Span, offset: u32) -> bool {
    if span.contains(offset) {
        return true;
    }

    let previous_offset = offset.saturating_sub(1);
    if span.contains(previous_offset) {
        return true;
    }

    span.start == offset && span.end == offset
}

/// Find the significant token span that owns one cursor offset in a query context.
pub(crate) fn token_span_at_cursor_offset(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<ast::TokenSpan> {
    let mut candidate = None;

    // scan tokens until the cursor falls inside one token
    for token in ctx.source_context().tokens() {
        if token.span.file != ctx.file_id {
            continue;
        }

        if is_trivia_token(token.token.ty) {
            continue;
        }

        if token.span.contains(offset) {
            return Some(*token);
        }

        if token.span.start > offset {
            break;
        }

        candidate = Some(*token);
    }

    // allow the cursor at the end of one token span
    let candidate = candidate?;
    if candidate.span.end == offset {
        return Some(candidate);
    }

    None
}

/// Read one token slice from the source text.
pub(crate) fn token_text(source: &str, span: Span) -> Option<&str> {
    source.get(span.start as usize..span.end as usize)
}

/// Check whether an expression can act as one statement head.
fn expression_is_statement_head_candidate(
    ast_tree: &ast::NodeTree,
    expr: &ast::Expression,
) -> bool {
    match expr {
        ast::Expression::Statement(inner) => {
            let inner = ast_tree.get(*inner);
            !inner.is_wide()
        }
        ast::Expression::Missing => false,
        _ => !expr.is_wide(),
    }
}

/// Check whether an expression still owns one trailing missing slot.
fn expression_has_trailing_missing_slot(
    ast_tree: &ast::NodeTree,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ast_tree.get(expr_id);

    match expr {
        ast::Expression::Statement(inner) => expression_has_trailing_missing_slot(ast_tree, *inner),
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

/// Resolve the cursor's statement position inside one block.
pub(crate) fn block_statement_position(
    ctx: &QueryContext<'_>,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<BlockStatementPosition> {
    // only block spans can expose statement positions
    if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Block {
        return None;
    }

    let ast_tree = ctx.ast_context().tree();
    let block_id = ast::LocalNodeId::<ast::Block>::new(enc.idx);
    let block = ast_tree.get(block_id);

    // empty blocks always expose one statement gap
    if block.expressions.is_empty() {
        return Some(BlockStatementPosition::StatementGap);
    }

    // track the last completed expression before the cursor
    let mut last_expression_before_cursor = None;

    for expr_id in &block.expressions {
        let span = ast_tree.source_map.get(expr_id.id);

        // statement heads only count when the cursor is on the owning main span
        if span_contains_cursor_boundary(span, offset) {
            let main_span = ast_tree.source_map.get_main(expr_id.id).unwrap_or(span);
            if !span_contains_cursor_boundary(main_span, offset) {
                return None;
            }

            let expr = ast_tree.get(*expr_id);
            if expression_is_statement_head_candidate(ast_tree, expr) {
                return Some(BlockStatementPosition::StatementHead);
            }

            return None;
        }

        // remember the last expression before the cursor
        if span.end <= offset {
            last_expression_before_cursor = Some(*expr_id);
        }
    }

    // trailing missing slots still belong to the current statement
    if let Some(expr_id) = last_expression_before_cursor {
        if expression_has_trailing_missing_slot(ast_tree, expr_id) {
            return None;
        }
    }

    Some(BlockStatementPosition::StatementGap)
}

/// Collect enclosing spans around a cursor boundary.
///
/// This supplements the direct cursor probes with adjacent significant token probes
/// so recovered syntax ownership survives when the cursor sits in whitespace gaps.
pub(crate) fn enclosing_spans_at_cursor_boundary(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Vec<EnclosingSpan> {
    let mut offsets = vec![offset];

    // include the previous byte for ordinary boundary cases
    if offset > 0 {
        offsets.push(offset - 1);
    }

    // include the previous significant token interior when the cursor is after it
    if let Some(token) = previous_significant_token(ctx, offset)
        && token.span.end <= offset
        && token.span.start < token.span.end
    {
        offsets.push(token.span.end - 1);
    }

    // include the next significant token start when the cursor precedes it
    if let Some(token) = next_significant_token(ctx, offset)
        && token.span.start >= offset
    {
        offsets.push(token.span.start);
    }

    enclosing_spans_at_offsets(ctx.ast_context(), offsets)
}

/// Check whether an offset falls inside one AST type side span.
pub(crate) fn offset_is_in_ast_type_side_span(ctx: &QueryContext<'_>, offset: u32) -> bool {
    let previous_offset = offset.saturating_sub(1);

    for enclosing in enclosing_spans_with_previous(ctx, offset) {
        if let Some(span) = ctx
            .ast_context()
            .source_map()
            .get_side(enclosing.idx, NodeSpanType::Type)
            && (span.contains(offset) || span.contains(previous_offset))
        {
            return true;
        }
    }

    false
}

/// Resolve the innermost missing expression at the cursor boundary.
pub(crate) fn enclosing_missing_expression(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // walk inward to outward until one missing expression claims the cursor
    for enc in enclosing_spans_at_cursor_boundary(ctx, offset) {
        if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        if matches!(
            ctx.ast_context().tree().get(expr_id),
            ast::Expression::Missing
        ) {
            return Some(expr_id);
        }
    }

    None
}

/// Check whether the cursor sits in a missing declarator initializer slot.
pub(crate) fn missing_declarator_value_at_cursor(ctx: &QueryContext<'_>, offset: u32) -> bool {
    let parents = ctx.ast_context().parents();
    let ast_tree = ctx.ast_context().tree();
    let enclosing = enclosing_spans_with_previous(ctx, offset);

    // look through enclosing spans and their declarator parents
    for enc in enclosing {
        for parent_id in std::iter::once(enc.idx).chain(parents.walk_parents_by_id(enc.idx)) {
            if ast_tree.get_node_type(parent_id) != ast::NodeType::Declarator {
                continue;
            }

            // only missing initializer values count here
            let declarator = ast_tree.get(ast::LocalNodeId::<ast::Declarator>::new(parent_id));
            let Some(value) = declarator.value else {
                continue;
            };

            if matches!(ast_tree.get(value), ast::Expression::Missing) {
                return true;
            }
        }
    }

    false
}
