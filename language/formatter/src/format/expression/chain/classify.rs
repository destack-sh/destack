use super::*;

/// Get the root head expression of a chain.
pub(crate) fn chain_head_id(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current = node_id;

    loop {
        let next = chain_node_left_id(tree, current);

        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(next_id) => return next_id,
            None => return current,
        }
    }
}

/// Check whether the chain head is simple and short.
pub(crate) fn is_simple_chain_head(
    context: &DestackFormatContext<'_>,
    head_id: LocalNodeId<Expression>,
) -> bool {
    let threshold = usize::from(context.options.line_width) / 4;
    let expression = context.tree.get(head_id);

    match expression {
        // simple identifier style heads
        Expression::Path {
            path,
            static_arguments,
        } => {
            static_arguments.is_none()
                && !context.has_annotation(head_id)
                && path.segments.len() <= 2
                && expression_source_len(context, head_id) <= threshold.max(6)
        }
        _ => false,
    }
}

/// Check whether an argument is short enough for poorly breakable chains.
pub(crate) fn is_short_chain_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let threshold = usize::from(context.options.line_width) / 4;
    argument_is_simple_with_options(
        context,
        argument_id,
        ArgumentSimplicityOptions {
            reject_any_argument_annotation: true,
            reject_non_blank_argument_annotation: false,
            reject_value_annotation: true,
            reject_lambda_values: false,
            max_value_len: threshold.max(8),
        },
    )
}

/// Check whether a chain call has dynamic arguments that make it breakable.
fn chain_call_has_breakable_dynamic_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if call_arguments_force_expand_for_chain(context, call_node_id, dynamic_arguments) {
        return true;
    }

    match dynamic_arguments.len() {
        0 => false,
        1 => !is_short_chain_argument(context, dynamic_arguments[0]),
        _ => true,
    }
}

/// A chain that has no calls at all or only short call arguments.
pub(crate) fn is_poorly_breakable_chain(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    // only chains qualify
    if !is_chain_root(tree, node_id) && !is_expression_chain(tree, node_id) {
        return false;
    }

    // find the chain head
    let head_id = chain_head_id(tree, node_id);
    if !is_simple_chain_head(context, head_id) {
        return false;
    }

    // walk the chain from the node down to the head, checking call arguments
    let mut current = node_id;
    let mut has_call = false;

    loop {
        if chain_node_has_non_inline_annotation(context, current) {
            return false;
        }

        match tree.get(current) {
            Expression::Call {
                static_arguments,
                dynamic_arguments,
                ..
            } => {
                has_call = true;

                // only allow no args or a single short argument
                let static_breakable = match static_arguments {
                    None => false,
                    Some(arguments) => match arguments.len() {
                        0 => false,
                        1 => !is_short_chain_argument(context, arguments[0]),
                        _ => true,
                    },
                };
                let is_breakable_call =
                    chain_call_has_breakable_dynamic_arguments(context, current, dynamic_arguments);

                if static_breakable || is_breakable_call {
                    return false;
                }
            }
            Expression::Index { index, .. } => {
                // indexes with nontrivial expressions are breakable
                if let Some(index_id) = index {
                    let index_expr = tree.get(*index_id);
                    if !is_trivial_expression(tree, index_expr) || context.has_annotation(*index_id)
                    {
                        return false;
                    }
                }
            }
            Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. } => {}
            _ => break,
        }

        let next = chain_node_left_id(tree, current);

        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(_) | None => break,
        }
    }

    // plain member chains are also poorly breakable
    has_call || is_chain_root(tree, node_id)
}

/// Get the value expression of any argument variant.
pub(crate) fn argument_value_id(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> LocalNodeId<Expression> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    }
}

/// Check whether an expression is a lambda declaration.
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

/// Check whether a lambda body forces multi-line formatting.
pub(crate) fn lambda_body_forces_multiline(
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

    let body_id = transparent_inner_expression(context, *body_id);

    match tree.get(body_id) {
        Expression::Block(_) => true,
        Expression::TreeExpression {
            arguments,
            elements,
            ..
        } => tree_literal_should_break(context, arguments, elements),
        _ => false,
    }
}

