use super::super::declaration::expression_is_in_statement_position;
use crate::DestackFormatContext;
use destack_ast::{
    Declaration, Expression, FunctionKind, IfCondition, IfKind, LocalNodeId, NodeType,
};

/// Return whether one parent slot behaves like a type-relation left slot.
fn expression_is_type_relation_left_slot(
    parent_expression: &Expression,
    parent_slot_expression_id: LocalNodeId<Expression>,
) -> bool {
    match parent_expression {
        // template tag
        Expression::TaggedTemplateExpression { tag, .. } => *tag == parent_slot_expression_id,

        // unary-like rhs
        Expression::Unary { right, .. }
        | Expression::Delete { value: right }
        | Expression::Await { expression: right }
        | Expression::AwaitMaybe { expression: right } => *right == parent_slot_expression_id,

        // member or call lhs
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => *left == parent_slot_expression_id,

        // assignment lhs
        Expression::Assign { left, .. } => *left == parent_slot_expression_id,

        _ => false,
    }
}

/// Return whether one `as` or `satisfies` expression needs parentheses in its parent.
fn expression_as_or_satisfies_needs_parentheses_in_parent(
    parent_expression: &Expression,
    parent_slot_expression_id: LocalNodeId<Expression>,
) -> bool {
    match parent_expression {
        // ternary branches
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => true,

        // binary-like expressions
        Expression::Binary { .. } | Expression::Is { .. } | Expression::InstanceOf { .. } => true,

        _ => expression_is_type_relation_left_slot(parent_expression, parent_slot_expression_id),
    }
}

/// Return the effective expression parent and the outermost child in that parent slot.
fn effective_expression_parent_slot(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<(u32, NodeType, LocalNodeId<Expression>)> {
    let mut current_id = node_id;
    let mut parent_slot_expression_id = node_id;

    loop {
        let (parent_id, parent_type) = context.parent(current_id)?;

        if parent_type != NodeType::Expression {
            return Some((parent_id, parent_type, parent_slot_expression_id));
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);

        match context.tree.get(parent_expression_id) {
            // explicit parens stay transparent for parent slot lookup
            Expression::Parenthesized { expression } if *expression == current_id => {
                parent_slot_expression_id = parent_expression_id;
                current_id = parent_expression_id;
            }

            _ => return Some((parent_id, parent_type, parent_slot_expression_id)),
        }
    }
}

/// Return whether one expression is a statement-sensitive identifier.
fn expression_is_statement_sensitive_identifier(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Identifier { name } = context.tree.get(node_id) else {
        return false;
    };

    matches!(
        context.strings.get(*name),
        "await" | "interface" | "type" | "using" | "yield"
    )
}

/// Return whether one expression is the left chain of `as` or `satisfies` in statement position.
fn expression_is_type_relation_left_chain_in_statement_position(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;
    let mut saw_type_relation = matches!(
        context.tree.get(current_id),
        Expression::As { .. } | Expression::Satisfies { .. }
    );

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            return saw_type_relation && expression_is_in_statement_position(context, current_id);
        };

        // non-expression parents decide statement position
        if parent_type != NodeType::Expression {
            let parent_is_lambda_body = if parent_type == NodeType::Declaration {
                let parent_declaration_id = LocalNodeId::<Declaration>::new(parent_id);

                matches!(
                    context.tree.get(parent_declaration_id),
                    Declaration::Function(function)
                        if function.signature.kind == FunctionKind::Lambda
                            && function
                                .body
                                .is_some_and(|body_expression_id| body_expression_id == current_id)
                )
            } else {
                false
            };

            return saw_type_relation
                && expression_is_in_statement_position(context, current_id)
                && !parent_is_lambda_body;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        let parent_expression = context.tree.get(parent_expression_id);

        match parent_expression {
            // explicit parens do not break the chain
            Expression::Parenthesized { expression } if *expression == current_id => {
                current_id = parent_expression_id;
            }

            // adjacent type relations stay in the same chain
            Expression::As { expression, .. } | Expression::Satisfies { expression, .. }
                if *expression == current_id =>
            {
                saw_type_relation = true;
                current_id = parent_expression_id;
            }

            _ => return false,
        }
    }
}

