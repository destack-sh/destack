use crate::format::chain::{
    Annotation, AnnotationPosition, Argument, ArgumentSimplicityOptions, ChainExpression,
    Declaration, DestackFormatContext, Expression, FunctionKind, LocalNodeId, NodeTree, NodeType,
    ScalarLiteral, Span, TokenType, argument_has_non_blank_annotation,
    argument_is_simple_with_options, call_arguments_force_expand_for_chain,
    call_has_non_blank_infix_annotation, chain_node_has_non_inline_annotation, chain_node_left_id,
    is_chain_expression, is_chain_root, is_expression_breakable, is_expression_chain,
    is_trivial_expression, should_break_chain, span_has_comment, transparent_inner_expression,
    tree_literal_should_break,
};
use crate::format::operator::expression_static_arguments;
use destack_ast::{Comment, CommentStyle};

// static argument hugging thresholds
const HUG_STATIC_ARGUMENT_MAX_COUNT: usize = 3;

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

/// Check whether source contains a newline between two expression nodes.
pub(crate) fn has_newline_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    context.has_newline(Span::new(left_span.file, left_span.end, right_span.start))
}

/// Check whether source contains a comment between two expression nodes.
pub(crate) fn has_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    span_has_comment(
        context,
        Span::new(left_span.file, left_span.end, right_span.start),
    )
}

/// Return whether a `//` comment exists between two expression nodes.
pub(crate) fn has_line_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    context
        .line_comment_spans
        .iter()
        .copied()
        .any(|comment_span| {
            comment_span.file == left_span.file
                && comment_span.start >= left_span.end
                && comment_span.end <= right_span.start
        })
}

/// Check whether a call argument forces multi-line formatting.
pub(crate) fn argument_forces_multiline(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    match context.tree.get(value_id) {
        Expression::Declaration(declaration_id) => {
            lambda_body_forces_multiline(context, *declaration_id)
        }
        Expression::TemplateExpression { .. } => context.node_has_newline(value_id),
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
    argument_is_simple_with_options(
        context,
        argument_id,
        ArgumentSimplicityOptions {
            reject_any_argument_annotation: true,
            reject_non_blank_argument_annotation: false,
            reject_value_annotation: true,
            reject_lambda_values: true,
        },
    )
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
        }
        Expression::This => !context.has_annotation(head_id),
        _ => false,
    }
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
                let value_id = argument_value_id(tree, *argument_id);
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
        if let Some((parent_id, parent_type)) = context.parent(declaration_id)
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
            let has_promotable_boundary_comment =
                chain_member_has_promotable_boundary_comment(context, *node_id);
            let has_annotations = (*emit_prefix_annotations || *emit_postfix_annotations)
                && chain_node_has_non_inline_annotation(context, *node_id)
                && !has_promotable_boundary_comment;
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

/// Return whether a member has only one promotable slash boundary comment annotation.
fn chain_member_has_promotable_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(node_id) else {
        return false;
    };

    let mut found_promotable_boundary_comment = false;
    for annotation_id in annotations {
        match context.annotation(annotation_id) {
            Annotation::Blank { .. } => {}
            Annotation::Comment {
                node,
                position: AnnotationPosition::LinePostfixBoundary,
            } => {
                let comment = context.tree.get::<Comment>(node);
                if comment.style != CommentStyle::Slash {
                    return false;
                }
                found_promotable_boundary_comment = true;
            }
            _ => return false,
        }
    }

    found_promotable_boundary_comment
}

/// Return the source span between one member receiver and property token.
fn member_receiver_property_gap_span(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<Span> {
    match context.tree.get(node_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            let property_span = context.tree.get_main_span(node_id)?;
            let left_span = context.span(*left);
            let left_anchor_end = expression_trivia_anchor_end(context, *left);

            // guard malformed spans
            if property_span.start <= left_anchor_end {
                return None;
            }

            Some(Span::new(
                left_span.file,
                left_anchor_end,
                property_span.start,
            ))
        }
        _ => None,
    }
}

