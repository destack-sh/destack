use super::transparent_inner_expression;
use crate::DestackFormatContext;
use crate::format::operator::is_object_like_type_expression;
use destack_ast::{Argument, Declaration, Expression, FunctionKind, LocalNodeId, NodeTree};

/// Get the value expression of any argument variant.
pub(crate) fn argument_value_id_if_present(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => Some(*value),
        Argument::Error => None,
    }
}

/// Return whether static arguments are structurally safe for hugged inline formatting.
pub(crate) fn static_argument_list_is_hug_safe(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.len() != 1 {
        return false;
    }

    static_arguments.iter().copied().all(|argument_id| {
        if context.has_annotation(argument_id) {
            return false;
        }

        let Some(argument_value_id) = argument_value_id_if_present(context.tree, argument_id)
        else {
            return false;
        };
        let argument_value_id = transparent_inner_expression(context, argument_value_id);
        if !context
            .raw_type_position_comments_for(argument_value_id)
            .is_empty()
        {
            return false;
        }

        // object-like type arguments stay hugged even when the source was multiline
        if is_object_like_type_expression(context, argument_value_id) {
            return true;
        }

        if context.node_has_newline(argument_value_id) {
            return false;
        }

        static_argument_expression_is_hug_safe(context, argument_value_id)
    })
}

/// Check whether one expression is a lambda declaration.
pub(crate) fn is_lambda_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    let Expression::Declaration(declaration_id) = tree.get(expression_id) else {
        return false;
    };

    matches!(
        tree.get(*declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

/// Check whether an expression is a lambda whose body is another lambda.
pub(crate) fn is_nested_lambda_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    let Expression::Declaration(declaration_id) = tree.get(expression_id) else {
        return false;
    };

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = tree.get(*declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    is_lambda_expression(context, *body_id)
}

/// Return whether one static argument expression is structurally safe for hugged inline formatting.
fn static_argument_expression_is_hug_safe(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::Identifier { .. }
        | Expression::PrivateIdentifier { .. } => true,
        Expression::QualifiedReference {
            static_arguments, ..
        } => static_arguments.as_deref().is_none_or(|static_arguments| {
            static_argument_list_is_hug_safe(context, static_arguments)
        }),
        Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            static_argument_expression_is_hug_safe(context, *left)
                && static_arguments.as_deref().is_none_or(|static_arguments| {
                    static_argument_list_is_hug_safe(context, static_arguments)
                })
        }
        Expression::Index { .. } | Expression::TypeIndex { .. } => false,
        Expression::Parenthesized { expression } => {
            static_argument_expression_is_hug_safe(context, *expression)
        }
        Expression::Binary {
            operator:
                destack_ast::BinaryOperator::ElementwiseAnd
                | destack_ast::BinaryOperator::ElementwiseOr
                | destack_ast::BinaryOperator::ElementwiseXor,
            left,
            right,
        } => {
            static_argument_expression_is_hug_safe(context, *left)
                && static_argument_expression_is_hug_safe(context, *right)
        }
        _ => false,
    }
}
