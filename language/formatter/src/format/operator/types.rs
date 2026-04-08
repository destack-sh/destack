use super::r#type::write_type_expression_with_inline_prefix_annotations_from;
use crate::format::chain::transparent_inner_expression;
use crate::format::declaration::expression_is_in_statement_position;
use crate::format::expression::{
    parenthesized_has_leading_inner_trivia, write_expression_without_trailing_annotations,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Expression, LocalNodeId, NodeType, TokenType, TypeBinaryOperator};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{block_indent, format_with, group, soft_block_indent, space, token};
use destack_fir::{best_fitting, format_args, write};

/// Return whether one type binary left expression is simple enough to stay ungrouped.
pub(crate) fn is_simple_type_binary_left_expression(
    tree: &destack_ast::NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Parenthesized { expression } => {
            is_simple_type_binary_left_expression(tree, *expression)
        }
        Expression::TypeBinary { left, operator, .. } => {
            matches!(
                operator,
                TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
            ) && is_simple_type_binary_left_expression(tree, *left)
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
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. }
        | Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_) => true,
        _ => false,
    }
}

/// Return whether one type expression is object-like.
pub(crate) fn is_object_like_type_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    )
}

/// Return whether one expression appears inside a type template literal interpolation.
pub(crate) fn is_in_type_template_literal_interpolation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id.id;

    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        if parent_type == NodeType::Expression
            && matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::TypeTemplateLiteral { .. }
            )
        {
            return true;
        }

        current_id = parent_id;
    }

    false
}

/// Return whether one expression span uses angle assertion syntax in source.
pub(crate) fn expression_uses_angle_assertion_syntax(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    allow_parenthesis_prefix: bool,
) -> bool {
    if context.options.language_type.supports_jsx() {
        return false;
    }

    let Some(main_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    let mut token_index = 0usize;
    while let Some(token) = context.nth_non_trivia_token_in_span(main_span, token_index) {
        match token.token.ty {
            TokenType::LessThan => return true,
            TokenType::OpenParenthesis if allow_parenthesis_prefix => token_index += 1,
            _ => return false,
        }
    }

    false
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

/// Return the effective left expression for one cast or satisfies expression.
fn normalized_type_binary_left_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &TypeBinaryOperator,
) -> LocalNodeId<Expression> {
    let cast_uses_angle_assertion = *operator == TypeBinaryOperator::Cast
        && expression_uses_angle_assertion_syntax(context, node_id, true);
    if cast_uses_angle_assertion {
        return left;
    }

    if !matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) {
        return left;
    }

    let Expression::Parenthesized { expression } = context.tree.get(left) else {
        return left;
    };
    if should_drop_type_binary_left_parentheses(context, node_id, left, *expression) {
        return *expression;
    }

    left
}

/// Format one angle-assertion style cast expression.
fn format_angle_assertion_type_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let break_after_cast = !matches!(
        f.context().tree.get(left),
        Expression::ArrayExpression { .. } | Expression::ObjectExpression { .. }
    );
    let format_cast = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [token("<"), group(&soft_block_indent(&right)), token(">")]
        )
    });

    if break_after_cast {
        write!(
            f,
            [best_fitting![
                format_args![format_cast, left],
                format_args![
                    format_cast,
                    group(&format_args![token("("), block_indent(&left), token(")")])
                ],
                format_args![format_cast, left]
            ]]
        )
    } else {
        write!(f, [format_cast, left])
    }
}

/// Format one keyword-style cast or satisfies expression.
fn format_as_or_satisfies_type_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let left_span = f.context().span(left);
    let right_span = f.context().span(right);
    let comments = {
        let comments = f.context().comments();
        comments
            .comments_in_range(left_span.end, right_span.start)
            .to_vec()
    };
    let multiline_block_comment_index = comments
        .iter()
        .position(|comment| comment.is_block() && f.context().has_newline(comment.span));
    let block_comments = multiline_block_comment_index
        .map_or_else(|| comments.as_slice(), |index| &comments[..index]);
    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if block_comments.is_empty() {
            write_expression_without_trailing_annotations(f, left)?;
            write!(f, [space(), operator, space()])?;
        } else {
            write!(f, [left, space(), operator, space()])?;
        }

        write_type_expression_with_inline_prefix_annotations_from(f, right, left_span.end)
    });

    if type_binary_is_callee_or_object_context(f.context(), node_id) {
        write!(f, [group(&soft_block_indent(&format_inner))])
    } else {
        write!(f, [format_inner])
    }
}

/// Format a type-binary expression with chain-aware left-hand expansion.
pub(crate) fn format_type_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let cast_uses_angle_assertion = *operator == TypeBinaryOperator::Cast
        && expression_uses_angle_assertion_syntax(f.context(), node_id, true);
    let formatted_left =
        normalized_type_binary_left_expression(f.context(), node_id, left, operator);
    if cast_uses_angle_assertion {
        return format_angle_assertion_type_binary_expression(f, formatted_left, right);
    }

    format_as_or_satisfies_type_binary_expression(f, node_id, formatted_left, operator, right)
}

/// Decide whether cast or satisfies can drop a parenthesized left side.
fn should_drop_type_binary_left_parentheses(
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

    let left_is_cast_chain = matches!(
        context.tree.get(left_id),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    );

    if context.has_annotation(parenthesized_id) {
        return false;
    }

    if context.has_annotation(left_id) && !left_is_cast_chain {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, left_id)
        && !left_is_cast_chain
    {
        return false;
    }

    is_simple_type_binary_left_expression(context.tree, left_id)
}
