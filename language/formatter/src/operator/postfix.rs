use crate::expression::{
    should_preserve_source_parentheses, write_expression_without_derived_parentheses,
};
use crate::{TsppFormatContext, TsppFormatter};
use tspp_dir::{Expression, Literal, LocalNodeId, NodeType, OperatorPrecedence, Tree};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::{format_with, group, soft_block_indent, token};
use tspp_fir::{format_args, write};

/// Return whether postfix formatting requires parentheses.
#[inline]
pub(crate) fn needs_parens_in_postfix_position(
    tree: &Tree,
    expr_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(
        tree.get(expr_id),
        Expression::ObjectExpression { .. } | Expression::TreeExpression { .. }
    ) {
        return true;
    }

    tree.get(expr_id).precedence() < OperatorPrecedence::Postfix
}

/// Format one expression as the receiver of a postfix operation.
pub(crate) fn write_postfix_base_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let parent_expression_id = postfix_parent_expression_id(f.context(), expression_id);
    let needs_integer_member_parentheses = parent_expression_id.is_some_and(|parent_id| {
        matches!(
            f.context().tree.get(expression_id),
            Expression::Literal(Literal::Integer(_))
        ) && matches!(f.context().tree.get(parent_id), Expression::Member { .. })
    });
    let needs_parentheses = needs_parens_in_postfix_position(f.context().tree, expression_id)
        || needs_integer_member_parentheses
        || should_preserve_source_parentheses(f.context(), expression_id);
    if needs_parentheses {
        write!(
            f,
            [group(&format_args![
                token("("),
                soft_block_indent(&format_with(|f| {
                    write_expression_without_derived_parentheses(f, expression_id)
                })),
                token(")")
            ])]
        )?;
    } else {
        write!(f, [expression_id])?;
    }
    Ok(())
}

/// Return one postfix parent expression id when this expression is used as a chain receiver.
fn postfix_parent_expression_id(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    if let Some((parent_id, parent_type)) = context.parent(expression_id)
        && parent_type == NodeType::Expression
    {
        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        let parent_expression = context.tree.get(parent_expression_id);
        let uses_expression_as_left = matches!(
            parent_expression,
            Expression::Member { left, .. }
                | Expression::Call { left, .. }
                | Expression::Index { left, .. }
                | Expression::Instantiation { left, .. }
                | Expression::Maybe { left, .. }
                | Expression::Must { left, .. }
                | Expression::Chain { expression: left }
                if *left == expression_id
        );
        if uses_expression_as_left {
            return Some(parent_expression_id);
        }
    }

    None
}

/// Return whether one expression is a postfix chain expression.
pub(crate) fn is_chain_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Member { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Instantiation { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    )
}
