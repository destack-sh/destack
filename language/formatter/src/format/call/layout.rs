use crate::CallArgumentLayoutFacts;
use crate::format::analysis::timing::tags;
use crate::format::call::arguments::{
    can_format_multiline_call_argument_list_with_last_separator_line_comment,
    single_argument_separator_line_comment_fact,
};
use crate::format::call::facts::{
    ArgumentSimplicityOptions, CallArgumentCommentProfile,
    argument_has_callback_blocking_comment_annotation,
    argument_has_leading_prefix_annotation_outside_span, argument_has_line_comment_annotation,
    argument_has_non_blank_annotation, argument_has_separator_line_comment_annotation,
    argument_is_collection_literal, argument_is_interpolated_template_literal,
    argument_is_plain_call_argument, argument_is_reference_like, argument_is_simple_with_options,
    call_arguments_are_multiline_span, call_force_expand_single_collection_for_type_binary_callee,
    call_has_leading_block_callback_with_simple_tail, call_has_non_blank_infix_annotation,
    call_has_static_arguments, collect_call_argument_comment_profile,
    resolve_regular_call_argument_expansion_profile,
};
use crate::format::expression::{
    Argument, Declaration, DestackFormatContext, Expression, FunctionKind, GroupId, LocalNodeId,
    NodeTree, NodeType, ScalarLiteral, TrailingComma, argument_is_array_literal,
    argument_is_block_callback, argument_is_function_expression, argument_is_lambda_expression,
    argument_is_object_literal, argument_is_template_literal, argument_value_id,
    is_block_lambda_argument, is_complex_argument, is_trivial_argument, is_trivial_expression,
    transparent_inner_expression,
};

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

/// Return whether a call argument layout class is simple and unannotated.
pub(crate) fn call_argument_layout_facts_are_simple_multi_unannotated(
    layout_facts: CallArgumentLayoutFacts,
) -> bool {
    !layout_facts.has_call_infix_annotations && layout_facts.all_compact_simple_unannotated
}

/// Return the leading dynamic arguments before the last argument.
fn leading_dynamic_arguments(
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> &[LocalNodeId<Argument>] {
    if let Some((_, leading_arguments)) = dynamic_arguments.split_last() {
        leading_arguments
    } else {
        &[]
    }
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
        build_scanned_call_argument_layout_facts(
            context,
            call_node_id,
            dynamic_arguments,
            has_call_infix_annotations,
            has_call_chain_parent,
        )
    };

    context.store_call_argument_layout_facts(call_node_id, layout_facts);

    layout_facts
}

/// Return whether a call can short-circuit to single-argument inline layout.
#[derive(Clone, Copy)]
pub(crate) struct SingleSimpleArgumentShortCircuitOptions {
    /// Whether the call has static type arguments.
    pub(crate) call_has_static_arguments: bool,
    /// Whether the call node has infix annotations.
    pub(crate) has_call_infix_annotations: bool,
    /// Whether the single argument should force expansion.
    pub(crate) single_argument_force_expand: bool,
}

/// Return whether a single simple argument can stay inline.
#[derive(Clone, Copy)]
pub(crate) struct SingleSimpleArgumentInlineOptions {
    /// Whether single long static arguments force expansion.
    pub(crate) force_expand_single_long_with_static_arguments: bool,
    /// Whether single collection arguments force expansion for type-binary callees.
    pub(crate) force_expand_single_collection_for_type_binary_callee: bool,
    /// Whether the single argument should force expansion.
    pub(crate) single_argument_force_expand: bool,
}

/// Store shared call argument planning inputs.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentLayoutBaseState {
    /// Whether the call has static type arguments.
    pub(crate) call_has_static_arguments: bool,
    /// Whether the call has non-blank infix annotations.
    pub(crate) has_call_infix_annotations: bool,
    /// One-pass argument shape facts.
    pub(crate) argument_shape: CallArgumentShape,
}

/// Store shared call argument planning inputs.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentLayoutState {
    /// Whether the call has non-blank infix annotations.
    pub(crate) has_call_infix_annotations: bool,
    /// Whether the single argument should force expanded list layout.
    pub(crate) single_argument_force_expand: bool,
    /// Whether one long single argument with static arguments should force expansion.
    pub(crate) force_expand_single_long_with_static_arguments: bool,
    /// Whether one single collection argument in type-binary callee should force expansion.
    pub(crate) force_expand_single_collection_for_type_binary_callee: bool,
    /// One-pass argument shape facts.
    pub(crate) argument_shape: CallArgumentShape,
}

