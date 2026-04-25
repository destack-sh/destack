use crate::format::annotation::{
    FormatLeadingComments, FormatTrailingComments, format_node_with_trailing_comments,
};
use crate::format::chain::transparent_inner_expression;
use crate::format::expression::{
    write_expression_without_trailing_comments, write_type_expression_node,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Expression, GenericArgument, LocalNodeId, NodeType, TypeExpression, TypeLiteral,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    format_with, group, soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};

/// Write one type expression with inline prefix annotations.
pub(crate) fn write_type_expression_with_inline_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(node_id);
    write_type_expression_node(f, node_id, expression, true)
}

/// Write one type annotation prefix.
pub(crate) fn write_type_annotation_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    separator_start: u32,
) -> FormatResult<()> {
    let leading_comments = f
        .context()
        .comments()
        .comments_before(separator_start)
        .to_vec();

    if !leading_comments.is_empty() {
        write!(
            f,
            [space(), FormatLeadingComments::Comments(&leading_comments)]
        )?;
    }

    write!(f, [token(":"), space()])
}

/// Write one colon-prefixed type annotation.
pub(crate) fn write_colon_prefixed_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    write!(f, [token(":"), space()])?;
    write_type_expression_with_inline_prefix_annotations(f, node_id)
}

/// Return whether one type expression should hug inside a singleton generic-argument list.
fn should_hug_single_generic_type_argument(
    context: &DestackFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(type_id) {
        TypeExpression::ScalarLiteral { .. }
        | TypeExpression::Literal { .. }
        | TypeExpression::Intrinsic
        | TypeExpression::This
        | TypeExpression::TemplateLiteral { .. }
        | TypeExpression::Object { .. }
        | TypeExpression::Mapped { .. } => true,
        TypeExpression::Reference {
            generic_arguments, ..
        }
        | TypeExpression::Member {
            generic_arguments, ..
        }
        | TypeExpression::Import {
            generic_arguments, ..
        } => generic_arguments.is_empty(),
        TypeExpression::Union { elements } => {
            should_hug_single_generic_union_argument(context, type_id, elements)
        }
        _ => false,
    }
}

/// Return whether one union should hug inside a singleton generic-argument list.
fn should_hug_single_generic_union_argument(
    context: &DestackFormatContext<'_>,
    union_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
) -> bool {
    if elements.len() == 1 {
        return true;
    }

    let has_object_like_element = elements.iter().copied().any(|element_id| {
        matches!(
            context.tree.get(element_id),
            TypeExpression::Object { .. }
                | TypeExpression::Mapped { .. }
                | TypeExpression::Reference { .. }
                | TypeExpression::Member { .. }
        )
    });
    if !has_object_like_element {
        return false;
    }

    let void_like_count = elements
        .iter()
        .copied()
        .filter(|element_id| {
            matches!(
                context.tree.get(*element_id),
                TypeExpression::Literal {
                    value: TypeLiteral::Void | TypeLiteral::Null | TypeLiteral::Undefined
                }
            )
        })
        .count();
    if elements.len().saturating_sub(1) != void_like_count {
        return false;
    }

    let mut start = context.span(union_id).start;
    for element_id in elements.iter().copied() {
        let element_span = context.span(element_id);
        if context
            .comments()
            .has_comment_in_range(start, element_span.start)
        {
            return false;
        }

        start = element_span.end;
    }

    true
}

/// Return whether one singleton generic-argument list should stay inline.
fn generic_argument_list_should_hug(
    context: &DestackFormatContext<'_>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> bool {
    if generic_arguments.len() != 1 {
        return false;
    }

    let GenericArgument::Type { value } = context.tree.get(generic_arguments[0]) else {
        return false;
    };

    should_hug_single_generic_type_argument(context, *value)
}

/// Format generic arguments without multiline trailing commas.
pub(crate) fn format_generic_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    if generic_arguments.is_empty() {
        return write!(f, [token("<>")]);
    }

    let format_arguments = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (index, argument_id) in generic_arguments.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(",")])?;
                write!(f, [soft_line_break_or_space()])?;
            }

            write!(f, [argument_id])?;
        }

        Ok(())
    });

    if generic_argument_list_should_hug(f.context(), generic_arguments) {
        return write!(f, [token("<"), format_arguments, token(">")]);
    }

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
        | Expression::TaggedTemplateExpression {
            generic_arguments, ..
        }
        | Expression::TreeExpression {
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
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::QualifiedReference {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            expression_has_generic_arguments(context, *left)
        }
        Expression::TaggedTemplateExpression {
            tag,
            generic_arguments,
            ..
        } => !generic_arguments.is_empty() || expression_has_generic_arguments(context, *tag),
        Expression::TreeExpression {
            left,
            generic_arguments,
            ..
        } => {
            !generic_arguments.is_empty()
                || left.is_some_and(|left_id| expression_has_generic_arguments(context, left_id))
        }
        Expression::Call {
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
        _ => false,
    }
}

/// Return whether one cast or satisfies expression is in callee or object position.
fn is_callee_or_object_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);

    match context.tree.get(parent_id) {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. } => *left == node_id,
        _ => false,
    }
}

/// Format one `as` or `satisfies` assertion expression.
fn format_as_or_satisfies_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
    type_annotation_id: LocalNodeId<TypeExpression>,
    operation: &'static str,
) -> FormatResult<()> {
    let is_callee_or_object = is_callee_or_object_context(f.context(), node_id);

    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let type_expression = f.context().tree.get(type_annotation_id);
        let type_start = f.context().span(type_annotation_id).start;
        let comments = f
            .context()
            .comments()
            .comments_in_range(f.context().span(expression_id).end, type_start)
            .to_vec();
        let multiline_comment_position = comments
            .iter()
            .position(|comment| comment.is_multiline_block());
        let block_comments = if let Some(position) = multiline_comment_position {
            &comments[..position]
        } else {
            &comments[..]
        };

        // const assertions
        if !comments.is_empty() && matches!(type_expression, TypeExpression::Const) {
            let trailing_comments = &comments[block_comments.len()..];

            write_expression_without_trailing_comments(f, expression_id)?;

            if !block_comments.is_empty() {
                write!(f, [FormatTrailingComments::Comments(block_comments)])?;
            }

            write!(f, [space(), token(operation), space(), token("const")])?;

            if !trailing_comments.is_empty() {
                write!(f, [FormatTrailingComments::Comments(trailing_comments)])?;
            }

            return Ok(());
        }

        // direct cast
        if block_comments.is_empty() {
            write_expression_without_trailing_comments(f, expression_id)?;
            write!(f, [space(), token(operation), space()])?;
            write_type_expression_with_inline_prefix_annotations(f, type_annotation_id)?;

            return Ok(());
        }

        // commented cast
        write!(
            f,
            [
                format_node_with_trailing_comments(
                    f.context().span(node_id),
                    expression_id,
                    type_start
                ),
                space(),
                token(operation),
                space()
            ]
        )?;
        write_type_expression_with_inline_prefix_annotations(f, type_annotation_id)
    });

    if is_callee_or_object {
        write!(f, [group(&soft_block_indent(&format_inner))])
    } else {
        write!(f, [format_inner])
    }
}

/// Format one `as` assertion expression.
pub(crate) fn format_as_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: LocalNodeId<Expression>,
    type_annotation: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    format_as_or_satisfies_expression(f, node_id, expression, type_annotation, "as")
}

/// Format one `satisfies` assertion expression.
pub(crate) fn format_satisfies_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: LocalNodeId<Expression>,
    type_annotation: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    format_as_or_satisfies_expression(f, node_id, expression, type_annotation, "satisfies")
}
