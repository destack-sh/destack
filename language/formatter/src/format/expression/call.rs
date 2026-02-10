use super::super::timing::tags;
use super::*;
use crate::{CachedCallArgumentExpansionProfile, CachedCallArgumentFacts};
use destack_ast::TemplateLiteral;
use destack_fir::write;

const HUG_LAST_INLINE_OVERFLOW_SKIP_MARGIN: usize = 16;

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
    let mut has_first_newline = false;
    let mut current_line_has_content = false;

    for character in between.chars() {
        match character {
            '\r' => {}
            '\n' => {
                if has_first_newline && !current_line_has_content {
                    return true;
                }
                has_first_newline = true;
                current_line_has_content = false;
            }
            c if c.is_whitespace() => {}
            _ => {
                if has_first_newline {
                    current_line_has_content = true;
                }
            }
        }
    }

    false
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
#[derive(Clone, Copy)]
struct CallArgumentExpansionProfile {
    /// The final force expand decision.
    force_expand: bool,
    /// Whether the call has a non blank infix annotation.
    has_call_infix_annotations: bool,
    /// Whether the last argument is a collection literal.
    trailing_collection_argument: bool,
}

impl From<CachedCallArgumentExpansionProfile> for CallArgumentExpansionProfile {
    fn from(cached: CachedCallArgumentExpansionProfile) -> Self {
        Self {
            force_expand: cached.force_expand,
            has_call_infix_annotations: cached.has_call_infix_annotations,
            trailing_collection_argument: cached.trailing_collection_argument,
        }
    }
}

impl From<CallArgumentExpansionProfile> for CachedCallArgumentExpansionProfile {
    fn from(profile: CallArgumentExpansionProfile) -> Self {
        Self {
            force_expand: profile.force_expand,
            has_call_infix_annotations: profile.has_call_infix_annotations,
            trailing_collection_argument: profile.trailing_collection_argument,
        }
    }
}

/// Store one-pass facts for call argument expansion heuristics.
#[derive(Debug, Clone, Copy, Default)]
struct CallArgumentFacts {
    /// Whether any argument has a line comment annotation.
    has_line_comment_annotations: bool,
    /// Whether the last argument is a collection literal.
    trailing_collection_argument: bool,
    /// Whether any argument is a block callback.
    has_block_callback_argument: bool,
    /// Whether the first argument is a block callback.
    first_argument_is_block_callback: bool,
    /// Whether the last argument is a block callback.
    last_argument_is_block_callback: bool,
    /// Whether any non-last, non-callback argument is non-trivial.
    has_non_trivial_non_callback_argument: bool,
    /// Number of block callback arguments before the last argument.
    non_last_block_callback_count: usize,
    /// Index of the first block callback before the last argument.
    non_last_block_callback_index: Option<usize>,
    /// Number of lambda arguments.
    arrow_argument_count: usize,
    /// Number of function expression arguments.
    function_argument_count: usize,
    /// Whether any argument is a spread argument.
    has_spread_argument: bool,
    /// Whether any non-callback argument is complex and non-tree.
    has_complex_non_callback_argument: bool,
}

impl From<CachedCallArgumentFacts> for CallArgumentFacts {
    fn from(cached: CachedCallArgumentFacts) -> Self {
        Self {
            has_line_comment_annotations: cached.has_line_comment_annotations,
            trailing_collection_argument: cached.trailing_collection_argument,
            has_block_callback_argument: cached.has_block_callback_argument,
            first_argument_is_block_callback: cached.first_argument_is_block_callback,
            last_argument_is_block_callback: cached.last_argument_is_block_callback,
            has_non_trivial_non_callback_argument: cached.has_non_trivial_non_callback_argument,
            non_last_block_callback_count: cached.non_last_block_callback_count,
            non_last_block_callback_index: cached.non_last_block_callback_index,
            arrow_argument_count: cached.arrow_argument_count,
            function_argument_count: cached.function_argument_count,
            has_spread_argument: cached.has_spread_argument,
            has_complex_non_callback_argument: cached.has_complex_non_callback_argument,
        }
    }
}

impl From<CallArgumentFacts> for CachedCallArgumentFacts {
    fn from(facts: CallArgumentFacts) -> Self {
        Self {
            has_line_comment_annotations: facts.has_line_comment_annotations,
            trailing_collection_argument: facts.trailing_collection_argument,
            has_block_callback_argument: facts.has_block_callback_argument,
            first_argument_is_block_callback: facts.first_argument_is_block_callback,
            last_argument_is_block_callback: facts.last_argument_is_block_callback,
            has_non_trivial_non_callback_argument: facts.has_non_trivial_non_callback_argument,
            non_last_block_callback_count: facts.non_last_block_callback_count,
            non_last_block_callback_index: facts.non_last_block_callback_index,
            arrow_argument_count: facts.arrow_argument_count,
            function_argument_count: facts.function_argument_count,
            has_spread_argument: facts.has_spread_argument,
            has_complex_non_callback_argument: facts.has_complex_non_callback_argument,
        }
    }
}

