use crate::DestackFormatContext;
use crate::declaration::expression_is_in_statement_context;
use crate::operator::{binary_operator_format_precedence, should_flatten_binary};
use destack_dir::{
    Argument, AssignPattern, BinaryOperator, Declaration, Expression, FunctionForm, IfCondition,
    IfForm, LocalNodeId, MatchCase, MatchForm, NodeType, OperatorPrecedence, Property,
    TypeExpression,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// Return whether one expression is a class extends expression.
fn is_class_extends(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Class(class) = context.tree.get(declaration_id) else {
        return false;
    };

    class
        .extends_expression
        .is_some_and(|expression_id| expression_id == parent_child_id)
}

/// Return whether one class extends expression needs parentheses.
fn class_extends_expression_needs_parentheses(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ObjectExpression { .. }
            | Expression::StructExpression { .. }
            | Expression::New { .. }
            | Expression::NewMaybe { .. }
            | Expression::Unary { .. }
            | Expression::Await { .. }
            | Expression::AwaitMaybe { .. }
            | Expression::AwaitMust { .. }
            | Expression::RangeExpression { .. }
            | Expression::Binary { .. }
            | Expression::Is { .. }
            | Expression::InstanceOf { .. }
            | Expression::If {
                form: IfForm::Ternary,
                ..
            }
            | Expression::As { .. }
            | Expression::Satisfies { .. }
            | Expression::Must { .. }
    )
}

/// Return whether one expression is a match case expression body.
fn expression_is_match_case_body(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::MatchCase {
        return false;
    }

    let case_id = LocalNodeId::<MatchCase>::new(parent_id);
    matches!(context.tree.get(case_id), MatchCase::Expression { body, .. } if *body == parent_child_id)
}

/// Return whether one expression is the body of a switch case.
fn expression_is_switch_case_body(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::MatchCase {
        return false;
    }

    let case_id = LocalNodeId::<MatchCase>::new(parent_id);
    let case_body_matches = match context.tree.get(case_id) {
        MatchCase::Expression { body, .. } => *body == parent_child_id,
        MatchCase::Block { .. } => false,
    };
    if !case_body_matches {
        return false;
    }

    let Some((match_id, NodeType::Expression)) = context.parent(case_id) else {
        return false;
    };
    matches!(
        context.tree.get(LocalNodeId::<Expression>::new(match_id)),
        Expression::Match {
            form: MatchForm::Switch,
            ..
        }
    )
}

/// Return whether one expression is a spread value.
fn expression_is_spread_value(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    match parent_type {
        NodeType::Argument => {
            let argument_id = LocalNodeId::<Argument>::new(parent_id);
            matches!(context.tree.get(argument_id), Argument::Spread { value, .. } if *value == parent_child_id)
        }
        NodeType::Property => {
            let property_id = LocalNodeId::<Property>::new(parent_id);
            matches!(context.tree.get(property_id), Property::Spread { value } if *value == parent_child_id)
        }
        _ => false,
    }
}

/// Return whether one expression has statement-like value syntax.
fn expression_is_statement_like_value(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::If {
            form: IfForm::If,
            ..
        } | Expression::Match { .. }
            | Expression::Try { .. }
    )
}

/// Return whether one parent requires type-cast-like parentheses for a child.
fn type_cast_like_needs_parentheses(
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    match parent_expression {
        // template tag
        Expression::TaggedTemplateExpression { tag, .. } => *tag == parent_child_id,

        // unary-like rhs
        Expression::Unary { right, .. }
        | Expression::Await { expression: right }
        | Expression::AwaitMaybe { expression: right }
        | Expression::AwaitMust { expression: right }
        | Expression::Must { left: right, .. } => *right == parent_child_id,

        // member or call lhs
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. } => *left == parent_child_id,

        _ => false,
    }
}