/// Store shared state for rendering a decided call argument layout.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentLayoutRenderState {
    /// The call expression node id.
    pub(crate) call_node_id: LocalNodeId<Expression>,
    /// The active list group id.
    pub(crate) group_id: GroupId,
    /// Whether all arguments can use the plain writer.
    pub(crate) all_plain_call_arguments: bool,
}

/// Build shared call argument planning base inputs.
pub(crate) fn build_call_argument_layout_base_state(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    layout_facts: CallArgumentLayoutFacts,
) -> CallArgumentLayoutBaseState {
    let call_has_static_arguments = call_has_static_arguments(context, call_node_id);

    CallArgumentLayoutBaseState {
        call_has_static_arguments,
        has_call_infix_annotations: layout_facts.has_call_infix_annotations,
        argument_shape: call_argument_shape_from_layout_facts(layout_facts),
    }
}

/// Return whether a single collection argument should force expansion for a type-binary callee.
fn layout_force_expand_single_collection_for_type_binary_callee(
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

/// Build full call argument layout state.
pub(crate) fn build_call_argument_layout_state(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    base_state: CallArgumentLayoutBaseState,
    single_argument_force_expand: bool,
    force_expand_single_long_with_static_arguments: bool,
) -> CallArgumentLayoutState {
    // single collection arguments in type-binary calls may need forced expansion
    let force_expand_single_collection_for_type_binary_callee =
        layout_force_expand_single_collection_for_type_binary_callee(
            context,
            call_node_id,
            dynamic_arguments,
        );

    // keep layout state construction as one explicit data assembly step
    CallArgumentLayoutState {
        has_call_infix_annotations: base_state.has_call_infix_annotations,
        single_argument_force_expand,
        force_expand_single_long_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
        argument_shape: base_state.argument_shape,
    }
}

/// Store annotation-shape facts collected while scanning dynamic arguments.
#[derive(Default)]
pub(crate) struct CallArgumentAnnotationScanState {
    pub(crate) has_any_argument_annotation: bool,
    pub(crate) has_line_comment_annotations: bool,
    pub(crate) all_single_line_and_unannotated: bool,
    pub(crate) all_compact_simple_unannotated: bool,
}

impl CallArgumentAnnotationScanState {
    /// Create an annotation scan state with optimistic defaults.
    pub(crate) fn new() -> Self {
        Self {
            all_single_line_and_unannotated: true,
            all_compact_simple_unannotated: true,
            ..Self::default()
        }
    }
}

/// Store callback-shape facts collected while scanning dynamic arguments.
#[derive(Default)]
pub(crate) struct CallArgumentCallbackScanState {
    pub(crate) has_block_callback_argument: bool,
    pub(crate) first_argument_is_block_callback: bool,
    pub(crate) last_argument_is_block_callback: bool,
    pub(crate) non_last_block_callback_count: usize,
    pub(crate) non_last_block_callback_index: Option<usize>,
    pub(crate) arrow_argument_count: usize,
    pub(crate) function_argument_count: usize,
}

/// Store non-callback and argument-shape facts collected while scanning dynamic arguments.
#[derive(Default)]
pub(crate) struct CallArgumentComplexityScanState {
    pub(crate) all_plain_call_arguments: bool,
    pub(crate) trailing_collection_argument: bool,
    pub(crate) has_non_trivial_non_callback_argument: bool,
    pub(crate) has_spread_argument: bool,
    pub(crate) has_complex_non_callback_argument: bool,
}

impl CallArgumentComplexityScanState {
    /// Create a complexity scan state with optimistic defaults.
    pub(crate) fn new() -> Self {
        Self {
            all_plain_call_arguments: true,
            ..Self::default()
        }
    }
}

/// Store incremental layout facts collected while scanning dynamic arguments.
#[derive(Default)]
pub(crate) struct CallArgumentLayoutScanState {
    pub(crate) annotation: CallArgumentAnnotationScanState,
    pub(crate) callback: CallArgumentCallbackScanState,
    pub(crate) complexity: CallArgumentComplexityScanState,
}

impl CallArgumentLayoutScanState {
    /// Create a layout scan state with optimistic defaults.
    pub(crate) fn new() -> Self {
        Self {
            annotation: CallArgumentAnnotationScanState::new(),
            complexity: CallArgumentComplexityScanState::new(),
            ..Self::default()
        }
    }
}

/// Store callback flags for one argument value.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CallArgumentCallbackFlags {
    pub(crate) is_lambda_argument: bool,
    pub(crate) is_function_argument: bool,
    pub(crate) is_block_callback: bool,
}

