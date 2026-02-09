use super::*;
use destack_ast::TemplateLiteral;
use destack_fir::write;

/// Store shared argument simplicity checks for call and chain classifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ArgumentSimplicityOptions {
    /// Reject any annotations on the argument node.
    pub reject_any_argument_annotation: bool,
    /// Reject non-blank annotations on the argument node.
    pub reject_non_blank_argument_annotation: bool,
    /// Reject annotations on the argument value expression.
    pub reject_value_annotation: bool,
    /// Reject lambda declaration values.
    pub reject_lambda_values: bool,
    /// Set the maximum allowed source length for the argument value.
    pub max_value_len: usize,
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
pub(super) fn argument_is_simple_with_options(
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
        && expression_source_len(context, value_id) <= options.max_value_len
}

/// Return whether an expression appears in call-like argument position.
pub(super) fn is_call_like_argument(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let Some((expression_id, expression_type)) = context.get_parent_by_id(argument_id) else {
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
pub(super) fn is_tree_attribute_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let Some((expression_id, expression_type)) = context.get_parent_by_id(argument_id) else {
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
pub(super) fn is_simple_static_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let line_width = usize::from(context.options.line_width);
    argument_is_simple_with_options(
        context,
        argument_id,
        ArgumentSimplicityOptions {
            reject_any_argument_annotation: false,
            reject_non_blank_argument_annotation: true,
            reject_value_annotation: false,
            reject_lambda_values: false,
            max_value_len: line_width / 2,
        },
    )
}

/// Return whether an argument is a string or template literal.
pub(super) fn argument_is_string_like(
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

/// Return whether an argument is a plain string literal.
pub(super) fn argument_is_plain_string_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    matches!(
        context.tree.get(value_id),
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    )
}

/// Return whether an argument is an interpolated template literal.
fn argument_is_interpolated_template_literal(
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
pub(super) fn argument_is_collection_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_is_object_literal(context, argument_id)
        || argument_is_array_literal(context, argument_id)
}

/// Return whether an argument is a reference style expression.
pub(super) fn argument_is_reference_like(
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
pub(super) fn call_callee_has_test_like_member_name(
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
pub(super) fn call_should_force_hug_test_like_callback(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 2 {
        return false;
    }

    if !call_callee_has_test_like_member_name(context, call_node_id) {
        return false;
    }

    if !argument_is_string_like(context, dynamic_arguments[0]) {
        return false;
    }

    argument_is_lambda_expression(context, dynamic_arguments[1])
        || argument_is_function_expression(context, dynamic_arguments[1])
}

/// Return whether call arguments span multiple lines in source.
pub(super) fn call_arguments_are_multiline_in_source(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first), Some(last)) = (dynamic_arguments.first(), dynamic_arguments.last()) else {
        return false;
    };

    let first_span = context.get_span(*first);
    let last_span = context.get_span(*last);
    if first_span.file != last_span.file || first_span.start >= last_span.end {
        return false;
    }

    context.has_newline(Span::new(first_span.file, first_span.start, last_span.end))
}

/// Return whether source text between two arguments contains an explicit blank line.
pub(super) fn call_arguments_preserve_blank_line_between(
    context: &DestackFormatContext<'_>,
    left_argument_id: LocalNodeId<Argument>,
    right_argument_id: LocalNodeId<Argument>,
) -> bool {
    let left_span = context.get_span(left_argument_id);
    let right_span = context.get_span(right_argument_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    let between = context.get_span_str(Span::new(left_span.file, left_span.end, right_span.start));
    let normalized_between = between.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized_between.split('\n').collect();
    if lines.len() < 3 {
        return false;
    }

    lines
        .iter()
        .skip(1)
        .take(lines.len().saturating_sub(2))
        .any(|line| line.trim().is_empty())
}

/// Select how call argument expansion heuristics should be evaluated.
#[derive(Clone, Copy, Eq, PartialEq)]
enum CallArgumentExpansionMode {
    /// Apply the standalone call formatting heuristics.
    Regular,
    /// Apply chain formatting heuristics.
    Chain,
}

/// Store derived call argument expansion flags.
struct CallArgumentExpansionProfile {
    /// The final force expand decision.
    force_expand: bool,
    /// Whether the call has a non blank infix annotation.
    has_call_infix_annotations: bool,
    /// Whether the last argument is a collection literal.
    trailing_collection_argument: bool,
}

/// Return whether a single static argument call should expand.
fn call_force_expand_single_long_with_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    mode: CallArgumentExpansionMode,
) -> bool {
    if dynamic_arguments.len() != 1
        || !call_has_static_arguments(context, call_node_id)
        || argument_has_non_blank_annotation(context, dynamic_arguments[0])
    {
        return false;
    }

    let line_width = usize::from(context.options.line_width);
    let call_len = expression_source_len(context, call_node_id);
    if call_len > line_width {
        return true;
    }

    mode == CallArgumentExpansionMode::Regular
        && is_expression_chain(context.tree, call_node_id)
        && call_len > line_width / 2
}

/// Return whether a single collection argument should expand for type binary callees.
fn call_force_expand_single_collection_for_type_binary_callee(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    dynamic_arguments.len() == 1
        && argument_is_collection_literal(context, dynamic_arguments[0])
        && call_like_has_type_binary_callee(context, call_node_id)
}

/// Return whether any non callback argument is complex.
fn call_arguments_force_expand_for_complex_non_callback(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    dynamic_arguments.len() > 1
        && dynamic_arguments.iter().any(|argument_id| {
            if argument_is_block_callback(context, *argument_id) {
                return false;
            }

            let value_id = argument_value_id(context.tree, *argument_id);
            let value_id = transparent_inner_expression(context, value_id);
            if matches!(
                context.tree.get(value_id),
                Expression::TreeExpression { .. }
            ) {
                return false;
            }

            let argument = context.tree.get(*argument_id);
            is_complex_argument(context.tree, argument)
        })
}

/// Build the expansion profile used for call argument formatting.
fn build_call_argument_expansion_profile(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    mode: CallArgumentExpansionMode,
    has_line_comment_annotations_override: Option<bool>,
) -> CallArgumentExpansionProfile {
    let line_width = usize::from(context.options.line_width);
    let has_call_infix_annotations = call_has_non_blank_infix_annotation(context, call_node_id);
    let has_line_comment_annotations = has_line_comment_annotations_override.unwrap_or_else(|| {
        dynamic_arguments
            .iter()
            .any(|argument_id| argument_has_line_comment_annotation(context, *argument_id))
    });
    let force_expand_jsx = has_multiline_jsx_argument(context.tree, dynamic_arguments);
    let trailing_collection_argument = dynamic_arguments
        .last()
        .is_some_and(|argument_id| argument_is_collection_literal(context, *argument_id));
    let has_block_callback_argument = dynamic_arguments
        .iter()
        .any(|argument_id| argument_is_block_callback(context, *argument_id));
    let last_argument_is_block_callback = dynamic_arguments
        .last()
        .is_some_and(|argument_id| argument_is_block_callback(context, *argument_id));
    let first_argument_is_block_callback = dynamic_arguments
        .first()
        .is_some_and(|argument_id| argument_is_block_callback(context, *argument_id));
    let has_non_trivial_non_callback_argument = dynamic_arguments
        .iter()
        .take(dynamic_arguments.len().saturating_sub(1))
        .any(|argument_id| {
            let argument = context.tree.get(*argument_id);
            !argument_is_block_callback(context, *argument_id)
                && !is_trivial_argument(context.tree, argument)
        });
    let non_last_block_callback_count = dynamic_arguments
        .iter()
        .take(dynamic_arguments.len().saturating_sub(1))
        .filter(|argument_id| argument_is_block_callback(context, **argument_id))
        .count();
    let non_last_block_callback_index = dynamic_arguments
        .iter()
        .take(dynamic_arguments.len().saturating_sub(1))
        .position(|argument_id| argument_is_block_callback(context, *argument_id));
    let allow_non_last_block_callback_with_collection_tail = !last_argument_is_block_callback
        && trailing_collection_argument
        && non_last_block_callback_count == 1
        && non_last_block_callback_index.is_some_and(|index| index > 0)
        && argument_is_reference_like(context, dynamic_arguments[0])
        && !has_non_trivial_non_callback_argument;
    let force_expand_first_block_callback_with_collection_tail = dynamic_arguments.len() == 2
        && first_argument_is_block_callback
        && trailing_collection_argument;
    let has_leading_block_callback_with_simple_tail =
        call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments);
    let should_expand_for_block_callback = dynamic_arguments.len() > 1
        && has_block_callback_argument
        && (force_expand_first_block_callback_with_collection_tail
            || has_non_trivial_non_callback_argument
            || (!last_argument_is_block_callback
                && !allow_non_last_block_callback_with_collection_tail
                && !has_leading_block_callback_with_simple_tail));
    let arrow_argument_count = dynamic_arguments
        .iter()
        .filter(|argument_id| argument_is_lambda_expression(context, **argument_id))
        .count();
    let function_argument_count = dynamic_arguments
        .iter()
        .filter(|argument_id| argument_is_function_expression(context, **argument_id))
        .count();
    let has_any_function_argument = arrow_argument_count > 0 || function_argument_count > 0;
    let has_multiple_function_arguments = arrow_argument_count >= 2 || function_argument_count >= 2;
    let has_spread_argument = dynamic_arguments
        .iter()
        .any(|argument_id| matches!(context.tree.get(*argument_id), Argument::Spread { .. }));
    let force_expand_multiline_function_composition = mode == CallArgumentExpansionMode::Regular
        && context.has_newline(context.get_span(call_node_id))
        && dynamic_arguments.len() >= 3
        && has_any_function_argument
        && !has_spread_argument;
    let force_expand_single_commented_callback = dynamic_arguments.len() == 1
        && argument_is_block_callback(context, dynamic_arguments[0])
        && (argument_has_comment_annotation(context, dynamic_arguments[0])
            || has_call_infix_annotations);
    let force_expand_single_multiline_argument =
        mode == CallArgumentExpansionMode::Regular && dynamic_arguments.len() == 1 && {
            let argument_id = dynamic_arguments[0];
            context.has_newline(context.get_span(argument_id))
                && !argument_is_collection_literal(context, argument_id)
                && !argument_is_lambda_expression(context, argument_id)
                && !argument_is_function_expression(context, argument_id)
                && !argument_is_tree_expression(context, argument_id)
                && !argument_is_template_literal(context, argument_id)
        };
    let force_expand_single_long_with_static_arguments =
        call_force_expand_single_long_with_static_arguments(
            context,
            call_node_id,
            dynamic_arguments,
            mode,
        );
    let force_expand_single_collection_for_type_binary_callee = mode
        == CallArgumentExpansionMode::Regular
        && call_force_expand_single_collection_for_type_binary_callee(
            context,
            call_node_id,
            dynamic_arguments,
        );
    let force_expand_complex =
        call_arguments_force_expand_for_complex_non_callback(context, dynamic_arguments);
    let force_expand_long = dynamic_arguments.len() > 1 && {
        let arguments_len = arguments_rendered_len(context, dynamic_arguments);
        if mode == CallArgumentExpansionMode::Regular {
            call_has_direct_call_parent(context, call_node_id) && arguments_len >= line_width / 2
        } else {
            arguments_len >= line_width / 2
        }
    };
    let force_expand = force_expand_jsx
        || force_expand_long
        || force_expand_complex
        || has_line_comment_annotations
        || force_expand_single_commented_callback
        || force_expand_single_multiline_argument
        || force_expand_single_long_with_static_arguments
        || force_expand_single_collection_for_type_binary_callee
        || should_expand_for_block_callback
        || has_multiple_function_arguments
        || force_expand_multiline_function_composition
        || (mode == CallArgumentExpansionMode::Regular && has_call_infix_annotations);

    CallArgumentExpansionProfile {
        force_expand,
        has_call_infix_annotations,
        trailing_collection_argument,
    }
}

/// Return whether single argument hugged formatting should force expansion.
fn call_should_force_hugged_expand(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand_single_collection_for_type_binary_callee: bool,
) -> bool {
    if force_expand_single_collection_for_type_binary_callee {
        return true;
    }

    if dynamic_arguments.len() != 1 {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    let is_arrow_argument = matches!(
        context.tree.get(value_id),
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function { signature, .. }
                    if signature.kind == FunctionKind::Lambda
            )
    );
    if !is_arrow_argument {
        return false;
    }

    let threshold = usize::from(context.options.line_width).saturating_sub(1);
    let call_len = expression_source_len(context, call_node_id);
    let call_len = match context.tree.get(call_node_id) {
        Expression::Call { left, .. } => {
            let is_chain_call = matches!(
                context.tree.get(*left),
                Expression::Member { .. }
                    | Expression::PrivateMember { .. }
                    | Expression::Call { .. }
                    | Expression::Index { .. }
                    | Expression::Maybe { .. }
                    | Expression::Must { .. }
            );
            if is_chain_call {
                let left_len = expression_source_len(context, *left);
                call_len.saturating_sub(left_len)
            } else {
                call_len
            }
        }
        _ => call_len,
    };

    call_len >= threshold
}

/// Format call arguments with list-group awareness.
pub(super) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    // set up the list group id for conditional formatting
    let group_id = f.group_id("call_args");
    let previous_group_id = f.context().current_argument_group_id;
    f.context_mut().current_argument_group_id = Some(group_id);

    let result = (|| {
        let line_width = usize::from(f.context().options.line_width);
        let force_expand_single_long_with_static_arguments =
            call_force_expand_single_long_with_static_arguments(
                f.context(),
                call_node_id,
                dynamic_arguments,
                CallArgumentExpansionMode::Regular,
            );
        let force_expand_single_collection_for_type_binary_callee =
            call_force_expand_single_collection_for_type_binary_callee(
                f.context(),
                call_node_id,
                dynamic_arguments,
            );

        let force_hugged_expand = call_should_force_hugged_expand(
            f.context(),
            call_node_id,
            dynamic_arguments,
            force_expand_single_collection_for_type_binary_callee,
        );

        // try hugged format for single object/array arguments
        let can_use_hugged = if dynamic_arguments.len() == 1 {
            !argument_has_multiline_prefix_annotation(f.context(), dynamic_arguments[0])
                && !call_has_non_blank_infix_annotation(f.context(), call_node_id)
                && !force_expand_single_long_with_static_arguments
        } else {
            true
        };
        if can_use_hugged
            && format_hugged(
                f,
                dynamic_arguments,
                HugOptions::CALL,
                Some(group_id),
                force_hugged_expand,
            )?
        {
            return Ok(());
        }

        // keep short single positional arguments inline
        let use_single_simple_argument = if dynamic_arguments.len() == 1 {
            if force_expand_single_long_with_static_arguments
                || force_expand_single_collection_for_type_binary_callee
            {
                false
            } else {
                let argument_id = dynamic_arguments[0];
                if argument_has_non_blank_annotation(f.context(), argument_id) {
                    false
                } else {
                    let argument_span = f.context().get_span(argument_id);
                    if f.context().has_newline(argument_span) {
                        false
                    } else {
                        let value_id = argument_value_id(f.context().tree, argument_id);
                        let value_id = transparent_inner_expression(f.context(), value_id);
                        let value = f.context().tree.get(value_id);
                        let line_width = usize::from(f.context().options.line_width);
                        is_trivial_expression(f.context().tree, value)
                            && ((argument_is_plain_string_literal(f.context(), argument_id)
                                && !call_has_static_arguments(f.context(), call_node_id))
                                || expression_source_len(f.context(), value_id) <= line_width / 2)
                    }
                }
            }
        } else {
            false
        };

        if use_single_simple_argument {
            write!(f, [token("("), dynamic_arguments[0], token(")")])?;
            return Ok(());
        }

        let use_leading_block_callback_inline =
            call_has_leading_block_callback_with_simple_tail(
                f.context(),
                call_node_id,
                dynamic_arguments,
            ) && arguments_rendered_len(f.context(), dynamic_arguments)
                <= line_width.saturating_sub(8);

        if use_leading_block_callback_inline {
            write!(f, [token("(")])?;
            for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [*argument_id])?;
            }
            write!(f, [token(")")])?;
            return Ok(());
        }

        let has_line_comment_annotations = dynamic_arguments
            .iter()
            .any(|argument_id| argument_has_line_comment_annotation(f.context(), *argument_id));
        let has_prefix_line_comment_annotations = dynamic_arguments.iter().any(|argument_id| {
            argument_has_prefix_line_comment_annotation(f.context(), *argument_id)
        });
        let has_deferred_inline_boundary_comment =
            dynamic_arguments
                .iter()
                .skip(1)
                .copied()
                .any(|argument_id| {
                    !call_argument_inline_boundary_prefix_annotations(f.context(), argument_id)
                        .is_empty()
                });
        if dynamic_arguments.len() > 1
            && (has_line_comment_annotations
                || has_prefix_line_comment_annotations
                || has_deferred_inline_boundary_comment)
        {
            let use_trailing_comma = f.context().options.trailing_comma == TrailingComma::All;

            write!(f, [token("("), hard_line_break()])?;
            let format_result = write!(
                f,
                [block_indent(&format_with(
                    |f: &mut DestackFormatter<'ast, '_>| {
                        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                            if index > 0 {
                                let deferred_boundary_comments =
                                    call_argument_inline_boundary_prefix_annotations(
                                        f.context(),
                                        *argument_id,
                                    );
                                for annotation_id in deferred_boundary_comments {
                                    let content = format_with(|f| {
                                        write!(f, [space(), annotation_id])?;
                                        Ok(())
                                    });
                                    write!(f, [line_postfix(&content, 0)])?;
                                }
                                let left_argument_id = dynamic_arguments[index - 1];
                                if call_arguments_preserve_blank_line_between(
                                    f.context(),
                                    left_argument_id,
                                    *argument_id,
                                ) {
                                    write!(f, [empty_line()])?;
                                } else {
                                    write!(f, [hard_line_break()])?;
                                }
                            }

                            write!(f, [group(argument_id)])?;
                            if index + 1 < dynamic_arguments.len() || use_trailing_comma {
                                write!(f, [token(",")])?;
                            }
                        }
                        Ok(())
                    }
                ))]
            );
            format_result?;
            write!(f, [hard_line_break(), token(")")])?;
            return Ok(());
        }

        // decide if the argument list must expand
        let expansion_profile = build_call_argument_expansion_profile(
            f.context(),
            call_node_id,
            dynamic_arguments,
            CallArgumentExpansionMode::Regular,
            Some(has_line_comment_annotations),
        );
        let force_expand = expansion_profile.force_expand;
        let trailing_collection_argument = expansion_profile.trailing_collection_argument;
        let has_call_infix_annotations = expansion_profile.has_call_infix_annotations;

        let has_single_template_literal_argument = dynamic_arguments.len() == 1
            && argument_is_template_literal(f.context(), dynamic_arguments[0]);
        let last_argument_has_line_comment = dynamic_arguments.last().is_some_and(|argument_id| {
            argument_has_line_comment_annotation(f.context(), *argument_id)
        });
        let single_callback_without_leading_prefix = dynamic_arguments.len() == 1
            && (argument_is_lambda_expression(f.context(), dynamic_arguments[0])
                || argument_is_function_expression(f.context(), dynamic_arguments[0]))
            && !argument_has_leading_prefix_annotation_outside_span(
                f.context(),
                dynamic_arguments[0],
            );
        let force_hug_test_like_callback =
            call_should_force_hug_test_like_callback(f.context(), call_node_id, dynamic_arguments);
        let force_hug_reference_callback_with_collection_tail = dynamic_arguments.len() >= 3
            && trailing_collection_argument
            && argument_is_reference_like(f.context(), dynamic_arguments[0])
            && dynamic_arguments
                .iter()
                .skip(1)
                .take(dynamic_arguments.len().saturating_sub(2))
                .any(|argument_id| {
                    argument_is_lambda_expression(f.context(), *argument_id)
                        || argument_is_function_expression(f.context(), *argument_id)
                });
        let force_hug_last_argument =
            force_hug_test_like_callback || force_hug_reference_callback_with_collection_tail;
        let list_format = format_with(|f| {
            let mut list = list_like("(", ")", ",", dynamic_arguments);
            list.with_group_id(Some(group_id))
                .should_expand(force_expand);
            if dynamic_arguments.len() == 1
                && argument_is_collection_literal(f.context(), dynamic_arguments[0])
            {
                list.disallow_trailing_separator();
            }
            if has_single_template_literal_argument
                && !argument_is_interpolated_template_literal(f.context(), dynamic_arguments[0])
            {
                list.disallow_trailing_separator();
            }
            if last_argument_has_line_comment || single_callback_without_leading_prefix {
                list.disallow_trailing_separator();
            }
            write!(f, [list])
        });

        let can_hug_last_argument = (!force_expand || force_hug_last_argument)
            && dynamic_arguments.len() > 1
            && !has_line_comment_annotations
            && !has_call_infix_annotations
            && dynamic_arguments.last().is_some_and(|argument_id| {
                is_block_lambda_argument(f.context(), *argument_id)
                    || argument_is_object_literal(f.context(), *argument_id)
                    || argument_is_array_literal(f.context(), *argument_id)
                    || argument_is_function_expression(f.context(), *argument_id)
            });

        if can_hug_last_argument {
            let hug_last_format = format_with(|f| {
                write!(f, [token("(")])?;

                for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                    if index > 0 {
                        write!(f, [token(","), space()])?;
                    }

                    write!(f, [*argument_id])?;
                }

                write!(f, [token(")")])
            });

            if force_hug_last_argument {
                hug_last_format.format(f)?;
            } else {
                best_fitting![hug_last_format, list_format]
                    .with_mode(BestFittingMode::AllLines)
                    .format(f)?;
            }
            return Ok(());
        }

        list_format.format(f)
    })();

    f.context_mut().current_argument_group_id = previous_group_id;

    result
}

