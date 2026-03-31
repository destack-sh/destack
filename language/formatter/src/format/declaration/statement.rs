use destack_ast::{
    Block, BlockFormat, Comment, Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeType,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;

use crate::format::declaration::sequence::{
    block_allows_value_tail, format_block_body_narrow, format_block_body_wide,
    program_statement_sequence,
};
use crate::format::directive::{has_file_ignore_directive, write_ignored_span};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};

/// Create a formatter for a list of expression statements.
pub fn statement_list<'ast>(
    expressions: &'ast [LocalNodeId<Expression>],
) -> impl Format<DestackFormatContext<'ast>> + 'ast {
    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        // respect file-level ignore directives for top-level formatting
        if f.context().options.respect_file_ignore
            && statement_list_is_file_root(f.context(), expressions)
            && has_file_ignore_directive(f.context())
        {
            f.context().mark_file_ignore_applied();
            let full_file_span = Span::new(f.context().file.id, 0, f.context().file.len);
            write_ignored_span(f, full_file_span)?;
            return Ok(());
        }

        write!(f, [program_statement_sequence(expressions)])?;
        if !expressions.is_empty() {
            write!(f, [hard_line_break()])?;
        }

        Ok(())
    })
}

/// Return whether this statement list is the file root expression list.
fn statement_list_is_file_root(
    context: &DestackFormatContext<'_>,
    expressions: &[LocalNodeId<Expression>],
) -> bool {
    expressions
        .first()
        .is_some_and(|expression_id| context.parent(*expression_id).is_none())
}

/// Return raw own-line comments immediately before one block head.
pub(crate) fn block_leading_line_comment_nodes(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> Vec<LocalNodeId<Comment>> {
    let block_span = context.span(block_id);
    let Some(previous_token) = context.previous_non_trivia_token_before_span(block_span) else {
        return Vec::new();
    };
    if previous_token.span.file != block_span.file || previous_token.span.end >= block_span.start {
        return Vec::new();
    }

    context
        .comment_nodes_in_range(previous_token.span.end, block_span.start)
        .into_iter()
        .filter(|comment_id| {
            let comment = context.tree.get(*comment_id);
            comment.style == destack_ast::CommentStyle::Slash
                || context.span_starts_on_own_line(context.span(*comment_id))
        })
        .collect()
}

/// Return raw comments immediately before one block close brace.
pub(crate) fn block_trailing_comment_nodes(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> Vec<LocalNodeId<Comment>> {
    let block = context.tree.get(block_id);
    let block_span = context.span(block_id);
    let Some(close_brace_token) = context.last_non_trivia_token_in_span(block_span) else {
        return Vec::new();
    };

    let gap_start = if let Some(last_expression_id) = block.expressions.last().copied() {
        context.span(last_expression_id).end
    } else if let Some(open_brace_token) = context.first_non_trivia_token_in_span(block_span) {
        open_brace_token.span.end
    } else {
        block_span.start
    };
    if gap_start >= close_brace_token.span.start {
        return Vec::new();
    }

    context.comment_nodes_in_range(gap_start, close_brace_token.span.start)
}

/// Return whether one block carries raw comments that force expanded layout.
pub(crate) fn block_has_raw_internal_comments(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    !block_leading_line_comment_nodes(context, block_id).is_empty()
        || !block_trailing_comment_nodes(context, block_id).is_empty()
}

/// Format an empty block with infix annotations.
///
/// Example.
/// ```
/// {
///     // infix comment
/// }
/// ```
pub fn empty_block_with_infix_annotations<'ast, T>(
    node_id: LocalNodeId<T>,
) -> impl Format<DestackFormatContext<'ast>>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        // keep empty blocks compact unless they carry infix annotations
        if !f.context().has_infix_annotation(node_id) {
            return write!(f, [token("{"), token("}")]);
        }

        write!(
            f,
            [group(&format_args![
                token("{"),
                soft_block_indent(&format_args![
                    if_group_fits_on_line(&token("")),
                    &crate::format::annotation::block_infix_annotations(f.context(), node_id)
                ]),
                token("}")
            ])]
        )
    })
}

