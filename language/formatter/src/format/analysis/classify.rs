use crate::analysis::scan::previous_non_whitespace_token_before_annotation;
use crate::expression::{
    Annotation, AnnotationPosition, Argument, Declaration, DestackFormatContext, Expression,
    FunctionKind, LocalNodeId, NodeType, ScalarLiteral, Span, TokenType, TypeBinaryOperator,
    argument_is_array_literal, argument_is_block_callback, argument_is_function_expression,
    argument_is_lambda_expression, argument_is_object_literal, argument_value_id,
    is_trivial_argument, is_trivial_expression, transparent_inner_expression,
};
use destack_ast::{Comment, CommentStyle, TemplateLiteral};

/// Store shared argument simplicity checks for call and chain classifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ArgumentSimplicityOptions {
    /// Reject any annotations on the argument node.
    pub reject_any_argument_annotation: bool,
    /// Reject non-blank annotations on the argument node.
    pub reject_non_blank_argument_annotation: bool,
    /// Reject annotations on the argument value expression.
    pub reject_value_annotation: bool,
    /// Reject lambda declaration values.
    pub reject_lambda_values: bool,
}

/// Return whether an expression node is a lambda declaration.
fn expression_is_lambda_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function { signature, .. }
                    if signature.kind == FunctionKind::Lambda
            )
    )
}

/// Return whether an argument satisfies shared call and chain simplicity constraints.
pub(crate) fn argument_is_simple_with_options(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    options: ArgumentSimplicityOptions,
) -> bool {
    if options.reject_any_argument_annotation && context.has_annotation(argument_id) {
        return false;
    }
    if options.reject_non_blank_argument_annotation
        && argument_has_non_blank_annotation(context, argument_id)
    {
        return false;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    if options.reject_lambda_values && expression_is_lambda_declaration(context, value_id) {
        return false;
    }
    if options.reject_value_annotation && context.has_annotation(value_id) {
        return false;
    }

    let value = context.tree.get(value_id);
    is_trivial_expression(context.tree, value)
}

/// Return whether an expression appears in call-like argument position.
pub(crate) fn is_call_like_argument(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let Some((expression_id, expression_type)) = context.parent_by_id(argument_id) else {
        return false;
    };
    if expression_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. }
    )
}

/// Check whether an expression is the value of a tree/JSX attribute argument.
pub(crate) fn is_tree_attribute_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let Some((expression_id, expression_type)) = context.parent_by_id(argument_id) else {
        return false;
    };
    if expression_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::TreeExpression { .. }
    )
}

/// Return whether a static argument should stay inline in a path.
pub(crate) fn is_simple_static_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_is_simple_with_options(
        context,
        argument_id,
        ArgumentSimplicityOptions {
            reject_any_argument_annotation: false,
            reject_non_blank_argument_annotation: true,
            reject_value_annotation: false,
            reject_lambda_values: false,
        },
    )
}

/// Return whether an argument is a string or template literal.
pub(crate) fn argument_is_string_like(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    matches!(
        context.tree.get(value_id),
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    )
}

/// Return whether an argument is an interpolated template literal.
pub(crate) fn argument_is_interpolated_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    matches!(
        context.tree.get(value_id),
        Expression::TemplateExpression {
            value: TemplateLiteral::InterpolatedString { .. }
        }
    )
}

/// Return whether an argument is a collection literal.
pub(crate) fn argument_is_collection_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_is_object_literal(context, argument_id)
        || argument_is_array_literal(context, argument_id)
}

/// Return whether an argument is a reference style expression.
pub(crate) fn argument_is_reference_like(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    matches!(
        context.tree.get(value_id),
        Expression::Path { .. }
            | Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
            | Expression::This
            | Expression::Super
            | Expression::PrivateIdentifier { .. }
    )
}

