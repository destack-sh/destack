use destack_ast::{
    Declaration, Expression, FunctionKind, IfKind, LocalNodeId, NodeTree, NodeType, Parameter,
    ScalarLiteral, TypeUnaryOperator, UnaryOperator, WhileKind,
};

use crate::DestackFormatContext;

/// Return whether one lambda signature can print without parentheses.
fn lambda_signature_can_avoid_parentheses(
    tree: &NodeTree,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Declaration::Function { signature, .. } = tree.get(declaration_id) else {
        return false;
    };
    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    if signature.this_parameter.is_some() || signature.generics.is_some() {
        return false;
    }

    if signature.dynamic_parameters.len() != 1 {
        return false;
    }

    let parameter_id = signature.dynamic_parameters[0];
    matches!(
        tree.get(parameter_id),
        Parameter::Named {
            modifiers: None,
            ty: None,
            default: None,
            ..
        }
    )
}

/// Return whether one declaration starts with one ASI hazard token.
fn declaration_starts_with_asi_hazard(
    tree: &NodeTree,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let declaration = tree.get(declaration_id);
    if !matches!(
        declaration,
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    ) {
        return false;
    }

    !lambda_signature_can_avoid_parentheses(tree, declaration_id)
}

/// Return whether one expression starts with one ASI hazard token.
fn expression_starts_with_asi_hazard(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Statement(inner_expression_id) => {
            expression_starts_with_asi_hazard(tree, *inner_expression_id)
        }
        Expression::Declaration(declaration_id) => {
            declaration_starts_with_asi_hazard(tree, *declaration_id)
        }
        Expression::Parenthesized { .. }
        | Expression::ArrayExpression { .. }
        | Expression::TupleExpression { .. }
        | Expression::TemplateExpression { .. }
        | Expression::TreeExpression { .. } => true,
        Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }) => true,
        Expression::TaggedTemplateExpression { tag, .. } => {
            expression_starts_with_asi_hazard(tree, *tag)
        }
        Expression::SequenceExpression { expressions } => expressions
            .first()
            .is_some_and(|expression_id| expression_starts_with_asi_hazard(tree, *expression_id)),
        Expression::Unary { operator, .. } => matches!(
            operator,
            UnaryOperator::Plus | UnaryOperator::Negate | UnaryOperator::WrappingNegate
        ),
        Expression::TypeUnary { operator, right } => {
            matches!(
                operator,
                TypeUnaryOperator::AsComptime
                    | TypeUnaryOperator::AsConst
                    | TypeUnaryOperator::Must
            ) && expression_starts_with_asi_hazard(tree, *right)
        }
        Expression::TypeBinary { left, .. }
        | Expression::TypeConditional { left, .. }
        | Expression::TypeIndex { left, .. }
        | Expression::Binary { left, .. }
        | Expression::Assign { left, .. }
        | Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => expression_starts_with_asi_hazard(tree, *left),
        _ => false,
    }
}

/// Return whether one owner starts with one ASI hazard token.
pub(crate) fn owner_starts_with_asi_hazard(tree: &NodeTree, owner_id: u32) -> Option<bool> {
    match tree.get_node_type(owner_id) {
        NodeType::Expression => {
            let expression_id = LocalNodeId::<Expression>::new(owner_id);
            Some(expression_starts_with_asi_hazard(tree, expression_id))
        }
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(owner_id);
            Some(declaration_starts_with_asi_hazard(tree, declaration_id))
        }
        _ => None,
    }
}

/// Return whether one block expression needs a trailing statement terminator.
pub(crate) fn expression_needs_statement_terminator(
    context: &DestackFormatContext<'_>,
    expression: &Expression,
    is_expression_context_tail: bool,
) -> bool {
    if is_expression_context_tail {
        return false;
    }

    matches!(
        expression,
        Expression::Import { .. } | Expression::Let { .. } | Expression::Using { .. }
    ) || matches!(
        expression,
        Expression::While {
            kind: WhileKind::DoWhile,
            ..
        }
    ) || matches!(
        expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function {
                    descriptor,
                    signature,
                    ..
                }
                if descriptor.name.is_none() && signature.kind == FunctionKind::Lambda
            )
    ) || (!matches!(expression, Expression::Statement(_))
        && !matches!(expression, Expression::Stub | Expression::Error)
        && !expression.ends_statement_on_newline())
}

/// Return whether one statement wrapper should keep its trailing semicolon.
pub(crate) fn statement_wrapper_needs_semicolon(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);
    if let Expression::Statement(inner_expression_id) = expression {
        return statement_wrapper_needs_semicolon(context, *inner_expression_id);
    }

    if matches!(
        expression,
        Expression::Declaration(_) | Expression::Block(_)
    ) {
        return false;
    }

    if let Expression::Try {
        catch_expression,
        catch_pattern,
        finally_expression,
        ..
    } = expression
        && (catch_expression.is_some() || catch_pattern.is_some() || finally_expression.is_some())
    {
        return false;
    }

    if matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        } | Expression::While {
            kind: WhileKind::While,
            ..
        } | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Loop { .. }
            | Expression::Match { .. }
            | Expression::Labelled { .. }
    ) {
        return false;
    }

    if matches!(expression, Expression::Stub | Expression::Error) {
        return false;
    }

    true
}
