use crate::format::analysis::{
    ArgumentSimplicityOptions, argument_has_leading_prefix_annotation_outside_span,
    argument_has_line_comment_annotation, argument_is_collection_literal,
    argument_is_interpolated_template_literal, argument_is_simple_with_options,
    call_arguments_are_multiline_span, call_has_leading_block_callback_with_simple_tail,
    call_has_static_arguments, timing,
};
use crate::format::call::arguments::{
    argument_has_separator_line_comment_annotation, argument_is_plain_call_argument,
    can_format_multiline_call_argument_list_with_separator_line_comment,
    single_argument_separator_line_comment_source,
};
use crate::format::expression::{
    Argument, Declaration, DestackFormatContext, Expression, FunctionKind, LocalNodeId, NodeType,
    ScalarLiteral, TrailingComma, argument_is_array_literal, argument_is_block_callback,
    argument_is_function_expression, argument_is_lambda_expression, argument_is_object_literal,
    argument_is_template_literal, argument_value_id, is_block_lambda_argument, is_complex_argument,
    is_expression_chain, is_trivial_argument, transparent_inner_expression,
};
use crate::format::tree::has_multiline_jsx_argument;
use crate::{CallArgumentExpansionCache, CallArgumentExpansionsCache, CallArgumentLayoutCache};
use destack_ast::TypeBinaryOperator;

/// Return whether all leading arguments before the last are compact and simple.
pub(crate) fn leading_arguments_are_compact_simple_unannotated(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    dynamic_arguments
        .split_last()
        .map_or(&[][..], |(_, leading_arguments)| leading_arguments)
        .iter()
        .copied()
        .all(|argument_id| argument_is_compact_simple_unannotated(context, argument_id))
}