/// Return the effective expression parent and outermost child in that parent.
fn effective_expression_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<(u32, NodeType, LocalNodeId<Expression>)> {
    let mut current_id = node_id;
    let mut parent_child_id = node_id;

    loop {
        let (parent_id, parent_type) = context.parent(current_id)?;

        if parent_type != NodeType::Expression {
            return Some((parent_id, parent_type, parent_child_id));
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);

        match context.tree.get(parent_expression_id) {
            // explicit parens stay transparent for parent lookup
            Expression::Parenthesized { expression } if *expression == current_id => {
                parent_child_id = parent_expression_id;
                current_id = parent_expression_id;
            }

            _ => return Some((parent_id, parent_type, parent_child_id)),
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
        "await"
            | "component"
            | "hook"
            | "interface"
            | "let"
            | "module"
            | "type"
            | "using"
            | "yield"
    )
}

/// Return whether one statement-context expression is the left chain of `as` or `satisfies`.
fn expression_is_type_relation_left_chain_in_statement_context(
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
            return saw_type_relation && expression_is_in_statement_context(context, current_id);
        };

        // non-expression parents decide statement context
        if parent_type != NodeType::Expression {
            let parent_is_lambda_body = if parent_type == NodeType::Declaration {
                let parent_declaration_id = LocalNodeId::<Declaration>::new(parent_id);

                matches!(
                    context.tree.get(parent_declaration_id),
                    Declaration::Function(function)
                        if function.signature.form == FunctionForm::Lambda
                            && function
                                .body
                                .is_some_and(|body_expression_id| body_expression_id == current_id)
                )
            } else {
                false
            };

            return saw_type_relation
                && expression_is_in_statement_context(context, current_id)
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

/// Return whether one expression sits in a call-like callee or tag.
fn expression_is_call_like_callee(
    context: &DestackFormatContext<'_>,
    parent_expression_id: LocalNodeId<Expression>,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(parent_expression_id) {
        Expression::Call { left, .. } | Expression::Instantiation { left, .. } => {
            *left == parent_child_id
        }
        Expression::TaggedTemplateExpression { tag, .. } => *tag == parent_child_id,
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

    matches!(
        context.tree.get(*declaration_id),
        Declaration::Class(_) | Declaration::Function(_)
    )
}

/// Return whether one expression is a lambda declaration.
fn expression_is_lambda_declaration(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(node_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        Declaration::Function(function) if function.signature.form == FunctionForm::Lambda
    )
}

/// Return whether one assignment expression needs parentheses in statement context.
fn expression_assignment_needs_parentheses_in_statement_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<AssignPattern>,
) -> bool {
    if expression_is_lambda_body_position(context, node_id) {
        return true;
    }

    matches!(context.tree.get(left), AssignPattern::Object { .. })
}

/// Return whether one named class or function declaration is in declaration statement context.
fn expression_is_named_declaration_statement(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if !expression_is_in_statement_context(context, node_id) {
        return false;
    }

    let Expression::Declaration(declaration_id) = context.tree.get(node_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        Declaration::Class(class) if class.name.is_some()
    ) || matches!(
        context.tree.get(*declaration_id),
        Declaration::Function(function)
            if function.name.is_some() && function.signature.form == FunctionForm::Function
    )
}

/// Return whether one expression is the body of one lambda declaration.
fn expression_is_lambda_body_position(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Function(function) = context.tree.get(declaration_id) else {
        return false;
    };

    function.signature.form == FunctionForm::Lambda
        && function
            .body
            .is_some_and(|body_expression_id| body_expression_id == node_id)
}

/// Return whether one decorated class expression is used as class extends.
fn decorated_class_extends_needs_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parent_id: u32,
    parent_type: NodeType,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Declaration {
        return false;
    }

    let Expression::Declaration(class_declaration_id) = context.tree.get(node_id) else {
        return false;
    };

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Class(_) = context.tree.get(declaration_id) else {
        return false;
    };

    let declaration_has_decorators = !context.annotation_ids(*class_declaration_id).is_empty();
    if !declaration_has_decorators {
        return false;
    }

    is_class_extends(context, parent_id, parent_type, parent_child_id)
}

/// Return whether one lambda expression needs parentheses in its parent.
fn expression_lambda_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    parent_expression_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(
        parent_expression,
        Expression::As { .. }
            | Expression::Satisfies { .. }
            | Expression::Unary { .. }
            | Expression::Await { .. }
            | Expression::AwaitMaybe { .. }
            | Expression::AwaitMust { .. }
            | Expression::RangeExpression { .. }
            | Expression::Binary { .. }
            | Expression::Is { .. }
            | Expression::InstanceOf { .. }
    ) {
        return true;
    }

    if matches!(
        parent_expression,
        Expression::If {
            form: IfForm::Ternary,
            condition,
            ..
        } if matches!(
            condition,
            IfCondition::Expression { condition } if *condition == parent_child_id
        )
    ) {
        return true;
    }

    type_cast_like_needs_parentheses(parent_expression, parent_child_id)
        || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
}

