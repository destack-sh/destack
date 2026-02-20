use destack_ast::{
    Block, BlockContext, Declaration, Expression, FunctionMode, IfCondition, LocalNodeId, Member,
    NodeType, Property,
};

use crate::DestackFormatContext;

/// Return true when the final expression in this block is value-position.
pub(crate) fn block_allows_value_tail(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    if block.context != BlockContext::Expression {
        return false;
    }

    let Some((block_expression_id, block_expression_type)) = context.parent(block_id) else {
        return false;
    };
    if block_expression_type == NodeType::MatchCase {
        return true;
    }
    if block_expression_type != NodeType::Expression {
        return false;
    }

    let block_expression_id = LocalNodeId::<Expression>::new(block_expression_id);
    let Expression::Block(inner_block_id) = context.tree.get(block_expression_id) else {
        return false;
    };
    if *inner_block_id != block_id {
        return false;
    }

    !expression_is_in_statement_position(context, block_expression_id)
}

/// Return true when this expression is in statement position.
fn expression_is_in_statement_position(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return true;
    };

    match parent_type {
        NodeType::Expression => {
            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            let parent_expression = context.tree.get(parent_expression_id);
            if matches!(
                parent_expression,
                Expression::Statement(inner_expression_id) if *inner_expression_id == expression_id
            ) {
                return true;
            }

            let should_inherit_parent_position = match parent_expression {
                Expression::If {
                    condition,
                    then_expression,
                    else_expression,
                    ..
                } => {
                    let branch_inherits_statement_position =
                        if_condition_inherits_statement_position(context, condition);

                    branch_inherits_statement_position
                        && (then_expression.id == expression_id.id
                            || else_expression.as_ref().is_some_and(|else_expression| {
                                else_expression.id == expression_id.id
                            }))
                }
                Expression::While { body, .. }
                | Expression::ForEach { body, .. }
                | Expression::For { body, .. }
                | Expression::Loop { body } => body.id == expression_id.id,
                Expression::Try {
                    try_expression,
                    catch_expression,
                    finally_expression,
                    ..
                } => {
                    try_expression.id == expression_id.id
                        || catch_expression
                            .as_ref()
                            .is_some_and(|catch_expression| catch_expression.id == expression_id.id)
                        || finally_expression
                            .as_ref()
                            .is_some_and(|finally_expression| {
                                finally_expression.id == expression_id.id
                            })
                }
                Expression::Labelled { body, .. } => body.id == expression_id.id,
                _ => false,
            };

            should_inherit_parent_position
                && expression_is_in_statement_position(context, parent_expression_id)
        }
        NodeType::Block => expression_is_in_statement_position_inside_parent_block(
            context,
            LocalNodeId::<Block>::new(parent_id),
            expression_id,
        ),
        NodeType::Declaration => expression_is_in_statement_position_inside_parent_declaration(
            context,
            LocalNodeId::<Declaration>::new(parent_id),
            expression_id,
        ),
        NodeType::Member => expression_is_in_statement_position_inside_parent_member(
            context,
            LocalNodeId::<Member>::new(parent_id),
            expression_id,
        ),
        NodeType::Property => expression_is_in_statement_position_inside_parent_property(
            context,
            LocalNodeId::<Property>::new(parent_id),
            expression_id,
        ),
        _ => false,
    }
}

/// Return true when one condition should inherit statement-position from its parent `if`.
fn if_condition_inherits_statement_position(
    context: &DestackFormatContext<'_>,
    condition: &IfCondition,
) -> bool {
    match condition {
        // `if let` branches should preserve expression tails
        IfCondition::Let { .. } => false,
        // `if (comptime ...)` branches should preserve expression tails
        IfCondition::Expression { condition } => {
            !matches!(context.tree.get(*condition), Expression::Comptime { .. })
        }
    }
}

/// Return true when one child expression is statement-position inside one parent block.
fn expression_is_in_statement_position_inside_parent_block(
    context: &DestackFormatContext<'_>,
    parent_block_id: LocalNodeId<Block>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_block = context.tree.get(parent_block_id);

    if parent_block.context == BlockContext::Statement {
        return true;
    }

    let is_last_expression = parent_block
        .expressions
        .last()
        .is_some_and(|last_expression_id| *last_expression_id == expression_id);
    if !is_last_expression {
        return true;
    }

    !block_allows_value_tail(context, parent_block_id)
}

/// Return true when one child expression is statement-position inside one parent declaration.
fn expression_is_in_statement_position_inside_parent_declaration(
    context: &DestackFormatContext<'_>,
    parent_declaration_id: LocalNodeId<Declaration>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_declaration = context.tree.get(parent_declaration_id);

    match parent_declaration {
        Declaration::Function {
            body, signature, ..
        } => body.as_ref().is_some_and(|body_expression_id| {
            if body_expression_id.id != expression_id.id {
                return false;
            }

            function_body_is_statement_position(signature.mode)
                && !function_has_self_return_type(context, signature.return_type)
        }),
        Declaration::Global { expressions, .. } | Declaration::Namespace { expressions, .. } => {
            expressions.contains(&expression_id)
        }
        _ => false,
    }
}

/// Return true when one child expression is statement-position inside one parent member.
fn expression_is_in_statement_position_inside_parent_member(
    context: &DestackFormatContext<'_>,
    parent_member_id: LocalNodeId<Member>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_member = context.tree.get(parent_member_id);

    match parent_member {
        Member::Method { body, .. } => body
            .as_ref()
            .is_some_and(|body_expression_id| body_expression_id.id == expression_id.id),
        Member::StaticBlock { body, .. } | Member::ComptimeBlock { body, .. } => {
            body.id == expression_id.id
        }
        _ => false,
    }
}

/// Return true when one child expression is statement-position inside one parent property method.
fn expression_is_in_statement_position_inside_parent_property(
    context: &DestackFormatContext<'_>,
    parent_property_id: LocalNodeId<Property>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_property = context.tree.get(parent_property_id);

    match parent_property {
        Property::Method { body, .. } => body
            .as_ref()
            .is_some_and(|body_expression_id| body_expression_id.id == expression_id.id),
        _ => false,
    }
}

/// Return true when one function-like body should be statement-position.
fn function_body_is_statement_position(mode: Option<FunctionMode>) -> bool {
    if matches!(mode, Some(FunctionMode::Constructor | FunctionMode::Setter)) {
        return true;
    }

    true
}

/// Return true when one function return type is exactly `Self`.
fn function_has_self_return_type(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(return_type_id) = return_type else {
        return false;
    };

    expression_is_self_type_path(context, return_type_id)
}

/// Return true when one expression is a simple `Self` type path.
fn expression_is_self_type_path(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::Path { path, .. } => {
            path.segments.len() == 1 && context.strings.get(path.segments[0]) == "Self"
        }
        Expression::Parenthesized { expression } => {
            expression_is_self_type_path(context, *expression)
        }
        _ => false,
    }
}