/// Collect deferred callee boundary comments for empty call argument lists.
pub(super) fn collect_deferred_empty_call_boundary_comments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> (Option<String>, Option<String>, Option<String>) {
    let (left, dynamic_arguments) = match context.tree.get(call_node_id) {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => (*left, dynamic_arguments),
        _ => return (None, None, None),
    };

    if !dynamic_arguments.is_empty() {
        return (None, None, None);
    }

    let mut inline_argument_comment: Option<String> = None;
    let mut line_argument_comment: Option<String> = None;
    let mut trailing_optional_comment: Option<String> = None;
    for expression_id in callee_expression_chain_ids(context, left) {
        let Some(annotations) = context.get_annotations(expression_id) else {
            continue;
        };

        for annotation_id in annotations {
            let Annotation::Comment {
                node: comment_id,
                position: annotation_position,
            } = context.tree.get::<Annotation>(annotation_id)
            else {
                continue;
            };
            let comment = context.tree.get::<destack_ast::Comment>(*comment_id);
            let annotation_span = context.get_span::<Annotation>(annotation_id);
            let annotation_source = context.get_span_str(annotation_span).trim().to_string();
            let previous_character =
                previous_non_whitespace_before_annotation(context, annotation_id);
            let next_character = next_non_whitespace_after_annotation(context, annotation_id);

            if comment.style == destack_ast::CommentStyle::Star
                && *annotation_position == AnnotationPosition::BlockPostfix
                && previous_character == Some('(')
                && next_character == Some(')')
            {
                inline_argument_comment = Some(annotation_source);
            } else if comment.style == destack_ast::CommentStyle::Slash
                && matches!(
                    *annotation_position,
                    AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockPostfix
                )
                && previous_character == Some('(')
                && next_character == Some(')')
            {
                line_argument_comment = Some(annotation_source);
            } else if comment.style == destack_ast::CommentStyle::Slash
                && matches!(
                    *annotation_position,
                    AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
                )
                && next_character == Some('?')
            {
                trailing_optional_comment = Some(annotation_source);
            }
        }
    }

    (
        inline_argument_comment,
        line_argument_comment,
        trailing_optional_comment,
    )
}