/// Return whether a block should stay inline.
#[inline]
pub(crate) fn should_inline_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = f.context().tree.get(block_id);
    let span = f.context().span(block_id);

    // can only inline if there is at most one expression
    if block.expressions.len() > 1
        || f.context().has_infix_annotation(block_id)
        || block_has_raw_internal_comments(f.context(), block_id)
    {
        return false;
    } else if block.expressions.is_empty() {
        // keep empty control flow blocks expanded
        if empty_block_prefers_multiline(f.context(), block_id) {
            return false;
        }

        return true;
    }

    // explicit non-value blocks should stay expanded, except empty blocks above
    if block.format == BlockFormat::Explicit && !block_allows_value_tail(f.context(), block_id) {
        return false;
    }

    // check whether the block is inlinable based on its contents
    // if any expression is not inline, then the entire block shouldn't be
    let is_body_inlinable = block.expressions.is_empty()
        || block
            .expressions
            .iter()
            .all(|expr_id| f.context().node(*expr_id).is_narrow());

    // container (default to self, mostly for testing)
    let (mut container_node_id, mut container_node_type) = f
        .context()
        .parent_by_id(block_id.id)
        .unwrap_or((block_id.id, NodeType::Block));
    if container_node_type == NodeType::Expression {
        (container_node_id, container_node_type) = f
            .context()
            .parent_by_id(container_node_id)
            .unwrap_or((container_node_id, NodeType::Block));
    }

    is_body_inlinable
        && !f.context().is_at_line_start(block_id.id)
        && !f.context().is_at_line_start(container_node_id)
        && !f.context().has_newline(span)
        && container_node_type != NodeType::Declaration
}

/// Return whether an empty block should stay multiline in control flow contexts.
fn empty_block_prefers_multiline<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let Some((parent_expression_id, parent_type)) = context.parent(block_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_expression_id);
    let Expression::Block(inner_block_id) = context.tree.get(parent_expression_id) else {
        return false;
    };
    if *inner_block_id != block_id {
        return false;
    }

    let Some((container_id, container_type)) = context.parent(parent_expression_id) else {
        return false;
    };
    if container_type == NodeType::MatchCase {
        return true;
    }

    if container_type == NodeType::Block {
        let container_block_id = LocalNodeId::<Block>::new(container_id);
        let container_block = context.tree.get(container_block_id);
        if container_block.format == BlockFormat::Implicit {
            let Some((container_owner_id, container_owner_type)) =
                context.parent(container_block_id)
            else {
                return false;
            };
            if container_owner_type == NodeType::MatchCase {
                let container_owner_id =
                    LocalNodeId::<destack_ast::MatchCase>::new(container_owner_id);
                let container_owner = context.tree.get(container_owner_id);
                if matches!(container_owner, destack_ast::MatchCase::Block { .. }) {
                    return true;
                }
            }
        }
        return false;
    }

    if container_type == NodeType::Expression {
        let container_id = LocalNodeId::<Expression>::new(container_id);
        return match context.tree.get(container_id) {
            Expression::Try { .. } => true,
            Expression::If {
                then_expression,
                else_expression,
                ..
            } => {
                then_expression.id == parent_expression_id.id
                    || else_expression.is_some_and(|id| id.id == parent_expression_id.id)
            }
            _ => false,
        };
    }

    false
}

/// Format a block (without a nested group!).
/// Format a block with opening and closing braces.
pub fn format_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(
        f,
        [crate::format::annotation::prefix_annotations(
            f.context(),
            node_id
        )]
    )?;
    if should_inline_block(f, node_id) {
        format_block_body_narrow(f, node_id)?;
    } else {
        format_block_body_wide(f, node_id)?;
    }
    write!(
        f,
        [crate::format::annotation::postfix_annotations(
            f.context(),
            node_id
        )]
    )?;
    Ok(())
}

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        node_id: LocalNodeId<Block>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [crate::format::annotation::prefix_annotations(
                f.context(),
                node_id
            )]
        )?;
        if should_inline_block(f, node_id) {
            write!(
                f,
                [group(&format_with(|f| format_block_body_narrow(
                    f, node_id
                )))]
            )?;
        } else {
            write!(
                f,
                [group(&format_with(|f| format_block_body_wide(f, node_id)))]
            )?;
        }
        write!(
            f,
            [crate::format::annotation::postfix_annotations(
                f.context(),
                node_id
            )]
        )?;
        Ok(())
    }
}