impl CallArgumentCallbackScanState {
    /// Observe callback flags for one argument position.
    pub(crate) fn observe_argument_callback_flags(
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

/// Return callback flags for one argument value.
pub(crate) fn call_argument_callback_flags(
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

impl CallArgumentAnnotationScanState {
    /// Observe one argument's annotation and newline surface.
    pub(crate) fn observe_argument_surface(
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

impl CallArgumentComplexityScanState {
    /// Observe argument-level shape for one argument.
    pub(crate) fn observe_argument_shape(
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
    pub(crate) fn observe_non_callback_complexity(
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

/// Return whether a call expression is used as the callee or receiver of a parent postfix chain.
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

/// Build one layout class for a call with no dynamic arguments.
pub(crate) fn build_empty_call_argument_layout_facts(
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

/// Return whether scanned facts should force hugging the last argument inline.
pub(crate) fn resolve_force_hug_last_inline(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
    has_any_argument_annotation: bool,
    all_single_line_and_unannotated: bool,
    trailing_collection_argument: bool,
) -> bool {
    dynamic_arguments.len() > 1
        && !has_call_infix_annotations
        && !has_any_argument_annotation
        && all_single_line_and_unannotated
        && call_arguments_force_hug_last_inline(
            context,
            call_node_id,
            dynamic_arguments,
            trailing_collection_argument,
        )
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
pub(crate) fn scan_call_argument_layout_state(
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

/// Build one layout class from scanned argument facts.
pub(crate) fn build_scanned_call_argument_layout_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
    has_call_chain_parent: bool,
) -> CallArgumentLayoutFacts {
    let state = scan_call_argument_layout_state(context, dynamic_arguments);
    let force_hug_last_inline = resolve_force_hug_last_inline(
        context,
        call_node_id,
        dynamic_arguments,
        has_call_infix_annotations,
        state.annotation.has_any_argument_annotation,
        state.annotation.all_single_line_and_unannotated,
        state.complexity.trailing_collection_argument,
    );
    let is_multiline_in_source = call_arguments_are_multiline_span(context, dynamic_arguments);

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

/// Store list-default layout flags resolved by layout rules.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentDefaultListLayout {
    /// Whether the list should expand.
    pub(crate) force_expand: bool,
    /// Whether separator-comment multiline rendering should be used.
    pub(crate) use_separator_comment_multiline: bool,
    /// Whether plain default list writing can bypass generic list emission.
    pub(crate) use_plain_default_short_circuit: bool,
    /// Whether the list should disallow trailing separators.
    pub(crate) disallow_trailing_separator: bool,
    /// Whether the list should force a trailing separator.
    pub(crate) force_trailing_separator: bool,
}

/// Store comment-expanded layout flags resolved by layout rules.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentCommentExpandedLayout {
    /// Whether separator-comment multiline rendering should be used.
    pub(crate) use_separator_comment_multiline: bool,
    /// Whether the single-plain-argument separator path should be used.
    pub(crate) use_single_plain_separator_comment_layout: bool,
    /// Whether a trailing comma should be emitted.
    pub(crate) use_trailing_comma: bool,
    /// Whether trailing comma is forced for separator-comment ownership.
    pub(crate) force_trailing_comma_for_separator_comment: bool,
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
    CommentExpanded(CallArgumentCommentExpandedLayout),
    /// Render with the default list formatter.
    ListDefault(CallArgumentDefaultListLayout),
}

/// Return whether a call can short-circuit to single-argument inline layout.
pub(crate) fn call_arguments_use_single_simple_argument_short_circuit(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    options: SingleSimpleArgumentShortCircuitOptions,
    shape: CallArgumentShape,
) -> bool {
    if dynamic_arguments.len() != 1
        || options.call_has_static_arguments
        || options.has_call_infix_annotations
        || options.single_argument_force_expand
        || shape.is_multiline_in_source
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
pub(crate) fn call_arguments_use_single_simple_argument_inline(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    options: SingleSimpleArgumentInlineOptions,
    shape: CallArgumentShape,
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

    is_trivial_expression(context.tree, value)
}

/// Store hug-last call argument layout result.
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
    force_expand: bool,
    trailing_collection_argument: bool,
) -> Option<HugLastCallArgumentLayout> {
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

/// Return whether the trailing collection argument has non blank comment signals.
pub(crate) fn trailing_collection_argument_has_comment_signal(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };

    if !argument_is_collection_literal(context, last_argument_id) {
        return false;
    }

    if context.has_non_blank_annotation(last_argument_id) {
        return true;
    }

    let last_argument_value_id = argument_value_id(context.tree, last_argument_id);
    context.has_non_blank_annotation(last_argument_value_id)
}

/// Return whether comment profiling forces explicit multiline expansion.
pub(crate) fn call_argument_comments_force_expanded_layout(
    dynamic_arguments: &[LocalNodeId<Argument>],
    comment_profile: &CallArgumentCommentProfile,
) -> bool {
    if dynamic_arguments.len() <= 1 {
        return false;
    }

    comment_profile.has_line_comment_annotations
        || comment_profile.has_prefix_line_comment_annotations
}

/// Build call argument comment profile only when annotations require it.
pub(crate) fn resolve_call_argument_comment_profile(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_call_infix_annotations: bool,
) -> CallArgumentCommentProfile {
    if has_any_argument_annotation || has_call_infix_annotations {
        let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_PROFILE);
        context.increment_counter("call.arguments.comment_profile.calls", 1);
        return collect_call_argument_comment_profile(context, dynamic_arguments);
    }

    context.increment_counter("call.arguments.comment_profile.skip_no_annotation", 1);
    CallArgumentCommentProfile::default()
}

/// Build comment-expanded layout options from resolved call layout facts.
pub(crate) fn resolve_comment_expanded_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentCommentExpandedLayout {
    let use_separator_comment_multiline =
        can_format_multiline_call_argument_list_with_last_separator_line_comment(
            context,
            call_node_id,
            dynamic_arguments,
        );

    let use_single_plain_separator_comment_layout = if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];
        argument_is_plain_call_argument(context, argument_id)
            && single_argument_separator_line_comment_fact(context, call_node_id, argument_id)
                .is_some()
    } else {
        false
    };

    let has_trailing_collection_comment_signal =
        trailing_collection_argument_has_comment_signal(context, dynamic_arguments);
    let has_single_separator_line_comment_annotation = dynamic_arguments.len() == 1
        && argument_has_separator_line_comment_annotation(context, dynamic_arguments[0]);
    let has_last_separator_line_comment_annotation =
        dynamic_arguments
            .last()
            .copied()
            .is_some_and(|argument_id| {
                argument_has_separator_line_comment_annotation(context, argument_id)
            });
    let last_separator_line_comment_source_missing =
        dynamic_arguments
            .last()
            .copied()
            .is_some_and(|argument_id| {
                single_argument_separator_line_comment_fact(context, call_node_id, argument_id)
                    .is_none()
            });
    let use_trailing_comma = context.options.trailing_comma == TrailingComma::All
        && !has_trailing_collection_comment_signal
        && !has_single_separator_line_comment_annotation;
    let force_trailing_comma_for_separator_comment =
        has_last_separator_line_comment_annotation && last_separator_line_comment_source_missing;

    CallArgumentCommentExpandedLayout {
        use_separator_comment_multiline,
        use_single_plain_separator_comment_layout,
        use_trailing_comma,
        force_trailing_comma_for_separator_comment,
    }
}

/// Return whether one single template literal argument can stay inline.
pub(crate) fn call_arguments_use_single_template_literal_argument_inline(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_state: CallArgumentLayoutState,
    has_boundary_comments: bool,
) -> bool {
    if dynamic_arguments.len() != 1
        || has_boundary_comments
        || layout_state.has_call_infix_annotations
        || layout_state.argument_shape.has_any_argument_annotation
    {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    argument_is_template_literal(context, argument_id)
        && !argument_is_interpolated_template_literal(context, argument_id)
}

/// Return whether trailing collection comments should force list expansion.
pub(crate) fn call_argument_trailing_collection_comment_force_expand(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    trailing_collection_argument: bool,
    has_line_comment_annotations: bool,
) -> bool {
    if dynamic_arguments.len() <= 1 || !trailing_collection_argument {
        return false;
    }

    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };

    let has_last_line_comment_annotation = has_line_comment_annotations
        && argument_has_line_comment_annotation(context, last_argument_id);
    let last_argument_span = context.span(last_argument_id);
    let has_last_source_comment = context.has_comment(last_argument_span);

    has_last_line_comment_annotation || has_last_source_comment
}

/// Return whether default list rendering should use trailing collection expansion.
pub(crate) fn call_arguments_use_trailing_collection_expanded_layout(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    has_block_callback_argument: bool,
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    has_boundary_comments: bool,
) -> bool {
    let has_trailing_collection_argument = dynamic_arguments
        .last()
        .copied()
        .is_some_and(|argument_id| argument_is_collection_literal(context, argument_id));
    if !force_expand
        || dynamic_arguments.len() <= 1
        || !has_trailing_collection_argument
        || has_block_callback_argument
        || has_any_argument_annotation
        || has_line_comment_annotations
        || has_boundary_comments
    {
        return false;
    }

    !trailing_collection_argument_has_comment_signal(context, dynamic_arguments)
}

/// Return whether any call argument is a block callback.
pub(crate) fn call_arguments_have_block_callback(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    dynamic_arguments
        .iter()
        .copied()
        .any(|argument_id| argument_is_block_callback(context, argument_id))
}

/// Return whether structural trailing collection layout should force expansion.
pub(crate) fn call_arguments_force_expand_for_structural_trailing_collection_layout(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    trailing_collection_argument: bool,
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    has_boundary_comments: bool,
) -> bool {
    if dynamic_arguments.len() < 3
        || !trailing_collection_argument
        || has_any_argument_annotation
        || has_line_comment_annotations
        || has_boundary_comments
        || trailing_collection_argument_has_comment_signal(context, dynamic_arguments)
    {
        return false;
    }

    leading_arguments_are_compact_simple_unannotated(context, dynamic_arguments)
}

/// Build default list-layout options from resolved call layout facts.
pub(crate) fn resolve_default_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    has_line_comment_annotations: bool,
    has_any_argument_annotation: bool,
) -> CallArgumentDefaultListLayout {
    let is_single_argument = dynamic_arguments.len() == 1;
    let single_argument_id = dynamic_arguments.first().copied();
    let has_single_template_literal_argument = single_argument_id
        .is_some_and(|argument_id| argument_is_template_literal(context, argument_id));
    let has_single_interpolated_template_literal_argument = single_argument_id
        .is_some_and(|argument_id| argument_is_interpolated_template_literal(context, argument_id));
    let has_trailing_collection_comment_signal =
        trailing_collection_argument_has_comment_signal(context, dynamic_arguments);
    let has_single_separator_line_comment_annotation = is_single_argument
        && single_argument_id.is_some_and(|argument_id| {
            argument_has_separator_line_comment_annotation(context, argument_id)
        });
    let has_last_separator_line_comment_annotation =
        dynamic_arguments
            .last()
            .copied()
            .is_some_and(|argument_id| {
                argument_has_separator_line_comment_annotation(context, argument_id)
            });
    let use_separator_comment_multiline =
        can_format_multiline_call_argument_list_with_last_separator_line_comment(
            context,
            call_node_id,
            dynamic_arguments,
        );
    let use_plain_default_short_circuit = !context.has_ignore_directive_markers()
        && dynamic_arguments.len() > 1
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !has_trailing_collection_comment_signal
        && !is_single_argument;
    let disallow_trailing_separator = (is_single_argument
        && single_argument_id
            .is_some_and(|argument_id| argument_is_collection_literal(context, argument_id)))
        || (has_single_template_literal_argument
            && !has_single_interpolated_template_literal_argument)
        || has_trailing_collection_comment_signal;
    let force_trailing_separator =
        has_last_separator_line_comment_annotation || has_single_separator_line_comment_annotation;

    CallArgumentDefaultListLayout {
        force_expand,
        use_separator_comment_multiline,
        use_plain_default_short_circuit,
        disallow_trailing_separator,
        force_trailing_separator,
    }
}

/// Store carry-forward layout signals collected by the early decision path.
pub(crate) struct EarlyCallArgumentLayoutSignals {
    /// Whether any dynamic argument has annotations.
    pub(crate) has_any_argument_annotation: bool,
    /// Whether line comment annotations exist in the argument list.
    pub(crate) has_line_comment_annotations: bool,
}

/// Store the result of running the early call layout decision path.
pub(crate) enum EarlyCallArgumentLayoutOutcome {
    /// Return the final decision from early policy rules.
    Final(CallArgumentLayoutDecision),
    /// Continue through profiled layout rules with carry-forward signals.
    Continue(EarlyCallArgumentLayoutSignals),
}

/// Store resolved comment-driven early-layout outputs.
pub(crate) struct EarlyCommentLayoutResult {
    /// The forced early decision when comments require one.
    pub(crate) decision: Option<CallArgumentLayoutDecision>,
    /// Whether any line comment annotations exist across dynamic arguments.
    pub(crate) has_line_comment_annotations: bool,
}

/// Resolve comment-driven early layout rules and carry-forward signals.
pub(crate) fn resolve_comment_layout_result(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_call_infix_annotations: bool,
) -> EarlyCommentLayoutResult {
    // comments can force explicit multiline argument rendering
    let comment_profile = resolve_call_argument_comment_profile(
        context,
        dynamic_arguments,
        has_any_argument_annotation,
        has_call_infix_annotations,
    );
    let has_line_comment_annotations = comment_profile.has_line_comment_annotations;
    if call_argument_comments_force_expanded_layout(dynamic_arguments, &comment_profile) {
        context.increment_counter("call.arguments.path.comment_expanded", 1);
        let comment_expanded_layout =
            resolve_comment_expanded_call_argument_layout(context, call_node_id, dynamic_arguments);
        return EarlyCommentLayoutResult {
            decision: Some(CallArgumentLayoutDecision::CommentExpanded(
                comment_expanded_layout,
            )),
            has_line_comment_annotations,
        };
    }

    EarlyCommentLayoutResult {
        decision: None,
        has_line_comment_annotations,
    }
}

/// Resolve single-argument inline rules before profiled layout.
pub(crate) fn resolve_single_argument_inline_layout(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_state: CallArgumentLayoutState,
    has_boundary_comments: bool,
) -> Option<CallArgumentLayoutDecision> {
    // keep single callback arguments wrapped directly to avoid list-style trailing commas
    let use_single_callback_argument_inline = call_arguments_use_single_callback_argument_inline(
        context,
        dynamic_arguments,
        layout_state.has_call_infix_annotations,
        layout_state.force_expand_single_long_with_static_arguments,
        layout_state.force_expand_single_collection_for_type_binary_callee,
    ) && !has_boundary_comments;
    if use_single_callback_argument_inline {
        context.increment_counter("call.arguments.path.single_callback_inline", 1);
        return Some(CallArgumentLayoutDecision::InlineSingle);
    }

    // keep short single positional arguments inline
    let use_single_simple_argument = call_arguments_use_single_simple_argument_inline(
        context,
        dynamic_arguments,
        SingleSimpleArgumentInlineOptions {
            force_expand_single_long_with_static_arguments: layout_state
                .force_expand_single_long_with_static_arguments,
            force_expand_single_collection_for_type_binary_callee: layout_state
                .force_expand_single_collection_for_type_binary_callee,
            single_argument_force_expand: layout_state.single_argument_force_expand,
        },
        layout_state.argument_shape,
    );
    if use_single_simple_argument {
        context.increment_counter("call.arguments.path.single_simple", 1);
        return Some(CallArgumentLayoutDecision::InlineSingle);
    }

    // keep non-interpolated template literal snapshot arguments wrapped inline
    if call_arguments_use_single_template_literal_argument_inline(
        context,
        dynamic_arguments,
        layout_state,
        has_boundary_comments,
    ) {
        context.increment_counter("call.arguments.path.single_template_inline", 1);
        return Some(CallArgumentLayoutDecision::InlineSingle);
    }

    None
}

/// Resolve multi-argument inline rules based on recognized call patterns.
pub(crate) fn resolve_pattern_inline_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_boundary_comments: bool,
) -> Option<CallArgumentLayoutDecision> {
    // keep hook-like callback plus deps-array call arguments in inline join mode
    if !has_boundary_comments
        && call_has_react_hook_like_callback_deps_array(context, dynamic_arguments)
    {
        context.increment_counter("call.arguments.path.react_hook_like_inline", 1);
        return Some(CallArgumentLayoutDecision::InlineAll);
    }

    // keep leading callback plus short tail calls inline
    if call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments) {
        context.increment_counter("call.arguments.path.leading_block_callback_inline", 1);
        return Some(CallArgumentLayoutDecision::InlineAll);
    }

    None
}

/// Resolve early call argument layout rules before expansion profile work.
pub(crate) fn resolve_early_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_state: CallArgumentLayoutState,
    has_boundary_comments: bool,
) -> EarlyCallArgumentLayoutOutcome {
    // single-argument inline rules
    if let Some(decision) = resolve_single_argument_inline_layout(
        context,
        dynamic_arguments,
        layout_state,
        has_boundary_comments,
    ) {
        return EarlyCallArgumentLayoutOutcome::Final(decision);
    }

    // recognized call-pattern inline rules
    if let Some(decision) = resolve_pattern_inline_layout(
        context,
        call_node_id,
        dynamic_arguments,
        has_boundary_comments,
    ) {
        return EarlyCallArgumentLayoutOutcome::Final(decision);
    }

    // boundary comments should block aggressive inline and hug-last behavior
    let has_any_argument_annotation = layout_state.argument_shape.has_any_argument_annotation;

    // comment signals can force expanded layout
    let comment_layout_result = resolve_comment_layout_result(
        context,
        call_node_id,
        dynamic_arguments,
        has_any_argument_annotation,
        layout_state.has_call_infix_annotations,
    );
    if let Some(decision) = comment_layout_result.decision {
        return EarlyCallArgumentLayoutOutcome::Final(decision);
    }

    EarlyCallArgumentLayoutOutcome::Continue(EarlyCallArgumentLayoutSignals {
        has_any_argument_annotation,
        has_line_comment_annotations: comment_layout_result.has_line_comment_annotations,
    })
}

/// Store force-expand signals resolved for profiled layout.
pub(crate) struct ProfileForceExpandState {
    /// Whether list-default rendering must expand.
    pub(crate) force_expand: bool,
    /// Whether any argument is a block callback.
    pub(crate) has_block_callback_argument: bool,
    /// Whether the last argument is a collection literal.
    pub(crate) trailing_collection_argument: bool,
}

/// Resolve force-expand state from expansion profile and structural signals.
pub(crate) fn resolve_profile_force_expand_state(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand_from_profile: bool,
    trailing_collection_argument: bool,
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    has_boundary_comments: bool,
) -> ProfileForceExpandState {
    let has_block_callback_argument =
        call_arguments_have_block_callback(context, dynamic_arguments);
    let force_expand_for_structural_trailing_collection =
        call_arguments_force_expand_for_structural_trailing_collection_layout(
            context,
            dynamic_arguments,
            trailing_collection_argument,
            has_any_argument_annotation,
            has_line_comment_annotations,
            has_boundary_comments,
        );
    let force_expand = force_expand_from_profile
        || has_boundary_comments
        || force_expand_for_structural_trailing_collection
        || call_argument_trailing_collection_comment_force_expand(
            context,
            dynamic_arguments,
            trailing_collection_argument,
            has_line_comment_annotations,
        );

    ProfileForceExpandState {
        force_expand,
        has_block_callback_argument,
        trailing_collection_argument,
    }
}

/// Resolve hug-last layout when candidates are eligible.
pub(crate) fn resolve_hug_last_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand_state: &ProfileForceExpandState,
    has_line_comment_annotations: bool,
    has_boundary_comments: bool,
    has_call_infix_annotations: bool,
) -> Option<CallArgumentLayoutDecision> {
    // hug-last candidates can still end in default list rendering
    let can_consider_hug_last_argument = can_consider_hug_last_call_arguments(
        context,
        dynamic_arguments,
        has_line_comment_annotations || has_boundary_comments,
        has_call_infix_annotations,
    );
    if !can_consider_hug_last_argument {
        return None;
    }

    let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_HUG_LAST);
    let layout = resolve_hug_last_call_argument_layout(
        context,
        call_node_id,
        dynamic_arguments,
        force_expand_state.force_expand,
        force_expand_state.trailing_collection_argument,
    )?;