/// Return whether one expression sits in a call-like callee or tag slot.
fn expression_is_call_like_parent_slot(
    context: &DestackFormatContext<'_>,
    parent_expression_id: LocalNodeId<Expression>,
    parent_slot_expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(parent_expression_id) {
        Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. } => *left == parent_slot_expression_id,
        Expression::TaggedTemplateExpression { tag, .. } => *tag == parent_slot_expression_id,
        _ => false,
    }
}

/// Return whether one declaration expression behaves like a class or lambda.
fn expression_is_class_or_function_declaration(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(node_id) else {
        return false;
    };

    match context.tree.get(*declaration_id) {
        Declaration::Class(_) => true,
        Declaration::Function(function) => function.signature.kind == FunctionKind::Lambda,
        _ => false,
    }
}

/// Return whether one expression needs derived parentheses in its parent.
pub(crate) fn expression_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    // statement-sensitive identifiers on the left of `as` and `satisfies` must stay parenthesized
    if !matches!(
        context.tree.get(node_id),
        Expression::Parenthesized { .. } | Expression::As { .. } | Expression::Satisfies { .. }
    ) && expression_is_statement_sensitive_identifier(context, node_id)
        && expression_is_type_relation_left_chain_in_statement_position(context, node_id)
    {
        return true;
    }

    let Some((parent_id, parent_type, parent_slot_expression_id)) =
        effective_expression_parent_slot(context, node_id)
    else {
        return false;
    };

    // statement position
    if parent_type != NodeType::Expression {
        return match context.tree.get(node_id) {
            Expression::Assign { .. } => true,
            Expression::ObjectExpression { .. } => {
                expression_is_in_statement_position(context, node_id)
            }
            Expression::Declaration(_)
                if expression_is_class_or_function_declaration(context, node_id) =>
            {
                expression_is_in_statement_position(context, node_id)
            }
            _ => false,
        };
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);

    // assignment expressions need a shell unless they are already in assignment position
    if let Expression::Assign { .. } = context.tree.get(node_id) {
        return match parent_expression {
            Expression::Assign { .. } => false,
            Expression::Index { left, .. } => *left == parent_slot_expression_id,
            _ => true,
        };
    }

    // class and function expressions need a shell when used as callee or tag
    if expression_is_class_or_function_declaration(context, node_id) {
        return expression_is_call_like_parent_slot(
            context,
            parent_expression_id,
            parent_slot_expression_id,
        ) || matches!(
            parent_expression,
            Expression::TaggedTemplateExpression { .. }
        );
    }

    // object expressions in ternary branches need a shell
    if matches!(
        context.tree.get(node_id),
        Expression::ObjectExpression { .. }
    ) {
        return matches!(
            parent_expression,
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        );
    }

    // ternary conditions need a shell when nested inside another ternary condition
    if matches!(
        context.tree.get(node_id),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    ) {
        return matches!(
            parent_expression,
            Expression::If {
                kind: IfKind::Ternary,
                condition,
                ..
            } if matches!(
                condition,
                IfCondition::Expression { condition } if *condition == parent_slot_expression_id
            )
        );
    }

    // `as` and `satisfies` need a shell in tighter or ambiguous parent positions
    if matches!(
        context.tree.get(node_id),
        Expression::As { .. } | Expression::Satisfies { .. }
    ) {
        return expression_as_or_satisfies_needs_parentheses_in_parent(
            parent_expression,
            parent_slot_expression_id,
        );
    }

    false
}

/// Return whether one expression appears inside a template interpolation.
pub(crate) fn expression_is_in_template_literal_interpolation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id.id;

    loop {
        let Some(parent_id) = context.parents.get_by_id(current_id) else {
            return false;
        };

        let is_template_parent = context.tree.get_node_type(parent_id) == NodeType::Expression
            && matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::TemplateExpression { .. }
            );

        if is_template_parent {
            return true;
        }

        current_id = parent_id;
    }
}