/// Compute the maximum nested callback depth inside an expression.
pub(crate) fn expression_callback_depth(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> usize {
    let tree = context.tree;
    let expression_id = transparent_inner_expression(context, expression_id);

    match tree.get(expression_id) {
        Expression::Call {
            dynamic_arguments, ..
        } => {
            let mut max_depth = 0usize;

            for argument_id in dynamic_arguments {
                let argument = tree.get(*argument_id);
                let value_id = match argument {
                    Argument::Positional { value, .. } | Argument::Spread { value, .. } => *value,
                    Argument::Named { value, .. } | Argument::Labeled { value, .. } => *value,
                };

                let value_id = transparent_inner_expression(context, value_id);

                let Expression::Declaration(declaration_id) = tree.get(value_id) else {
                    continue;
                };

                let Declaration::Function {
                    signature,
                    body: Some(body_id),
                    ..
                } = tree.get(*declaration_id)
                else {
                    continue;
                };

                if signature.kind != FunctionKind::Lambda {
                    continue;
                }

                let nested_depth = expression_callback_depth(context, *body_id);
                max_depth = max_depth.max(1 + nested_depth);
            }

            max_depth
        }
        _ => 0,
    }
}

/// Check whether a chain expression should break in its current context.
pub(crate) fn expression_chain_should_break(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    if !is_chain_expression(tree.get(expression_id)) {
        return false;
    }

    let mut chain = Vec::new();
    let mut current = expression_id;
    loop {
        chain.push(current);

        let next = chain_node_left_id(tree, current);

        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(_) | None => break,
        }
    }

    should_break_chain(context, &chain)
}

/// Check whether an expression is nested inside a tree literal argument.
pub(crate) fn expression_is_in_tree_literal_child(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id.id;

    // walk up the parent chain looking for tree element arguments
    while let Some((parent_id, parent_type)) = context.get_parent_by_id(current_id) {
        if parent_type == NodeType::Argument {
            let argument_id = LocalNodeId::<Argument>::new(parent_id);

            // tree literals store both attributes and children as arguments
            if let Some((grand_id, grand_type)) = context.get_parent_by_id(parent_id)
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

    let body_expr_id = transparent_inner_expression(context, *body_id);
    let body_expr = tree.get(body_expr_id);
    if matches!(body_expr, Expression::Block(_)) {
        return false;
    }

    if let Expression::TreeExpression {
        arguments,
        elements,
        ..
    } = body_expr
    {
        if tree_literal_should_break(context, arguments, elements) {
            return true;
        }

        // tree returning callbacks in tree literals should break for readability
        if let Some((parent_id, parent_type)) = context.get_parent(declaration_id)
            && parent_type == NodeType::Expression
        {
            let expression_id = LocalNodeId::<Expression>::new(parent_id);
            if expression_is_in_tree_literal_child(context, expression_id) {
                return true;
            }
        }
    }

    if expression_chain_should_break(context, body_expr_id) {
        return true;
    }

    if expression_callback_depth(context, body_expr_id) >= 2 {
        return true;
    }

    false
}

/// Check whether a call argument forces multi-line formatting.
pub(crate) fn argument_forces_multiline(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);

    match context.tree.get(value_id) {
        Expression::Declaration(declaration_id) => {
            lambda_body_forces_multiline(context, *declaration_id)
        }
        Expression::TreeExpression {
            arguments,
            elements,
            ..
        } => tree_literal_should_break(context, arguments, elements),
        _ => false,
    }
}

/// Check whether an argument is a block-bodied lambda.
pub(crate) fn is_block_lambda_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = context.tree.get(*declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    matches!(context.tree.get(*body_id), Expression::Block(_))
}

/// Check whether an argument is simple enough to stay inline in chains.
pub(crate) fn is_simple_chain_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let threshold = usize::from(context.options.line_width) / 4;
    argument_is_simple_with_options(
        context,
        argument_id,
        ArgumentSimplicityOptions {
            reject_any_argument_annotation: true,
            reject_non_blank_argument_annotation: false,
            reject_value_annotation: true,
            reject_lambda_values: true,
            max_value_len: threshold.max(10),
        },
    )
}