/// Collect callee chain expression ids where boundary comments may be attached.
pub(super) fn callee_expression_chain_ids(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Expression>> {
    let mut result = vec![left_id];
    let mut current_id = left_id;

    loop {
        let next_id = match context.tree.get(current_id) {
            Expression::Maybe { left, .. } | Expression::Must { left, .. } => Some(*left),
            Expression::Parenthesized { expression } => Some(*expression),
            _ => None,
        };

        let Some(next_id) = next_id else {
            break;
        };

        result.push(next_id);
        current_id = next_id;
    }

    result
}

/// Return the enclosing empty call expression for a callee expression chain.
pub(super) fn enclosing_empty_call_id_for_callee_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    if matches!(
        context.tree.get(expression_id),
        Expression::Call {
            dynamic_arguments, ..
        } | Expression::New {
            dynamic_arguments, ..
        } if dynamic_arguments.is_empty()
    ) {
        return Some(expression_id);
    }

    let mut current_id = expression_id;
    loop {
        let (parent_id, parent_type) = context.get_parent_by_id(current_id.id)?;
        if parent_type != NodeType::Expression {
            return None;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get(parent_id) {
            Expression::Call {
                left,
                dynamic_arguments,
                ..
            }
            | Expression::New {
                left,
                dynamic_arguments,
                ..
            } => {
                if *left != current_id || !dynamic_arguments.is_empty() {
                    return None;
                }

                return Some(parent_id);
            }
            Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
                if *left != current_id {
                    return None;
                }

                current_id = parent_id;
            }
            Expression::Parenthesized { expression } => {
                if *expression != current_id {
                    return None;
                }

                current_id = parent_id;
            }
            _ => return None,
        }
    }
}

