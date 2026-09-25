use super::shape::expression_is_lambda_declaration;
use super::ternary::{expression_is_ternary_branch, ternary_branch_is_tree_like};
use crate::TsppFormatContext;
use crate::declaration::expression_is_in_statement_context;
use tspp_dir::{
    Argument, AssignPattern, BinaryOperator, Declaration, Expression, FunctionForm, IfForm,
    LocalNodeId, MatchArm, NodeType, OperatorPrecedence, Property, TypeExpression,
};
use tspp_source::{NodeSpanRegion, NodeSpanType, Span};

/// Return whether one expression is a match arm expression body.
fn expression_is_match_arm_body(
    context: &TsppFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::MatchArm {
        return false;
    }

    let arm_id = LocalNodeId::<MatchArm>::new(parent_id);
    matches!(context.tree.get(arm_id), MatchArm::Expression { body, .. } if *body == parent_child_id)
}

/// Return whether one expression is a spread value.
fn expression_is_spread_value(
    context: &TsppFormatContext<'_>,
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

/// Return whether one parent requires a primary expression in the child's position.
fn parent_requires_primary_expression(
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    match parent_expression {
        // template tag
        Expression::TaggedTemplateExpression { tag, .. } => *tag == parent_child_id,

        // prefix operand
        Expression::Unary { right, .. }
        | Expression::BorrowOf { right, .. }
        | Expression::Await { expression: right }
        | Expression::AwaitMaybe { expression: right }
        | Expression::AwaitMust { expression: right }
        | Expression::Must { left: right, .. } => *right == parent_child_id,

        // member or call lhs
        Expression::Member { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. } => *left == parent_child_id,

        _ => false,
    }
}

/// Return whether one expression is a statement-sensitive identifier.
fn expression_is_statement_sensitive_identifier(
    context: &TsppFormatContext<'_>,
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

/// Return whether one compound condition requires parentheses around an expression operand.
fn expression_condition_operand_needs_parentheses(
    context: &TsppFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    operand: LocalNodeId<Expression>,
) -> bool {
    let condition = match parent_type {
        NodeType::Expression => {
            let parent = LocalNodeId::<Expression>::new(parent_id);
            match context.tree.get(parent) {
                Expression::If { condition, .. } | Expression::While { condition, .. } => condition,
                _ => return false,
            }
        }
        NodeType::MatchArm => {
            let parent = LocalNodeId::<MatchArm>::new(parent_id);
            let Some(condition) = context.tree.get(parent).guard() else {
                return false;
            };

            condition
        }
        _ => return false,
    };

    condition.operands.len() > 1
        && condition
            .expressions()
            .any(|expression| expression == operand)
        && context.tree.get(operand).precedence() < OperatorPrecedence::LogicalAnd
}

/// Return whether one statement-context expression is the left chain of `as` or `satisfies`.
fn expression_is_type_relation_left_chain_in_statement_context(
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
    parent_expression_id: LocalNodeId<Expression>,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(parent_expression_id) {
        Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. } => *left == parent_child_id,
        Expression::TaggedTemplateExpression { tag, .. } => *tag == parent_child_id,
        _ => false,
    }
}

/// Return whether one declaration expression behaves like a class or lambda.
fn expression_is_class_or_function_declaration(
    context: &TsppFormatContext<'_>,
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

/// Return whether one assignment expression needs parentheses in statement context.
fn expression_assignment_needs_parentheses_in_statement_context(
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
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

/// Return whether one lambda expression needs parentheses in its parent.
fn expression_lambda_needs_parentheses_in_parent(
    context: &TsppFormatContext<'_>,
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
        } if condition.as_expression() == Some(parent_child_id)
    ) {
        return true;
    }

    parent_requires_primary_expression(parent_expression, parent_child_id)
        || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
}

/// Return whether one assertion expression targets a composite type.
fn assertion_expression_has_composite_target(
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
    _parent_id: u32,
    parent_type: NodeType,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Expression {
        return false;
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
        _ => parent_requires_primary_expression(parent_expression, parent_child_id),
    }
}

/// Return whether one await-like expression needs parentheses in its parent.
fn expression_await_like_needs_parentheses_in_parent(
    _context: &TsppFormatContext<'_>,
    _parent_id: u32,
    parent_type: NodeType,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Expression {
        return false;
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
        } if condition.as_expression() == Some(parent_child_id)
    ) {
        return true;
    }

    parent_requires_primary_expression(parent_expression, parent_child_id)
}

/// Return whether one binary-like expression needs parentheses in its parent.
fn expression_binary_like_needs_parentheses_in_parent(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    _parent_id: u32,
    parent_type: NodeType,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Expression {
        return false;
    }

    // assertion parents
    if matches!(
        parent_expression,
        Expression::As { .. } | Expression::Satisfies { .. }
    ) {
        return true;
    }

    let Expression::Binary { operator, .. } = context.tree.get(node_id) else {
        return parent_requires_primary_expression(parent_expression, parent_child_id);
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

        let parent_precedence = parent_operator.precedence();
        let precedence = operator.precedence();

        if parent_precedence > precedence {
            return true;
        }

        let is_right = *right == node_id;
        if parent_precedence == precedence {
            if parent_precedence.is_right_associative() {
                return !is_right;
            }

            return is_right;
        }

        if binary_operator_is_bitwise_or_shift(*parent_operator) {
            return true;
        }

        if parent_precedence < precedence
            && *operator == BinaryOperator::Remainder
            && parent_operator.precedence() == OperatorPrecedence::Addition
        {
            return true;
        }

        return false;
    }

    parent_requires_primary_expression(parent_expression, parent_child_id)
}