/// Store comment and boundary data for call argument layout decisions.
#[derive(Default)]
struct CallArgumentCommentProfile {
    /// Whether any argument has a line comment annotation.
    has_line_comment_annotations: bool,
    /// Whether any argument has a prefix line comment annotation.
    has_prefix_line_comment_annotations: bool,
    /// Whether any non-leading argument has deferred boundary comments.
    has_deferred_inline_boundary_comment: bool,
    /// Deferred boundary comments keyed by argument index.
    deferred_boundary_prefix_annotations: Option<Vec<Vec<LocalNodeId<Annotation>>>>,
}

/// Collect one-pass comment data for call arguments.
fn collect_call_argument_comment_profile(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentCommentProfile {
    let mut has_line_comment_annotations = false;
    let mut has_prefix_line_comment_annotations = false;
    let mut has_deferred_inline_boundary_comment = false;
    let mut deferred_boundary_prefix_annotations = None;

    for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
        let argument_has_annotation = context.has_annotation(argument_id);
        if argument_has_annotation {
            if !has_line_comment_annotations
                && argument_has_line_comment_annotation(context, argument_id)
            {
                has_line_comment_annotations = true;
            }
            if !has_prefix_line_comment_annotations
                && argument_has_prefix_line_comment_annotation(context, argument_id)
            {
                has_prefix_line_comment_annotations = true;
            }
        }

        // only non-leading arguments can attach deferred boundary comments
        if index > 0 && argument_has_annotation {
            let deferred_boundary_comments =
                call_argument_inline_boundary_prefix_annotations(context, argument_id);
            if !deferred_boundary_comments.is_empty() {
                has_deferred_inline_boundary_comment = true;
                let comments_by_index = deferred_boundary_prefix_annotations
                    .get_or_insert_with(|| vec![Vec::new(); dynamic_arguments.len()]);
                comments_by_index[index] = deferred_boundary_comments;
            }
        }
    }

    CallArgumentCommentProfile {
        has_line_comment_annotations,
        has_prefix_line_comment_annotations,
        has_deferred_inline_boundary_comment,
        deferred_boundary_prefix_annotations,
    }
}

/// Collect one-pass facts used by call argument expansion heuristics.
fn collect_call_argument_facts(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_line_comment_annotations_override: Option<bool>,
) -> CallArgumentFacts {
    let mut has_line_comment_annotations = has_line_comment_annotations_override.unwrap_or(false);
    let use_line_comment_override = has_line_comment_annotations_override.is_some();
    let mut trailing_collection_argument = false;
    let mut has_block_callback_argument = false;
    let mut first_argument_is_block_callback = false;
    let mut last_argument_is_block_callback = false;
    let mut has_non_trivial_non_callback_argument = false;
    let mut non_last_block_callback_count = 0;
    let mut non_last_block_callback_index = None;
    let mut arrow_argument_count = 0;
    let mut function_argument_count = 0;
    let mut has_spread_argument = false;
    let mut has_complex_non_callback_argument = false;
    let last_argument_index = dynamic_arguments.len().saturating_sub(1);

    context.increment_counter(
        "profile.call_arguments.layout.scan.arguments",
        dynamic_arguments.len(),
    );

    for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
        if !use_line_comment_override
            && !has_line_comment_annotations
            && context.has_annotation(argument_id)
            && argument_has_line_comment_annotation(context, argument_id)
        {
            has_line_comment_annotations = true;
        }

        let argument = context.tree.get(argument_id);
        let value_id = argument_value_id(context.tree, argument_id);
        let value_id = transparent_inner_expression(context, value_id);
        let value = context.tree.get(value_id);
        let is_spread_argument = matches!(argument, Argument::Spread { .. });
        if is_spread_argument {
            has_spread_argument = true;
        }

        let (is_lambda_argument, is_function_argument, is_block_callback) = match value {
            Expression::Declaration(declaration_id) => match context.tree.get(*declaration_id) {
                Declaration::Function {
                    signature, body, ..
                } => {
                    let is_lambda_argument = signature.kind == FunctionKind::Lambda;
                    let is_function_argument = !is_lambda_argument;
                    let is_block_callback = is_lambda_argument
                        && body.is_some_and(|body_id| {
                            let body_id = transparent_inner_expression(context, body_id);
                            matches!(context.tree.get(body_id), Expression::Block(_))
                        });

                    (is_lambda_argument, is_function_argument, is_block_callback)
                }
                _ => (false, false, false),
            },
            _ => (false, false, false),
        };
        if is_lambda_argument {
            arrow_argument_count += 1;
        }
        if is_function_argument {
            function_argument_count += 1;
        }

        let is_last_argument = index == last_argument_index;

        if is_last_argument {
            trailing_collection_argument = matches!(
                value,
                Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
            );
            last_argument_is_block_callback = is_block_callback;
        }
        if index == 0 {
            first_argument_is_block_callback = is_block_callback;
        }

        if is_block_callback {
            has_block_callback_argument = true;
            if !is_last_argument {
                non_last_block_callback_count += 1;
                if non_last_block_callback_index.is_none() {
                    non_last_block_callback_index = Some(index);
                }
            }
            continue;
        }

        if !is_last_argument && !is_trivial_argument(context.tree, argument) {
            has_non_trivial_non_callback_argument = true;
        }

        if dynamic_arguments.len() > 1 {
            let value_is_tree_expression = matches!(value, Expression::TreeExpression { .. });
            if !value_is_tree_expression && is_complex_argument(context.tree, argument) {
                has_complex_non_callback_argument = true;
            }
        }
    }

    CallArgumentFacts {
        has_line_comment_annotations,
        trailing_collection_argument,
        has_block_callback_argument,
        first_argument_is_block_callback,
        last_argument_is_block_callback,
        has_non_trivial_non_callback_argument,
        non_last_block_callback_count,
        non_last_block_callback_index,
        arrow_argument_count,
        function_argument_count,
        has_spread_argument,
        has_complex_non_callback_argument,
    }
}

