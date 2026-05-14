use crate::DestackFormatContext;
use destack_dir::{Argument, Declaration, Expression, FunctionForm, LocalNodeId, Tree};

/// Get the value expression of any argument variant.
pub(crate) fn argument_value_id_if_present(
    tree: &Tree,
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
        Declaration::Function(function) if function.signature.form == FunctionForm::Lambda
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

    let Declaration::Function(function) = tree.get(*declaration_id) else {
        return false;
    };

    let Some(body_id) = function.body else {
        return false;
    };

    if function.signature.form != FunctionForm::Lambda {
        return false;
    }

    is_lambda_expression(context, body_id)
}