/// Return whether a path spans a newline before its first `.` separator.
fn path_has_newline_before_first_separator(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Path { path, .. } = context.tree.get(node_id) else {
        return false;
    };
    if path.segments.len() <= 1 {
        return false;
    }

    let span = context.span(node_id);
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.end <= span.start);
    let mut has_newline_before_separator = false;

    while let Some(token) = tokens.get(index).copied() {
        if token.span.start >= span.end {
            break;
        }

        index += 1;
        if token.token.ty == TokenType::Dot {
            return has_newline_before_separator;
        }

        if token.token.ty == TokenType::Newline {
            has_newline_before_separator = true;
        }
    }

    false
}

/// Check if a member access has an intervening comment between receiver and property.
pub(crate) fn member_has_intervening_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    member_receiver_property_gap_span(context, node_id)
        .is_some_and(|span| span_has_comment(context, span))
}

/// Check if a member access has an intervening break or comment between receiver and property.
pub(crate) fn member_has_intervening_break_or_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    // member and private member: inspect the trivia gap between receiver and property
    if let Some(span) = member_receiver_property_gap_span(context, node_id) {
        return context.has_newline(span) || span_has_comment(context, span);
    }

    // path chains: only the first separator can influence root splitting
    if matches!(context.tree.get(node_id), Expression::Path { .. }) {
        let span = context.span(node_id);
        return path_has_newline_before_first_separator(context, node_id)
            || span_has_comment(context, span);
    }

    false
}

/// Return an expression end anchor used for chain trivia checks.
pub(crate) fn expression_trivia_anchor_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression = context.tree.get(expression_id);
    let span = context.span(expression_id);

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

/// Check if a chain node has source breaks or comments before its parent operator.
pub(crate) fn chain_has_parent_intervening_break_or_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
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

    let node_span = context.span(node_id);
    let node_anchor_end = expression_trivia_anchor_end(context, node_id);
    let Some(parent_main_span) = context.tree.get_main_span(parent_id) else {
        return false;
    };
    if node_span.file != parent_main_span.file || parent_main_span.start <= node_anchor_end {
        return false;
    }
    let parent_gap_end = parent_main_span.start;

    let between_span = Span::new(node_span.file, node_anchor_end, parent_gap_end);

    context.has_newline(between_span) || span_has_comment(context, between_span)
}

/// Record stable facts for one poorly-breakable chain candidate.
struct PoorChainFacts {
    has_chain_shape: bool,
    has_simple_head: bool,
}