/// Return whether one assertion expression targets a composite type.
fn assertion_expression_has_composite_target(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let target_type = match context.tree.get(expression_id) {
        Expression::As { target_type, .. } | Expression::Satisfies { target_type, .. } => {
            *target_type
        }
        _ => return false,
    };

    matches!(
        context.tree.get(target_type),
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements }
            if elements.len() > 1
    )
}

/// Return whether one `as` or `satisfies` expression needs parentheses in its parent.
fn expression_as_or_satisfies_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Expression {
        return is_class_extends(context, parent_id, parent_type, parent_child_id);
    }

    match parent_expression {
        // chained assertions
        Expression::As { expression, .. } | Expression::Satisfies { expression, .. }
            if *expression == parent_child_id =>
        {
            assertion_expression_has_composite_target(context, parent_child_id)
        }

        // ternary branches
        Expression::If {
            form: IfForm::Ternary,
            ..
        } => true,

        // binary-like expressions
        Expression::RangeExpression { .. }
        | Expression::Binary { .. }
        | Expression::Is { .. }
        | Expression::InstanceOf { .. } => true,

        // default
        _ => type_cast_like_needs_parentheses(parent_expression, parent_child_id),
    }
}

/// Return whether one await-like expression needs parentheses in its parent.
fn expression_await_like_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Expression {
        return is_class_extends(context, parent_id, parent_type, parent_child_id);
    }

    if matches!(
        parent_expression,
        Expression::Unary { .. }
            | Expression::As { .. }
            | Expression::Satisfies { .. }
            | Expression::RangeExpression { .. }
            | Expression::Binary { .. }
            | Expression::Is { .. }
            | Expression::InstanceOf { .. }
    ) {
        return true;
    }

    if matches!(
        parent_expression,
        Expression::If {
            form: IfForm::Ternary,
            condition,
            ..
        } if matches!(
            condition,
            IfCondition::Expression { condition } if *condition == parent_child_id
        )
    ) {
        return true;
    }

    type_cast_like_needs_parentheses(parent_expression, parent_child_id)
}

/// Return whether one binary-like expression needs parentheses in its parent.
fn expression_binary_like_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parent_id: u32,
    parent_type: NodeType,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Expression {
        return is_class_extends(context, parent_id, parent_type, parent_child_id);
    }

    // assertion parents
    if matches!(
        parent_expression,
        Expression::As { .. } | Expression::Satisfies { .. }
    ) {
        return true;
    }

    let Expression::Binary { operator, .. } = context.tree.get(node_id) else {
        return type_cast_like_needs_parentheses(parent_expression, parent_child_id);
    };

    // coalesce needs grouping inside conditionals
    if *operator == BinaryOperator::Coalesce
        && matches!(
            parent_expression,
            Expression::If {
                form: IfForm::Ternary,
                ..
            }
        )
    {
        return true;
    }

    if let Expression::Binary {
        operator: parent_operator,
        right,
        ..
    } = parent_expression
    {
        if binary_operator_is_logical(*parent_operator) && binary_operator_is_logical(*operator) {
            return parent_operator != operator;
        }

        let parent_precedence = binary_operator_format_precedence(*parent_operator);
        let precedence = binary_operator_format_precedence(*operator);

        if parent_precedence > precedence {
            return true;
        }

        let is_right = *right == node_id;
        if is_right && parent_precedence == precedence {
            return true;
        }

        if binary_operator_is_bitwise_or_shift(*parent_operator) {
            return true;
        }

        if parent_precedence < precedence
            && *operator == BinaryOperator::Remainder
            && parent_operator.precedence_group() == OperatorPrecedence::Addition
        {
            return true;
        }

        return parent_precedence == precedence
            && !should_flatten_binary(*parent_operator, *operator);
    }

    type_cast_like_needs_parentheses(parent_expression, parent_child_id)
}

/// Return whether one binary operator groups like bitwise or shift.
fn binary_operator_is_bitwise_or_shift(operator: BinaryOperator) -> bool {
    matches!(
        operator.precedence_group(),
        OperatorPrecedence::BitwiseOr
            | OperatorPrecedence::BitwiseXor
            | OperatorPrecedence::BitwiseAnd
            | OperatorPrecedence::Shift
    )
}