    match layout {
        HugLastCallArgumentLayout::Inline => Some(CallArgumentLayoutDecision::InlineAll),
    }
}

/// Resolve profiled call argument layout rules from expansion facts.
pub(crate) fn resolve_profiled_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    has_boundary_comments: bool,
) -> CallArgumentLayoutDecision {
    // expansion profile drives default and hug-last policy paths
    let expansion_profile = {
        let _timing =
            context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_PROFILE);
        resolve_regular_call_argument_expansion_profile(context, call_node_id, dynamic_arguments)
    };
    let has_call_infix_annotations = expansion_profile.has_call_infix_annotations;
    let force_expand_state = resolve_profile_force_expand_state(
        context,
        dynamic_arguments,
        expansion_profile.force_expand,
        expansion_profile.trailing_collection_argument,
        has_any_argument_annotation,
        has_line_comment_annotations,
        has_boundary_comments,
    );

    // hug-last candidates can still end in default list rendering
    if let Some(decision) = resolve_hug_last_layout(
        context,
        call_node_id,
        dynamic_arguments,
        &force_expand_state,
        has_line_comment_annotations,
        has_boundary_comments,
        has_call_infix_annotations,
    ) {
        return decision;
    }

    if call_arguments_use_trailing_collection_expanded_layout(
        context,
        dynamic_arguments,
        force_expand_state.force_expand,
        force_expand_state.has_block_callback_argument,
        has_any_argument_annotation,
        has_line_comment_annotations,
        has_boundary_comments,
    ) {
        context.increment_counter("call.arguments.path.trailing_collection_expanded", 1);
        return CallArgumentLayoutDecision::TrailingCollectionExpanded;
    }

    context.increment_counter("call.arguments.path.list_default", 1);
    let default_list_layout = resolve_default_call_argument_layout(
        context,
        call_node_id,
        dynamic_arguments,
        force_expand_state.force_expand,
        has_line_comment_annotations,
        has_any_argument_annotation,
    );
    CallArgumentLayoutDecision::ListDefault(default_list_layout)
}

