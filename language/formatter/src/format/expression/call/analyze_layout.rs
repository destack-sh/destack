use super::super::*;
use super::analyze::{
    CallArgumentCommentProfile, argument_has_callback_blocking_comment_annotation,
    argument_is_plain_call_argument, call_force_expand_single_collection_for_type_binary_callee,
    call_inline_len_without_static_arguments,
};
use super::classify::*;
use crate::CachedCallArgumentLayoutClass;

/// Return whether one argument is compact, unannotated, and simple.
pub(super) fn argument_is_compact_simple_unannotated(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if let Some(cached) = context.cached_argument_compact_simple_unannotated(argument_id) {
        context.increment_counter("profile.call.arguments.compact_simple.cache.hits", 1);
        return cached;
    }

    context.increment_counter("profile.call.arguments.compact_simple.cache.misses", 1);
    let is_compact_simple_unannotated = !context.has_annotation(argument_id)
        && !context.node_has_newline(argument_id)
        && argument_is_simple_with_options(
            context,
            argument_id,
            ArgumentSimplicityOptions {
                reject_any_argument_annotation: true,
                reject_non_blank_argument_annotation: true,
                reject_value_annotation: true,
                reject_lambda_values: true,
            },
        );
    context.cache_argument_compact_simple_unannotated(argument_id, is_compact_simple_unannotated);

    is_compact_simple_unannotated
}

/// Return whether a call expression is used as the callee/receiver of a parent postfix call chain.
pub(super) fn call_has_call_chain_parent(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(call_node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);
    match parent_expression {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => left.id == call_node_id.id,
        _ => false,
    }
}

/// Return whether a call can use the single-argument fast path before heavy profiling.
#[derive(Clone, Copy)]
pub(super) struct SingleSimpleArgumentFastPathOptions {
    /// The line width budget.
    pub(super) line_width: usize,
    /// Whether the call has static type arguments.
    pub(super) call_has_static_arguments: bool,
    /// Whether the call node has infix annotations.
    pub(super) has_call_infix_annotations: bool,
    /// Whether the single argument should force expansion.
    pub(super) single_argument_force_expand: bool,
}

/// Store one-pass call argument shape data shared by layout decisions.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct CallArgumentShape {
    /// Whether any dynamic argument has annotations.
    pub(super) has_any_argument_annotation: bool,
    /// Whether argument source between first and last spans multiple lines.
    pub(super) is_multiline_in_source: bool,
    /// Whether all dynamic arguments are single-line and unannotated.
    pub(super) all_single_line_and_unannotated: bool,
}

/// Build one call argument shape from cached layout-class facts.
pub(super) fn call_argument_shape_from_layout_class(
    layout_class: CachedCallArgumentLayoutClass,
) -> CallArgumentShape {
    CallArgumentShape {
        has_any_argument_annotation: layout_class.has_any_argument_annotation,
        is_multiline_in_source: layout_class.is_multiline_in_source,
        all_single_line_and_unannotated: layout_class.all_single_line_and_unannotated,
    }
}

/// Store incremental layout facts collected while scanning dynamic arguments.
#[derive(Default)]
struct CallArgumentLayoutScanState {
    has_any_argument_annotation: bool,
    all_single_line_and_unannotated: bool,
    all_compact_simple_unannotated: bool,
    all_plain_call_arguments: bool,
    has_line_comment_annotations: bool,
    trailing_collection_argument: bool,
    has_block_callback_argument: bool,
    first_argument_is_block_callback: bool,
    last_argument_is_block_callback: bool,
    has_non_trivial_non_callback_argument: bool,
    non_last_block_callback_count: usize,
    non_last_block_callback_index: Option<usize>,
    arrow_argument_count: usize,
    function_argument_count: usize,
    has_spread_argument: bool,
    has_complex_non_callback_argument: bool,
}

impl CallArgumentLayoutScanState {
    /// Create a layout scan state with default optimistic flags.
    fn new() -> Self {
        Self {
            all_single_line_and_unannotated: true,
            all_compact_simple_unannotated: true,
            all_plain_call_arguments: true,
            ..Self::default()
        }
    }
}