/// Return whether one binary operator maps to the logical expression family.
fn binary_operator_is_logical(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    )
}

/// Return whether one expression is a binary-like expression.
fn expression_is_binary_like(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Binary { .. } | Expression::Is { .. } | Expression::InstanceOf { .. }
    )
}

/// Return whether one range expression needs parentheses in its parent.
fn expression_range_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    parent_expression_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if type_cast_like_needs_parentheses(parent_expression, parent_child_id)
        || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
    {
        return true;
    }

    matches!(
        parent_expression,
        Expression::RangeExpression { .. }
            | Expression::Binary { .. }
            | Expression::Is { .. }
            | Expression::InstanceOf { .. }
            | Expression::As { .. }
            | Expression::Satisfies { .. }
            | Expression::If {
                form: IfForm::Ternary,
                ..
            }
    )
}

/// Return whether one expression has lower precedence than update, member, or call positions.
fn expression_is_update_or_lower_precedence(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(node_id),
        Expression::Declaration(_)
            | Expression::ObjectExpression { .. }
            | Expression::StructExpression { .. }
            | Expression::Unary { .. }
            | Expression::Await { .. }
            | Expression::AwaitMaybe { .. }
            | Expression::AwaitMust { .. }
            | Expression::Comptime { .. }
            | Expression::RangeExpression { .. }
            | Expression::Binary { .. }
            | Expression::Is { .. }
            | Expression::InstanceOf { .. }
            | Expression::If {
                form: IfForm::Ternary,
                ..
            }
            | Expression::Match { .. }
            | Expression::Try { .. }
            | Expression::As { .. }
            | Expression::Satisfies { .. }
            | Expression::Assign { .. }
            | Expression::SequenceExpression { .. }
            | Expression::Yield { .. }
    )
}

/// Return whether one statement-like value needs parentheses in a tighter parent.
fn statement_like_value_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Expression {
        return expression_is_spread_value(context, parent_id, parent_type, parent_child_id);
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);

    type_cast_like_needs_parentheses(parent_expression, parent_child_id)
        || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
        || matches!(
            parent_expression,
            Expression::RangeExpression { .. }
                | Expression::Binary { .. }
                | Expression::Is { .. }
                | Expression::InstanceOf { .. }
        )
}

/// Return whether one explicit wrapper is required by a postfix parent.
fn parenthesized_wrapper_required_by_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if !expression_is_update_or_lower_precedence(context, expression_id) {
        return false;
    }

    let Some((parent_id, parent_type, parent_child_id)) =
        effective_expression_parent(context, node_id)
    else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return expression_is_statement_like_value(context.tree.get(expression_id))
            && expression_is_spread_value(context, parent_id, parent_type, parent_child_id);
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);

    type_cast_like_needs_parentheses(parent_expression, parent_child_id)
        || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
}

/// Return whether one explicit wrapper is required by a postfix parent.
fn parenthesized_postfix_wrapper_required_by_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type, parent_child_id)) =
        effective_expression_parent(context, node_id)
    else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);

    postfix_wrapper_is_semantic_in_parent(
        context.tree.get(expression_id),
        parent_expression,
        parent_child_id,
    )
}

/// Return whether one node came from a skipped transparent wrapper.
fn expression_has_transparent_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .tree
        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Wrapper))
        .is_some()
}

/// Return whether one expression is a callable selection.
fn expression_is_callable_selection(child_expression: &Expression) -> bool {
    match child_expression {
        Expression::Member { .. } | Expression::PrivateMember { .. } | Expression::Index { .. } => {
            true
        }
        Expression::QualifiedReference { path, .. } => path.segments.len() > 1,
        _ => false,
    }
}

/// Return whether one wrapper changes postfix parsing.
fn postfix_wrapper_is_semantic_in_parent(
    child_expression: &Expression,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    match parent_expression {
        Expression::Call {
            left,
            generic_arguments,
            ..
        } if *left == parent_child_id => {
            expression_is_callable_selection(child_expression)
                || (!generic_arguments.is_empty()
                    && matches!(child_expression, Expression::Instantiation { .. }))
        }
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
            if *left == parent_child_id =>
        {
            matches!(child_expression, Expression::Instantiation { .. })
        }
        _ => false,
    }
}

