use super::analyze::{
    argument_has_callback_blocking_comment_annotation, argument_is_plain_call_argument,
    call_force_expand_single_collection_for_type_binary_callee,
    call_inline_width_hint_without_static_arguments,
};
use super::classify::{
    ArgumentSimplicityOptions, argument_has_leading_prefix_annotation_outside_span,
    argument_has_non_blank_annotation, argument_is_collection_literal, argument_is_reference_like,
    argument_is_simple_with_options, call_arguments_are_multiline_in_source,
    call_has_non_blank_infix_annotation, call_has_static_arguments,
    call_should_force_hug_test_like_callback,
};
use crate::CallArgumentLayoutFacts;
use crate::expression::{
    Argument, CallArgumentCommentProfile, Declaration, DestackFormatContext, Expression,
    FunctionKind, GroupId, LocalNodeId, NodeTree, NodeType, argument_is_array_literal,
    argument_is_function_expression, argument_is_lambda_expression, argument_is_object_literal,
    argument_value_id, is_block_lambda_argument, is_complex_argument, is_trivial_argument,
    is_trivial_expression, transparent_inner_expression,
};

/// Return whether one argument is compact, unannotated, and simple.
pub(crate) fn argument_is_compact_simple_unannotated(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if let Some(cached) = context.lookup_argument_compact_simple_unannotated(argument_id) {
        context.increment_counter("call.arguments.compact_simple.cache.hits", 1);
        return cached;
    }

    context.increment_counter("call.arguments.compact_simple.cache.misses", 1);
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
    context.store_argument_compact_simple_unannotated(argument_id, is_compact_simple_unannotated);

    is_compact_simple_unannotated
}