/// Return whether a callee ends in a test style member name.
pub(crate) fn call_callee_has_test_like_member_name(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = context.tree.get(call_node_id) else {
        return false;
    };

    let mut current_id = *left;
    loop {
        current_id = transparent_inner_expression(context, current_id);
        match context.tree.get(current_id) {
            Expression::Member { name, .. } | Expression::PrivateMember { name, .. } => {
                let name = context.strings.get(*name);
                return matches!(
                    name,
                    "test"
                        | "it"
                        | "describe"
                        | "only"
                        | "skip"
                        | "todo"
                        | "fixme"
                        | "serial"
                        | "parallel"
                );
            }
            Expression::Path { path, .. } => {
                let Some(last_segment) = path.segments.last() else {
                    return false;
                };
                let name = context.strings.get(*last_segment);
                return matches!(
                    name,
                    "test"
                        | "it"
                        | "describe"
                        | "only"
                        | "skip"
                        | "todo"
                        | "fixme"
                        | "serial"
                        | "parallel"
                );
            }
            Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
                current_id = *left;
            }
            _ => return false,
        }
    }
}

/// Return whether a call should keep leading string arguments with callback tails.
pub(crate) fn call_should_force_hug_test_like_callback(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 2 {
        return false;
    }

    if !argument_is_string_like(context, dynamic_arguments[0]) {
        return false;
    }

    if !(argument_is_lambda_expression(context, dynamic_arguments[1])
        || argument_is_function_expression(context, dynamic_arguments[1]))
    {
        return false;
    }

    call_callee_has_test_like_member_name(context, call_node_id)
}

/// Return whether call arguments span multiple lines in source.
pub(crate) fn call_arguments_are_multiline_in_source(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first), Some(last)) = (dynamic_arguments.first(), dynamic_arguments.last()) else {
        return false;
    };

    let first_span = context.span(*first);
    let last_span = context.span(*last);
    if first_span.file != last_span.file || first_span.start >= last_span.end {
        return false;
    }

    context.has_newline(Span::new(first_span.file, first_span.start, last_span.end))
}

/// Return whether one span intersects any line-style comment token.
fn span_has_line_comment_token(context: &DestackFormatContext<'_>, span: Span) -> bool {
    if span.start >= span.end {
        return false;
    }

    let line_comment_spans = &context.line_comment_spans;
    if line_comment_spans.is_empty() {
        return false;
    }

    let first_relevant_index =
        line_comment_spans.partition_point(|comment_span| comment_span.end < span.start);

    for comment_span in &line_comment_spans[first_relevant_index..] {
        if comment_span.start > span.end {
            break;
        }

        if span.intersects(*comment_span) {
            return true;
        }
    }

    false
}

/// Return the next close parenthesis token start after one source offset.
fn next_close_parenthesis_start_after(
    context: &DestackFormatContext<'_>,
    start: u32,
) -> Option<u32> {
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.start < start);

    while let Some(token) = tokens.get(index) {
        if token.token.ty == TokenType::CloseParenthesis {
            return Some(token.span.start);
        }
        index += 1;
    }

    None
}