/// Build one layout class for a call with no dynamic arguments.
fn build_empty_call_argument_layout_class(
    has_call_infix_annotations: bool,
    has_call_chain_parent: bool,
) -> CachedCallArgumentLayoutClass {
    CachedCallArgumentLayoutClass {
        has_call_infix_annotations,
        all_single_line_and_unannotated: true,
        all_compact_simple_unannotated: true,
        all_plain_call_arguments: true,
        has_call_chain_parent,
        ..CachedCallArgumentLayoutClass::default()
    }
}

/// Return callback flags for one argument value.
fn call_argument_callback_flags(
    context: &DestackFormatContext<'_>,
    value: &Expression,
) -> (bool, bool, bool) {
    match value {
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
    }
}

/// Scan one dynamic call argument and update layout facts.
fn scan_call_argument_layout_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    index: usize,
    last_argument_index: usize,
    state: &mut CallArgumentLayoutScanState,
) {
    let has_annotation = context.has_annotation(argument_id);
    let has_newline = context.node_has_newline(argument_id);

    // annotation and single-line shape flags
    if has_annotation {
        state.has_any_argument_annotation = true;
        if !state.has_line_comment_annotations
            && context
                .argument_annotation_profile(argument_id)
                .has_line_comment
        {
            state.has_line_comment_annotations = true;
        }
    }

    let is_single_line_and_unannotated = !has_annotation && !has_newline;
    state.all_single_line_and_unannotated &= is_single_line_and_unannotated;
    if state.all_compact_simple_unannotated {
        state.all_compact_simple_unannotated = if is_single_line_and_unannotated {
            argument_is_compact_simple_unannotated(context, argument_id)
        } else {
            false
        };
    }

    if state.all_plain_call_arguments {
        state.all_plain_call_arguments &= argument_is_plain_call_argument(context, argument_id);
    }

    // argument and value shape flags
    let argument = context.tree.get(argument_id);
    if matches!(argument, Argument::Spread { .. }) {
        state.has_spread_argument = true;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    let value = context.tree.get(value_id);
    let (is_lambda_argument, is_function_argument, is_block_callback) =
        call_argument_callback_flags(context, value);
    if is_lambda_argument {
        state.arrow_argument_count += 1;
    }
    if is_function_argument {
        state.function_argument_count += 1;
    }

    // first and last argument role flags
    let is_last_argument = index == last_argument_index;
    if index == 0 {
        state.first_argument_is_block_callback = is_block_callback;
    }
    if is_last_argument {
        state.trailing_collection_argument = matches!(
            value,
            Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
        );
        state.last_argument_is_block_callback = is_block_callback;
    }

    // callback-tail counters for non-last callback arguments
    if is_block_callback {
        state.has_block_callback_argument = true;
        if !is_last_argument {
            state.non_last_block_callback_count += 1;
            if state.non_last_block_callback_index.is_none() {
                state.non_last_block_callback_index = Some(index);
            }
        }
        return;
    }

    // complexity flags for non-callback arguments
    if !state.has_complex_non_callback_argument
        && !matches!(value, Expression::TreeExpression { .. })
        && is_complex_argument(context.tree, argument)
    {
        state.has_complex_non_callback_argument = true;
    }

    if !state.has_non_trivial_non_callback_argument
        && !is_last_argument
        && !is_trivial_argument(context.tree, argument)
    {
        state.has_non_trivial_non_callback_argument = true;
    }
}

/// Scan dynamic arguments and collect one-pass layout facts.
fn scan_call_argument_layout_state(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentLayoutScanState {
    let mut state = CallArgumentLayoutScanState::new();
    let last_argument_index = dynamic_arguments.len().saturating_sub(1);
    context.increment_counter(
        "profile.call_arguments.layout.scan.arguments",
        dynamic_arguments.len(),
    );

    // scan each argument once and collect layout flags
    for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
        scan_call_argument_layout_argument(
            context,
            argument_id,
            index,
            last_argument_index,
            &mut state,
        );
    }

    state
}

/// Build a cached layout class from scanned argument facts.
fn build_scanned_call_argument_layout_class(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
    has_call_chain_parent: bool,
    state: CallArgumentLayoutScanState,
) -> CachedCallArgumentLayoutClass {
    let force_hug_last_inline = dynamic_arguments.len() > 1
        && !has_call_infix_annotations
        && !state.has_any_argument_annotation
        && state.all_single_line_and_unannotated
        && call_arguments_force_hug_last_inline(
            context,
            call_node_id,
            dynamic_arguments,
            state.trailing_collection_argument,
        );
    let is_multiline_in_source = call_arguments_are_multiline_in_source(context, dynamic_arguments);

    CachedCallArgumentLayoutClass {
        has_call_infix_annotations,
        has_any_argument_annotation: state.has_any_argument_annotation,
        is_multiline_in_source,
        all_single_line_and_unannotated: state.all_single_line_and_unannotated,
        all_compact_simple_unannotated: state.all_compact_simple_unannotated,
        all_plain_call_arguments: state.all_plain_call_arguments,
        has_line_comment_annotations: state.has_line_comment_annotations,
        has_block_callback_argument: state.has_block_callback_argument,
        first_argument_is_block_callback: state.first_argument_is_block_callback,
        last_argument_is_block_callback: state.last_argument_is_block_callback,
        has_non_trivial_non_callback_argument: state.has_non_trivial_non_callback_argument,
        non_last_block_callback_count: state.non_last_block_callback_count,
        non_last_block_callback_index: state.non_last_block_callback_index,
        arrow_argument_count: state.arrow_argument_count,
        function_argument_count: state.function_argument_count,
        has_spread_argument: state.has_spread_argument,
        has_complex_non_callback_argument: state.has_complex_non_callback_argument,
        has_call_chain_parent,
        trailing_collection_argument: state.trailing_collection_argument,
        force_hug_last_inline,
    }
}

/// Resolve one cached call argument layout class.
pub(super) fn resolve_call_argument_layout_class(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CachedCallArgumentLayoutClass {
    if let Some(cached) = context.cached_call_argument_layout_class(call_node_id) {
        context.increment_counter("profile.call.arguments.layout_class.cache.hits", 1);
        return cached;
    }

    context.increment_counter("profile.call.arguments.layout_class.cache.misses", 1);
    context.increment_counter("profile.call.arguments.layout_class.builds", 1);
    let has_call_infix_annotations = call_has_non_blank_infix_annotation(context, call_node_id);
    let has_call_chain_parent = call_has_call_chain_parent(context, call_node_id);

    let layout_class = if dynamic_arguments.is_empty() {
        build_empty_call_argument_layout_class(has_call_infix_annotations, has_call_chain_parent)
    } else {
        let state = scan_call_argument_layout_state(context, dynamic_arguments);
        build_scanned_call_argument_layout_class(
            context,
            call_node_id,
            dynamic_arguments,
            has_call_infix_annotations,
            has_call_chain_parent,
            state,
        )
    };

    context.cache_call_argument_layout_class(call_node_id, layout_class);

    layout_class
}

/// Return the leading dynamic arguments before the last argument.
pub(super) fn leading_dynamic_arguments(
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> &[LocalNodeId<Argument>] {
    if let Some((_, leading_arguments)) = dynamic_arguments.split_last() {
        leading_arguments
    } else {
        &[]
    }
}

/// Return whether a call argument layout class is simple and unannotated.
pub(super) fn call_argument_layout_class_is_simple_multi_unannotated(
    layout_class: CachedCallArgumentLayoutClass,
) -> bool {
    !layout_class.has_call_infix_annotations && layout_class.all_compact_simple_unannotated
}

/// Return whether all leading arguments before the last are compact and simple.
pub(super) fn leading_arguments_are_compact_simple_unannotated(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    leading_dynamic_arguments(dynamic_arguments)
        .iter()
        .copied()
        .all(|argument_id| argument_is_compact_simple_unannotated(context, argument_id))
}

/// Return whether all leading arguments before the last are compact callback-tail candidates.
pub(super) fn leading_arguments_are_compact_callback_tail_candidates(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    leading_dynamic_arguments(dynamic_arguments)
        .iter()
        .copied()
        .all(|argument_id| {
            if context.node_has_newline(argument_id)
                || argument_has_callback_blocking_comment_annotation(context, argument_id)
            {
                return false;
            }

            argument_is_simple_with_options(
                context,
                argument_id,
                ArgumentSimplicityOptions {
                    reject_any_argument_annotation: false,
                    reject_non_blank_argument_annotation: false,
                    reject_value_annotation: true,
                    reject_lambda_values: true,
                },
            )
        })
}

/// Return whether a call can use the single-argument fast path before heavy profiling.
pub(super) fn call_arguments_use_single_simple_argument_fast_path(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    options: SingleSimpleArgumentFastPathOptions,
    shape: CallArgumentShape,
    inline_call_len_without_static_arguments: Option<usize>,
) -> bool {
    if dynamic_arguments.len() != 1
        || options.call_has_static_arguments
        || options.has_call_infix_annotations
        || options.single_argument_force_expand
        || shape.is_multiline_in_source
    {
        return false;
    }

    if inline_call_len_without_static_arguments
        .is_none_or(|inline_len| inline_len > options.line_width)
    {
        return false;
    }

    if !shape.all_single_line_and_unannotated {
        return false;
    }
    let argument_id = dynamic_arguments[0];

    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    let value = context.tree.get(value_id);

    is_trivial_expression(context.tree, value)
}

/// Return whether a single callback argument can stay inline.
pub(super) fn call_arguments_use_single_callback_argument_inline(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
    force_expand_single_long_with_static_arguments: bool,
    force_expand_single_collection_for_type_binary_callee: bool,
) -> bool {
    if dynamic_arguments.len() != 1
        || has_call_infix_annotations
        || force_expand_single_long_with_static_arguments
        || force_expand_single_collection_for_type_binary_callee
    {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    !argument_has_callback_blocking_comment_annotation(context, argument_id)
        && !argument_has_leading_prefix_annotation_outside_span(context, argument_id)
        && (argument_is_lambda_expression(context, argument_id)
            || argument_is_function_expression(context, argument_id))
}

/// Return whether a single simple argument can stay inline.
#[derive(Clone, Copy)]
pub(super) struct SingleSimpleArgumentInlineOptions {
    /// The call expression node id.
    pub(super) call_node_id: LocalNodeId<Expression>,
    /// The line width budget.
    pub(super) line_width: usize,
    /// Whether the call has static type arguments.
    pub(super) call_has_static_arguments: bool,
    /// Whether single long static arguments force expansion.
    pub(super) force_expand_single_long_with_static_arguments: bool,
    /// Whether single collection arguments force expansion for type-binary callees.
    pub(super) force_expand_single_collection_for_type_binary_callee: bool,
    /// Whether the single argument should force expansion.
    pub(super) single_argument_force_expand: bool,
}

/// Return whether a single simple argument can stay inline.
pub(super) fn call_arguments_use_single_simple_argument_inline(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    options: SingleSimpleArgumentInlineOptions,
    shape: CallArgumentShape,
    inline_call_len_without_static_arguments: Option<usize>,
) -> bool {
    if dynamic_arguments.len() != 1
        || options.force_expand_single_long_with_static_arguments
        || options.force_expand_single_collection_for_type_binary_callee
        || options.single_argument_force_expand
        || !shape.all_single_line_and_unannotated
    {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    if argument_has_non_blank_annotation(context, argument_id) {
        return false;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    let value = context.tree.get(value_id);
    let inline_len = if options.call_has_static_arguments {
        expression_source_len(context, options.call_node_id)
    } else {
        inline_call_len_without_static_arguments.unwrap_or(usize::MAX)
    };

    is_trivial_expression(context.tree, value) && inline_len <= options.line_width
}

/// Store shared call argument planning inputs.
#[derive(Clone, Copy)]
pub(super) struct CallArgumentPlannerBaseState {
    /// The configured line width.
    pub(super) line_width: usize,
    /// Whether the call has static type arguments.
    pub(super) call_has_static_arguments: bool,
    /// Whether the call has non-blank infix annotations.
    pub(super) has_call_infix_annotations: bool,
    /// One-pass argument shape facts.
    pub(super) argument_shape: CallArgumentShape,
}

/// Store shared call argument planning inputs.
#[derive(Clone, Copy)]
pub(super) struct CallArgumentPlannerState {
    /// The configured line width.
    pub(super) line_width: usize,
    /// Whether the call has static type arguments.
    pub(super) call_has_static_arguments: bool,
    /// Whether the call has non-blank infix annotations.
    pub(super) has_call_infix_annotations: bool,
    /// Whether the single argument should force expanded list layout.
    pub(super) single_argument_force_expand: bool,
    /// Whether one long single argument with static arguments should force expansion.
    pub(super) force_expand_single_long_with_static_arguments: bool,
    /// Whether one single collection argument in type-binary callee should force expansion.
    pub(super) force_expand_single_collection_for_type_binary_callee: bool,
    /// Estimated one-line call length for plain dynamic calls.
    pub(super) inline_call_len_without_static_arguments: Option<usize>,
    /// One-pass argument shape facts.
    pub(super) argument_shape: CallArgumentShape,
}

/// Resolve and cache the inline call length estimate for dynamic-only calls.
pub(super) fn resolve_inline_call_len_without_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    call_has_static_arguments: bool,
) -> Option<usize> {
    if let Some(cached) = context.cached_call_inline_len_without_static_arguments(call_node_id) {
        context.increment_counter("profile.call.arguments.inline_len.cache.hits", 1);
        return cached;
    }
    context.increment_counter("profile.call.arguments.inline_len.cache.misses", 1);

    let inline_call_len_without_static_arguments = call_inline_len_without_static_arguments(
        context,
        call_node_id,
        call_has_static_arguments,
        dynamic_arguments,
    );
    context.cache_call_inline_len_without_static_arguments(
        call_node_id,
        inline_call_len_without_static_arguments,
    );

    inline_call_len_without_static_arguments
}

/// Store hug-last call argument layout outcomes.
pub(super) enum HugLastCallArgumentLayout {
    /// Keep the argument list inline.
    Inline,
    /// Keep the default list-like layout.
    ListDefault,
}

/// Return whether call arguments can use the hug-last policy.
pub(super) fn can_consider_hug_last_call_arguments(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_line_comment_annotations: bool,
    has_call_infix_annotations: bool,
) -> bool {
    dynamic_arguments.len() > 1
        && !has_line_comment_annotations
        && !has_call_infix_annotations
        && dynamic_arguments.last().is_some_and(|argument_id| {
            is_block_lambda_argument(context, *argument_id)
                || argument_is_object_literal(context, *argument_id)
                || argument_is_array_literal(context, *argument_id)
                || argument_is_function_expression(context, *argument_id)
        })
}

/// Return whether call arguments should force hug-last inline layout.
pub(super) fn call_arguments_force_hug_last_inline(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    trailing_collection_argument: bool,
) -> bool {
    let can_force_hug_test_like_callback = dynamic_arguments.len() == 2;
    let can_force_hug_reference_callback_with_collection_tail = dynamic_arguments.len() >= 3
        && trailing_collection_argument
        && argument_is_reference_like(context, dynamic_arguments[0]);
    let can_force_hug_simple_block_lambda_tail = dynamic_arguments.len() <= 3
        && dynamic_arguments
            .last()
            .is_some_and(|argument_id| is_block_lambda_argument(context, *argument_id))
        && !context.node_has_newline(call_node_id);

    let force_hug_test_like_callback = can_force_hug_test_like_callback
        && call_should_force_hug_test_like_callback(context, call_node_id, dynamic_arguments);
    let force_hug_reference_callback_with_collection_tail =
        can_force_hug_reference_callback_with_collection_tail
            && dynamic_arguments
                .iter()
                .skip(1)
                .take(dynamic_arguments.len().saturating_sub(2))
                .any(|argument_id| {
                    argument_is_lambda_expression(context, *argument_id)
                        || argument_is_function_expression(context, *argument_id)
                });
    let force_hug_simple_block_lambda_tail = can_force_hug_simple_block_lambda_tail
        && leading_arguments_are_compact_simple_unannotated(context, dynamic_arguments);

    force_hug_test_like_callback
        || force_hug_reference_callback_with_collection_tail
        || force_hug_simple_block_lambda_tail
}

/// Return last-argument tail shape flags used by hug-last policy.
fn hug_last_tail_flags(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> (bool, bool) {
    let last_argument_id = dynamic_arguments.last().copied();
    let last_argument_is_collection_literal = last_argument_id
        .is_some_and(|argument_id| argument_is_collection_literal(context, argument_id));
    let last_argument_is_callback_like = last_argument_id.is_some_and(|argument_id| {
        is_block_lambda_argument(context, argument_id)
            || argument_is_function_expression(context, argument_id)
    });

    (
        last_argument_is_collection_literal,
        last_argument_is_callback_like,
    )
}

/// Return whether hug-last can inline by fitting within the configured line width.
fn hug_last_can_inline_by_width(
    force_expand: bool,
    inline_call_len: Option<usize>,
    line_width: usize,
) -> bool {
    !force_expand && inline_call_len.is_some_and(|inline_len| inline_len <= line_width)
}

/// Return whether hug-last can inline callback tails with compact leading arguments.
fn hug_last_can_inline_callback_tail(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    last_argument_is_callback_like: bool,
) -> bool {
    !force_expand
        && last_argument_is_callback_like
        && leading_arguments_are_compact_callback_tail_candidates(context, dynamic_arguments)
}

/// Return whether hug-last can inline overflowing tails when the tail is callback-like or collection-like.
fn hug_last_can_inline_overflow_tail(
    force_expand: bool,
    inline_call_len: Option<usize>,
    line_width: usize,
    last_argument_is_collection_literal: bool,
    last_argument_is_callback_like: bool,
) -> bool {
    !force_expand
        && inline_call_len.is_some_and(|inline_len| inline_len > line_width)
        && (last_argument_is_collection_literal || last_argument_is_callback_like)
}

/// Return whether hug-last should skip probing and keep default list layout for collection tails.
fn hug_last_should_skip_probe_collection(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    trailing_collection_argument: bool,
) -> bool {
    trailing_collection_argument
        && leading_arguments_are_compact_simple_unannotated(context, dynamic_arguments)
}

/// Resolve hug-last call argument layout.
pub(super) fn resolve_hug_last_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    planner_state: CallArgumentPlannerState,
    force_expand: bool,
    trailing_collection_argument: bool,
) -> Option<HugLastCallArgumentLayout> {
    // keep cached inline length and lazily resolve when needed
    let mut inline_call_len_without_static_arguments =
        planner_state.inline_call_len_without_static_arguments;

    let mut resolve_inline_call_len = || {
        if inline_call_len_without_static_arguments.is_none()
            && !planner_state.call_has_static_arguments
        {
            inline_call_len_without_static_arguments =
                resolve_inline_call_len_without_static_arguments(
                    context,
                    call_node_id,
                    dynamic_arguments,
                    planner_state.call_has_static_arguments,
                );
        }

        inline_call_len_without_static_arguments
    };

    // apply forced hug-last policy first
    if call_arguments_force_hug_last_inline(
        context,
        call_node_id,
        dynamic_arguments,
        trailing_collection_argument,
    ) {
        context.increment_counter("profile.call.arguments.path.hug_last_forced", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    // prefer deterministic inline when line fit is known
    if hug_last_can_inline_by_width(
        force_expand,
        resolve_inline_call_len(),
        planner_state.line_width,
    ) {
        context.increment_counter("profile.call.arguments.hug_last.fast_path", 1);
        context.increment_counter("profile.call.arguments.path.hug_last_fast", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    // compute tail shape and try callback-tail inline fallback
    let (last_argument_is_collection_literal, last_argument_is_callback_like) =
        hug_last_tail_flags(context, dynamic_arguments);
    if hug_last_can_inline_callback_tail(
        context,
        dynamic_arguments,
        force_expand,
        last_argument_is_callback_like,
    ) {
        context.increment_counter("profile.call.arguments.hug_last.fallback_inline", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    // allow overflow inline when tail shape is explicitly hug-last-friendly
    if hug_last_can_inline_overflow_tail(
        force_expand,
        resolve_inline_call_len(),
        planner_state.line_width,
        last_argument_is_collection_literal,
        last_argument_is_callback_like,
    ) {
        context.increment_counter("profile.call.arguments.hug_last.overflow_inline", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    // keep list-default when collection tails should skip additional probing
    if hug_last_should_skip_probe_collection(
        context,
        dynamic_arguments,
        trailing_collection_argument,
    ) {
        context.increment_counter("profile.call.arguments.hug_last.skip_probe.collection", 1);
        return Some(HugLastCallArgumentLayout::ListDefault);
    }

    None
}
/// Store planned post-hugged call argument layouts.
pub(super) enum CallArgumentLayoutDecision {
    /// Keep the entire argument list inline.
    InlineAll,
    /// Keep one argument wrapped inline.
    InlineSingle,
    /// Render with explicit comment-expanded multiline argument layout.
    CommentExpanded(CallArgumentCommentProfile),
    /// Render with the default list formatter.
    ListDefault {
        /// Whether the list should expand.
        force_expand: bool,
        /// Whether line comment annotations are present.
        has_line_comment_annotations: bool,
    },
}

/// Store shared state for rendering a decided call argument layout.
#[derive(Clone, Copy)]
pub(super) struct CallArgumentLayoutRenderState {
    /// The active list group id.
    pub(super) group_id: GroupId,
    /// Whether any argument has annotations.
    pub(super) has_any_argument_annotation: bool,
    /// Whether all arguments can use the plain writer.
    pub(super) all_plain_call_arguments: bool,
}

/// Build shared call argument planning base inputs.
pub(super) fn build_call_argument_planner_base_state(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    layout_class: CachedCallArgumentLayoutClass,
) -> CallArgumentPlannerBaseState {
    let call_has_static_arguments = call_has_static_arguments(context, call_node_id);

    CallArgumentPlannerBaseState {
        line_width: usize::from(context.options.line_width),
        call_has_static_arguments,
        has_call_infix_annotations: layout_class.has_call_infix_annotations,
        argument_shape: call_argument_shape_from_layout_class(layout_class),
    }
}

/// Return whether a single collection argument should force expansion for a type-binary callee.
fn planner_force_expand_single_collection_for_type_binary_callee(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 1 {
        return false;
    }

    call_force_expand_single_collection_for_type_binary_callee(
        context,
        call_node_id,
        dynamic_arguments,
    )
}

/// Build full call argument planner state.
pub(super) fn build_call_argument_planner_state(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    base_state: CallArgumentPlannerBaseState,
    single_argument_force_expand: bool,
    force_expand_single_long_with_static_arguments: bool,
    inline_call_len_without_static_arguments: Option<usize>,
) -> CallArgumentPlannerState {
    // single collection arguments in type-binary calls may need forced expansion
    let force_expand_single_collection_for_type_binary_callee =
        planner_force_expand_single_collection_for_type_binary_callee(
            context,
            call_node_id,
            dynamic_arguments,
        );

    // keep planner state construction as one explicit data assembly step
    CallArgumentPlannerState {
        line_width: base_state.line_width,
        call_has_static_arguments: base_state.call_has_static_arguments,
        has_call_infix_annotations: base_state.has_call_infix_annotations,
        single_argument_force_expand,
        force_expand_single_long_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
        inline_call_len_without_static_arguments,
        argument_shape: base_state.argument_shape,
    }
}
