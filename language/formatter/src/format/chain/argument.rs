use super::transparent_inner_expression;
use crate::DestackFormatContext;
use crate::format::expression::is_expression_breakable;
use crate::format::operator::expression_static_arguments;
use crate::format::tree::tree_literal_should_break;
use destack_ast::{
    Argument, Declaration, Expression, FunctionKind, LocalNodeId, NodeTree, NodeType, Property,
    ScalarLiteral, TypeBinaryOperator, UnaryOperator,
};

// static argument hugging thresholds
const HUG_STATIC_ARGUMENT_MAX_COUNT: usize = 3;

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

/// Check whether an argument is simple enough to stay inline in chains.
pub(crate) fn is_simple_chain_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.has_annotation(argument_id) {
        return false;
    }

    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };

    expression_is_simple_chain_argument(context, value_id, 2)
}

/// Check whether static arguments are simple enough for chain heads.
pub(crate) fn is_simple_chain_static_arguments(
    context: &DestackFormatContext<'_>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
) -> bool {
    match static_arguments {
        None => true,
        Some(arguments) => {
            arguments.len() <= 1
                && arguments
                    .iter()
                    .copied()
                    .all(|argument_id| is_simple_chain_argument(context, argument_id))
        }
    }
}

/// Decide whether static argument lists should expand at the list level.
pub(crate) fn should_expand_static_argument_list(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.len() != 1 {
        return false;
    }

    // keep single direct object-like type arguments hugged as `<{ ... }>`
    let Some(value_id) = argument_value_id_if_present(context.tree, static_arguments[0]) else {
        return true;
    };
    let value_id = transparent_inner_expression(context, value_id);
    if matches!(
        context.tree.get(value_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    ) {
        return false;
    }

    // expand list-level wrappers only when nested type arguments are structurally multiline
    expression_static_arguments(context.tree.get(value_id)).is_some_and(|nested_arguments| {
        nested_arguments.iter().copied().any(|nested_argument_id| {
            let Some(nested_value_id) =
                argument_value_id_if_present(context.tree, nested_argument_id)
            else {
                return true;
            };
            let nested_value_id = transparent_inner_expression(context, nested_value_id);
            static_argument_expression_has_complex_nested_structure(context, nested_value_id)
        })
    })
}

/// Return whether static arguments are structurally safe for hugged inline formatting.
pub(crate) fn static_argument_list_is_hug_safe(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.is_empty() || static_arguments.len() > HUG_STATIC_ARGUMENT_MAX_COUNT {
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
        if context.node_has_newline(argument_value_id) {
            return false;
        }

        if let Expression::Path { path, .. } = context.tree.get(argument_value_id)
            && path.segments.len() == 1
            && context.strings.get(path.segments[0]).chars().count() > 12
        {
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

/// Check whether a lambda expression body should break across lines.
pub(crate) fn lambda_expression_should_break(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let tree = context.tree;

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = tree.get(declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    let body_expression_id = transparent_inner_expression(context, *body_id);
    let body_expression = tree.get(body_expression_id);
    if matches!(body_expression, Expression::Block(_)) {
        return false;
    }

    if let Expression::TreeExpression {
        arguments,
        elements,
        ..
    } = body_expression
    {
        if tree_literal_should_break(context, arguments, elements) {
            return true;
        }

        // tree returning callbacks in tree literals should break for readability
        if let Some((parent_id, parent_type)) = context.parent(declaration_id)
            && parent_type == NodeType::Expression
        {
            let expression_id = LocalNodeId::<Expression>::new(parent_id);
            if expression_is_in_tree_literal_child(context, expression_id) {
                return true;
            }
        }
    }

    false
}

/// Check whether one expression is simple enough for chain-call heuristics.
fn expression_is_simple_chain_argument(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    depth: usize,
) -> bool {
    if depth == 0 {
        return false;
    }

    let expression_id = transparent_inner_expression(context, expression_id);
    if context.has_annotation(expression_id) {
        return false;
    }

    match context.tree.get(expression_id) {
        Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) => {
            context.strings.get(*content).chars().count() <= 5
        }
        Expression::ScalarLiteral(_) | Expression::This | Expression::Super => true,
        Expression::TemplateExpression { .. } => !context.node_has_newline(expression_id),
        Expression::Import { arguments, .. } => arguments.as_ref().is_none_or(|arguments| {
            arguments.len() <= depth
                && arguments
                    .iter()
                    .copied()
                    .all(|argument_id| simple_argument_is_simple(context, argument_id, depth - 1))
        }),
        Expression::Path {
            static_arguments, ..
        } => static_arguments.as_ref().is_none_or(|arguments| {
            arguments.is_empty()
                || (arguments.len() <= depth
                    && arguments.iter().copied().all(|argument_id| {
                        simple_argument_is_simple(context, argument_id, depth - 1)
                    }))
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
            expression_is_simple_chain_argument(context, *left, depth)
                && static_arguments.as_ref().is_none_or(|arguments| {
                    arguments.is_empty()
                        || (arguments.len() <= depth
                            && arguments.iter().copied().all(|argument_id| {
                                simple_argument_is_simple(context, argument_id, depth - 1)
                            }))
                })
        }
        Expression::Index { left, index, .. } => {
            expression_is_simple_chain_argument(context, *left, depth)
                && index.is_some_and(|index_id| {
                    expression_is_simple_chain_argument(context, index_id, depth)
                })
        }
        Expression::Call {
            left,
            static_arguments,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            expression_is_simple_chain_argument(context, *left, depth)
                && dynamic_arguments.len() <= depth
                && dynamic_arguments
                    .iter()
                    .copied()
                    .all(|argument_id| simple_argument_is_simple(context, argument_id, depth - 1))
                && static_arguments.as_ref().is_none_or(|arguments| {
                    arguments.is_empty()
                        || (arguments.len() <= depth
                            && arguments.iter().copied().all(|argument_id| {
                                simple_argument_is_simple(context, argument_id, depth - 1)
                            }))
                })
        }
        Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
            expression_is_simple_chain_argument(context, *left, depth)
        }
        Expression::Unary { operator, right } => {
            let operator_is_simple = matches!(
                operator,
                UnaryOperator::PreIncrement
                    | UnaryOperator::PreDecrement
                    | UnaryOperator::Not
                    | UnaryOperator::Plus
                    | UnaryOperator::Negate
                    | UnaryOperator::WrappingNegate
                    | UnaryOperator::ElementwiseNot
            ) || operator.is_postfix();

            operator_is_simple && expression_is_simple_chain_argument(context, *right, depth)
        }
        Expression::ObjectExpression { properties, .. } => {
            properties.iter().copied().all(|property_id| {
                depth > 0
                    && !context.has_annotation(property_id)
                    && match context.tree.get(property_id) {
                        Property::Field {
                            key,
                            value,
                            default,
                            ..
                        } => {
                            !matches!(
                                key,
                                Some(
                                    destack_ast::Key::Expression(_)
                                        | destack_ast::Key::NamedExpression { .. }
                                )
                            ) && default.is_none()
                                && value.is_none_or(|value_id| {
                                    expression_is_simple_chain_argument(
                                        context,
                                        value_id,
                                        depth - 1,
                                    )
                                })
                        }
                        Property::Method { .. } | Property::Spread { .. } | Property::Error => {
                            false
                        }
                    }
            })
        }
        Expression::ArrayExpression { elements } => elements
            .iter()
            .copied()
            .all(|argument_id| simple_argument_is_simple(context, argument_id, depth - 1)),
        Expression::TypeBinary {
            operator,
            left,
            right,
        } => {
            matches!(
                operator,
                TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
            ) && expression_is_simple_chain_argument(context, *left, depth)
                && !is_expression_breakable(context.tree, context.tree.get(*right))
        }
        _ => false,
    }
}

/// Check whether one child argument is simple.
fn simple_argument_is_simple(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    depth: usize,
) -> bool {
    if depth == 0 || context.has_annotation(argument_id) {
        return false;
    }

    match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. } => {
            expression_is_simple_chain_argument(context, *value, depth)
        }
        Argument::Spread { .. } | Argument::Error => false,
    }
}

/// Check whether an expression is nested inside a tree literal argument.
fn expression_is_in_tree_literal_child(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id.id;

    // walk up the parent chain looking for tree element arguments
    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        if parent_type == NodeType::Argument {
            let argument_id = LocalNodeId::<Argument>::new(parent_id);

            // tree literals store both attributes and children as arguments
            if let Some((grand_id, grand_type)) = context.parent_by_id(parent_id)
                && grand_type == NodeType::Expression
            {
                let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(grand_id));
                if let Expression::TreeExpression {
                    arguments,
                    elements,
                    ..
                } = parent_expression
                    && (arguments
                        .as_ref()
                        .is_some_and(|arguments| arguments.contains(&argument_id))
                        || elements
                            .as_ref()
                            .is_some_and(|elements| elements.contains(&argument_id)))
                {
                    return true;
                }
            }
        }

        current_id = parent_id;
    }

    false
}

/// Return whether one static argument expression has nested structure that should force expansion.
fn static_argument_expression_has_complex_nested_structure(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ObjectExpression { properties, .. } => properties.len() > 1,
        Expression::TypeMapped { .. } => true,
        _ => expression_static_arguments(context.tree.get(expression_id)).is_some_and(
            |nested_arguments| {
                nested_arguments.iter().copied().any(|nested_argument_id| {
                    let Some(nested_value_id) =
                        argument_value_id_if_present(context.tree, nested_argument_id)
                    else {
                        return true;
                    };
                    let nested_value_id = transparent_inner_expression(context, nested_value_id);
                    static_argument_expression_has_complex_nested_structure(
                        context,
                        nested_value_id,
                    )
                })
            },
        ),
    }
}

/// Return whether one static argument expression is structurally safe for hugged inline formatting.
fn static_argument_expression_is_hug_safe(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Path {
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
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
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