/// Return whether source text around call argument boundaries contains line comments.
pub(crate) fn call_arguments_have_boundary_comments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(has_boundary_comments) =
        context.lookup_call_argument_boundary_comments(call_node_id)
    {
        context.increment_counter("call.arguments.boundary_comments.cache.hits", 1);
        return has_boundary_comments;
    }
    context.increment_counter("call.arguments.boundary_comments.cache.misses", 1);

    if dynamic_arguments.is_empty() {
        context.store_call_argument_boundary_comments(call_node_id, false);
        return false;
    }

    #[cfg(debug_assertions)]
    {
        let call_dynamic_argument_len = match context.tree.get(call_node_id) {
            Expression::Call {
                dynamic_arguments, ..
            }
            | Expression::New {
                dynamic_arguments, ..
            } => dynamic_arguments.len(),
            _ => dynamic_arguments.len(),
        };
        debug_assert_eq!(
            call_dynamic_argument_len,
            dynamic_arguments.len(),
            "call boundary comment detection requires the full dynamic argument list",
        );
    }

    // line comments between adjacent arguments are boundary comments
    for argument_pair in dynamic_arguments.windows(2) {
        let left_span = context.span(argument_pair[0]);
        let right_span = context.span(argument_pair[1]);
        if left_span.file != right_span.file || left_span.end >= right_span.start {
            continue;
        }

        let between_span = Span::new(left_span.file, left_span.end, right_span.start);
        if span_has_line_comment_token(context, between_span) {
            context.store_call_argument_boundary_comments(call_node_id, true);
            return true;
        }
    }

    // line comments after the final argument and before the call close are boundary comments
    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };
    let last_argument_span = context.span(last_argument_id);
    let call_span = context.span(call_node_id);
    if call_span.file != last_argument_span.file || last_argument_span.end >= call_span.end {
        context.store_call_argument_boundary_comments(call_node_id, false);
        return false;
    }

    let close_parenthesis_start =
        next_close_parenthesis_start_after(context, last_argument_span.end)
            .unwrap_or(call_span.end);
    let boundary_end = close_parenthesis_start.min(call_span.end);
    let boundary_span = Span::new(
        last_argument_span.file,
        last_argument_span.end,
        boundary_end,
    );
    let has_boundary_comments = span_has_line_comment_token(context, boundary_span);
    context.store_call_argument_boundary_comments(call_node_id, has_boundary_comments);
    has_boundary_comments
}

/// Return whether source text between two arguments contains an explicit blank line.
pub(crate) fn call_arguments_preserve_blank_line_between(
    context: &DestackFormatContext<'_>,
    left_argument_id: LocalNodeId<Argument>,
    right_argument_id: LocalNodeId<Argument>,
) -> bool {
    let left_span = context.span(left_argument_id);
    let right_span = context.span(right_argument_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    let right_value_id = argument_value_id(context.tree, right_argument_id);
    let has_blank_prefix_between = |annotation_id: LocalNodeId<Annotation>| {
        if !matches!(
            context.annotation(annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                ..
            }
        ) {
            return false;
        }

        let annotation_span = context.annotation_span(annotation_id);
        annotation_span.file == left_span.file
            && annotation_span.start >= left_span.end
            && annotation_span.end <= right_span.start
    };

    let right_argument_has_blank_prefix = context
        .annotations(right_argument_id)
        .is_some_and(|annotations| annotations.iter().copied().any(has_blank_prefix_between));
    if right_argument_has_blank_prefix {
        return true;
    }

    context
        .annotations(right_value_id)
        .is_some_and(|annotations| annotations.iter().copied().any(has_blank_prefix_between))
}

/// Return whether a call has a non-blank block infix annotation.
pub(crate) fn call_has_non_blank_infix_annotation(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    context.has_non_blank_infix_annotation(call_node_id)
}

/// Return whether an argument has a non-blank annotation.
pub(crate) fn argument_has_non_blank_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context.has_non_blank_annotation(argument_id)
}

/// Return whether an argument has multiline non-blank prefix annotations.
pub(crate) fn argument_has_multiline_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !context.node_has_newline(argument_id) {
        return false;
    }

    context
        .ensure_argument_annotation_facts(argument_id)
        .has_prefix_annotation
}

/// Return whether an argument has prefix annotations that start before the argument span.
pub(crate) fn argument_has_leading_prefix_annotation_outside_span(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let argument_span = context.span(argument_id);

    context
        .visit_annotations(argument_id, |annotations| {
            annotations
                .iter()
                .any(|annotation_id| match context.annotation(*annotation_id) {
                    Annotation::Blank { .. } => false,
                    Annotation::Doc { position, .. }
                    | Annotation::Comment { position, .. }
                    | Annotation::Decorator { position, .. } => {
                        if !matches!(
                            position,
                            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                        ) {
                            return false;
                        }
                        let annotation_span = context.annotation_span(*annotation_id);
                        annotation_span.start < argument_span.start
                    }
                })
        })
        .unwrap_or(false)
}

