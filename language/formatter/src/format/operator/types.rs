use super::r#type::{
    write_type_expression_with_inline_prefix_annotations,
    write_type_expression_with_inline_prefix_annotations_after_offset,
};
use crate::format::annotation::write_raw_comment_slice;
use crate::format::context::ParenthesizedExpressionView;
use crate::format::declaration::expression_is_in_statement_position;
use crate::format::expression::write_expression_without_trailing_annotations;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Expression, LocalNodeId, NodeType, TypeExpression};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{format_with, group, soft_block_indent, space, token};
use destack_fir::write;

/// Return whether one type expression is a `const` type reference.
fn type_expression_is_const_reference(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    matches!(context.tree.get(expression_id), TypeExpression::Const)
}

/// Return whether one type binary left expression is simple enough to stay ungrouped.
pub(crate) fn is_simple_type_binary_left_expression(
    tree: &destack_ast::NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Parenthesized { expression } => {
            is_simple_type_binary_left_expression(tree, *expression)
        }
        Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
            is_simple_type_binary_left_expression(tree, *expression)
        }
        Expression::Identifier { .. }
        | Expression::QualifiedReference { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Call { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. }
        | Expression::ImportMeta
        | Expression::NewTarget
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. }
        | Expression::ScalarLiteral(_) => true,
        _ => false,
    }
}

/// Return whether one cast or satisfies expression is in callee or object position.
fn type_binary_is_callee_or_object_context(
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

/// Return the effective left expression for one assertion expression.
fn normalized_assertion_left_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let Expression::Parenthesized { expression } = context.tree.get(left) else {
        return left;
    };
    if should_drop_assertion_left_parentheses(context, node_id, left, *expression) {
        return *expression;
    }

    left
}

/// Format one keyword-style assertion expression.
fn format_keyword_assertion_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    keyword: &'static str,
    right: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let boundary_comments = f.context().raw_type_position_comments_for(right);
    let boundary_block_comments = boundary_comments
        .iter()
        .copied()
        .filter(|comment: &destack_ast::Comment| comment.is_block())
        .collect::<Vec<_>>();
    let boundary_block_comment_end = boundary_block_comments
        .iter()
        .map(|comment| comment.span.end)
        .max();
    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // left side
        write_expression_without_trailing_annotations(f, left)?;

        // keyword
        write!(f, [space(), token(keyword)])?;

        if !boundary_block_comments.is_empty() {
            write_raw_comment_slice(f, &boundary_block_comments)?;
            write!(f, [space()])?;

            if let Some(boundary_block_comment_end) = boundary_block_comment_end {
                return write_type_expression_with_inline_prefix_annotations_after_offset(
                    f,
                    right,
                    boundary_block_comment_end,
                );
            }
        }

        write!(f, [space()])?;
        write_type_expression_with_inline_prefix_annotations(f, right)
    });

    if type_binary_is_callee_or_object_context(f.context(), node_id) {
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
    let formatted_left = normalized_assertion_left_expression(f.context(), node_id, expression);

    if type_expression_is_const_reference(f.context(), type_annotation) {
        let format_const = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [space(), token("as"), space(), token("const")])
        });

        if type_binary_is_callee_or_object_context(f.context(), node_id) {
            return write!(
                f,
                [group(&soft_block_indent(&format_with(
                    |f: &mut DestackFormatter<'ast, '_>| {
                        write_expression_without_trailing_annotations(f, formatted_left)?;
                        write!(f, [format_const])
                    }
                )))]
            );
        }

        return write!(f, [formatted_left, format_const]);
    }

    format_keyword_assertion_expression(f, node_id, formatted_left, "as", type_annotation)
}

/// Format one `satisfies` assertion expression.
pub(crate) fn format_satisfies_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: LocalNodeId<Expression>,
    type_annotation: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let formatted_left = normalized_assertion_left_expression(f.context(), node_id, expression);
    format_keyword_assertion_expression(f, node_id, formatted_left, "satisfies", type_annotation)
}

/// Decide whether one assertion can drop a parenthesized left side.
fn should_drop_assertion_left_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parenthesized_id: LocalNodeId<Expression>,
    left_id: LocalNodeId<Expression>,
) -> bool {
    if expression_is_in_statement_position(context, node_id)
        && !matches!(
            context.parent(node_id).map(|(_, parent_type)| parent_type),
            Some(NodeType::Declaration | NodeType::Member | NodeType::Property)
        )
    {
        return false;
    }

    let left_is_assertion_chain = matches!(
        context.tree.get(left_id),
        Expression::As { .. } | Expression::Satisfies { .. }
    );

    if context.has_annotation(parenthesized_id) {
        return false;
    }

    if context.has_annotation(left_id) && !left_is_assertion_chain {
        return false;
    }

    if ParenthesizedExpressionView::from_node(context, parenthesized_id)
        .is_some_and(ParenthesizedExpressionView::has_leading_inner_trivia)
    {
        let parent_is_adjacent_statement_wrapper =
            context
                .parent(node_id)
                .is_some_and(|(parent_id, parent_type)| {
                    parent_type == NodeType::Expression
                        && matches!(
                            context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                            Expression::Yield { .. }
                                | Expression::Return { .. }
                                | Expression::Throw { .. }
                        )
                });

        if !parent_is_adjacent_statement_wrapper {
            return false;
        }
    }

    is_simple_type_binary_left_expression(context.tree, left_id)
}