/// Decide post-hugged call argument layout.
pub(crate) fn decide_post_hugged_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_state: CallArgumentLayoutState,
    has_boundary_comments: bool,
) -> CallArgumentLayoutDecision {
    match resolve_early_call_argument_layout(
        context,
        call_node_id,
        dynamic_arguments,
        layout_state,
        has_boundary_comments,
    ) {
        EarlyCallArgumentLayoutOutcome::Final(decision) => decision,
        EarlyCallArgumentLayoutOutcome::Continue(signals) => resolve_profiled_call_argument_layout(
            context,
            call_node_id,
            dynamic_arguments,
            signals.has_any_argument_annotation,
            signals.has_line_comment_annotations,
            has_boundary_comments,
        ),
    }
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
        && leading_arguments_are_compact_simple_unannotated(context, dynamic_arguments)
        && leading_arguments_are_compact_callback_tail_candidates(context, dynamic_arguments);

    force_hug_test_like_callback
        || force_hug_reference_callback_with_collection_tail
        || force_hug_simple_block_lambda_tail
}

/// Return whether a call matches the react hook callback-plus-deps pattern.
pub(crate) fn call_has_react_hook_like_callback_deps_array(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() < 2 || dynamic_arguments.len() > 3 {
        return false;
    }

    let callback_index = if dynamic_arguments.len() == 2 { 0 } else { 1 };
    let deps_index = callback_index + 1;

    if dynamic_arguments.len() == 3
        && !argument_is_identifier_reference(context, dynamic_arguments[0])
    {
        return false;
    }

    argument_is_zero_parameter_block_lambda(context, dynamic_arguments[callback_index])
        && argument_is_array_literal(context, dynamic_arguments[deps_index])
}

/// Return whether one argument is a string or template literal.
fn argument_is_string_like(
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

/// Return whether an argument is a single-segment identifier path.
fn argument_is_identifier_reference(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    matches!(
        context.tree.get(value_id),
        Expression::Path {
            path,
            static_arguments: None
        } if path.segments.len() == 1
    )
}

/// Return whether an argument is a zero-parameter block lambda.
fn argument_is_zero_parameter_block_lambda(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_block_callback(context, argument_id) {
        return false;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };
    let Declaration::Function { signature, .. } = context.tree.get(*declaration_id) else {
        return false;
    };

    signature.kind == FunctionKind::Lambda && signature.dynamic_parameters.is_empty()
}

/// Return whether a callee ends in a test style member name.
fn call_callee_has_test_like_member_name(
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
fn call_should_force_hug_test_like_callback(
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