/// Return call argument facts with optional per-call caching and line-comment override.
fn resolve_call_argument_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_line_comment_annotations_override: Option<bool>,
) -> CallArgumentFacts {
    let base_facts = if let Some(cached) = context.cached_call_argument_facts(call_node_id) {
        context.increment_counter("profile.call_arguments.layout.cache.hits", 1);
        CallArgumentFacts::from(cached)
    } else {
        context.increment_counter("profile.call_arguments.layout.cache.misses", 1);
        let facts = collect_call_argument_facts(context, dynamic_arguments, None);
        context.cache_call_argument_facts(call_node_id, CachedCallArgumentFacts::from(facts));
        facts
    };

    if let Some(has_line_comment_annotations) = has_line_comment_annotations_override {
        CallArgumentFacts {
            has_line_comment_annotations,
            ..base_facts
        }
    } else {
        base_facts
    }
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

/// Build the expansion profile used for call argument formatting.
fn build_call_argument_expansion_profile(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    mode: CallArgumentExpansionMode,
    has_call_infix_annotations_override: Option<bool>,
    has_line_comment_annotations_override: Option<bool>,
    force_expand_single_long_with_static_arguments_override: Option<bool>,
    force_expand_single_collection_for_type_binary_callee_override: Option<bool>,
    has_leading_block_callback_with_simple_tail_override: Option<bool>,
) -> CallArgumentExpansionProfile {
    context.increment_counter("profile.call_arguments.layout.builds", 1);

    let line_width = usize::from(context.options.line_width);
    let has_call_infix_annotations = has_call_infix_annotations_override
        .unwrap_or_else(|| call_has_non_blank_infix_annotation(context, call_node_id));
    if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];
        let has_line_comment_annotations =
            has_line_comment_annotations_override.unwrap_or_else(|| {
                context.has_annotation(argument_id)
                    && argument_has_line_comment_annotation(context, argument_id)
            });
        let value_id = argument_value_id(context.tree, argument_id);
        let value_id = transparent_inner_expression(context, value_id);
        let trailing_collection_argument = matches!(
            context.tree.get(value_id),
            Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
        );
        let force_expand_jsx = has_multiline_jsx_argument(context.tree, dynamic_arguments);
        let force_expand_single_commented_callback =
            argument_is_block_callback(context, argument_id)
                && (argument_has_comment_annotation(context, argument_id)
                    || has_call_infix_annotations);
        let force_expand_single_multiline_argument = mode == CallArgumentExpansionMode::Regular
            && context.node_has_newline(argument_id)
            && !argument_is_collection_literal(context, argument_id)
            && !argument_is_lambda_expression(context, argument_id)
            && !argument_is_function_expression(context, argument_id)
            && !argument_is_tree_expression(context, argument_id)
            && !argument_is_template_literal(context, argument_id);
        let force_expand_single_long_with_static_arguments =
            force_expand_single_long_with_static_arguments_override.unwrap_or_else(|| {
                call_force_expand_single_long_with_static_arguments(
                    context,
                    call_node_id,
                    dynamic_arguments,
                    mode,
                )
            });
        let force_expand_single_collection_for_type_binary_callee = mode
            == CallArgumentExpansionMode::Regular
            && force_expand_single_collection_for_type_binary_callee_override.unwrap_or_else(
                || {
                    call_force_expand_single_collection_for_type_binary_callee(
                        context,
                        call_node_id,
                        dynamic_arguments,
                    )
                },
            );
        let force_expand = force_expand_jsx
            || has_line_comment_annotations
            || force_expand_single_commented_callback
            || force_expand_single_multiline_argument
            || force_expand_single_long_with_static_arguments
            || force_expand_single_collection_for_type_binary_callee
            || (mode == CallArgumentExpansionMode::Regular && has_call_infix_annotations);

        return CallArgumentExpansionProfile {
            force_expand,
            has_call_infix_annotations,
            trailing_collection_argument,
        };
    }

    // many short unannotated argument lists can skip the full expansion classifier
    let short_argument_len_limit = (line_width / 5).max(8);
    let can_use_simple_multi_argument_fast_path = dynamic_arguments.len() <= 3
        && !has_call_infix_annotations
        && dynamic_arguments.iter().copied().all(|argument_id| {
            !context.has_annotation(argument_id)
                && !context.node_has_newline(argument_id)
                && argument_is_simple_with_options(
                    context,
                    argument_id,
                    ArgumentSimplicityOptions {
                        reject_any_argument_annotation: true,
                        reject_non_blank_argument_annotation: true,
                        reject_value_annotation: true,
                        reject_lambda_values: true,
                        max_value_len: short_argument_len_limit,
                    },
                )
        });
    if can_use_simple_multi_argument_fast_path {
        context.increment_counter("profile.call_arguments.layout.simple_fast_path", 1);
        return CallArgumentExpansionProfile {
            force_expand: false,
            has_call_infix_annotations,
            trailing_collection_argument: false,
        };
    }

    let argument_facts = resolve_call_argument_facts(
        context,
        call_node_id,
        dynamic_arguments,
        has_line_comment_annotations_override,
    );
    let has_line_comment_annotations = argument_facts.has_line_comment_annotations;
    let force_expand_jsx = has_multiline_jsx_argument(context.tree, dynamic_arguments);
    let trailing_collection_argument = argument_facts.trailing_collection_argument;
    let has_block_callback_argument = argument_facts.has_block_callback_argument;
    let last_argument_is_block_callback = argument_facts.last_argument_is_block_callback;
    let first_argument_is_block_callback = argument_facts.first_argument_is_block_callback;
    let has_non_trivial_non_callback_argument =
        argument_facts.has_non_trivial_non_callback_argument;
    let non_last_block_callback_count = argument_facts.non_last_block_callback_count;
    let non_last_block_callback_index = argument_facts.non_last_block_callback_index;
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
        has_leading_block_callback_with_simple_tail_override.unwrap_or_else(|| {
            call_has_leading_block_callback_with_simple_tail(
                context,
                call_node_id,
                dynamic_arguments,
            )
        });
    let should_expand_for_block_callback = dynamic_arguments.len() > 1
        && has_block_callback_argument
        && (force_expand_first_block_callback_with_collection_tail
            || has_non_trivial_non_callback_argument
            || (!last_argument_is_block_callback
                && !allow_non_last_block_callback_with_collection_tail
                && !has_leading_block_callback_with_simple_tail));
    let arrow_argument_count = argument_facts.arrow_argument_count;
    let function_argument_count = argument_facts.function_argument_count;
    let has_any_function_argument = arrow_argument_count > 0 || function_argument_count > 0;
    let has_multiple_function_arguments = arrow_argument_count >= 2 || function_argument_count >= 2;
    let has_spread_argument = argument_facts.has_spread_argument;
    let force_expand_multiline_function_composition = mode == CallArgumentExpansionMode::Regular
        && context.node_has_newline(call_node_id)
        && dynamic_arguments.len() >= 3
        && has_any_function_argument
        && !has_spread_argument;
    let force_expand_complex =
        dynamic_arguments.len() > 1 && argument_facts.has_complex_non_callback_argument;
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