/// Return whether an annotation is deferred to call rendering for empty call boundaries.
pub(super) fn is_deferred_empty_call_boundary_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    annotation_id: LocalNodeId<Annotation>,
    annotation_position: AnnotationPosition,
) -> bool {
    if enclosing_empty_call_id_for_callee_expression(context, expression_id).is_none() {
        return false;
    }

    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };
    let comment = context.tree.get::<destack_ast::Comment>(*node);
    let previous_character = previous_non_whitespace_before_annotation(context, annotation_id);
    let next_character = next_non_whitespace_after_annotation(context, annotation_id);

    if comment.style == destack_ast::CommentStyle::Star
        && matches!(
            annotation_position,
            AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockPostfix
        )
        && ((previous_character == Some('(') && next_character == Some(')'))
            || next_character == Some('?'))
    {
        return true;
    }

    comment.style == destack_ast::CommentStyle::Slash
        && matches!(
            annotation_position,
            AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockPostfix
        )
        && (previous_character == Some('(') && next_character == Some(')')
            || next_character == Some('?'))
}

/// Return whether an expression participates in a deferred empty call boundary comment chain.
pub(super) fn expression_is_in_deferred_empty_call_boundary_chain(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(call_id) = enclosing_empty_call_id_for_callee_expression(context, expression_id)
    else {
        return false;
    };
    let (inline_argument_comment, line_argument_comment, trailing_optional_comment) =
        collect_deferred_empty_call_boundary_comments(context, call_id);

    inline_argument_comment.is_some()
        || line_argument_comment.is_some()
        || trailing_optional_comment.is_some()
}