/// Sum the source lengths of argument values.
pub(crate) fn arguments_total_len(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> usize {
    // accumulate argument value lengths
    let mut total_len = 0usize;

    for argument_id in arguments {
        let value_id = argument_value_id(context.tree, *argument_id);
        let value_len = expression_source_len(context, value_id);
        total_len = total_len.saturating_add(value_len);
    }

    total_len
}

/// Estimate the rendered length of arguments when printed inline.
pub(crate) fn arguments_rendered_len(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> usize {
    // sum the value widths
    let values_len = arguments_total_len(context, arguments);

    // account for `, ` separators
    let separators_len = arguments.len().saturating_sub(1) * 2;

    values_len.saturating_add(separators_len)
}

/// Check whether an expression is a numeric scalar literal.
pub(crate) fn is_numeric_scalar_literal(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Bigint(_) | ScalarLiteral::Float(_)
        )
    )
}

/// Check whether an index expression is a numeric literal without annotations.
pub(crate) fn is_numeric_index_expression(
    context: &DestackFormatContext<'_>,
    index_id: LocalNodeId<Expression>,
) -> bool {
    // annotated indexes are never numeric simple
    if context.has_annotation(index_id) {
        return false;
    }

    let expression = context.tree.get(index_id);
    is_numeric_scalar_literal(expression)
}

/// Check whether an optional index is numeric-simple.
pub(crate) fn is_numeric_index(
    context: &DestackFormatContext<'_>,
    index: &Option<LocalNodeId<Expression>>,
) -> bool {
    match index {
        Some(index_id) => is_numeric_index_expression(context, *index_id),
        None => false,
    }
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

/// Check whether a static argument list is simple enough for chain heads.
pub(crate) fn is_simple_chain_static_argument_list(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    static_arguments.len() <= 1
        && static_arguments
            .iter()
            .copied()
            .all(|argument_id| is_simple_chain_argument(context, argument_id))
}

/// Check whether a chain call is simple enough to stay in the head.
pub(crate) fn is_simple_chain_call(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    // annotated calls are never simple
    if call_has_non_blank_infix_annotation(context, node_id) {
        return false;
    }

    if call_arguments_force_expand_for_chain(context, node_id, dynamic_arguments) {
        return false;
    }

    if !is_simple_chain_static_arguments(context, static_arguments) {
        return false;
    }

    dynamic_arguments.len() <= 1
        && dynamic_arguments
            .iter()
            .copied()
            .all(|argument_id| is_simple_chain_argument(context, argument_id))
}

/// Check whether a chain operation is simple enough for head promotion.
pub(crate) fn is_simple_chain_operation(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    match operation {
        ChainExpression::Member {
            node_id,
            static_arguments,
            emit_prefix_annotations,
            emit_postfix_annotations,
            ..
        } => {
            let has_intervening_comment = member_has_intervening_comment(context, *node_id);
            let has_annotations = (*emit_prefix_annotations || *emit_postfix_annotations)
                && chain_node_has_non_inline_annotation(context, *node_id);
            let has_break = has_intervening_comment || has_annotations;
            !has_break && is_simple_chain_static_arguments(context, static_arguments)
        }
        ChainExpression::Call {
            node_id,
            static_arguments,
            dynamic_arguments,
            ..
        } => is_simple_chain_call(context, *node_id, static_arguments, dynamic_arguments),
        ChainExpression::Instantiation {
            node_id,
            static_arguments,
        } => {
            !chain_node_has_non_inline_annotation(context, *node_id)
                && is_simple_chain_static_argument_list(context, static_arguments)
        }
        ChainExpression::Index { node_id, index, .. } => {
            !chain_node_has_non_inline_annotation(context, *node_id)
                && is_numeric_index(context, index)
        }
        ChainExpression::Maybe { node_id, .. } | ChainExpression::Must { node_id, .. } => {
            !chain_node_has_non_inline_annotation(context, *node_id)
        }
    }
}

/// Check if a member access has an intervening comment between receiver and property.
pub(crate) fn member_has_intervening_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(node_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            let Some(property_span) = context.tree.get_main_span(node_id) else {
                return false;
            };
            let left_span = context.get_span(*left);
            let left_anchor_end = expression_trivia_anchor_end(context, *left);

            // guard against malformed spans
            if property_span.start <= left_anchor_end {
                return false;
            }

            let span = Span::new(left_span.file, left_anchor_end, property_span.start);
            let between = context.get_span_str(span);
            between.contains("/*") || between.contains("//")
        }
        _ => false,
    }
}