/// Return whether a call expression is used as the callee/receiver of a parent postfix call chain.
pub(crate) fn call_has_call_chain_parent(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(call_node_id) else {
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

/// Return whether a call can short-circuit to single-argument inline layout.
#[derive(Clone, Copy)]
pub(crate) struct SingleSimpleArgumentShortCircuitOptions {
    /// The line width budget.
    pub(crate) line_width: usize,
    /// Whether the call has static type arguments.
    pub(crate) call_has_static_arguments: bool,
    /// Whether the call node has infix annotations.
    pub(crate) has_call_infix_annotations: bool,
    /// Whether the single argument should force expansion.
    pub(crate) single_argument_force_expand: bool,
}

/// Store one-pass call argument shape data shared by layout decisions.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CallArgumentShape {
    /// Whether any dynamic argument has annotations.
    pub(crate) has_any_argument_annotation: bool,
    /// Whether argument source between first and last spans multiple lines.
    pub(crate) is_multiline_in_source: bool,
    /// Whether all dynamic arguments are single-line and unannotated.
    pub(crate) all_single_line_and_unannotated: bool,
}

/// Build one call argument shape from cached layout-class facts.
pub(crate) fn call_argument_shape_from_layout_facts(
    layout_facts: CallArgumentLayoutFacts,
) -> CallArgumentShape {
    CallArgumentShape {
        has_any_argument_annotation: layout_facts.has_any_argument_annotation,
        is_multiline_in_source: layout_facts.is_multiline_in_source,
        all_single_line_and_unannotated: layout_facts.all_single_line_and_unannotated,
    }
}

/// Store annotation-shape facts collected while scanning dynamic arguments.
#[derive(Default)]
struct CallArgumentAnnotationScanState {
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    all_single_line_and_unannotated: bool,
    all_compact_simple_unannotated: bool,
}

impl CallArgumentAnnotationScanState {
    /// Create an annotation scan state with optimistic defaults.
    fn new() -> Self {
        Self {
            all_single_line_and_unannotated: true,
            all_compact_simple_unannotated: true,
            ..Self::default()
        }
    }

    /// Observe one argument's annotation and newline surface.
    fn observe_argument_surface(
        &mut self,
        context: &DestackFormatContext<'_>,
        argument_id: LocalNodeId<Argument>,
        has_annotation: bool,
        has_newline: bool,
    ) {
        if has_annotation {
            self.has_any_argument_annotation = true;
            if !self.has_line_comment_annotations
                && context
                    .ensure_argument_annotation_facts(argument_id)
                    .has_line_comment
            {
                self.has_line_comment_annotations = true;
            }
        }

        let is_single_line_and_unannotated = !has_annotation && !has_newline;
        self.all_single_line_and_unannotated &= is_single_line_and_unannotated;
        if self.all_compact_simple_unannotated {
            self.all_compact_simple_unannotated = if is_single_line_and_unannotated {
                argument_is_compact_simple_unannotated(context, argument_id)
            } else {
                false
            };
        }
    }
}

/// Store callback-shape facts collected while scanning dynamic arguments.
#[derive(Default)]
struct CallArgumentCallbackScanState {
    has_block_callback_argument: bool,
    first_argument_is_block_callback: bool,
    last_argument_is_block_callback: bool,
    non_last_block_callback_count: usize,
    non_last_block_callback_index: Option<usize>,
    arrow_argument_count: usize,
    function_argument_count: usize,
}

impl CallArgumentCallbackScanState {
    /// Observe callback flags for one argument position.
    fn observe_argument_callback_flags(
        &mut self,
        argument_index: usize,
        is_last_argument: bool,
        flags: CallArgumentCallbackFlags,
    ) {
        if flags.is_lambda_argument {
            self.arrow_argument_count += 1;
        }
        if flags.is_function_argument {
            self.function_argument_count += 1;
        }

        if argument_index == 0 {
            self.first_argument_is_block_callback = flags.is_block_callback;
        }
        if is_last_argument {
            self.last_argument_is_block_callback = flags.is_block_callback;
        }

        if flags.is_block_callback {
            self.has_block_callback_argument = true;
            if !is_last_argument {
                self.non_last_block_callback_count += 1;
                if self.non_last_block_callback_index.is_none() {
                    self.non_last_block_callback_index = Some(argument_index);
                }
            }
        }
    }
}

/// Store non-callback and argument-shape facts collected while scanning dynamic arguments.
#[derive(Default)]
struct CallArgumentComplexityScanState {
    all_plain_call_arguments: bool,
    trailing_collection_argument: bool,
    has_non_trivial_non_callback_argument: bool,
    has_spread_argument: bool,
    has_complex_non_callback_argument: bool,
}

impl CallArgumentComplexityScanState {
    /// Create a complexity scan state with optimistic defaults.
    fn new() -> Self {
        Self {
            all_plain_call_arguments: true,
            ..Self::default()
        }
    }

    /// Observe argument-level shape for one argument.
    fn observe_argument_shape(
        &mut self,
        context: &DestackFormatContext<'_>,
        argument_id: LocalNodeId<Argument>,
        argument: &Argument,
        argument_value: &Expression,
        is_last_argument: bool,
    ) {
        if self.all_plain_call_arguments {
            self.all_plain_call_arguments &= argument_is_plain_call_argument(context, argument_id);
        }

        if matches!(argument, Argument::Spread { .. }) {
            self.has_spread_argument = true;
        }

        if is_last_argument {
            self.trailing_collection_argument = matches!(
                argument_value,
                Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
            );
        }
    }

    /// Observe non-callback complexity for one argument.
    fn observe_non_callback_complexity(
        &mut self,
        tree: &NodeTree,
        argument: &Argument,
        argument_value: &Expression,
        is_last_argument: bool,
    ) {
        if !self.has_complex_non_callback_argument
            && !matches!(argument_value, Expression::TreeExpression { .. })
            && is_complex_argument(tree, argument)
        {
            self.has_complex_non_callback_argument = true;
        }

        if !self.has_non_trivial_non_callback_argument
            && !is_last_argument
            && !is_trivial_argument(tree, argument)
        {
            self.has_non_trivial_non_callback_argument = true;
        }
    }
}

/// Store incremental layout facts collected while scanning dynamic arguments.
#[derive(Default)]
struct CallArgumentLayoutScanState {
    annotation: CallArgumentAnnotationScanState,
    callback: CallArgumentCallbackScanState,
    complexity: CallArgumentComplexityScanState,
}

impl CallArgumentLayoutScanState {
    /// Create a layout scan state with optimistic defaults.
    fn new() -> Self {
        Self {
            annotation: CallArgumentAnnotationScanState::new(),
            complexity: CallArgumentComplexityScanState::new(),
            ..Self::default()
        }
    }
}

/// Build one layout class for a call with no dynamic arguments.
fn build_empty_call_argument_layout_facts(
    has_call_infix_annotations: bool,
    has_call_chain_parent: bool,
) -> CallArgumentLayoutFacts {
    CallArgumentLayoutFacts {
        has_call_infix_annotations,
        all_single_line_and_unannotated: true,
        all_compact_simple_unannotated: true,
        all_plain_call_arguments: true,
        has_call_chain_parent,
        ..CallArgumentLayoutFacts::default()
    }
}

/// Store callback flags for one argument value.
#[derive(Clone, Copy, Debug, Default)]
struct CallArgumentCallbackFlags {
    is_lambda_argument: bool,
    is_function_argument: bool,
    is_block_callback: bool,
}

/// Return callback flags for one argument value.
fn call_argument_callback_flags(
    context: &DestackFormatContext<'_>,
    value: &Expression,
) -> CallArgumentCallbackFlags {
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

                CallArgumentCallbackFlags {
                    is_lambda_argument,
                    is_function_argument,
                    is_block_callback,
                }
            }
            _ => CallArgumentCallbackFlags::default(),
        },
        _ => CallArgumentCallbackFlags::default(),
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
    state
        .annotation
        .observe_argument_surface(context, argument_id, has_annotation, has_newline);

    let argument = context.tree.get(argument_id);
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    let value = context.tree.get(value_id);
    let is_last_argument = index == last_argument_index;
    let callback_flags = call_argument_callback_flags(context, value);
    state
        .callback
        .observe_argument_callback_flags(index, is_last_argument, callback_flags);
    state.complexity.observe_argument_shape(
        context,
        argument_id,
        argument,
        value,
        is_last_argument,
    );
    if callback_flags.is_block_callback {
        return;
    }

    state.complexity.observe_non_callback_complexity(
        context.tree,
        argument,
        value,
        is_last_argument,
    );
}