/// Return whether a call has a non-blank block infix annotation.
pub(super) fn call_has_non_blank_infix_annotation(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.get_annotations(call_node_id) else {
        return false;
    };

    annotations.iter().any(
        |annotation_id| match context.tree.get::<Annotation>(*annotation_id) {
            Annotation::Blank { .. } => false,
            Annotation::Doc { position, .. }
            | Annotation::Comment { position, .. }
            | Annotation::Decorator { position, .. } => *position == AnnotationPosition::BlockInfix,
        },
    )
}

/// Return whether an argument has a non-blank annotation.
pub(super) fn argument_has_non_blank_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        !matches!(
            context.tree.get::<Annotation>(*annotation_id),
            Annotation::Blank { .. }
        )
    })
}

/// Return whether an argument has multiline non-blank prefix annotations.
pub(super) fn argument_has_multiline_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    let argument_span = context.get_span(argument_id);
    if !context.has_newline(argument_span) {
        return false;
    }

    annotations.iter().any(
        |annotation_id| match context.tree.get::<Annotation>(*annotation_id) {
            Annotation::Blank { .. } => false,
            Annotation::Doc { position, .. }
            | Annotation::Comment { position, .. }
            | Annotation::Decorator { position, .. } => matches!(
                position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ),
        },
    )
}