/// Build static facts for one poorly-breakable chain candidate.
fn collect_poor_chain_facts(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> PoorChainFacts {
    let tree = context.tree;
    let has_chain_shape = is_chain_root(tree, node_id) || is_expression_chain(tree, node_id);

    let has_simple_head = if has_chain_shape {
        let head_id = chain_head_id(tree, node_id);
        is_simple_chain_head(context, head_id)
    } else {
        false
    };

    PoorChainFacts {
        has_chain_shape,
        has_simple_head,
    }
}

/// Return whether one dynamic argument list makes a call breakable for poor-chain checks.
fn dynamic_arguments_are_breakable(
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

/// Return whether one static argument is short enough for poor-chain checks.
fn static_argument_is_short(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if is_short_chain_argument(context, argument_id) {
        return true;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    if context.has_annotation(value_id) {
        return false;
    }

    !matches!(
        context.tree.get(value_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    )
}

/// Return whether one static argument list makes a call breakable for poor-chain checks.
fn static_arguments_are_breakable(
    context: &DestackFormatContext<'_>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
) -> bool {
    match static_arguments {
        None => false,
        Some(arguments) => match arguments.len() {
            0 => false,
            1 => !static_argument_is_short(context, arguments[0]),
            _ => true,
        },
    }
}

/// Return whether one chain index operation is breakable for poor-chain checks.
fn index_operation_is_breakable(
    context: &DestackFormatContext<'_>,
    index: &Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(index_id) = index else {
        return false;
    };

    let index_expression = context.tree.get(*index_id);
    !is_trivial_expression(context.tree, index_expression) || context.has_annotation(*index_id)
}

/// Return whether one chain operation is breakable for poor-chain checks.
fn operation_is_breakable_for_poor_chain(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if chain_node_has_non_inline_annotation(context, node_id) {
        return true;
    }

    match context.tree.get(node_id) {
        Expression::Call {
            static_arguments,
            dynamic_arguments,
            ..
        } => {
            static_arguments_are_breakable(context, static_arguments)
                || dynamic_arguments_are_breakable(context, node_id, dynamic_arguments)
        }
        Expression::Index { index, .. } => index_operation_is_breakable(context, index),
        Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => false,
        _ => true,
    }
}

/// Return whether one argument is short enough for poorly breakable chains.
fn is_short_chain_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let is_simple = argument_is_simple_with_options(
        context,
        argument_id,
        ArgumentSimplicityOptions {
            reject_any_argument_annotation: true,
            reject_non_blank_argument_annotation: false,
            reject_value_annotation: true,
            reject_lambda_values: false,
        },
    );
    if is_simple {
        return true;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    if context.has_annotation(value_id) {
        return false;
    }

    match context.tree.get(value_id) {
        Expression::Call {
            static_arguments,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            static_arguments,
            dynamic_arguments,
            ..
        } => {
            !call_has_non_blank_infix_annotation(context, value_id)
                && static_arguments
                    .as_ref()
                    .is_none_or(|arguments| arguments.is_empty())
                && dynamic_arguments.is_empty()
        }
        Expression::Instantiation {
            static_arguments, ..
        } => static_arguments.is_empty(),
        _ => false,
    }
}

/// Return whether one chain is poorly breakable:
/// either member-only or calls with only short arguments.
pub(crate) fn is_poorly_breakable_chain(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let facts = collect_poor_chain_facts(context, node_id);
    if !facts.has_chain_shape || !facts.has_simple_head {
        return false;
    }

    let tree = context.tree;
    let mut current = node_id;
    let mut has_call = false;

    loop {
        if operation_is_breakable_for_poor_chain(context, current) {
            if !matches!(
                tree.get(current),
                Expression::Member { .. }
                    | Expression::PrivateMember { .. }
                    | Expression::Maybe { .. }
                    | Expression::Must { .. }
                    | Expression::Call { .. }
                    | Expression::Index { .. }
            ) {
                break;
            }

            return false;
        }

        if matches!(tree.get(current), Expression::Call { .. }) {
            has_call = true;
        }

        let next = chain_node_left_id(tree, current);
        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(_) | None => break,
        }
    }

    has_call || is_expression_chain(tree, node_id) || is_chain_root(tree, node_id)
}

/// Return the concrete span that corresponds to one annotation node.
fn annotation_content_span(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Span {
    context.annotation_span(annotation_id)
}

/// Check whether a member access uses a private hash (`.#name`).
pub(crate) fn member_is_private_hash(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(context.tree.get(node_id), Expression::PrivateMember { .. }) {
        return true;
    }

    let Some(property_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    let token_idx = context
        .tokens
        .iter()
        .position(|token| token.span.start == property_span.start);
    let Some(token_idx) = token_idx else {
        return false;
    };

    let prev_token = token_idx
        .checked_sub(1)
        .and_then(|index| context.tokens.get(index));
    let Some(prev_token) = prev_token else {
        return false;
    };

    prev_token.token.ty == TokenType::Hash
}

/// Decide whether postfix annotations on a path belong after the last segment.
pub(crate) fn path_postfix_annotations_emit_on_tail(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> bool {
    let Some(last_segment_start) = path_last_segment_start(context, node_id, segments_len) else {
        return false;
    };

    context
        .visit_annotations(node_id, |annotations| {
            let mut has_postfix = false;
            for annotation_id in annotations {
                let annotation = context.annotation(*annotation_id);
                let position = annotation.position();
                let is_postfix = matches!(
                    position,
                    AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                );
                if !is_postfix {
                    continue;
                }

                has_postfix = true;
                let span = annotation_content_span(context, *annotation_id);
                if span.start < last_segment_start {
                    return false;
                }
            }

            if !has_postfix {
                return true;
            }

            true
        })
        .unwrap_or(true)
}

/// Find the start byte of the last path segment token.
pub(crate) fn path_last_segment_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> Option<u32> {
    if segments_len == 0 {
        return None;
    }

    let span = context.span(node_id);
    let mut count = 0usize;
    for token in context.tokens.iter() {
        if token.span.start < span.start {
            continue;
        }
        if token.span.start >= span.end {
            break;
        }
        if token.token.ty == TokenType::Identifier {
            count += 1;
            if count == segments_len {
                return Some(token.span.start);
            }
        }
    }

    None
}

/// Return whether a chain call can stay in the head even when its arguments expand.
pub(crate) fn chain_call_can_expand_in_head(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if call_has_non_blank_infix_annotation(context, call_node_id) {
        return false;
    }

    if chain_node_has_non_inline_annotation(context, call_node_id) {
        return false;
    }

    if !is_simple_chain_static_arguments(context, static_arguments) {
        return false;
    }

    if dynamic_arguments
        .iter()
        .any(|argument_id| argument_has_non_blank_annotation(context, *argument_id))
    {
        return false;
    }

    call_arguments_force_expand_for_chain(context, call_node_id, dynamic_arguments)
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
    let value_id = argument_value_id(context.tree, static_arguments[0]);
    let value_id = transparent_inner_expression(context, value_id);
    if matches!(
        context.tree.get(value_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    ) {
        return false;
    }

    // only expand list-level generic wrappers when source is already multiline and
    // the nested type arguments include object-like forms
    if !context.node_has_newline(value_id) {
        return false;
    }

    expression_static_arguments(context.tree.get(value_id)).is_some_and(|nested_arguments| {
        nested_arguments.iter().copied().any(|nested_argument_id| {
            let nested_value_id = argument_value_id(context.tree, nested_argument_id);
            let nested_value_id = transparent_inner_expression(context, nested_value_id);
            matches!(
                context.tree.get(nested_value_id),
                Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
            )
        })
    })
}

/// Decide whether static argument lists should stay inline regardless of line width.
pub(crate) fn should_hug_static_argument_list(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    static_argument_list_is_hug_safe(context, static_arguments)
}

/// Return whether static arguments are structurally safe for hugged inline formatting.
fn static_argument_list_is_hug_safe(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if static_arguments.is_empty() || static_arguments.len() > HUG_STATIC_ARGUMENT_MAX_COUNT {
        return false;
    }

    static_arguments
        .iter()
        .copied()
        .all(|argument_id| static_argument_is_hug_safe(context, argument_id))
}

/// Return whether one static argument is structurally safe for hugged inline formatting.
fn static_argument_is_hug_safe(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if argument_has_non_blank_annotation(context, argument_id) {
        return false;
    }

    let argument_value_id = argument_value_id(context.tree, argument_id);
    let argument_value_id = transparent_inner_expression(context, argument_value_id);
    if context.node_has_newline(argument_value_id) {
        return false;
    }

    static_argument_expression_is_hug_safe(context, argument_value_id)
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
        Expression::Index { left, index, .. } => {
            static_argument_expression_is_hug_safe(context, *left)
                && index.is_none_or(|index_id| {
                    static_argument_expression_is_hug_safe(context, index_id)
                })
        }
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            static_argument_expression_is_hug_safe(context, *expression)
        }
        _ => false,
    }
}

/// Decide whether a mapped type should force multiline formatting.
pub(crate) fn should_force_multiline_mapped_type(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(node_id) {
        return true;
    }

    let span = context.span(node_id);
    if context.has_newline(span) {
        return true;
    }

    is_expression_breakable(context.tree, context.tree.get(value_id))
}