/// Resolve regular call argument expansion profile with per call caching.
fn resolve_regular_call_argument_expansion_profile(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
    has_line_comment_annotations: bool,
    force_expand_single_long_with_static_arguments: bool,
    force_expand_single_collection_for_type_binary_callee: bool,
    has_leading_block_callback_with_simple_tail: bool,
) -> CallArgumentExpansionProfile {
    if let Some(cached) = context.cached_call_argument_expansion_profile(call_node_id) {
        context.increment_counter("profile.call_arguments.regular.cache.hits", 1);
        let profile = CallArgumentExpansionProfile::from(cached);
        context.increment_counter(
            if profile.force_expand {
                "profile.call_arguments.regular.force_expand.true"
            } else {
                "profile.call_arguments.regular.force_expand.false"
            },
            1,
        );
        return profile;
    }

    context.increment_counter("profile.call_arguments.regular.cache.misses", 1);
    let profile = build_call_argument_expansion_profile(
        context,
        call_node_id,
        dynamic_arguments,
        CallArgumentExpansionMode::Regular,
        Some(has_call_infix_annotations),
        Some(has_line_comment_annotations),
        Some(force_expand_single_long_with_static_arguments),
        Some(force_expand_single_collection_for_type_binary_callee),
        Some(has_leading_block_callback_with_simple_tail),
    );
    context.increment_counter(
        if profile.force_expand {
            "profile.call_arguments.regular.force_expand.true"
        } else {
            "profile.call_arguments.regular.force_expand.false"
        },
        1,
    );
    context.cache_call_argument_expansion_profile(
        call_node_id,
        CachedCallArgumentExpansionProfile::from(profile),
    );

    profile
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

/// Estimate one-line `callee(arg1, arg2)` length for plain call expressions.
fn call_inline_len_without_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> Option<usize> {
    let Expression::Call { left, .. } = context.tree.get(call_node_id) else {
        return None;
    };
    if call_has_static_arguments(context, call_node_id) {
        return None;
    }

    let callee_len = expression_source_len(context, *left);
    let arguments_len = arguments_rendered_len(context, dynamic_arguments);
    Some(callee_len.saturating_add(arguments_len).saturating_add(2))
}

/// Estimate a compact lower bound for one-line `callee(arg1, arg2)` length.
fn call_inline_min_len_without_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> Option<usize> {
    let Expression::Call { left, .. } = context.tree.get(call_node_id) else {
        return None;
    };
    if call_has_static_arguments(context, call_node_id) {
        return None;
    }

    let callee_span = context.get_span(*left);
    let callee_len = source_min_inline_char_len(context.get_span_str(callee_span));
    let arguments_len = dynamic_arguments
        .iter()
        .fold(0usize, |total_len, argument_id| {
            let argument_span = context.get_span(*argument_id);
            let argument_source = context.get_span_str(argument_span);
            let argument_len = source_min_inline_char_len(argument_source);
            total_len.saturating_add(argument_len)
        });
    let separators_len = dynamic_arguments.len().saturating_sub(1) * 2;

    Some(
        callee_len
            .saturating_add(arguments_len)
            .saturating_add(separators_len)
            .saturating_add(2),
    )
}

/// Format call arguments with list-group awareness.
pub(super) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS);

    // set up the list group id for conditional formatting
    let group_id = f.group_id("call_args");
    let previous_group_id = f.context().current_argument_group_id;
    f.context_mut().current_argument_group_id = Some(group_id);

    let result = (|| {
        let line_width = usize::from(f.context().options.line_width);
        let call_has_static_arguments = call_has_static_arguments(f.context(), call_node_id);
        let has_call_infix_annotations =
            call_has_non_blank_infix_annotation(f.context(), call_node_id);
        let arguments_are_multiline_in_source =
            call_arguments_are_multiline_in_source(f.context(), dynamic_arguments);

        // fast path: small simple argument lists should stay inline
        let simple_inline_max_value_len = if dynamic_arguments.len() <= 3 {
            line_width / 3
        } else {
            line_width / 5
        };
        let use_fast_simple_inline = dynamic_arguments.len() > 1
            && dynamic_arguments.len() <= 4
            && matches!(f.context().tree.get(call_node_id), Expression::Call { .. })
            && !has_call_infix_annotations
            && !arguments_are_multiline_in_source
            && expression_source_len(f.context(), call_node_id) <= line_width
            && dynamic_arguments.iter().copied().all(|argument_id| {
                argument_is_simple_with_options(
                    f.context(),
                    argument_id,
                    ArgumentSimplicityOptions {
                        reject_any_argument_annotation: true,
                        reject_non_blank_argument_annotation: true,
                        reject_value_annotation: true,
                        reject_lambda_values: true,
                        max_value_len: simple_inline_max_value_len,
                    },
                )
            });
        if use_fast_simple_inline {
            f.context()
                .increment_counter("profile.call.arguments.path.fast_simple_inline", 1);
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

        // very common single argument calls can short-circuit before heavier layout profiling
        let use_single_simple_argument_fast_path = dynamic_arguments.len() == 1
            && !call_has_static_arguments
            && !has_call_infix_annotations
            && !arguments_are_multiline_in_source
            && {
                let argument_id = dynamic_arguments[0];
                if f.context().has_annotation(argument_id) {
                    false
                } else {
                    if f.context().node_has_newline(argument_id) {
                        false
                    } else {
                        let value_id = argument_value_id(f.context().tree, argument_id);
                        let value_id = transparent_inner_expression(f.context(), value_id);
                        let value = f.context().tree.get(value_id);
                        is_trivial_expression(f.context().tree, value)
                            && (argument_is_plain_string_literal(f.context(), argument_id)
                                || expression_source_len(f.context(), value_id) <= line_width / 2)
                    }
                }
            };
        if use_single_simple_argument_fast_path {
            f.context()
                .increment_counter("profile.call.arguments.single_simple.fast_path", 1);
            f.context()
                .increment_counter("profile.call.arguments.path.single_simple_fast_path", 1);
            write!(f, [token("("), dynamic_arguments[0], token(")")])?;
            return Ok(());
        }

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
                && !has_call_infix_annotations
                && !force_expand_single_long_with_static_arguments
        } else {
            true
        };
        if can_use_hugged {
            let used_hugged = format_hugged(
                f,
                dynamic_arguments,
                HugOptions::CALL,
                Some(group_id),
                force_hugged_expand,
            )?;
            if used_hugged {
                f.context()
                    .increment_counter("profile.call.arguments.path.hugged", 1);
                return Ok(());
            }
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
                    if f.context().node_has_newline(argument_id) {
                        false
                    } else {
                        let value_id = argument_value_id(f.context().tree, argument_id);
                        let value_id = transparent_inner_expression(f.context(), value_id);
                        let value = f.context().tree.get(value_id);
                        let line_width = usize::from(f.context().options.line_width);
                        is_trivial_expression(f.context().tree, value)
                            && ((argument_is_plain_string_literal(f.context(), argument_id)
                                && !call_has_static_arguments)
                                || expression_source_len(f.context(), value_id) <= line_width / 2)
                    }
                }
            }
        } else {
            false
        };

        if use_single_simple_argument {
            f.context()
                .increment_counter("profile.call.arguments.path.single_simple", 1);
            write!(f, [token("("), dynamic_arguments[0], token(")")])?;
            return Ok(());
        }

        let has_leading_block_callback_with_simple_tail =
            call_has_leading_block_callback_with_simple_tail(
                f.context(),
                call_node_id,
                dynamic_arguments,
            );
        let use_leading_block_callback_inline = has_leading_block_callback_with_simple_tail
            && arguments_rendered_len(f.context(), dynamic_arguments)
                <= line_width.saturating_sub(8);

        if use_leading_block_callback_inline {
            f.context().increment_counter(
                "profile.call.arguments.path.leading_block_callback_inline",
                1,
            );
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

        let has_any_argument_annotation = dynamic_arguments
            .iter()
            .copied()
            .any(|argument_id| f.context().has_annotation(argument_id));
        let comment_profile = if has_any_argument_annotation || has_call_infix_annotations {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_PROFILE);
            f.context()
                .increment_counter("profile.call.arguments.comment_profile.calls", 1);
            collect_call_argument_comment_profile(f.context(), dynamic_arguments)
        } else {
            f.context().increment_counter(
                "profile.call.arguments.comment_profile.skip_no_annotation",
                1,
            );
            CallArgumentCommentProfile::default()
        };
        let has_line_comment_annotations = comment_profile.has_line_comment_annotations;
        let has_prefix_line_comment_annotations =
            comment_profile.has_prefix_line_comment_annotations;
        let has_deferred_inline_boundary_comment =
            comment_profile.has_deferred_inline_boundary_comment;
        if dynamic_arguments.len() > 1
            && (has_line_comment_annotations
                || has_prefix_line_comment_annotations
                || has_deferred_inline_boundary_comment)
        {
            f.context()
                .increment_counter("profile.call.arguments.path.comment_expanded", 1);
            let use_trailing_comma = f.context().options.trailing_comma == TrailingComma::All;
            let deferred_boundary_prefix_annotations = comment_profile
                .deferred_boundary_prefix_annotations
                .as_ref();

            write!(f, [token("("), hard_line_break()])?;
            let format_result = write!(
                f,
                [block_indent(&format_with(
                    |f: &mut DestackFormatter<'ast, '_>| {
                        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                            if index > 0 {
                                let deferred_boundary_comments =
                                    deferred_boundary_prefix_annotations
                                        .and_then(|comments_by_index| comments_by_index.get(index))
                                        .map(|annotations| annotations.as_slice())
                                        .unwrap_or(&[]);
                                for annotation_id in deferred_boundary_comments.iter().copied() {
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
        let expansion_profile = {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_PROFILE);
            resolve_regular_call_argument_expansion_profile(
                f.context(),
                call_node_id,
                dynamic_arguments,
                has_call_infix_annotations,
                has_line_comment_annotations,
                force_expand_single_long_with_static_arguments,
                force_expand_single_collection_for_type_binary_callee,
                has_leading_block_callback_with_simple_tail,
            )
        };
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
        let force_hug_simple_block_lambda_tail = dynamic_arguments.len() <= 3
            && dynamic_arguments
                .last()
                .is_some_and(|argument_id| is_block_lambda_argument(f.context(), *argument_id))
            && !f.context().node_has_newline(call_node_id)
            && dynamic_arguments
                .iter()
                .take(dynamic_arguments.len().saturating_sub(1))
                .copied()
                .all(|argument_id| {
                    !f.context().node_has_newline(argument_id)
                        && !f.context().has_annotation(argument_id)
                        && argument_is_simple_with_options(
                            f.context(),
                            argument_id,
                            ArgumentSimplicityOptions {
                                reject_any_argument_annotation: true,
                                reject_non_blank_argument_annotation: true,
                                reject_value_annotation: true,
                                reject_lambda_values: true,
                                max_value_len: (line_width / 6).max(8),
                            },
                        )
                });
        let force_hug_last_argument = force_hug_test_like_callback
            || force_hug_reference_callback_with_collection_tail
            || force_hug_simple_block_lambda_tail;
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
                f.context()
                    .increment_counter("profile.call.arguments.path.hug_last_forced", 1);
                hug_last_format.format(f)?;
            } else {
                let line_width = usize::from(f.context().options.line_width);
                let call_source_len = f.context().node_span_char_len(call_node_id);
                let all_arguments_are_single_line_and_unannotated =
                    dynamic_arguments.iter().copied().all(|argument_id| {
                        !f.context().node_has_newline(argument_id)
                            && !f.context().has_annotation(argument_id)
                    });
                let inline_call_len = if all_arguments_are_single_line_and_unannotated {
                    call_inline_len_without_static_arguments(
                        f.context(),
                        call_node_id,
                        dynamic_arguments,
                    )
                } else {
                    None
                };
                let inline_call_min_len = if is_expression_chain(f.context().tree, call_node_id) {
                    None
                } else {
                    call_inline_min_len_without_static_arguments(
                        f.context(),
                        call_node_id,
                        dynamic_arguments,
                    )
                };
                let last_argument_id = dynamic_arguments.last().copied();
                let last_argument_is_collection_literal =
                    last_argument_id.is_some_and(|argument_id| {
                        argument_is_collection_literal(f.context(), argument_id)
                    });
                let leading_arguments_are_compact_simple = dynamic_arguments
                    .iter()
                    .take(dynamic_arguments.len().saturating_sub(1))
                    .copied()
                    .all(|argument_id| {
                        !f.context().node_has_newline(argument_id)
                            && !f.context().has_annotation(argument_id)
                            && argument_is_simple_with_options(
                                f.context(),
                                argument_id,
                                ArgumentSimplicityOptions {
                                    reject_any_argument_annotation: true,
                                    reject_non_blank_argument_annotation: true,
                                    reject_value_annotation: true,
                                    reject_lambda_values: true,
                                    max_value_len: (line_width / 5).max(12),
                                },
                            )
                    });
                if all_arguments_are_single_line_and_unannotated
                    && (call_source_len <= line_width
                        || inline_call_len.is_some_and(|inline_len| inline_len <= line_width))
                {
                    f.context()
                        .increment_counter("profile.call.arguments.hug_last.fast_path", 1);
                    f.context()
                        .increment_counter("profile.call.arguments.path.hug_last_fast", 1);
                    hug_last_format.format(f)?;
                } else if inline_call_min_len.is_some_and(|inline_len| inline_len > line_width) {
                    f.context().increment_counter(
                        "profile.call.arguments.hug_last.skip_probe_overflow_min",
                        1,
                    );
                    list_format.format(f)?;
                } else if last_argument_is_collection_literal
                    && leading_arguments_are_compact_simple
                {
                    f.context().increment_counter(
                        "profile.call.arguments.hug_last.skip_probe.collection",
                        1,
                    );
                    list_format.format(f)?;
                } else if inline_call_len.is_some_and(|inline_len| {
                    inline_len > line_width.saturating_add(HUG_LAST_INLINE_OVERFLOW_SKIP_MARGIN)
                }) {
                    f.context().increment_counter(
                        "profile.call.arguments.hug_last.skip_probe_overflow",
                        1,
                    );
                    list_format.format(f)?;
                } else {
                    if let Some(last_argument_id) = last_argument_id {
                        if is_block_lambda_argument(f.context(), last_argument_id) {
                            f.context().increment_counter(
                                "profile.call.arguments.hug_last.best_fit.lambda",
                                1,
                            );
                        } else if argument_is_function_expression(f.context(), last_argument_id) {
                            f.context().increment_counter(
                                "profile.call.arguments.hug_last.best_fit.function",
                                1,
                            );
                        } else if argument_is_object_literal(f.context(), last_argument_id) {
                            f.context().increment_counter(
                                "profile.call.arguments.hug_last.best_fit.object",
                                1,
                            );
                        } else if argument_is_array_literal(f.context(), last_argument_id) {
                            f.context().increment_counter(
                                "profile.call.arguments.hug_last.best_fit.array",
                                1,
                            );
                        }
                    }
                    f.context()
                        .record_best_fitting("best_fitting.expression.call", 2);
                    best_fitting![hug_last_format, list_format].format(f)?;
                }
            }
            return Ok(());
        }

        f.context()
            .increment_counter("profile.call.arguments.path.list_default", 1);
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
        context.with_annotations(expression_id, |annotations| {
            for annotation_id in annotations {
                let Annotation::Comment {
                    node: comment_id,
                    position: annotation_position,
                } = context.tree.get::<Annotation>(*annotation_id)
                else {
                    continue;
                };
                let comment = context.tree.get::<destack_ast::Comment>(*comment_id);
                let annotation_span = context.get_span::<Annotation>(*annotation_id);
                let annotation_source = context.get_span_str(annotation_span).trim().to_string();
                let previous_character =
                    previous_non_whitespace_before_annotation(context, *annotation_id);
                let next_character = next_non_whitespace_after_annotation(context, *annotation_id);

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
        });
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
    context.has_non_blank_infix_annotation(call_node_id)
}

/// Return whether an argument has a non-blank annotation.
pub(super) fn argument_has_non_blank_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context.has_non_blank_annotation(argument_id)
}

/// Return whether an argument has multiline non-blank prefix annotations.
pub(super) fn argument_has_multiline_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !context.node_has_newline(argument_id) {
        return false;
    }

    context
        .with_annotations(argument_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                match context.tree.get::<Annotation>(*annotation_id) {
                    Annotation::Blank { .. } => false,
                    Annotation::Doc { position, .. }
                    | Annotation::Comment { position, .. }
                    | Annotation::Decorator { position, .. } => matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    ),
                }
            })
        })
        .unwrap_or(false)
}