/// Return whether one skipped transparent wrapper must be restored in its parent.
pub(crate) fn transparent_wrapper_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if !expression_has_transparent_wrapper(context, node_id) {
        return false;
    }

    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));

    postfix_wrapper_is_semantic_in_parent(context.tree.get(node_id), parent_expression, node_id)
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
        && expression_is_type_relation_left_chain_in_statement_context(context, node_id)
    {
        return true;
    }

    // object assignment targets need statement start disambiguation
    if matches!(
        context.tree.get(node_id),
        Expression::Assign { left, .. }
            if matches!(context.tree.get(*left), AssignPattern::Object { .. })
    ) && expression_is_in_statement_context(context, node_id)
    {
        return true;
    }

    let Some((parent_id, parent_type, parent_child_id)) =
        effective_expression_parent(context, node_id)
    else {
        return false;
    };

    // statement context
    if parent_type != NodeType::Expression {
        let is_class_extends = is_class_extends(context, parent_id, parent_type, parent_child_id);
        let is_match_case_body =
            expression_is_match_case_body(context, parent_id, parent_type, parent_child_id);
        let is_switch_case_body =
            expression_is_switch_case_body(context, parent_id, parent_type, parent_child_id);
        if is_class_extends && class_extends_expression_needs_parentheses(context.tree.get(node_id))
        {
            return true;
        }
        if expression_is_statement_like_value(context.tree.get(node_id))
            && expression_is_spread_value(context, parent_id, parent_type, parent_child_id)
        {
            return true;
        }

        return match context.tree.get(node_id) {
            Expression::As { .. } | Expression::Satisfies { .. } => {
                parent_type == NodeType::AssignPattern || is_class_extends
            }
            Expression::Assign { left, .. } => {
                if is_switch_case_body {
                    return false;
                }

                if expression_is_in_statement_context(context, node_id) {
                    return expression_assignment_needs_parentheses_in_statement_context(
                        context, node_id, *left,
                    );
                }

                true
            }
            Expression::ObjectExpression { .. } => {
                is_match_case_body
                    || expression_is_in_statement_context(context, node_id)
                    || expression_is_type_relation_left_chain_in_statement_context(context, node_id)
                    || expression_is_lambda_body_position(context, node_id)
            }
            Expression::Declaration(_)
                if expression_is_class_or_function_declaration(context, node_id) =>
            {
                if expression_is_named_declaration_statement(context, node_id) {
                    return false;
                }

                expression_is_in_statement_context(context, node_id)
                    || decorated_class_extends_needs_parentheses(
                        context,
                        node_id,
                        parent_id,
                        parent_type,
                        parent_child_id,
                    )
            }
            _ => false,
        };
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);

    // skipped transparent wrappers are restored when they carry parse meaning
    if transparent_wrapper_needs_parentheses_in_parent(context, node_id) {
        return true;
    }

    // assignment expressions need parentheses unless they are already in assignment position
    if let Expression::Assign { .. } = context.tree.get(node_id) {
        return match parent_expression {
            Expression::Assign { .. } => false,
            Expression::Parenthesized { .. } => false,
            Expression::Index { .. } => false,
            Expression::For {
                initialization,
                increment,
                ..
            } => {
                !initialization.is_some_and(|child_id| child_id.id == node_id.id)
                    && !increment.is_some_and(|child_id| child_id.id == node_id.id)
            }
            _ => true,
        };
    }

    // class and function expressions need parentheses when used as callee or tag
    if expression_is_class_or_function_declaration(context, node_id) {
        if expression_is_lambda_declaration(context, node_id) {
            return expression_lambda_needs_parentheses_in_parent(
                context,
                parent_expression_id,
                parent_expression,
                parent_child_id,
            );
        }

        return type_cast_like_needs_parentheses(parent_expression, parent_child_id)
            || expression_is_call_like_callee(context, parent_expression_id, parent_child_id);
    }

    // object expressions need parentheses in ambiguous statement contexts
    if matches!(
        context.tree.get(node_id),
        Expression::ObjectExpression { .. }
    ) {
        return expression_is_type_relation_left_chain_in_statement_context(context, node_id);
    }

    // ternary conditions need parentheses when nested inside another ternary condition
    if matches!(
        context.tree.get(node_id),
        Expression::If {
            form: IfForm::Ternary,
            ..
        }
    ) {
        return matches!(
            parent_expression,
            Expression::As { .. } | Expression::Satisfies { .. }
        ) || type_cast_like_needs_parentheses(parent_expression, parent_child_id)
            || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
            || matches!(
                parent_expression,
                Expression::If {
                    form: IfForm::Ternary,
                    condition,
                    ..
                } if matches!(
                    condition,
                    IfCondition::Expression { condition } if *condition == parent_child_id
                )
            );
    }

    // statement-like values need parentheses in tighter parents
    if expression_is_statement_like_value(context.tree.get(node_id)) {
        return statement_like_value_needs_parentheses_in_parent(
            context,
            parent_id,
            parent_type,
            parent_child_id,
        );
    }

    // `as` and `satisfies` need parentheses in tighter or ambiguous parent positions
    if matches!(
        context.tree.get(node_id),
        Expression::As { .. } | Expression::Satisfies { .. }
    ) {
        return expression_as_or_satisfies_needs_parentheses_in_parent(
            context,
            parent_id,
            parent_type,
            parent_expression,
            parent_child_id,
        );
    }

    // await-like expressions need parentheses in lower-precedence or type-relation parents
    if matches!(
        context.tree.get(node_id),
        Expression::Await { .. } | Expression::AwaitMaybe { .. } | Expression::AwaitMust { .. }
    ) {
        return expression_await_like_needs_parentheses_in_parent(
            context,
            parent_id,
            parent_type,
            parent_expression,
            parent_child_id,
        );
    }

    // range expressions need parentheses in tighter expression positions
    if matches!(
        context.tree.get(node_id),
        Expression::RangeExpression { .. }
    ) {
        return expression_range_needs_parentheses_in_parent(
            context,
            parent_expression_id,
            parent_expression,
            parent_child_id,
        );
    }

    // binary-like expressions need parentheses in tighter expression positions
    let expression = context.tree.get(node_id);
    if expression_is_binary_like(expression)
        && expression_binary_like_needs_parentheses_in_parent(
            context,
            node_id,
            parent_id,
            parent_type,
            parent_expression,
            parent_child_id,
        )
    {
        return true;
    }

    false
}