/// Check if a member access has a newline or comment between its receiver and property.
pub(crate) fn member_has_intervening_break_or_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(node_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            let Some(property_span) = context.tree.get_main_span(node_id) else {
                return false;
            };
            let left_span = context.get_span(*left);
            let left_anchor_end = expression_trivia_anchor_end(context, *left);

            // guard against malformed spans
            if property_span.start <= left_anchor_end {
                return false;
            }

            let span = Span::new(left_span.file, left_anchor_end, property_span.start);
            let between = context.get_span_str(span);

            let has_between_comment_or_break = between.contains('\n')
                || between.contains('\r')
                || between.contains("/*")
                || between.contains("//");
            if has_between_comment_or_break {
                return true;
            }

            false
        }
        Expression::Path { .. } => {
            let span = context.get_span(node_id);
            let between = context.get_span_str(span);

            between.contains('\n')
                || between.contains('\r')
                || between.contains("/*")
                || between.contains("//")
        }
        _ => false,
    }
}

/// Return an expression end anchor used for chain trivia checks.
pub(crate) fn expression_trivia_anchor_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression = context.tree.get(expression_id);
    let span = context.get_span(expression_id);

    // member like nodes often include trailing boundary comments in their full spans
    // so anchor at the property token to inspect the comment gap before parent operators
    match expression {
        Expression::Member { .. } | Expression::PrivateMember { .. } => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |member_span| member_span.end),
        Expression::Path { path, .. } if path.segments.len() == 1 => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |path_span| path_span.end),
        _ => span.end,
    }
}

/// Check if a chain node has source trivia before its parent operator.
pub(crate) fn chain_has_parent_intervening_break_or_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent = context.tree.get(parent_id);
    let parent_uses_node_as_left = match parent {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => *left == node_id,
        _ => false,
    };
    if !parent_uses_node_as_left {
        return false;
    }

    let should_check_parent_gap = match parent {
        Expression::Member { .. } | Expression::PrivateMember { .. } => true,
        Expression::Call { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => matches!(
            context.tree.get(node_id),
            Expression::Member { .. } | Expression::PrivateMember { .. } | Expression::Path { .. }
        ),
        _ => false,
    };
    if !should_check_parent_gap {
        return false;
    }

    let node_span = context.get_span(node_id);
    let node_anchor_end = expression_trivia_anchor_end(context, node_id);
    let parent_gap_end = if let Some(parent_main_span) = context.tree.get_main_span(parent_id) {
        if node_span.file != parent_main_span.file || parent_main_span.start <= node_anchor_end {
            return false;
        }
        parent_main_span.start
    } else {
        let parent_span = context.get_span(parent_id);
        if node_span.file != parent_span.file || parent_span.end <= node_anchor_end {
            return false;
        }
        parent_span.end
    };

    let between = context.get_span_str(Span::new(node_span.file, node_anchor_end, parent_gap_end));
    between.contains('\n')
        || between.contains('\r')
        || between.contains("/*")
        || between.contains("//")
}