/// Return whether an argument has prefix annotations that start before the argument span.
pub(super) fn argument_has_leading_prefix_annotation_outside_span(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };
    let argument_span = context.get_span(argument_id);

    annotations.iter().any(
        |annotation_id| match context.tree.get::<Annotation>(*annotation_id) {
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
                let annotation_span = context.get_span::<Annotation>(*annotation_id);
                annotation_span.start < argument_span.start
            }
        },
    )
}

/// Return whether an argument has any slash style comment annotation.
pub(super) fn argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };
    let argument_span = context.get_span(argument_id);

    annotations.iter().any(|annotation_id| {
        let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(*annotation_id)
        else {
            return false;
        };
        let comment = context.tree.get::<destack_ast::Comment>(*node);
        if comment.style != destack_ast::CommentStyle::Slash {
            return false;
        }

        let annotation_span = context.get_span::<Annotation>(*annotation_id);
        annotation_span.start >= argument_span.end
    })
}

/// Return whether an argument has slash comments in prefix annotation positions.
pub(super) fn argument_has_prefix_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.tree.get::<Annotation>(*annotation_id)
        else {
            return false;
        };
        let comment = context.tree.get::<destack_ast::Comment>(*node);
        comment.style == destack_ast::CommentStyle::Slash
            && matches!(
                position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            )
    })
}