/// Return whether one explicit parenthesized wrapper must stay visible.
pub(crate) fn parenthesized_expression_needs_preserved_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    // object expressions need statement start disambiguation
    if expression_is_in_statement_context(context, node_id)
        && matches!(
            context.tree.get(expression_id),
            Expression::ObjectExpression { .. }
        )
    {
        return true;
    }

    // statement context owns its parentheses directly
    if expression_is_in_statement_context(context, node_id) {
        return false;
    }

    // postfix wrappers can carry parse meaning
    if parenthesized_postfix_wrapper_required_by_parent(context, node_id, expression_id) {
        return true;
    }

    // inner expressions that already need parentheses must not gain another pair
    if expression_needs_parentheses_in_parent(context, expression_id) {
        return false;
    }

    // postfix parents require the explicit wrapper around lower precedence children
    if parenthesized_wrapper_required_by_parent(context, node_id, expression_id) {
        return true;
    }

    // class and function wrappers in postfix-like parents should defer to the inner expression
    if expression_is_class_or_function_declaration(context, expression_id) {
        let Some((parent_id, parent_type, parent_child_id)) =
            effective_expression_parent(context, node_id)
        else {
            return false;
        };

        if decorated_class_extends_needs_parentheses(
            context,
            expression_id,
            parent_id,
            parent_type,
            parent_child_id,
        ) {
            return true;
        }

        if parent_type == NodeType::Expression {
            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            let parent_expression = context.tree.get(parent_expression_id);

            if type_cast_like_needs_parentheses(parent_expression, parent_child_id)
                || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
            {
                return false;
            }
        }
    }

    // wrapper span
    let outer_span = context.span(node_id);
    let outer_token_start = context.node_token_start(node_id);
    let inner_token_start = context.node_token_start(expression_id);
    let wrapper_start = Span::new(outer_span.file, outer_token_start, outer_token_start);

    // comment scan start
    let comment_scan_start = context
        .previous_non_trivia_token_before_span(wrapper_start)
        .map_or(outer_span.start, |token| token.span.end);

    // comments before `(` stay attached to the wrapper
    let has_leading_wrapper_comments = context
        .comment_tokens_in_range(comment_scan_start, inner_token_start)
        .iter()
        .any(|comment| comment.span.start < outer_token_start);
    if has_leading_wrapper_comments {
        return true;
    }

    // multiline wrappers stay visible
    context.has_newline(outer_span)
}
