use destack_ast::{
    Block, BlockFormat, Comment, Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeType,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;

use crate::format::annotation::{block_infix_annotations, postfix_annotations, prefix_annotations};
use crate::format::declaration::sequence::{
    block_allows_value_tail, expression_postfix_end, format_block_body_narrow,
    format_block_body_wide, program_statement_sequence,
};
use crate::format::file::{has_file_ignore_directive, write_ignored_span};
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

/// Return own-line comments immediately before one block head.
pub(crate) fn block_leading_line_comment_nodes(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> Vec<Comment> {
    let block_span = context.span(block_id);
    let Some(open_brace_token) = context.first_non_trivia_token_in_span(block_span) else {
        return Vec::new();
    };
    let Some(previous_token) = context.previous_non_trivia_token_before_span(block_span) else {
        return Vec::new();
    };
    if previous_token.span.file != block_span.file
        || previous_token.span.end >= open_brace_token.span.start
    {
        return Vec::new();
    }

    {
        let comments = context.comments();
        comments
            .comments_in_range(previous_token.span.end, open_brace_token.span.start)
            .to_vec()
    }
    .into_iter()
    .filter(|comment| comment.is_line() || context.span_starts_on_own_line(comment.span))
    .collect()
}

/// Return comments immediately before one block close brace.
pub(crate) fn block_trailing_comment_nodes(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> Vec<Comment> {
    let block = context.tree.get(block_id);
    let block_span = context.span(block_id);
    let Some(close_brace_token) = context.last_non_trivia_token_in_span(block_span) else {
        return Vec::new();
    };

    let gap_start = if let Some(last_expression_id) = block.last_expression() {
        let expression_span = context.span(last_expression_id);
        expression_postfix_end(context, last_expression_id, expression_span.end).saturating_add(1)
    } else if let Some(open_brace_token) = context.first_non_trivia_token_in_span(block_span) {
        open_brace_token.span.end
    } else {
        block_span.start
    };
    if gap_start >= close_brace_token.span.start {
        return Vec::new();
    }

    {
        let comments = context.comments();
        comments
            .comments_in_range(gap_start, close_brace_token.span.start)
            .to_vec()
    }
}

/// Return whether one block carries internal comments that force expanded layout.
pub(crate) fn block_has_internal_comments(
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
                    &block_infix_annotations(f.context(), node_id)
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
    if block.len() > 1
        || f.context().has_infix_annotation(block_id)
        || block_has_internal_comments(f.context(), block_id)
    {
        return false;
    } else if block.is_empty() {
        // keep empty control flow blocks expanded
        if empty_block_requires_expanded_layout(f.context(), block_id) {
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
    let is_body_inlinable = block.is_empty()
        || block
            .iter_expressions()
            .all(|expr_id| f.context().node(expr_id).is_narrow());

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

/// Return whether an empty block should keep expanded braces.
fn empty_block_requires_expanded_layout(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let mut current_block_id = block_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent_by_id(current_block_id.id) else {
            return false;
        };

        if parent_type == NodeType::MatchCase {
            return true;
        }

        if parent_type == NodeType::Expression {
            return empty_block_expands_in_expression_shell(context, current_block_id, parent_id);
        }

        if parent_type != NodeType::Block {
            return false;
        }

        let parent_block_id = LocalNodeId::<Block>::new(parent_id);
        let parent_block = context.tree.get(parent_block_id);
        if parent_block.format != BlockFormat::Implicit {
            return false;
        }

        current_block_id = parent_block_id;
    }
}

/// Return whether one empty block should expand inside its immediate expression shell.
fn empty_block_expands_in_expression_shell(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
    parent_expression_id: u32,
) -> bool {
    let parent_expression_id = LocalNodeId::<Expression>::new(parent_expression_id);
    let Expression::Block(inner_block_id) = context.tree.get(parent_expression_id) else {
        return false;
    };
    if *inner_block_id != block_id {
        return false;
    }

    let Some((shell_id, shell_type)) = context.parent_by_id(parent_expression_id.id) else {
        return false;
    };

    if shell_type == NodeType::MatchCase {
        return true;
    }
    if shell_type != NodeType::Expression {
        return false;
    }

    let shell_expression_id = LocalNodeId::<Expression>::new(shell_id);
    match context.tree.get(shell_expression_id) {
        Expression::If {
            then_expression,
            else_expression,
            ..
        } => {
            then_expression.id == parent_expression_id.id
                || else_expression.is_some_and(|id| id.id == parent_expression_id.id)
        }
        Expression::Try {
            try_expression,
            catch_expression,
            finally_expression,
            ..
        } => {
            try_expression.id == parent_expression_id.id
                || finally_expression.is_some_and(|id| id.id == parent_expression_id.id)
                || catch_expression.is_some_and(|id| {
                    id.id == parent_expression_id.id && finally_expression.is_some()
                })
        }
        _ => false,
    }
}

/// Format a block (without a nested group!).
/// Format a block with opening and closing braces.
pub(crate) fn write_block_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    if should_inline_block(f, node_id) {
        return format_block_body_narrow(f, node_id);
    }

    format_block_body_wide(f, node_id)
}

/// Format a block (without a nested group!).
/// Format a block with opening and closing braces.
pub fn format_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(f, [prefix_annotations(f.context(), node_id)])?;
    write_block_body(f, node_id)?;
    write!(f, [postfix_annotations(f.context(), node_id)])?;
    Ok(())
}

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        node_id: LocalNodeId<Block>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
        write!(f, [group(&format_with(|f| write_block_body(f, node_id)))])?;
        write!(f, [postfix_annotations(f.context(), node_id)])?;
        Ok(())
    }
}