/// Return whether an argument has prefix annotations that start before the argument span.
pub(super) fn argument_has_leading_prefix_annotation_outside_span(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let argument_span = context.get_span(argument_id);

    context
        .with_annotations(argument_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                match context.tree.get::<Annotation>(*annotation_id) {
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
                }
            })
        })
        .unwrap_or(false)
}

/// Return whether an argument has any slash style comment annotation.
pub(super) fn argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .argument_annotation_profile(argument_id)
        .has_line_comment
}

/// Return whether an argument has slash comments in prefix annotation positions.
pub(super) fn argument_has_prefix_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .argument_annotation_profile(argument_id)
        .has_prefix_line_comment
}

/// Return whether an argument has any comment annotation.
pub(super) fn argument_has_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context.argument_annotation_profile(argument_id).has_comment
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
    if let Some(force_expand) =
        context.call_chain_argument_expand_cache.borrow()[call_node_id.id as usize]
    {
        context.increment_counter("profile.call_arguments.chain.cache.hits", 1);
        context.increment_counter(
            if force_expand {
                "profile.call_arguments.chain.force_expand.true"
            } else {
                "profile.call_arguments.chain.force_expand.false"
            },
            1,
        );
        return force_expand;
    }

    context.increment_counter("profile.call_arguments.chain.cache.misses", 1);
    let force_expand = build_call_argument_expansion_profile(
        context,
        call_node_id,
        dynamic_arguments,
        CallArgumentExpansionMode::Chain,
        None,
        None,
        None,
        None,
        None,
    )
    .force_expand;
    context.call_chain_argument_expand_cache.borrow_mut()[call_node_id.id as usize] =
        Some(force_expand);
    context.increment_counter(
        if force_expand {
            "profile.call_arguments.chain.force_expand.true"
        } else {
            "profile.call_arguments.chain.force_expand.false"
        },
        1,
    );

    force_expand
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
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_EMPTY_ARGUMENTS);
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
    let _timing = f.context().timing_scope(tags::FORMAT_EXPRESSION_CALL);

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