/// Scan dynamic arguments and collect one-pass layout facts.
fn scan_call_argument_layout_state(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentLayoutScanState {
    let mut state = CallArgumentLayoutScanState::new();
    let last_argument_index = dynamic_arguments.len().saturating_sub(1);
    context.increment_counter(
        "call.arguments.layout.scan.arguments",
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
fn build_scanned_call_argument_layout_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
    has_call_chain_parent: bool,
    state: CallArgumentLayoutScanState,
) -> CallArgumentLayoutFacts {
    let force_hug_last_inline = dynamic_arguments.len() > 1
        && !has_call_infix_annotations
        && !state.annotation.has_any_argument_annotation
        && state.annotation.all_single_line_and_unannotated
        && call_arguments_force_hug_last_inline(
            context,
            call_node_id,
            dynamic_arguments,
            state.complexity.trailing_collection_argument,
        );
    let is_multiline_in_source = call_arguments_are_multiline_in_source(context, dynamic_arguments);

    CallArgumentLayoutFacts {
        has_call_infix_annotations,
        has_any_argument_annotation: state.annotation.has_any_argument_annotation,
        is_multiline_in_source,
        all_single_line_and_unannotated: state.annotation.all_single_line_and_unannotated,
        all_compact_simple_unannotated: state.annotation.all_compact_simple_unannotated,
        all_plain_call_arguments: state.complexity.all_plain_call_arguments,
        has_line_comment_annotations: state.annotation.has_line_comment_annotations,
        has_block_callback_argument: state.callback.has_block_callback_argument,
        first_argument_is_block_callback: state.callback.first_argument_is_block_callback,
        last_argument_is_block_callback: state.callback.last_argument_is_block_callback,
        has_non_trivial_non_callback_argument: state
            .complexity
            .has_non_trivial_non_callback_argument,
        non_last_block_callback_count: state.callback.non_last_block_callback_count,
        non_last_block_callback_index: state.callback.non_last_block_callback_index,
        arrow_argument_count: state.callback.arrow_argument_count,
        function_argument_count: state.callback.function_argument_count,
        has_spread_argument: state.complexity.has_spread_argument,
        has_complex_non_callback_argument: state.complexity.has_complex_non_callback_argument,
        has_call_chain_parent,
        trailing_collection_argument: state.complexity.trailing_collection_argument,
        force_hug_last_inline,
    }
}

/// Resolve one cached call argument layout class.
pub(crate) fn resolve_call_argument_layout_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentLayoutFacts {
    if let Some(cached) = context.lookup_call_argument_layout_facts(call_node_id) {
        context.increment_counter("call.arguments.layout_facts.cache.hits", 1);
        return cached;
    }

    context.increment_counter("call.arguments.layout_facts.cache.misses", 1);
    context.increment_counter("call.arguments.layout_facts.builds", 1);
    let has_call_infix_annotations = call_has_non_blank_infix_annotation(context, call_node_id);
    let has_call_chain_parent = call_has_call_chain_parent(context, call_node_id);

    let layout_facts = if dynamic_arguments.is_empty() {
        build_empty_call_argument_layout_facts(has_call_infix_annotations, has_call_chain_parent)
    } else {
        let state = scan_call_argument_layout_state(context, dynamic_arguments);
        build_scanned_call_argument_layout_facts(
            context,
            call_node_id,
            dynamic_arguments,
            has_call_infix_annotations,
            has_call_chain_parent,
            state,
        )
    };

    context.store_call_argument_layout_facts(call_node_id, layout_facts);

    layout_facts
}

/// Return the leading dynamic arguments before the last argument.
pub(crate) fn leading_dynamic_arguments(
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> &[LocalNodeId<Argument>] {
    if let Some((_, leading_arguments)) = dynamic_arguments.split_last() {
        leading_arguments
    } else {
        &[]
    }
}

/// Return whether a call argument layout class is simple and unannotated.
pub(crate) fn call_argument_layout_facts_are_simple_multi_unannotated(
    layout_facts: CallArgumentLayoutFacts,
) -> bool {
    !layout_facts.has_call_infix_annotations && layout_facts.all_compact_simple_unannotated
}

/// Return whether all leading arguments before the last are compact and simple.
pub(crate) fn leading_arguments_are_compact_simple_unannotated(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    leading_dynamic_arguments(dynamic_arguments)
        .iter()
        .copied()
        .all(|argument_id| argument_is_compact_simple_unannotated(context, argument_id))
}

/// Return whether all leading arguments before the last are compact callback-tail candidates.
pub(crate) fn leading_arguments_are_compact_callback_tail_candidates(
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

/// Return whether a call can short-circuit to single-argument inline layout.
pub(crate) fn call_arguments_use_single_simple_argument_short_circuit(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    options: SingleSimpleArgumentShortCircuitOptions,
    shape: CallArgumentShape,
    inline_call_width_hint_without_static_arguments: Option<usize>,
) -> bool {
    if dynamic_arguments.len() != 1
        || options.call_has_static_arguments
        || options.has_call_infix_annotations
        || options.single_argument_force_expand
        || shape.is_multiline_in_source
    {
        return false;
    }

    if inline_call_width_hint_without_static_arguments
        .is_none_or(|inline_width_hint| inline_width_hint > options.line_width)
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
pub(crate) fn call_arguments_use_single_callback_argument_inline(
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
    if argument_has_non_blank_annotation(context, argument_id) {
        return false;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    if context.has_non_blank_annotation(value_id) {
        return false;
    }

    !argument_has_callback_blocking_comment_annotation(context, argument_id)
        && !argument_has_leading_prefix_annotation_outside_span(context, argument_id)
        && (argument_is_lambda_expression(context, argument_id)
            || argument_is_function_expression(context, argument_id))
}

/// Return whether a single simple argument can stay inline.
#[derive(Clone, Copy)]
pub(crate) struct SingleSimpleArgumentInlineOptions {
    /// The line width budget.
    pub(crate) line_width: usize,
    /// Whether the call has static type arguments.
    pub(crate) call_has_static_arguments: bool,
    /// Whether single long static arguments force expansion.
    pub(crate) force_expand_single_long_with_static_arguments: bool,
    /// Whether single collection arguments force expansion for type-binary callees.
    pub(crate) force_expand_single_collection_for_type_binary_callee: bool,
    /// Whether the single argument should force expansion.
    pub(crate) single_argument_force_expand: bool,
}

/// Return whether a single simple argument can stay inline.
pub(crate) fn call_arguments_use_single_simple_argument_inline(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    options: SingleSimpleArgumentInlineOptions,
    shape: CallArgumentShape,
    inline_call_width_hint_without_static_arguments: Option<usize>,
) -> bool {
    if dynamic_arguments.len() != 1
        || options.force_expand_single_long_with_static_arguments
        || options.force_expand_single_collection_for_type_binary_callee
        || options.single_argument_force_expand
    {
        return false;
    }

    if shape.has_any_argument_annotation {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    if argument_has_non_blank_annotation(context, argument_id) {
        return false;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    let value = context.tree.get(value_id);
    let can_stay_inline = if options.call_has_static_arguments {
        true
    } else {
        inline_call_width_hint_without_static_arguments
            .is_none_or(|inline_width_hint| inline_width_hint <= options.line_width)
    };

    is_trivial_expression(context.tree, value) && can_stay_inline
}

/// Store shared call argument planning inputs.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentPlannerBaseState {
    /// The configured line width.
    pub(crate) line_width: usize,
    /// Whether the call has static type arguments.
    pub(crate) call_has_static_arguments: bool,
    /// Whether the call has non-blank infix annotations.
    pub(crate) has_call_infix_annotations: bool,
    /// One-pass argument shape facts.
    pub(crate) argument_shape: CallArgumentShape,
}

/// Store shared call argument planning inputs.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentPlannerState {
    /// The configured line width.
    pub(crate) line_width: usize,
    /// Whether the call has static type arguments.
    pub(crate) call_has_static_arguments: bool,
    /// Whether the call has non-blank infix annotations.
    pub(crate) has_call_infix_annotations: bool,
    /// Whether the single argument should force expanded list layout.
    pub(crate) single_argument_force_expand: bool,
    /// Whether one long single argument with static arguments should force expansion.
    pub(crate) force_expand_single_long_with_static_arguments: bool,
    /// Whether one single collection argument in type-binary callee should force expansion.
    pub(crate) force_expand_single_collection_for_type_binary_callee: bool,
    /// Estimated inline call width hint for plain dynamic calls.
    pub(crate) inline_call_width_hint_without_static_arguments: Option<usize>,
    /// One-pass argument shape facts.
    pub(crate) argument_shape: CallArgumentShape,
}

/// Resolve and cache the inline call width hint for dynamic-only calls.
pub(crate) fn resolve_inline_call_width_hint_without_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    call_has_static_arguments: bool,
) -> Option<usize> {
    if let Some(cached) =
        context.lookup_call_inline_width_hint_without_static_arguments(call_node_id)
    {
        context.increment_counter("call.arguments.inline_width_hint.cache.hits", 1);
        return cached;
    }
    context.increment_counter("call.arguments.inline_width_hint.cache.misses", 1);

    let inline_call_width_hint_without_static_arguments =
        call_inline_width_hint_without_static_arguments(
            context,
            call_node_id,
            call_has_static_arguments,
        );
    context.store_call_inline_width_hint_without_static_arguments(
        call_node_id,
        inline_call_width_hint_without_static_arguments,
    );

    inline_call_width_hint_without_static_arguments
}
pub(crate) enum HugLastCallArgumentLayout {
    /// Keep the argument list inline.
    Inline,
}

/// Return whether call arguments can use the hug-last policy.
pub(crate) fn can_consider_hug_last_call_arguments(
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
pub(crate) fn call_arguments_force_hug_last_inline(
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
            .is_some_and(|argument_id| is_block_lambda_argument(context, *argument_id));

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
    inline_call_width_hint: Option<usize>,
    line_width: usize,
) -> bool {
    !force_expand
        && inline_call_width_hint.is_some_and(|inline_width_hint| inline_width_hint <= line_width)
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

/// Resolve hug-last call argument layout.
pub(crate) fn resolve_hug_last_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    planner_state: CallArgumentPlannerState,
    force_expand: bool,
    trailing_collection_argument: bool,
) -> Option<HugLastCallArgumentLayout> {
    // keep cached inline length and lazily resolve when needed
    let mut inline_call_width_hint_without_static_arguments =
        planner_state.inline_call_width_hint_without_static_arguments;

    let mut resolve_inline_call_len = || {
        if inline_call_width_hint_without_static_arguments.is_none()
            && !planner_state.call_has_static_arguments
        {
            inline_call_width_hint_without_static_arguments =
                resolve_inline_call_width_hint_without_static_arguments(
                    context,
                    call_node_id,
                    planner_state.call_has_static_arguments,
                );
        }

        inline_call_width_hint_without_static_arguments
    };

    // apply forced hug-last policy first
    if call_arguments_force_hug_last_inline(
        context,
        call_node_id,
        dynamic_arguments,
        trailing_collection_argument,
    ) {
        context.increment_counter("call.arguments.path.hug_last_forced", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    // prefer explicit width-fit inline when line fit is known
    if hug_last_can_inline_by_width(
        force_expand,
        resolve_inline_call_len(),
        planner_state.line_width,
    ) {
        context.increment_counter("call.arguments.hug_last.width_inline", 1);
        context.increment_counter("call.arguments.path.hug_last.width_inline", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    // allow callback-tail inline when leading arguments stay compact
    let (_, last_argument_is_callback_like) = hug_last_tail_flags(context, dynamic_arguments);
    if hug_last_can_inline_callback_tail(
        context,
        dynamic_arguments,
        force_expand,
        last_argument_is_callback_like,
    ) {
        context.increment_counter("call.arguments.hug_last.callback_tail_inline", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    // keep collection-tail hugging for eligible hug-last calls
    if !force_expand && trailing_collection_argument {
        context.increment_counter("call.arguments.hug_last.collection_tail_inline", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    None
}
/// Store planned post-hugged call argument layouts.
pub(crate) enum CallArgumentLayoutDecision {
    /// Keep the entire argument list inline.
    InlineAll,
    /// Keep one argument wrapped inline.
    InlineSingle,
    /// Keep compact leading arguments inline and expand the trailing collection argument.
    TrailingCollectionExpanded,
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
pub(crate) struct CallArgumentLayoutRenderState {
    /// The call expression node id.
    pub(crate) call_node_id: LocalNodeId<Expression>,
    /// The active list group id.
    pub(crate) group_id: GroupId,
    /// Whether any argument has annotations.
    pub(crate) has_any_argument_annotation: bool,
    /// Whether all arguments can use the plain writer.
    pub(crate) all_plain_call_arguments: bool,
}

/// Build shared call argument planning base inputs.
pub(crate) fn build_call_argument_planner_base_state(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    layout_facts: CallArgumentLayoutFacts,
) -> CallArgumentPlannerBaseState {
    let call_has_static_arguments = call_has_static_arguments(context, call_node_id);

    CallArgumentPlannerBaseState {
        line_width: usize::from(context.options.line_width),
        call_has_static_arguments,
        has_call_infix_annotations: layout_facts.has_call_infix_annotations,
        argument_shape: call_argument_shape_from_layout_facts(layout_facts),
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
pub(crate) fn build_call_argument_planner_state(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    base_state: CallArgumentPlannerBaseState,
    single_argument_force_expand: bool,
    force_expand_single_long_with_static_arguments: bool,
    inline_call_width_hint_without_static_arguments: Option<usize>,
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
        inline_call_width_hint_without_static_arguments,
        argument_shape: base_state.argument_shape,
    }
}