/// Return whether an argument has any comment annotation.
pub(super) fn argument_has_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        matches!(
            context.tree.get::<Annotation>(*annotation_id),
            Annotation::Comment { .. }
        )
    })
}

/// Return whether a call-like expression has static type arguments.
pub(super) fn call_has_static_arguments(
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
pub(super) fn call_like_has_type_binary_callee(
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

/// Return whether this call expression is immediately invoked by a parent call.
pub(super) fn call_has_direct_call_parent(
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
    let Expression::Call { left, position, .. } = context.tree.get(parent_id) else {
        return false;
    };

    *left == node_id && *position == PostfixPosition::Direct
}

/// Return whether call arguments are a leading callback with a simple tail.
pub(super) fn call_has_leading_block_callback_with_simple_tail(
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

/// Return whether a call should expand its argument list when formatted in a chain.
pub(super) fn call_arguments_force_expand_for_chain(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    build_call_argument_expansion_profile(
        context,
        call_node_id,
        dynamic_arguments,
        CallArgumentExpansionMode::Chain,
        None,
    )
    .force_expand
}

/// Format call dynamic arguments while honoring deferred callee boundary comments.
pub(super) fn format_call_dynamic_arguments_with_deferred_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    let (inline_argument_comment, line_argument_comment, trailing_optional_comment) =
        collect_deferred_empty_call_boundary_comments(f.context(), call_node_id);

    if dynamic_arguments.is_empty() {
        if let Some(comment) = line_argument_comment {
            let content = format_with(|f| write!(f, [hard_line_break(), text(comment.as_str())]));
            write!(
                f,
                [token("("), indent(&content), hard_line_break(), token(")")]
            )?;
        } else if let Some(comment) = inline_argument_comment {
            write!(f, [token("("), text(comment.as_str()), token(")")])?;
        } else {
            write!(f, [token("("), token(")")])?;
        }
    } else {
        format_call_arguments(f, call_node_id, dynamic_arguments)?;
    }

    if let Some(comment) = trailing_optional_comment {
        let content = format_with(|f| write!(f, [space(), text(comment.as_str())]));
        write!(f, [line_postfix(&content, 0)])?;
    }

    Ok(())
}

/// Format a call expression without considering chaining.
/// Format a call expression.
#[inline]
pub(super) fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Call {
        position,
        left,
        static_arguments,
        dynamic_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(static_arguments) = static_arguments {
            format_static_argument_list(f, static_arguments)?;
        }

        format_call_dynamic_arguments_with_deferred_comments(f, node_id, dynamic_arguments)?;
    } else {
        debug_assert!(false, "unexpected expression kind for call formatter");
    }
    Ok(())
}

/// Format an instantiation expression without considering chaining.
/// Format an instantiation expression.
#[inline]
pub(super) fn format_instantiation_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Instantiation {
        left,
        static_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        format_static_argument_list(f, static_arguments)?;
    } else {
        debug_assert!(
            false,
            "unexpected expression kind for instantiation formatter"
        );
    }
    Ok(())
}