/// Return whether an argument has any slash style comment annotation.
pub(crate) fn argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .ensure_argument_annotation_facts(argument_id)
        .has_line_comment
}

/// Return whether an argument is an inline closure-cast object argument.
pub(crate) fn argument_is_inline_closure_cast_object(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.node_has_newline(argument_id) {
        return false;
    }

    let argument_span = context.span(argument_id);
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    if !matches!(
        context.tree.get(value_id),
        Expression::ObjectExpression { .. }
    ) {
        return false;
    }

    let value_span = context.span(value_id);
    let argument_has_inline_prefix = context
        .visit_annotations(argument_id, |annotations| {
            if annotations.is_empty() {
                return false;
            }

            annotations
                .iter()
                .all(|annotation_id| match context.annotation(*annotation_id) {
                    Annotation::Blank { .. } => true,
                    Annotation::Doc { position, .. } | Annotation::Comment { position, .. } => {
                        if !matches!(
                            position,
                            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                        ) {
                            return false;
                        }

                        let annotation_span = context.annotation_span(*annotation_id);
                        annotation_span.start < argument_span.start
                    }
                    Annotation::Decorator { .. } => false,
                })
        })
        .unwrap_or(false);
    let value_has_inline_prefix = context
        .visit_annotations(value_id, |annotations| {
            if annotations.is_empty() {
                return false;
            }

            annotations
                .iter()
                .all(|annotation_id| match context.annotation(*annotation_id) {
                    Annotation::Blank { .. } => true,
                    Annotation::Doc { position, .. } | Annotation::Comment { position, .. } => {
                        if !matches!(
                            position,
                            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                        ) {
                            return false;
                        }

                        let annotation_span = context.annotation_span(*annotation_id);
                        annotation_span.start < value_span.start
                    }
                    Annotation::Decorator { .. } => false,
                })
        })
        .unwrap_or(false);

    argument_has_inline_prefix || value_has_inline_prefix
}

/// Return whether an argument has a slash comment annotation preceded by a source comma.
pub(crate) fn argument_has_source_separator_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .visit_annotations(argument_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let Annotation::Comment { node, position } = context.annotation(*annotation_id)
                else {
                    return false;
                };

                if !matches!(
                    position,
                    AnnotationPosition::LinePrefix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockPostfix
                ) {
                    return false;
                }

                let comment = context.tree.get::<Comment>(node);
                if comment.style != CommentStyle::Slash {
                    return false;
                }

                previous_non_whitespace_token_before_annotation(context, *annotation_id)
                    .is_some_and(|token| token.token.ty == TokenType::Comma)
            })
        })
        .unwrap_or(false)
}

/// Return whether a call-like expression has static type arguments.
pub(crate) fn call_has_static_arguments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(node_id) {
        Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        _ => false,
    }
}

/// Return whether a call or new expression callee is a cast or satisfies expression.
pub(crate) fn call_like_has_type_binary_callee(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let callee = match context.tree.get(node_id) {
        Expression::Call { left, .. } | Expression::New { left, .. } => *left,
        _ => return false,
    };
    let callee = transparent_inner_expression(context, callee);

    matches!(
        context.tree.get(callee),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    )
}

/// Return whether call arguments are a leading callback with a simple tail.
pub(crate) fn call_has_leading_block_callback_with_simple_tail(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() < 2 {
        return false;
    }

    if !argument_is_block_callback(context, dynamic_arguments[0]) {
        return false;
    }

    if context.has_annotation(call_node_id)
        || dynamic_arguments
            .iter()
            .copied()
            .any(|argument_id| context.has_annotation(argument_id))
    {
        return false;
    }

    dynamic_arguments
        .iter()
        .skip(1)
        .copied()
        .all(|argument_id| {
            !argument_is_block_callback(context, argument_id)
                && !argument_is_collection_literal(context, argument_id)
                && is_trivial_argument(context.tree, context.tree.get(argument_id))
        })
}