/// Return whether all leading arguments before the last are compact callback-tail candidates.
pub(crate) fn leading_arguments_are_compact_callback_tail_candidates(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    dynamic_arguments
        .split_last()
        .map_or(&[][..], |(_, leading_arguments)| leading_arguments)
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
pub(crate) fn call_argument_layout_cache(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentLayoutCache {
    if let Some(cached) = context.lookup_call_argument_layout_cache(call_node_id) {
        context.increment_counter("call.arguments.layout_cache.cache.hits", 1);
        return cached;
    }

    context.increment_counter("call.arguments.layout_cache.cache.misses", 1);
    context.increment_counter("call.arguments.layout_cache.builds", 1);
    let has_call_infix_annotations = context.has_non_blank_infix_annotation(call_node_id);
    let has_call_chain_parent = call_has_call_chain_parent(context, call_node_id);

    let layout_cache = if dynamic_arguments.is_empty() {
        CallArgumentLayoutCache {
            has_call_infix_annotations,
            all_single_line_and_unannotated: true,
            all_compact_simple_unannotated: true,
            all_plain_call_arguments: true,
            has_call_chain_parent,
            ..CallArgumentLayoutCache::default()
        }
    } else {
        scan_call_argument_layout_cache(
            context,
            call_node_id,
            dynamic_arguments,
            has_call_infix_annotations,
            has_call_chain_parent,
        )
    };

    context.store_call_argument_layout_cache(call_node_id, layout_cache);

    layout_cache
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

/// Return whether one call expression is nested under an await expression.
pub(crate) fn call_has_await_ancestor(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    context.any_ancestor(call_node_id, |parent_id, parent_type| {
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        matches!(
            context.tree.get(parent_expression_id),
            Expression::Await { .. } | Expression::AwaitMaybe { .. }
        )
    })
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
    // blank seams should not declassify compact argument simplicity
    let is_compact_simple_unannotated = !context.has_non_blank_annotation(argument_id)
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

/// Build one layout cache entry from scanned argument signals.
pub(crate) fn scan_call_argument_layout_cache(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
    has_call_chain_parent: bool,
) -> CallArgumentLayoutCache {
    let mut has_any_argument_annotation = false;
    let mut has_line_comment_annotations = false;
    let mut all_single_line_and_unannotated = true;
    let mut all_compact_simple_unannotated = true;
    let mut has_block_callback_argument = false;
    let mut first_argument_is_block_callback = false;
    let mut last_argument_is_block_callback = false;
    let mut non_last_block_callback_count = 0usize;
    let mut non_last_block_callback_index = None;
    let mut arrow_argument_count = 0usize;
    let mut function_argument_count = 0usize;
    let mut all_plain_call_arguments = true;
    let mut trailing_collection_argument = false;
    let mut has_non_trivial_non_callback_argument = false;
    let mut has_spread_argument = false;
    let mut has_complex_non_callback_argument = false;

    let last_argument_index = dynamic_arguments.len().saturating_sub(1);
    context.increment_counter(
        "call.arguments.layout.scan.arguments",
        dynamic_arguments.len(),
    );

    // scan each argument once and collect layout flags
    for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
        // layout decisions only treat non blank annotations as comment signals
        let has_annotation = context.has_non_blank_annotation(argument_id);
        let has_newline = context.node_has_newline(argument_id);
        if has_annotation {
            has_any_argument_annotation = true;
            if !has_line_comment_annotations
                && context
                    .argument_annotation_cache(argument_id)
                    .has_line_comment
            {
                has_line_comment_annotations = true;
            }
        }

        let is_single_line_and_unannotated = !has_annotation && !has_newline;
        all_single_line_and_unannotated &= is_single_line_and_unannotated;
        if all_compact_simple_unannotated {
            all_compact_simple_unannotated = if is_single_line_and_unannotated {
                argument_is_compact_simple_unannotated(context, argument_id)
            } else {
                false
            };
        }

        if all_plain_call_arguments {
            all_plain_call_arguments &= argument_is_plain_call_argument(context, argument_id);
        }

        let argument = context.tree.get(argument_id);
        if matches!(argument, Argument::Spread { .. }) {
            has_spread_argument = true;
        }

        let value_id = argument_value_id(context.tree, argument_id);
        let value_id = transparent_inner_expression(context, value_id);
        let value = context.tree.get(value_id);
        let is_last_argument = index == last_argument_index;
        if is_last_argument {
            trailing_collection_argument = matches!(
                value,
                Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
            );
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

        if index == 0 {
            first_argument_is_block_callback = is_block_callback;
        }
        if is_last_argument {
            last_argument_is_block_callback = is_block_callback;
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

        if !has_complex_non_callback_argument
            && !matches!(value, Expression::TreeExpression { .. })
            && is_complex_argument(context.tree, argument)
        {
            has_complex_non_callback_argument = true;
        }

        if !has_non_trivial_non_callback_argument
            && !is_last_argument
            && !is_trivial_argument(context.tree, argument)
        {
            has_non_trivial_non_callback_argument = true;
        }
    }

    let force_hug_last_inline = dynamic_arguments.len() > 1
        && !has_call_infix_annotations
        && !has_any_argument_annotation
        && all_single_line_and_unannotated
        && should_force_hug_last_inline(
            context,
            call_node_id,
            dynamic_arguments,
            trailing_collection_argument,
        );
    let is_multiline_in_source = call_arguments_are_multiline_span(context, dynamic_arguments);

    CallArgumentLayoutCache {
        has_call_infix_annotations,
        has_any_argument_annotation,
        is_multiline_in_source,
        all_single_line_and_unannotated,
        all_compact_simple_unannotated,
        all_plain_call_arguments,
        has_line_comment_annotations,
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
        has_call_chain_parent,
        trailing_collection_argument,
        force_hug_last_inline,
    }
}

/// Return whether an argument is a reference-style expression.
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

/// Collect one-pass comment data for call arguments.
pub(crate) fn call_argument_comments(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> (bool, bool) {
    let mut has_line_comment_annotations = false;
    let mut has_prefix_line_comment_annotations = false;

    for argument_id in dynamic_arguments.iter().copied() {
        if !context.has_non_blank_annotation(argument_id) {
            continue;
        }

        let annotation_cache = context.argument_annotation_cache(argument_id);
        if !has_line_comment_annotations && annotation_cache.has_line_comment {
            has_line_comment_annotations = true;
        }
        if !has_prefix_line_comment_annotations && annotation_cache.has_prefix_line_comment {
            has_prefix_line_comment_annotations = true;
        }
    }

    (
        has_line_comment_annotations,
        has_prefix_line_comment_annotations,
    )
}

/// Return whether one argument has callback-blocking line or multiline prefix annotations.
pub(crate) fn argument_has_callback_blocking_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let annotation_cache = context.argument_annotation_cache(argument_id);
    annotation_cache.has_line_comment
        || annotation_cache.has_prefix_line_comment
        || (context.node_has_newline(argument_id) && annotation_cache.has_prefix_annotation)
}

// call argument layout thresholds
const NON_LAST_BLOCK_CALLBACK_COUNT_TARGET: usize = 1;
const NON_LAST_BLOCK_CALLBACK_MIN_INDEX: usize = 1;
const MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT: usize = 2;
const FUNCTION_COMPOSITION_MIN_ARGUMENTS: usize = 3;

/// Resolve regular call argument expansion with per-call caching.
pub(crate) fn call_argument_expansion(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentExpansionCache {
    if let Some(cached) = context.lookup_call_argument_expansion_cache(call_node_id) {
        context.increment_counter("call.arguments.regular.cache.hits", 1);
        let expansion = cached.regular;
        context.increment_counter(
            if expansion.force_expand {
                "call.arguments.regular.force_expand.true"
            } else {
                "call.arguments.regular.force_expand.false"
            },
            1,
        );

        return expansion;
    }

    context.increment_counter("call.arguments.regular.cache.misses", 1);

    let expansions = call_argument_expansions(context, call_node_id, dynamic_arguments);
    let expansion = expansions.regular;

    context.store_call_argument_expansion_cache(call_node_id, expansions);

    context.increment_counter(
        if expansion.force_expand {
            "call.arguments.regular.force_expand.true"
        } else {
            "call.arguments.regular.force_expand.false"
        },
        1,
    );

    expansion
}

/// Resolve chain call argument force-expand state with per-call caching.
pub(crate) fn chain_call_argument_force_expand(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(cached) = context.lookup_call_argument_expansion_cache(call_node_id) {
        context.increment_counter("call.arguments.chain.cache.hits", 1);

        let force_expand = cached.chain_force_expand;
        context.increment_counter(
            if force_expand {
                "call.arguments.chain.force_expand.true"
            } else {
                "call.arguments.chain.force_expand.false"
            },
            1,
        );

        return force_expand;
    }

    context.increment_counter("call.arguments.chain.cache.misses", 1);

    let expansions = call_argument_expansions(context, call_node_id, dynamic_arguments);
    let force_expand = expansions.chain_force_expand;

    context.store_call_argument_expansion_cache(call_node_id, expansions);

    context.increment_counter(
        if force_expand {
            "call.arguments.chain.force_expand.true"
        } else {
            "call.arguments.chain.force_expand.false"
        },
        1,
    );

    force_expand
}

/// Build regular and chain call expansion data for one call expression.
fn call_argument_expansions(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentExpansionsCache {
    context.increment_counter("call.arguments.layout.builds", 1);

    let has_call_infix_annotations = context.has_non_blank_infix_annotation(call_node_id);

    if dynamic_arguments.is_empty() {
        return CallArgumentExpansionsCache {
            regular: CallArgumentExpansionCache {
                force_expand: false,
                has_call_infix_annotations,
                trailing_collection_argument: false,
            },
            chain_force_expand: false,
        };
    }

    if dynamic_arguments.len() == 1 {
        return single_argument_expansions(
            context,
            call_node_id,
            dynamic_arguments,
            has_call_infix_annotations,
        );
    }

    multi_argument_expansions(
        context,
        call_node_id,
        dynamic_arguments,
        has_call_infix_annotations,
    )
}

/// Build expansion data for one multi-argument call.
fn multi_argument_expansions(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
) -> CallArgumentExpansionsCache {
    let layout_cache = call_argument_layout_cache(context, call_node_id, dynamic_arguments);

    // compact unannotated argument lists do not need full expansion scans
    if !layout_cache.has_call_infix_annotations && layout_cache.all_compact_simple_unannotated {
        context.increment_counter("call.arguments.layout.simple_short_circuit", 1);

        return CallArgumentExpansionsCache {
            regular: CallArgumentExpansionCache {
                force_expand: false,
                has_call_infix_annotations: layout_cache.has_call_infix_annotations,
                trailing_collection_argument: layout_cache.trailing_collection_argument,
            },
            chain_force_expand: false,
        };
    }

    let force_expand_jsx = has_multiline_jsx_argument(context.tree, dynamic_arguments);
    let trailing_collection_argument = layout_cache.trailing_collection_argument;

    let allow_non_last_block_callback_with_collection_tail = !layout_cache
        .last_argument_is_block_callback
        && trailing_collection_argument
        && layout_cache.non_last_block_callback_count == NON_LAST_BLOCK_CALLBACK_COUNT_TARGET
        && layout_cache
            .non_last_block_callback_index
            .is_some_and(|index| index >= NON_LAST_BLOCK_CALLBACK_MIN_INDEX)
        && argument_is_reference_like(context, dynamic_arguments[0])
        && !layout_cache.has_non_trivial_non_callback_argument;

    let has_leading_block_callback_with_simple_tail =
        call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments);

    let should_expand_for_block_callback = layout_cache.has_block_callback_argument
        && (layout_cache.has_non_trivial_non_callback_argument
            || (!layout_cache.last_argument_is_block_callback
                && !allow_non_last_block_callback_with_collection_tail
                && !has_leading_block_callback_with_simple_tail));

    let has_any_function_argument =
        layout_cache.arrow_argument_count > 0 || layout_cache.function_argument_count > 0;
    let has_multiple_function_arguments = layout_cache.arrow_argument_count
        >= MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT
        || layout_cache.function_argument_count >= MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT;

    if has_multiple_function_arguments {
        context.increment_counter("call.arguments.layout.multiple_function_short_circuit", 1);

        return CallArgumentExpansionsCache {
            regular: CallArgumentExpansionCache {
                force_expand: true,
                has_call_infix_annotations,
                trailing_collection_argument,
            },
            chain_force_expand: true,
        };
    }

    let force_expand_function_composition = dynamic_arguments.len()
        >= FUNCTION_COMPOSITION_MIN_ARGUMENTS
        && has_any_function_argument
        && !layout_cache.has_spread_argument;

    let has_non_complex_force_expand_signal = force_expand_jsx
        || layout_cache.has_line_comment_annotations
        || should_expand_for_block_callback
        || force_expand_function_composition
        || has_call_infix_annotations;

    let force_expand_complex =
        !has_non_complex_force_expand_signal && layout_cache.has_complex_non_callback_argument;

    let regular_force_expand = force_expand_jsx
        || force_expand_complex
        || layout_cache.has_line_comment_annotations
        || should_expand_for_block_callback
        || has_multiple_function_arguments
        || force_expand_function_composition
        || has_call_infix_annotations;

    let chain_force_expand = force_expand_jsx
        || force_expand_complex
        || layout_cache.has_line_comment_annotations
        || should_expand_for_block_callback
        || has_multiple_function_arguments;

    CallArgumentExpansionsCache {
        regular: CallArgumentExpansionCache {
            force_expand: regular_force_expand,
            has_call_infix_annotations: layout_cache.has_call_infix_annotations,
            trailing_collection_argument,
        },
        chain_force_expand,
    }
}

/// Return whether a single static argument call should force expansion.
pub(crate) fn call_force_expand_single_multiline_with_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 1
        || !call_has_static_arguments(context, call_node_id)
        || context.has_non_blank_annotation(dynamic_arguments[0])
    {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    if argument_is_lambda_expression(context, argument_id)
        || argument_is_function_expression(context, argument_id)
    {
        return false;
    }

    is_expression_chain(context.tree, call_node_id)
}

/// Return whether a single collection argument should expand for type binary callees.
pub(crate) fn call_force_expand_single_collection_for_type_binary_callee(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 1
        || !argument_is_collection_literal(context, dynamic_arguments[0])
    {
        return false;
    }

    let callee = match context.tree.get(call_node_id) {
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

/// Return whether a single argument call should force expanded list layout.
pub(crate) fn single_argument_requires_expanded_list(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    // only single argument calls can use this path
    if dynamic_arguments.len() != 1 {
        return false;
    }

    // require a chain shaped argument value
    let argument_id = dynamic_arguments[0];
    let raw_value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, raw_value_id);
    if !expression_is_chain_layout_candidate(context, value_id) {
        return false;
    }

    // force expand only from owned annotation signals
    let has_annotation_signal = context.has_non_blank_annotation(argument_id)
        || context.has_non_blank_annotation(raw_value_id)
        || context.has_non_blank_annotation(value_id);
    has_annotation_signal
}

/// Return whether an expression should use chain-aware single-argument call layout rules.
fn expression_is_chain_layout_candidate(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    // regular member/call chain expressions
    if is_expression_chain(context.tree, expression_id) {
        return true;
    }

    // path-chain calls normalize from `(a).b()` to `a.b()` across passes
    match context.tree.get(expression_id) {
        Expression::Path { path, .. } => path.segments.len() > 1,
        Expression::Call { left, .. } | Expression::Instantiation { left, .. } => matches!(
            context.tree.get(*left),
            Expression::Path { path, .. } if path.segments.len() > 1
        ),
        _ => false,
    }
}

/// Build expansion data for one single-argument call.
fn single_argument_expansions(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
) -> CallArgumentExpansionsCache {
    let argument_id = dynamic_arguments[0];
    let argument_annotation_cache = context.argument_annotation_cache(argument_id);
    let has_line_comment_annotations = argument_annotation_cache.has_line_comment;
    let trailing_collection_argument = argument_is_collection_literal(context, argument_id);
    let has_collection_source_comment =
        trailing_collection_argument && context.has_comment(context.span(argument_id));
    let has_line_comment_annotations =
        has_line_comment_annotations || has_collection_source_comment;

    let force_expand_jsx = has_multiline_jsx_argument(context.tree, dynamic_arguments);
    let force_expand_single_commented_callback = argument_is_block_callback(context, argument_id)
        && (argument_has_callback_blocking_comment_annotation(context, argument_id)
            || has_call_infix_annotations);

    let force_expand_single_multiline_with_static_arguments =
        call_force_expand_single_multiline_with_static_arguments(
            context,
            call_node_id,
            dynamic_arguments,
        );
    let force_expand_single_collection_for_type_binary_callee =
        call_force_expand_single_collection_for_type_binary_callee(
            context,
            call_node_id,
            dynamic_arguments,
        );
    let force_expand_single_chain_argument =
        single_argument_requires_expanded_list(context, dynamic_arguments);
    let force_expand_single_prefix_line_commented_argument =
        argument_annotation_cache.has_prefix_line_comment;

    let regular_force_expand = force_expand_jsx
        || has_line_comment_annotations
        || force_expand_single_commented_callback
        || force_expand_single_multiline_with_static_arguments
        || force_expand_single_chain_argument
        || force_expand_single_collection_for_type_binary_callee
        || force_expand_single_prefix_line_commented_argument
        || has_call_infix_annotations;

    let chain_force_expand = force_expand_jsx
        || has_line_comment_annotations
        || force_expand_single_commented_callback
        || force_expand_single_chain_argument
        || force_expand_single_multiline_with_static_arguments;

    CallArgumentExpansionsCache {
        regular: CallArgumentExpansionCache {
            force_expand: regular_force_expand,
            has_call_infix_annotations,
            trailing_collection_argument,
        },
        chain_force_expand,
    }
}

/// Store planned post-hugged call argument layouts.
pub(crate) enum CallArgumentLayout {
    /// Keep the entire argument list inline.
    InlineAll,
    /// Keep one argument wrapped inline.
    InlineSingle,
    /// Keep compact leading arguments inline and expand the trailing collection argument.
    TrailingCollectionExpanded,
    /// Render with explicit comment-expanded multiline argument layout.
    CommentExpanded {
        use_separator_comment_multiline: bool,
        use_single_plain_separator_comment_layout: bool,
        use_trailing_comma: bool,
        force_trailing_comma_for_separator_comment: bool,
    },
    /// Render with the default list formatter.
    ListDefault {
        force_expand: bool,
        use_separator_comment_multiline: bool,
        use_plain_default_short_circuit: bool,
        disallow_trailing_separator: bool,
        force_trailing_separator: bool,
    },
}

/// Store separator-comment layout facts shared across call argument layout branches.
struct CallSeparatorLayoutFacts {
    /// Whether trailing collection comments are present.
    has_trailing_collection_comment_signal: bool,
    /// Whether one single argument has one separator line comment annotation.
    has_single_separator_line_comment_annotation: bool,
    /// Whether the last argument has one separator line comment annotation.
    has_last_separator_line_comment_annotation: bool,
    /// Whether the last separator line comment has no detachable source.
    last_separator_line_comment_source_missing: bool,
    /// Whether one argument can use separator-comment multiline formatting.
    use_separator_comment_multiline: bool,
    /// Whether one single plain argument has one detachable separator comment source.
    use_single_plain_separator_comment_layout: bool,
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

/// Collect separator-comment facts used by comment-expanded and default list layouts.
fn collect_call_separator_layout_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallSeparatorLayoutFacts {
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
                single_argument_separator_line_comment_source(context, call_node_id, argument_id)
                    .is_none()
            });
    let use_separator_comment_multiline =
        can_format_multiline_call_argument_list_with_separator_line_comment(
            context,
            call_node_id,
            dynamic_arguments,
        );
    let use_single_plain_separator_comment_layout = dynamic_arguments.len() == 1 && {
        let argument_id = dynamic_arguments[0];
        argument_is_plain_call_argument(context, argument_id)
            && single_argument_separator_line_comment_source(context, call_node_id, argument_id)
                .is_some()
    };

    CallSeparatorLayoutFacts {
        has_trailing_collection_comment_signal,
        has_single_separator_line_comment_annotation,
        has_last_separator_line_comment_annotation,
        last_separator_line_comment_source_missing,
        use_separator_comment_multiline,
        use_single_plain_separator_comment_layout,
    }
}

/// Build one comment-expanded call argument layout.
fn build_comment_expanded_call_argument_layout(
    context: &DestackFormatContext<'_>,
    _dynamic_arguments: &[LocalNodeId<Argument>],
    facts: &CallSeparatorLayoutFacts,
) -> CallArgumentLayout {
    let use_trailing_comma = context.options.trailing_comma == TrailingComma::All
        && !facts.has_single_separator_line_comment_annotation;
    let force_trailing_comma_for_separator_comment = facts
        .has_last_separator_line_comment_annotation
        && facts.last_separator_line_comment_source_missing;

    CallArgumentLayout::CommentExpanded {
        use_separator_comment_multiline: facts.use_separator_comment_multiline,
        use_single_plain_separator_comment_layout: facts.use_single_plain_separator_comment_layout,
        use_trailing_comma,
        force_trailing_comma_for_separator_comment,
    }
}

/// Build one default-list call argument layout.
fn build_default_list_call_argument_layout(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    facts: &CallSeparatorLayoutFacts,
) -> CallArgumentLayout {
    let is_single_argument = dynamic_arguments.len() == 1;
    let single_argument_id = dynamic_arguments.first().copied();
    let has_single_template_literal_argument = single_argument_id
        .is_some_and(|argument_id| argument_is_template_literal(context, argument_id));
    let has_single_interpolated_template_literal_argument = single_argument_id
        .is_some_and(|argument_id| argument_is_interpolated_template_literal(context, argument_id));
    let use_plain_default_short_circuit = !context.has_ignore_directive_markers()
        && dynamic_arguments.len() > 1
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !facts.has_trailing_collection_comment_signal
        && !is_single_argument;
    let disallow_trailing_separator =
        has_single_template_literal_argument && !has_single_interpolated_template_literal_argument;
    let force_trailing_separator = facts.has_last_separator_line_comment_annotation
        || facts.has_single_separator_line_comment_annotation;

    CallArgumentLayout::ListDefault {
        force_expand,
        use_separator_comment_multiline: facts.use_separator_comment_multiline,
        use_plain_default_short_circuit,
        disallow_trailing_separator,
        force_trailing_separator,
    }
}

/// Try single-argument inline layout rules in priority order.
fn try_single_argument_inline_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_cache: &CallArgumentLayoutCache,
    single_argument_force_expand: bool,
    force_expand_single_multiline_with_static_arguments: bool,
    force_expand_single_collection_for_type_binary_callee: bool,
    has_boundary_comments: bool,
) -> Option<CallArgumentLayout> {
    if dynamic_arguments.len() != 1 {
        return None;
    }

    let argument_id = dynamic_arguments[0];
    let value_id = argument_value_id(context.tree, argument_id);
    let has_call_infix_annotations = layout_cache.has_call_infix_annotations;
    let has_any_argument_annotation = layout_cache.has_any_argument_annotation;
    let has_multiline_jsx_argument_signal =
        has_multiline_jsx_argument(context.tree, dynamic_arguments);

    // single function expression arguments can stay inline
    let use_single_function_argument_inline = !has_multiline_jsx_argument_signal
        && !has_call_infix_annotations
        && !force_expand_single_multiline_with_static_arguments
        && !force_expand_single_collection_for_type_binary_callee
        && !has_boundary_comments
        && !context.has_non_blank_annotation(argument_id)
        && !context.has_non_blank_annotation(value_id)
        && !context.node_has_newline(value_id)
        && !argument_has_callback_blocking_comment_annotation(context, argument_id)
        && !argument_has_leading_prefix_annotation_outside_span(context, argument_id)
        && argument_is_function_expression(context, argument_id);
    if use_single_function_argument_inline {
        context.increment_counter("call.arguments.path.single_function_inline", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    // single callback arguments can stay inline
    let use_single_callback_argument_inline = !has_multiline_jsx_argument_signal
        && !has_call_infix_annotations
        && !call_has_await_ancestor(context, call_node_id)
        && !force_expand_single_multiline_with_static_arguments
        && !force_expand_single_collection_for_type_binary_callee
        && !has_boundary_comments
        && !context.has_non_blank_annotation(argument_id)
        && !context.has_non_blank_annotation(value_id)
        && !context.node_has_newline(value_id)
        && !argument_has_callback_blocking_comment_annotation(context, argument_id)
        && !argument_has_leading_prefix_annotation_outside_span(context, argument_id)
        && argument_is_lambda_expression(context, argument_id);
    if use_single_callback_argument_inline {
        context.increment_counter("call.arguments.path.single_callback_inline", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    // short single positional arguments can stay inline
    let use_single_simple_argument = !force_expand_single_multiline_with_static_arguments
        && !force_expand_single_collection_for_type_binary_callee
        && !single_argument_force_expand
        && !has_any_argument_annotation
        && !context.has_non_blank_annotation(argument_id)
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
    if use_single_simple_argument {
        context.increment_counter("call.arguments.path.single_simple", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    // non-interpolated template literal snapshot arguments can stay inline
    let use_single_template_argument_inline = !has_boundary_comments
        && !has_call_infix_annotations
        && !has_any_argument_annotation
        && argument_is_template_literal(context, argument_id)
        && !argument_is_interpolated_template_literal(context, argument_id);
    if use_single_template_argument_inline {
        context.increment_counter("call.arguments.path.single_template_inline", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    // chained single-argument calls prefer inline argument docs:
    // chain layout should break at member separators, not inside one argument list
    let use_single_chain_argument_inline = layout_cache.has_call_chain_parent
        && !has_boundary_comments
        && !has_call_infix_annotations
        && !has_any_argument_annotation
        && !single_argument_force_expand
        && !force_expand_single_multiline_with_static_arguments
        && !force_expand_single_collection_for_type_binary_callee
        && !argument_has_line_comment_annotation(context, argument_id)
        && !argument_is_lambda_expression(context, argument_id)
        && !argument_is_function_expression(context, argument_id)
        && !argument_is_interpolated_template_literal(context, argument_id);
    if use_single_chain_argument_inline {
        context.increment_counter("call.arguments.path.single_chain_inline", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    None
}

/// Choose call argument layout from early and main rule phases.
struct CallCommentPhaseFacts {
    has_line_comment_annotations: bool,
    has_prefix_line_comment_annotations: bool,
    separator_facts: CallSeparatorLayoutFacts,
}

struct CallExpansionPhaseFacts {
    expansion: CallArgumentExpansionCache,
    force_expand: bool,
}

/// Collect comment and separator facts for call layout phases.
fn call_comment_phase_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_call_infix_annotations: bool,
) -> CallCommentPhaseFacts {
    let (has_line_comment_annotations, has_prefix_line_comment_annotations) =
        if has_any_argument_annotation || has_call_infix_annotations {
            let _timing =
                context.timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_SCAN);
            context.increment_counter("call.arguments.comment_scan.calls", 1);
            call_argument_comments(context, dynamic_arguments)
        } else {
            context.increment_counter("call.arguments.comment_scan.skip_no_annotation", 1);
            (false, false)
        };
    let separator_facts =
        collect_call_separator_layout_facts(context, call_node_id, dynamic_arguments);

    CallCommentPhaseFacts {
        has_line_comment_annotations,
        has_prefix_line_comment_annotations,
        separator_facts,
    }
}

/// Collect expansion and force-expand facts for call layout phases.
fn call_expansion_phase_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_boundary_comments: bool,
    has_line_comment_annotations: bool,
    separator_facts: &CallSeparatorLayoutFacts,
) -> CallExpansionPhaseFacts {
    let expansion = {
        let _timing = context.timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_SCAN);
        call_argument_expansion(context, call_node_id, dynamic_arguments)
    };

    let force_expand_for_structural_trailing_collection = dynamic_arguments.len() >= 3
        && expansion.trailing_collection_argument
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !has_boundary_comments
        && !separator_facts.has_trailing_collection_comment_signal
        && leading_arguments_are_compact_simple_unannotated(context, dynamic_arguments);
    let trailing_collection_comment_force_expand = dynamic_arguments.len() > 1
        && expansion.trailing_collection_argument
        && dynamic_arguments
            .last()
            .copied()
            .is_some_and(|last_argument_id| {
                let has_last_line_comment_annotation = has_line_comment_annotations
                    && argument_has_line_comment_annotation(context, last_argument_id);
                let has_last_source_comment = context.has_comment(context.span(last_argument_id));
                has_last_line_comment_annotation || has_last_source_comment
            });
    let force_expand = expansion.force_expand
        || has_boundary_comments
        || force_expand_for_structural_trailing_collection
        || trailing_collection_comment_force_expand;

    CallExpansionPhaseFacts {
        expansion,
        force_expand,
    }
}

/// Return whether call layout can consider hug-last inline rules.
fn call_can_consider_hug_last_argument(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_boundary_comments: bool,
    has_line_comment_annotations: bool,
    has_call_infix_annotations: bool,
) -> bool {
    dynamic_arguments.len() > 1
        && !(has_line_comment_annotations || has_boundary_comments)
        && !has_call_infix_annotations
        && dynamic_arguments.last().is_some_and(|argument_id| {
            is_block_lambda_argument(context, *argument_id)
                || argument_is_object_literal(context, *argument_id)
                || argument_is_array_literal(context, *argument_id)
                || argument_is_function_expression(context, *argument_id)
        })
}

pub(crate) fn call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    single_argument_force_expand: bool,
    force_expand_single_multiline_with_static_arguments: bool,
    force_expand_single_collection_for_type_binary_callee: bool,
    has_boundary_comments: bool,
) -> CallArgumentLayout {
    let layout_cache = call_argument_layout_cache(context, call_node_id, dynamic_arguments);
    let has_call_infix_annotations = layout_cache.has_call_infix_annotations;
    let has_any_argument_annotation = layout_cache.has_any_argument_annotation;
    if let Some(layout) = try_single_argument_inline_layout(
        context,
        call_node_id,
        dynamic_arguments,
        &layout_cache,
        single_argument_force_expand,
        force_expand_single_multiline_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
        has_boundary_comments,
    ) {
        return layout;
    }

    // hook-like callback plus deps-array arguments can stay inline
    if !has_boundary_comments
        && call_has_react_hook_like_callback_deps_array(context, dynamic_arguments)
    {
        context.increment_counter("call.arguments.path.react_hook_like_inline", 1);
        return CallArgumentLayout::InlineAll;
    }

    // leading callback plus short tail calls can stay inline
    if call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments) {
        context.increment_counter("call.arguments.path.leading_block_callback_inline", 1);
        return CallArgumentLayout::InlineAll;
    }

    // phase: comment and separator facts
    let comment_facts = call_comment_phase_facts(
        context,
        call_node_id,
        dynamic_arguments,
        has_any_argument_annotation,
        has_call_infix_annotations,
    );
    let has_line_comment_annotations = comment_facts.has_line_comment_annotations;
    let has_prefix_line_comment_annotations = comment_facts.has_prefix_line_comment_annotations;
    let separator_facts = &comment_facts.separator_facts;

    // phase: explicit comment-expanded multiline layout
    if dynamic_arguments.len() > 1
        && (has_line_comment_annotations || has_prefix_line_comment_annotations)
    {
        context.increment_counter("call.arguments.path.comment_expanded", 1);
        return build_comment_expanded_call_argument_layout(
            context,
            dynamic_arguments,
            separator_facts,
        );
    }

    // phase: expansion and force-expand
    let expansion_facts = call_expansion_phase_facts(
        context,
        call_node_id,
        dynamic_arguments,
        has_any_argument_annotation,
        has_boundary_comments,
        has_line_comment_annotations,
        separator_facts,
    );
    let expansion = expansion_facts.expansion;
    let force_expand = expansion_facts.force_expand;

    // phase: hug-last candidates can still end in default list rendering
    let can_consider_hug_last_argument = call_can_consider_hug_last_argument(
        context,
        dynamic_arguments,
        has_boundary_comments,
        has_line_comment_annotations,
        expansion.has_call_infix_annotations,
    );
    if can_consider_hug_last_argument {
        let _timing = context.timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_HUG_LAST);
        let force_hug_last_inline = should_force_hug_last_inline(
            context,
            call_node_id,
            dynamic_arguments,
            expansion.trailing_collection_argument,
        );
        if force_hug_last_inline {
            context.increment_counter("call.arguments.path.hug_last_forced", 1);
        }

        let last_argument_is_callback_like =
            dynamic_arguments
                .last()
                .copied()
                .is_some_and(|argument_id| {
                    is_block_lambda_argument(context, argument_id)
                        || argument_is_function_expression(context, argument_id)
                });
        let use_callback_tail_inline = !force_expand
            && last_argument_is_callback_like
            && leading_arguments_are_compact_callback_tail_candidates(context, dynamic_arguments);
        if use_callback_tail_inline {
            context.increment_counter("call.arguments.hug_last.callback_tail_inline", 1);
        }

        let use_collection_tail_inline = !force_expand && expansion.trailing_collection_argument;
        if !use_callback_tail_inline && use_collection_tail_inline {
            context.increment_counter("call.arguments.hug_last.collection_tail_inline", 1);
        }

        let use_hug_last_inline =
            force_hug_last_inline || use_callback_tail_inline || use_collection_tail_inline;

        if use_hug_last_inline {
            context.increment_counter("call.arguments.path.hug_last_inline", 1);
            return CallArgumentLayout::InlineAll;
        }
    }

    // phase: trailing collection patterns can force expanded list rendering
    let has_trailing_collection_argument = dynamic_arguments
        .last()
        .copied()
        .is_some_and(|argument_id| argument_is_collection_literal(context, argument_id));
    if force_expand
        && dynamic_arguments.len() > 1
        && has_trailing_collection_argument
        && !layout_cache.has_block_callback_argument
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !has_boundary_comments
        && !separator_facts.has_trailing_collection_comment_signal
    {
        context.increment_counter("call.arguments.path.trailing_collection_expanded", 1);
        return CallArgumentLayout::TrailingCollectionExpanded;
    }

    // fall back to default list rules
    context.increment_counter("call.arguments.path.list_default", 1);
    build_default_list_call_argument_layout(
        context,
        dynamic_arguments,
        force_expand,
        has_any_argument_annotation,
        has_line_comment_annotations,
        separator_facts,
    )
}

/// Return whether call arguments should force hug-last inline layout.
pub(crate) fn should_force_hug_last_inline(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    trailing_collection_argument: bool,
) -> bool {
    let can_force_hug_test_like_callback = dynamic_arguments.len() == 2
        && argument_is_string_like(context, dynamic_arguments[0])
        && (argument_is_lambda_expression(context, dynamic_arguments[1])
            || argument_is_function_expression(context, dynamic_arguments[1]));
    let can_force_hug_reference_callback_with_collection_tail = dynamic_arguments.len() >= 3
        && trailing_collection_argument
        && argument_is_reference_like(context, dynamic_arguments[0]);
    let can_force_hug_simple_block_lambda_tail = dynamic_arguments.len() <= 3
        && dynamic_arguments
            .last()
            .is_some_and(|argument_id| is_block_lambda_argument(context, *argument_id));

    let force_hug_test_like_callback = can_force_hug_test_like_callback
        && call_callee_has_test_like_member_name(context, call_node_id);
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
