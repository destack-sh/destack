use crate::format::annotation::write_raw_leading_comments;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Expression, GenericArgument, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenType,
    TypeExpression,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    format_with, group, soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::NodeSpanType;

/// Return the shell boundary start for one explicit type payload.
pub(crate) fn type_expression_boundary_start<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> Option<u32>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
{
    if let Some(start) = context.type_expression_leading_comment_start(node_id) {
        return Some(start);
    }

    if let Some(leading_span) = context.tree.get_side_span(node_id, NodeSpanType::Leading) {
        let token = context.first_non_trivia_token_in_span(leading_span)?;
        if matches!(
            token.token.ty,
            TokenType::ElementwiseOr | TokenType::ElementwiseAnd
        ) {
            return Some(leading_span.start);
        }
    }

    context
        .previous_non_trivia_token_before_span(context.span(node_id))
        .map(|token| token.span.end)
}

/// Run one formatter operation with one explicit type-expression root.
fn with_type_expression_root<'ast, T, N>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    operation: impl FnOnce(&mut DestackFormatter<'ast, '_>) -> T,
) -> T
where
    N: Node + Clone + FormatNode<'ast, N> + 'ast,
    NodeTree: NodeTreeImpl<N>,
{
    let context = f.context().clone();
    context.with_type_expression_root(node_id, || operation(f))
}

/// Run one formatter operation with one explicit type-expression root and leading boundary.
pub(crate) fn with_type_expression_root_from<'ast, T, N>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    leading_comment_start: u32,
    operation: impl FnOnce(&mut DestackFormatter<'ast, '_>) -> T,
) -> T
where
    N: Node + Clone + FormatNode<'ast, N> + 'ast,
    NodeTree: NodeTreeImpl<N>,
{
    let context = f.context().clone();
    context.with_type_expression_root_from(node_id, Some(leading_comment_start), || operation(f))
}

/// Write one type expression with inline prefix annotations.
pub(crate) fn write_type_expression_with_inline_prefix_annotations<'ast, N>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
) -> FormatResult<()>
where
    N: Node + Clone + FormatNode<'ast, N> + 'ast,
    NodeTree: NodeTreeImpl<N>,
{
    if f.context().type_expression_root_id(node_id).is_none()
        && let Some(boundary_start) = type_expression_boundary_start(f.context(), node_id)
    {
        return with_type_expression_root_from(f, node_id, boundary_start, |f| {
            write!(f, [node_id])
        });
    }

    with_type_expression_root(f, node_id, |f| write!(f, [node_id]))
}

/// Write one type expression after one owned comment boundary.
pub(crate) fn write_type_expression_with_inline_prefix_annotations_after_offset<'ast, N>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    _start_offset: u32,
) -> FormatResult<()>
where
    N: Node + Clone + FormatNode<'ast, N> + 'ast,
    NodeTree: NodeTreeImpl<N>,
{
    with_type_expression_root(f, node_id, |f| write!(f, [node_id]))
}

/// Write one colon-prefixed type annotation with group-aware boundary layout.
pub(crate) fn write_colon_prefixed_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    write!(f, [token(":"), space()])?;
    write_type_expression_with_inline_prefix_annotations(f, node_id)
}

/// Write one colon-prefixed type annotation after wrapper-owned trailing comments.
pub(crate) fn write_colon_prefixed_type_annotation_with_trailing_comments<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    owner_id: LocalNodeId<T>,
    node_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    let trailing_comments = f.context().raw_comments_in_trailing_for(owner_id);
    if !trailing_comments.is_empty() {
        write!(f, [space()])?;
        write_raw_leading_comments(f, &trailing_comments)?;
    }

    write_colon_prefixed_type_annotation(f, node_id)
}

/// Format generic arguments without multiline trailing commas.
pub(crate) fn format_generic_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    if generic_arguments.is_empty() {
        return write!(f, [token("<>")]);
    }

    if generic_arguments.len() == 1 && !f.context().node_has_newline(generic_arguments[0]) {
        write!(f, [token("<"), generic_arguments[0], token(">")])?;
        return Ok(());
    }

    let format_arguments = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (index, argument_id) in generic_arguments.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(","), soft_line_break_or_space()])?;
            }

            write!(f, [argument_id])?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_args![
            token("<"),
            soft_block_indent(&format_arguments),
            token(">")
        ])]
    )
}

/// Format generic arguments with relational spacing for index-following instantiations.
pub(crate) fn format_generic_argument_list_with_relational_spacing<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    write!(f, [space(), token("<"), space()])?;

    for (index, argument_id) in generic_arguments.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        write!(f, [argument_id])?;
    }

    write!(f, [space(), token(">"), space()])
}

/// Return generic arguments for expression variants that support them.
pub(crate) fn expression_generic_arguments(
    expression: &Expression,
) -> Option<&[LocalNodeId<GenericArgument>]> {
    match expression {
        Expression::QualifiedReference {
            generic_arguments, ..
        }
        | Expression::Member {
            generic_arguments, ..
        }
        | Expression::PrivateMember {
            generic_arguments, ..
        }
        | Expression::Instantiation {
            generic_arguments, ..
        }
        | Expression::Call {
            generic_arguments, ..
        }
        | Expression::New {
            generic_arguments, ..
        } => Some(generic_arguments.as_slice()),
        _ => None,
    }
}

/// Return whether an expression tree contains generic arguments.
pub(crate) fn expression_has_generic_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = context.transparent_inner_expression(expression_id);

    match context.tree.get(expression_id) {
        Expression::QualifiedReference {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),
        Expression::Member {
            left,
            generic_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            generic_arguments,
            ..
        }
        | Expression::Call {
            left,
            generic_arguments,
            ..
        }
        | Expression::New {
            left,
            generic_arguments,
            ..
        }
        | Expression::Instantiation {
            left,
            generic_arguments,
        } => !generic_arguments.is_empty() || expression_has_generic_arguments(context, *left),
        Expression::Parenthesized { expression } => {
            expression_has_generic_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether one expression sits in explicit type position.
pub(crate) fn expression_is_type_position(
    _context: &DestackFormatContext<'_>,
    _expression_id: LocalNodeId<Expression>,
) -> bool {
    false
}

/// Decide whether a parenthesized type expression can drop wrappers.
pub(crate) fn should_drop_parenthesized_type_expression(
    _context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<Expression>,
    _inner_id: LocalNodeId<Expression>,
) -> bool {
    false
}