/// Return whether one binary operator groups like bitwise or shift.
fn binary_operator_is_bitwise_or_shift(operator: BinaryOperator) -> bool {
    matches!(
        operator.precedence(),
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
    context: &TsppFormatContext<'_>,
    parent_expression_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_requires_primary_expression(parent_expression, parent_child_id)
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

/// Return whether one statement-like value needs parentheses in a tighter parent.
fn statement_like_value_needs_parentheses_in_parent(
    context: &TsppFormatContext<'_>,
    parent_id: u32,
    parent_type: NodeType,
    parent_child_id: LocalNodeId<Expression>,
) -> bool {
    if parent_type != NodeType::Expression {
        return expression_is_spread_value(context, parent_id, parent_type, parent_child_id);
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);

    parent_requires_primary_expression(parent_expression, parent_child_id)
        || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
        || matches!(
            parent_expression,
            Expression::RangeExpression { .. }
                | Expression::Binary { .. }
                | Expression::Is { .. }
                | Expression::InstanceOf { .. }
        )
}

/// Return the elided source parentheses around one expression.
pub(crate) fn source_parentheses_span(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<Span> {
    context
        .tree
        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Parentheses))
}

/// Return whether one expression is a callable selection.
fn expression_is_callable_selection(child_expression: &Expression) -> bool {
    matches!(
        child_expression,
        Expression::Member { .. } | Expression::Index { .. }
    )
}

/// Return whether parentheses change postfix parsing in one parent.
fn is_postfix_parent_changed_by_parentheses(
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
        Expression::Member { left, .. } | Expression::Index { left, .. }
            if *left == parent_child_id =>
        {
            matches!(child_expression, Expression::Instantiation { .. })
        }
        _ => false,
    }
}

/// Return whether elided source parentheses must be preserved.
pub(crate) fn should_preserve_source_parentheses(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(parentheses_span) = source_parentheses_span(context, node_id) else {
        return false;
    };

    // tree ternaries own branch grouping and comments
    if ternary_branch_is_tree_like(context, node_id)
        && expression_is_ternary_branch(context, node_id)
    {
        return false;
    }

    // preserve comments owned by the parentheses interior
    let expression_end = context.span(node_id).end;
    let interior_comments = context
        .comments()
        .comments_in_range(expression_end, parentheses_span.end);
    if !interior_comments.is_empty() {
        return true;
    }

    // preserve parentheses that change postfix parsing
    let Some(parent_id) = context.expression_parent(node_id) else {
        return false;
    };

    let parent_expression = context.tree.get(parent_id);

    is_postfix_parent_changed_by_parentheses(context.tree.get(node_id), parent_expression, node_id)
}

/// Return whether one expression needs derived parentheses in its parent.
pub(crate) fn expression_needs_parentheses_in_parent(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    // preserve source parentheses that own comments or affect postfix parsing
    if should_preserve_source_parentheses(context, node_id) {
        return true;
    }

    expression_requires_parentheses_in_parent(context, node_id)
}

/// Return whether one expression structurally needs parentheses in its parent.
pub(crate) fn expression_requires_parentheses_in_parent(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    // statement-sensitive identifiers on the left of `as` and `satisfies` must stay parenthesized
    if !matches!(
        context.tree.get(node_id),
        Expression::As { .. } | Expression::Satisfies { .. }
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

    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    let parent_child_id = node_id;

    // treat compound condition operands as children of their implicit logical and
    if expression_condition_operand_needs_parentheses(context, parent_id, parent_type, node_id) {
        return true;
    }

    // statement context
    if parent_type != NodeType::Expression {
        let is_match_arm_body =
            expression_is_match_arm_body(context, parent_id, parent_type, parent_child_id);
        if expression_is_statement_like_value(context.tree.get(node_id))
            && expression_is_spread_value(context, parent_id, parent_type, parent_child_id)
        {
            return true;
        }

        return match context.tree.get(node_id) {
            Expression::As { .. } | Expression::Satisfies { .. } => {
                parent_type == NodeType::AssignPattern
            }
            Expression::Assign { left, .. } => {
                if expression_is_in_statement_context(context, node_id) {
                    return expression_assignment_needs_parentheses_in_statement_context(
                        context, node_id, *left,
                    );
                }

                true
            }
            Expression::ObjectExpression { .. } => {
                is_match_arm_body
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
            }
            _ => false,
        };
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);

    // group calls and assertions within a constructor operand
    if let Expression::New { left, .. } = parent_expression
        && *left == node_id
    {
        let mut operand = node_id;
        loop {
            match context.tree.get(operand) {
                Expression::Member { left, .. }
                | Expression::Index { left, .. }
                | Expression::Instantiation { left, .. } => operand = *left,
                Expression::Call { .. }
                | Expression::Must { .. }
                | Expression::Maybe { .. }
                | Expression::Chain { .. } => return true,
                _ => break,
            }
        }
    }

    // assignment expressions need parentheses unless they are already in assignment position
    if let Expression::Assign { .. } = context.tree.get(node_id) {
        return match parent_expression {
            Expression::Assign { .. } => false,
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

        return parent_requires_primary_expression(parent_expression, parent_child_id)
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
        ) || parent_requires_primary_expression(parent_expression, parent_child_id)
            || expression_is_call_like_callee(context, parent_expression_id, parent_child_id)
            || matches!(
                parent_expression,
                Expression::If {
                    form: IfForm::Ternary,
                    condition,
                    ..
                } if condition.as_expression() == Some(parent_child_id)
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
