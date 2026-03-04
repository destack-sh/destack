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
    AnnotationPosition, Argument, Declaration, DestackFormatContext, Expression, FunctionKind,
    LocalNodeId, NodeType, ScalarLiteral, TrailingComma, argument_is_array_literal,
    argument_is_block_callback, argument_is_function_expression, argument_is_lambda_expression,
    argument_is_object_literal, argument_is_template_literal, argument_value_id,
    is_block_lambda_argument, is_complex_argument, is_expression_chain, is_trivial_argument,
    transparent_inner_expression,
};
use crate::format::tree::has_multiline_jsx_argument;
use crate::{
    Annotation, CallArgumentExpansionCache, CallArgumentExpansionsCache, CallArgumentLayoutCache,
};
use destack_ast::TypeBinaryOperator;

/// Return whether all leading arguments before the last are compact and simple.
pub(crate) fn leading_arguments_are_compact_simple_unannotated(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    dynamic_arguments
        .split_last()
        .map_or(&[][..], |(_, leading_arguments)| leading_arguments)
        .iter()
        .copied()
        .all(|argument_id| argument_is_compact_simple_unannotated(ctx, argument_id))
}

/// Return whether all leading arguments before the last are compact callback-tail candidates.
pub(crate) fn leading_arguments_are_compact_callback_tail_candidates(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    dynamic_arguments
        .split_last()
        .map_or(&[][..], |(_, leading_arguments)| leading_arguments)
        .iter()
        .copied()
        .all(|argument_id| {
            if ctx.node_has_newline(argument_id)
                || argument_has_callback_blocking_comment_annotation(ctx, argument_id)
            {
                return false;
            }

            argument_is_simple_with_options(
                ctx,
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
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentLayoutCache {
    if let Some(cached) = ctx.lookup_call_argument_layout_cache(call_node_id) {
        ctx.increment_counter("call.arguments.layout_cache.cache.hits", 1);
        return cached;
    }

    ctx.increment_counter("call.arguments.layout_cache.cache.misses", 1);
    ctx.increment_counter("call.arguments.layout_cache.builds", 1);
    let has_call_infix_annotations = ctx.has_non_blank_infix_annotation(call_node_id);
    let has_call_chain_parent = call_has_call_chain_parent(ctx, call_node_id);

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
            ctx,
            call_node_id,
            dynamic_arguments,
            has_call_infix_annotations,
            has_call_chain_parent,
        )
    };

    ctx.store_call_argument_layout_cache(call_node_id, layout_cache);

    layout_cache
}

/// Return whether a call expression is used as the callee or receiver of a parent postfix chain.
pub(crate) fn call_has_call_chain_parent(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = ctx.parent(call_node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = ctx.tree.get(parent_expression_id);
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
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    ctx.any_ancestor(call_node_id, |parent_id, parent_type| {
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        matches!(
            ctx.tree.get(parent_expression_id),
            Expression::Await { .. } | Expression::AwaitMaybe { .. }
        )
    })
}

/// Return whether one argument is compact, unannotated, and simple.
pub(crate) fn argument_is_compact_simple_unannotated(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if let Some(cached) = ctx.lookup_argument_compact_simple_unannotated(argument_id) {
        ctx.increment_counter("call.arguments.compact_simple.cache.hits", 1);
        return cached;
    }

    ctx.increment_counter("call.arguments.compact_simple.cache.misses", 1);
    // blank seams should not declassify compact argument simplicity
    let is_compact_simple_unannotated = !ctx.has_non_blank_annotation(argument_id)
        && !ctx.node_has_newline(argument_id)
        && argument_is_simple_with_options(
            ctx,
            argument_id,
            ArgumentSimplicityOptions {
                reject_any_argument_annotation: true,
                reject_non_blank_argument_annotation: true,
                reject_value_annotation: true,
                reject_lambda_values: true,
            },
        );
    ctx.store_argument_compact_simple_unannotated(argument_id, is_compact_simple_unannotated);

    is_compact_simple_unannotated
}

/// Build one layout cache entry from scanned argument signals.
pub(crate) fn scan_call_argument_layout_cache(
    ctx: &DestackFormatContext<'_>,
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
    ctx.increment_counter(
        "call.arguments.layout.scan.arguments",
        dynamic_arguments.len(),
    );

    // scan each argument once and collect layout flags
    for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
        // layout decisions only treat non blank annotations as comment signals
        let has_annotation = ctx.has_non_blank_annotation(argument_id);
        let has_newline = ctx.node_has_newline(argument_id);
        if has_annotation {
            has_any_argument_annotation = true;
            if !has_line_comment_annotations
                && ctx.argument_annotation_cache(argument_id).has_line_comment
            {
                has_line_comment_annotations = true;
            }
        }

        let is_single_line_and_unannotated = !has_annotation && !has_newline;
        all_single_line_and_unannotated &= is_single_line_and_unannotated;
        if all_compact_simple_unannotated {
            all_compact_simple_unannotated = if is_single_line_and_unannotated {
                argument_is_compact_simple_unannotated(ctx, argument_id)
            } else {
                false
            };
        }

        if all_plain_call_arguments {
            all_plain_call_arguments &= argument_is_plain_call_argument(ctx, argument_id);
        }

        let argument = ctx.tree.get(argument_id);
        if matches!(argument, Argument::Spread { .. }) {
            has_spread_argument = true;
        }

        let value_id = argument_value_id(ctx.tree, argument_id);
        let value_id = transparent_inner_expression(ctx, value_id);
        let value = ctx.tree.get(value_id);
        let is_last_argument = index == last_argument_index;
        if is_last_argument {
            trailing_collection_argument = matches!(
                value,
                Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
            );
        }

        let (is_lambda_argument, is_function_argument, is_block_callback) = match value {
            Expression::Declaration(declaration_id) => match ctx.tree.get(*declaration_id) {
                Declaration::Function {
                    signature, body, ..
                } => {
                    let is_lambda_argument = signature.kind == FunctionKind::Lambda;
                    let is_function_argument = !is_lambda_argument;
                    let is_block_callback = is_lambda_argument
                        && body.is_some_and(|body_id| {
                            let body_id = transparent_inner_expression(ctx, body_id);
                            matches!(ctx.tree.get(body_id), Expression::Block(_))
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
            && is_complex_argument(ctx.tree, argument)
        {
            has_complex_non_callback_argument = true;
        }

        if !has_non_trivial_non_callback_argument
            && !is_last_argument
            && !is_trivial_argument(ctx.tree, argument)
        {
            has_non_trivial_non_callback_argument = true;
        }
    }

    let force_hug_last_inline = dynamic_arguments.len() > 1
        && !has_call_infix_annotations
        && !has_any_argument_annotation
        && all_single_line_and_unannotated
        && should_force_hug_last_inline(
            ctx,
            call_node_id,
            dynamic_arguments,
            trailing_collection_argument,
        );
    let is_multiline_in_source = call_arguments_are_multiline_span(ctx, dynamic_arguments);

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
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(ctx.tree, argument_id);
    let value_id = transparent_inner_expression(ctx, value_id);

    matches!(
        ctx.tree.get(value_id),
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
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> (bool, bool) {
    let mut has_line_comment_annotations = false;
    let mut has_prefix_line_comment_annotations = false;

    for argument_id in dynamic_arguments.iter().copied() {
        if !ctx.has_non_blank_annotation(argument_id) {
            continue;
        }

        let annotation_cache = ctx.argument_annotation_cache(argument_id);
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
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let annotation_cache = ctx.argument_annotation_cache(argument_id);
    annotation_cache.has_line_comment
        || annotation_cache.has_prefix_line_comment
        || (ctx.node_has_newline(argument_id) && annotation_cache.has_prefix_annotation)
}

// call argument layout thresholds
const NON_LAST_BLOCK_CALLBACK_COUNT_TARGET: usize = 1;
const NON_LAST_BLOCK_CALLBACK_MIN_INDEX: usize = 1;
const MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT: usize = 2;
const FUNCTION_COMPOSITION_MIN_ARGUMENTS: usize = 3;

/// Resolve regular call argument expansion with per-call caching.
pub(crate) fn call_argument_expansion(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentExpansionCache {
    if let Some(cached) = ctx.lookup_call_argument_expansion_cache(call_node_id) {
        ctx.increment_counter("call.arguments.regular.cache.hits", 1);
        let expansion = cached.regular;
        ctx.increment_counter(
            if expansion.force_expand {
                "call.arguments.regular.force_expand.true"
            } else {
                "call.arguments.regular.force_expand.false"
            },
            1,
        );

        return expansion;
    }

    ctx.increment_counter("call.arguments.regular.cache.misses", 1);

    let expansions = call_argument_expansions(ctx, call_node_id, dynamic_arguments);
    let expansion = expansions.regular;

    ctx.store_call_argument_expansion_cache(call_node_id, expansions);

    ctx.increment_counter(
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
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(cached) = ctx.lookup_call_argument_expansion_cache(call_node_id) {
        ctx.increment_counter("call.arguments.chain.cache.hits", 1);

        let force_expand = cached.chain_force_expand;
        ctx.increment_counter(
            if force_expand {
                "call.arguments.chain.force_expand.true"
            } else {
                "call.arguments.chain.force_expand.false"
            },
            1,
        );

        return force_expand;
    }

    ctx.increment_counter("call.arguments.chain.cache.misses", 1);

    let expansions = call_argument_expansions(ctx, call_node_id, dynamic_arguments);
    let force_expand = expansions.chain_force_expand;

    ctx.store_call_argument_expansion_cache(call_node_id, expansions);

    ctx.increment_counter(
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
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentExpansionsCache {
    ctx.increment_counter("call.arguments.layout.builds", 1);

    let has_call_infix_annotations = ctx.has_non_blank_infix_annotation(call_node_id);

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
            ctx,
            call_node_id,
            dynamic_arguments,
            has_call_infix_annotations,
        );
    }

    multi_argument_expansions(
        ctx,
        call_node_id,
        dynamic_arguments,
        has_call_infix_annotations,
    )
}

/// Build expansion data for one multi-argument call.
fn multi_argument_expansions(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
) -> CallArgumentExpansionsCache {
    let layout_cache = call_argument_layout_cache(ctx, call_node_id, dynamic_arguments);

    // compact unannotated argument lists do not need full expansion scans
    if !layout_cache.has_call_infix_annotations && layout_cache.all_compact_simple_unannotated {
        ctx.increment_counter("call.arguments.layout.simple_short_circuit", 1);

        return CallArgumentExpansionsCache {
            regular: CallArgumentExpansionCache {
                force_expand: false,
                has_call_infix_annotations: layout_cache.has_call_infix_annotations,
                trailing_collection_argument: layout_cache.trailing_collection_argument,
            },
            chain_force_expand: false,
        };
    }

    let force_expand_jsx = has_multiline_jsx_argument(ctx.tree, dynamic_arguments);
    let trailing_collection_argument = layout_cache.trailing_collection_argument;

    let allow_non_last_block_callback_with_collection_tail = !layout_cache
        .last_argument_is_block_callback
        && trailing_collection_argument
        && layout_cache.non_last_block_callback_count == NON_LAST_BLOCK_CALLBACK_COUNT_TARGET
        && layout_cache
            .non_last_block_callback_index
            .is_some_and(|index| index >= NON_LAST_BLOCK_CALLBACK_MIN_INDEX)
        && argument_is_reference_like(ctx, dynamic_arguments[0])
        && !layout_cache.has_non_trivial_non_callback_argument;

    let has_leading_block_callback_with_simple_tail =
        call_has_leading_block_callback_with_simple_tail(ctx, call_node_id, dynamic_arguments);

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
        ctx.increment_counter("call.arguments.layout.multiple_function_short_circuit", 1);

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
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 1
        || !call_has_static_arguments(ctx, call_node_id)
        || ctx.has_non_blank_annotation(dynamic_arguments[0])
    {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    if argument_is_lambda_expression(ctx, argument_id)
        || argument_is_function_expression(ctx, argument_id)
    {
        return false;
    }

    is_expression_chain(ctx.tree, call_node_id)
}

/// Return whether a single collection argument should expand for type binary callees.
pub(crate) fn call_force_expand_single_collection_for_type_binary_callee(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 1 || !argument_is_collection_literal(ctx, dynamic_arguments[0]) {
        return false;
    }

    let callee = match ctx.tree.get(call_node_id) {
        Expression::Call { left, .. } | Expression::New { left, .. } => *left,
        _ => return false,
    };
    let callee = transparent_inner_expression(ctx, callee);

    matches!(
        ctx.tree.get(callee),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    )
}

/// Return whether a single argument call should force expanded list layout.
pub(crate) fn single_argument_requires_expanded_list(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    // only single argument calls can use this path
    if dynamic_arguments.len() != 1 {
        return false;
    }

    // require a chain shaped argument value
    let argument_id = dynamic_arguments[0];
    let raw_value_id = argument_value_id(ctx.tree, argument_id);
    let value_id = transparent_inner_expression(ctx, raw_value_id);
    if !expression_is_chain_layout_candidate(ctx, value_id) {
        return false;
    }

    // force expand only from non-boundary annotation signals on the argument value path
    let has_annotation_signal = argument_has_non_blank_non_boundary_annotation(ctx, argument_id)
        || expression_has_non_blank_non_boundary_annotation(ctx, raw_value_id)
        || expression_has_non_blank_non_boundary_annotation(ctx, value_id);
    let has_boundary_signal = argument_has_boundary_comment_annotation(ctx, argument_id)
        || expression_has_boundary_comment_annotation(ctx, raw_value_id)
        || expression_has_boundary_comment_annotation(ctx, value_id);

    has_annotation_signal || has_boundary_signal
}

/// Return whether one argument has non-blank annotations excluding boundary postfix markers.
fn argument_has_non_blank_non_boundary_annotation(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.visit_annotations(argument_id, |annotations| {
        annotation_ids_have_non_blank_non_boundary_annotation(ctx, annotations)
    })
    .unwrap_or(false)
}

/// Return whether one argument has one boundary postfix comment annotation.
fn argument_has_boundary_comment_annotation(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.visit_annotations(argument_id, |annotations| {
        annotation_ids_have_boundary_comment_annotation(ctx, annotations)
    })
    .unwrap_or(false)
}

/// Return whether one expression has non-blank annotations excluding boundary postfix markers.
fn expression_has_non_blank_non_boundary_annotation(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    ctx.visit_annotations(expression_id, |annotations| {
        annotation_ids_have_non_blank_non_boundary_annotation(ctx, annotations)
    })
    .unwrap_or(false)
}

/// Return whether one expression has one boundary postfix comment annotation.
fn expression_has_boundary_comment_annotation(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    ctx.visit_annotations(expression_id, |annotations| {
        annotation_ids_have_boundary_comment_annotation(ctx, annotations)
    })
    .unwrap_or(false)
}

/// Return whether one annotation list has a non-blank non-boundary annotation.
fn annotation_ids_have_non_blank_non_boundary_annotation(
    ctx: &DestackFormatContext<'_>,
    annotation_ids: &[LocalNodeId<Annotation>],
) -> bool {
    annotation_ids
        .iter()
        .any(|annotation_id| match ctx.annotation(*annotation_id) {
            Annotation::Blank { .. } => false,
            Annotation::Comment { position, .. }
            | Annotation::Doc { position, .. }
            | Annotation::Decorator { position, .. } => {
                position != AnnotationPosition::LinePostfixBoundary
            }
        })
}

/// Return whether one annotation list has a boundary postfix comment annotation.
fn annotation_ids_have_boundary_comment_annotation(
    ctx: &DestackFormatContext<'_>,
    annotation_ids: &[LocalNodeId<Annotation>],
) -> bool {
    annotation_ids.iter().any(|annotation_id| {
        matches!(
            ctx.annotation(*annotation_id),
            Annotation::Comment {
                position: AnnotationPosition::LinePostfixBoundary,
                ..
            }
        )
    })
}

/// Return whether an expression should use chain-aware single-argument call layout rules.
fn expression_is_chain_layout_candidate(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    // regular member/call chain expressions
    if is_expression_chain(ctx.tree, expression_id) {
        return true;
    }

    // path-chain calls normalize from `(a).b()` to `a.b()` across passes
    match ctx.tree.get(expression_id) {
        Expression::Path { path, .. } => path.segments.len() > 1,
        Expression::Call { left, .. } | Expression::Instantiation { left, .. } => matches!(
            ctx.tree.get(*left),
            Expression::Path { path, .. } if path.segments.len() > 1
        ),
        _ => false,
    }
}

/// Build expansion data for one single-argument call.
fn single_argument_expansions(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
) -> CallArgumentExpansionsCache {
    let argument_id = dynamic_arguments[0];
    let argument_annotation_cache = ctx.argument_annotation_cache(argument_id);
    let has_line_comment_annotations = argument_annotation_cache.has_line_comment;
    let trailing_collection_argument = argument_is_collection_literal(ctx, argument_id);
    let has_collection_source_comment =
        trailing_collection_argument && ctx.has_comment(ctx.span(argument_id));
    let has_line_comment_annotations =
        has_line_comment_annotations || has_collection_source_comment;

    let force_expand_jsx = has_multiline_jsx_argument(ctx.tree, dynamic_arguments);
    let force_expand_single_commented_callback = argument_is_block_callback(ctx, argument_id)
        && (argument_has_callback_blocking_comment_annotation(ctx, argument_id)
            || has_call_infix_annotations);

    let force_expand_single_multiline_with_static_arguments =
        call_force_expand_single_multiline_with_static_arguments(
            ctx,
            call_node_id,
            dynamic_arguments,
        );
    let force_expand_single_collection_for_type_binary_callee =
        call_force_expand_single_collection_for_type_binary_callee(
            ctx,
            call_node_id,
            dynamic_arguments,
        );
    let force_expand_single_chain_argument =
        single_argument_requires_expanded_list(ctx, dynamic_arguments);
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
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };

    if !argument_is_collection_literal(ctx, last_argument_id) {
        return false;
    }

    if ctx.has_non_blank_annotation(last_argument_id) {
        return true;
    }

    let last_argument_value_id = argument_value_id(ctx.tree, last_argument_id);
    ctx.has_non_blank_annotation(last_argument_value_id)
}

/// Collect separator-comment facts used by comment-expanded and default list layouts.
fn collect_call_separator_layout_facts(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallSeparatorLayoutFacts {
    let single_argument_id = if dynamic_arguments.len() == 1 {
        Some(dynamic_arguments[0])
    } else {
        None
    };
    let last_argument_id = dynamic_arguments.last().copied();

    let single_argument_separator_source_exists = single_argument_id.is_some_and(|argument_id| {
        single_argument_separator_line_comment_source(ctx, call_node_id, argument_id).is_some()
    });
    let last_argument_separator_source_exists = if single_argument_id.is_some() {
        single_argument_separator_source_exists
    } else {
        last_argument_id.is_some_and(|argument_id| {
            single_argument_separator_line_comment_source(ctx, call_node_id, argument_id).is_some()
        })
    };

    let has_trailing_collection_comment_signal =
        trailing_collection_argument_has_comment_signal(ctx, dynamic_arguments);
    let has_single_separator_line_comment_annotation =
        single_argument_id.is_some_and(|argument_id| {
            argument_has_separator_line_comment_annotation(ctx, argument_id)
        });
    let has_last_separator_line_comment_annotation = last_argument_id.is_some_and(|argument_id| {
        argument_has_separator_line_comment_annotation(ctx, argument_id)
    });
    let last_separator_line_comment_source_missing =
        has_last_separator_line_comment_annotation && !last_argument_separator_source_exists;
    let use_separator_comment_multiline =
        can_format_multiline_call_argument_list_with_separator_line_comment(
            ctx,
            call_node_id,
            dynamic_arguments,
        );
    let use_single_plain_separator_comment_layout = single_argument_id.is_some_and(|argument_id| {
        argument_is_plain_call_argument(ctx, argument_id) && single_argument_separator_source_exists
    });

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
    ctx: &DestackFormatContext<'_>,
    _dynamic_arguments: &[LocalNodeId<Argument>],
    facts: &CallSeparatorLayoutFacts,
) -> CallArgumentLayout {
    let use_trailing_comma = ctx.options.trailing_comma == TrailingComma::All
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
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    facts: &CallSeparatorLayoutFacts,
) -> CallArgumentLayout {
    let is_single_argument = dynamic_arguments.len() == 1;
    let single_argument_id = dynamic_arguments.first().copied();
    let has_single_template_literal_argument = single_argument_id
        .is_some_and(|argument_id| argument_is_template_literal(ctx, argument_id));
    let has_single_interpolated_template_literal_argument = single_argument_id
        .is_some_and(|argument_id| argument_is_interpolated_template_literal(ctx, argument_id));
    let use_plain_default_short_circuit = !ctx.has_ignore_directive_markers()
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

/// Hold shared predicates for single-argument inline layout routing.
struct SingleArgumentInlineFacts {
    /// Whether one boundary annotation exists around the argument seam.
    has_boundary_comments: bool,
    /// Whether call infix annotations exist.
    has_call_infix_annotations: bool,
    /// Whether any argument annotation exists.
    has_any_argument_annotation: bool,
    /// Whether single-argument expansion is forced.
    single_argument_force_expand: bool,
    /// Whether static-plus-multiline expansion is forced.
    force_expand_single_multiline_with_static_arguments: bool,
    /// Whether type-binary callee collection expansion is forced.
    force_expand_single_collection_for_type_binary_callee: bool,
    /// Whether the call is part of a postfix chain.
    has_call_chain_parent: bool,
    /// Whether the call has an await ancestor.
    has_await_ancestor: bool,
    /// Whether the argument has a non blank annotation.
    argument_has_non_blank_annotation: bool,
    /// Whether the argument value has a non blank annotation.
    value_has_non_blank_annotation: bool,
    /// Whether callback-blocking annotations exist for the argument.
    has_callback_blocking_comment_annotation: bool,
    /// Whether a leading prefix annotation sits outside argument span.
    has_leading_prefix_annotation_outside_span: bool,
    /// Whether line comment annotations exist on the argument.
    has_line_comment_annotation: bool,
    /// Whether the argument is a function expression.
    is_function_expression: bool,
    /// Whether the argument is a lambda expression.
    is_lambda_expression: bool,
    /// Whether the argument is an interpolated template literal.
    is_interpolated_template_literal: bool,
    /// Whether the argument is a template literal.
    is_template_literal: bool,
    /// Whether the argument is simple under strict call options.
    is_simple_argument: bool,
}

/// Build single-argument inline layout facts when exactly one argument exists.
fn single_argument_inline_facts(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_cache: &CallArgumentLayoutCache,
    single_argument_force_expand: bool,
    force_expand_single_multiline_with_static_arguments: bool,
    force_expand_single_collection_for_type_binary_callee: bool,
    has_boundary_comments: bool,
) -> Option<SingleArgumentInlineFacts> {
    if dynamic_arguments.len() != 1 {
        return None;
    }

    let argument_id = dynamic_arguments[0];
    let value_id = argument_value_id(ctx.tree, argument_id);
    let is_simple_argument = argument_is_simple_with_options(
        ctx,
        argument_id,
        ArgumentSimplicityOptions {
            reject_any_argument_annotation: true,
            reject_non_blank_argument_annotation: true,
            reject_value_annotation: true,
            reject_lambda_values: true,
        },
    );

    Some(SingleArgumentInlineFacts {
        has_boundary_comments,
        has_call_infix_annotations: layout_cache.has_call_infix_annotations,
        has_any_argument_annotation: layout_cache.has_any_argument_annotation,
        single_argument_force_expand,
        force_expand_single_multiline_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
        has_call_chain_parent: layout_cache.has_call_chain_parent,
        has_await_ancestor: call_has_await_ancestor(ctx, call_node_id),
        argument_has_non_blank_annotation: ctx.has_non_blank_annotation(argument_id),
        value_has_non_blank_annotation: ctx.has_non_blank_annotation(value_id),
        has_callback_blocking_comment_annotation: argument_has_callback_blocking_comment_annotation(
            ctx,
            argument_id,
        ),
        has_leading_prefix_annotation_outside_span:
            argument_has_leading_prefix_annotation_outside_span(ctx, argument_id),
        has_line_comment_annotation: argument_has_line_comment_annotation(ctx, argument_id),
        is_function_expression: argument_is_function_expression(ctx, argument_id),
        is_lambda_expression: argument_is_lambda_expression(ctx, argument_id),
        is_interpolated_template_literal: argument_is_interpolated_template_literal(
            ctx,
            argument_id,
        ),
        is_template_literal: argument_is_template_literal(ctx, argument_id),
        is_simple_argument,
    })
}

/// Return whether function-expression single argument can stay inline.
fn can_use_single_function_argument_inline(facts: &SingleArgumentInlineFacts) -> bool {
    !facts.has_call_infix_annotations
        && !facts.force_expand_single_multiline_with_static_arguments
        && !facts.force_expand_single_collection_for_type_binary_callee
        && !facts.has_boundary_comments
        && !facts.argument_has_non_blank_annotation
        && !facts.value_has_non_blank_annotation
        && !facts.has_callback_blocking_comment_annotation
        && !facts.has_leading_prefix_annotation_outside_span
        && facts.is_function_expression
}

/// Return whether lambda single argument can stay inline.
fn can_use_single_callback_argument_inline(facts: &SingleArgumentInlineFacts) -> bool {
    !facts.has_call_infix_annotations
        && !facts.has_await_ancestor
        && !facts.force_expand_single_multiline_with_static_arguments
        && !facts.force_expand_single_collection_for_type_binary_callee
        && !facts.has_boundary_comments
        && !facts.argument_has_non_blank_annotation
        && !facts.value_has_non_blank_annotation
        && !facts.has_callback_blocking_comment_annotation
        && !facts.has_leading_prefix_annotation_outside_span
        && facts.is_lambda_expression
}

/// Return whether one simple single argument can stay inline.
fn can_use_single_simple_argument_inline(facts: &SingleArgumentInlineFacts) -> bool {
    !facts.force_expand_single_multiline_with_static_arguments
        && !facts.force_expand_single_collection_for_type_binary_callee
        && !facts.single_argument_force_expand
        && !facts.has_any_argument_annotation
        && !facts.argument_has_non_blank_annotation
        && facts.is_simple_argument
}

/// Return whether one non-interpolated template argument can stay inline.
fn can_use_single_template_argument_inline(facts: &SingleArgumentInlineFacts) -> bool {
    !facts.has_boundary_comments
        && !facts.has_call_infix_annotations
        && !facts.has_any_argument_annotation
        && facts.is_template_literal
        && !facts.is_interpolated_template_literal
}

/// Return whether one chain argument should stay inline to preserve chain breaks.
fn can_use_single_chain_argument_inline(facts: &SingleArgumentInlineFacts) -> bool {
    facts.has_call_chain_parent
        && !facts.has_boundary_comments
        && !facts.has_call_infix_annotations
        && !facts.has_any_argument_annotation
        && !facts.single_argument_force_expand
        && !facts.force_expand_single_multiline_with_static_arguments
        && !facts.force_expand_single_collection_for_type_binary_callee
        && !facts.has_line_comment_annotation
        && !facts.is_lambda_expression
        && !facts.is_function_expression
        && !facts.is_interpolated_template_literal
}

/// Try single-argument inline layout rules in priority order.
fn try_single_argument_inline_layout(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_cache: &CallArgumentLayoutCache,
    single_argument_force_expand: bool,
    force_expand_single_multiline_with_static_arguments: bool,
    force_expand_single_collection_for_type_binary_callee: bool,
    has_boundary_comments: bool,
) -> Option<CallArgumentLayout> {
    let facts = single_argument_inline_facts(
        ctx,
        call_node_id,
        dynamic_arguments,
        layout_cache,
        single_argument_force_expand,
        force_expand_single_multiline_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
        has_boundary_comments,
    )?;

    // function expression fast path
    if can_use_single_function_argument_inline(&facts) {
        ctx.increment_counter("call.arguments.path.single_function_inline", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    // lambda callback fast path
    if can_use_single_callback_argument_inline(&facts) {
        ctx.increment_counter("call.arguments.path.single_callback_inline", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    // simple argument fast path
    if can_use_single_simple_argument_inline(&facts) {
        ctx.increment_counter("call.arguments.path.single_simple", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    // non-interpolated template fast path
    if can_use_single_template_argument_inline(&facts) {
        ctx.increment_counter("call.arguments.path.single_template_inline", 1);
        return Some(CallArgumentLayout::InlineSingle);
    }

    // chain continuation fast path
    if can_use_single_chain_argument_inline(&facts) {
        ctx.increment_counter("call.arguments.path.single_chain_inline", 1);
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
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_call_infix_annotations: bool,
) -> CallCommentPhaseFacts {
    let (has_line_comment_annotations, has_prefix_line_comment_annotations) =
        if has_any_argument_annotation || has_call_infix_annotations {
            let _timing = ctx.timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_SCAN);
            ctx.increment_counter("call.arguments.comment_scan.calls", 1);
            call_argument_comments(ctx, dynamic_arguments)
        } else {
            ctx.increment_counter("call.arguments.comment_scan.skip_no_annotation", 1);
            (false, false)
        };
    let separator_facts = collect_call_separator_layout_facts(ctx, call_node_id, dynamic_arguments);

    CallCommentPhaseFacts {
        has_line_comment_annotations,
        has_prefix_line_comment_annotations,
        separator_facts,
    }
}

/// Collect expansion and force-expand facts for call layout phases.
fn call_expansion_phase_facts(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_boundary_comments: bool,
    has_line_comment_annotations: bool,
    separator_facts: &CallSeparatorLayoutFacts,
) -> CallExpansionPhaseFacts {
    let expansion = {
        let _timing = ctx.timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_SCAN);
        call_argument_expansion(ctx, call_node_id, dynamic_arguments)
    };

    let force_expand_for_structural_trailing_collection = dynamic_arguments.len() >= 3
        && expansion.trailing_collection_argument
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !has_boundary_comments
        && !separator_facts.has_trailing_collection_comment_signal
        && leading_arguments_are_compact_simple_unannotated(ctx, dynamic_arguments);
    let trailing_collection_comment_force_expand = dynamic_arguments.len() > 1
        && expansion.trailing_collection_argument
        && dynamic_arguments
            .last()
            .copied()
            .is_some_and(|last_argument_id| {
                let has_last_line_comment_annotation = has_line_comment_annotations
                    && argument_has_line_comment_annotation(ctx, last_argument_id);
                let has_last_source_comment = ctx.has_comment(ctx.span(last_argument_id));
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
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_boundary_comments: bool,
    has_line_comment_annotations: bool,
    has_call_infix_annotations: bool,
) -> bool {
    dynamic_arguments.len() > 1
        && !(has_line_comment_annotations || has_boundary_comments)
        && !has_call_infix_annotations
        && dynamic_arguments.last().is_some_and(|argument_id| {
            is_block_lambda_argument(ctx, *argument_id)
                || argument_is_object_literal(ctx, *argument_id)
                || argument_is_array_literal(ctx, *argument_id)
                || argument_is_function_expression(ctx, *argument_id)
        })
}

/// Try to select hug-last inline layout for callback and collection tails.
fn try_hug_last_inline_layout(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_boundary_comments: bool,
    has_line_comment_annotations: bool,
    expansion: CallArgumentExpansionCache,
    force_expand: bool,
) -> Option<CallArgumentLayout> {
    let can_consider_hug_last_argument = call_can_consider_hug_last_argument(
        ctx,
        dynamic_arguments,
        has_boundary_comments,
        has_line_comment_annotations,
        expansion.has_call_infix_annotations,
    );
    if !can_consider_hug_last_argument {
        return None;
    }

    let _timing = ctx.timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_HUG_LAST);
    let force_hug_last_inline = should_force_hug_last_inline(
        ctx,
        call_node_id,
        dynamic_arguments,
        expansion.trailing_collection_argument,
    );
    if force_hug_last_inline {
        ctx.increment_counter("call.arguments.path.hug_last_forced", 1);
    }

    let last_argument_is_callback_like =
        dynamic_arguments
            .last()
            .copied()
            .is_some_and(|argument_id| {
                is_block_lambda_argument(ctx, argument_id)
                    || argument_is_function_expression(ctx, argument_id)
            });
    let use_callback_tail_inline = !force_expand
        && last_argument_is_callback_like
        && leading_arguments_are_compact_callback_tail_candidates(ctx, dynamic_arguments);
    if use_callback_tail_inline {
        ctx.increment_counter("call.arguments.hug_last.callback_tail_inline", 1);
    }

    let use_collection_tail_inline = !force_expand && expansion.trailing_collection_argument;
    if !use_callback_tail_inline && use_collection_tail_inline {
        ctx.increment_counter("call.arguments.hug_last.collection_tail_inline", 1);
    }

    let use_hug_last_inline =
        force_hug_last_inline || use_callback_tail_inline || use_collection_tail_inline;
    if !use_hug_last_inline {
        return None;
    }

    ctx.increment_counter("call.arguments.path.hug_last_inline", 1);
    Some(CallArgumentLayout::InlineAll)
}

/// Return whether call layout should use trailing-collection expanded rendering.
fn should_use_trailing_collection_expanded_layout(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_cache: CallArgumentLayoutCache,
    force_expand: bool,
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    has_boundary_comments: bool,
    separator_facts: &CallSeparatorLayoutFacts,
) -> bool {
    let has_trailing_collection_argument = dynamic_arguments
        .last()
        .copied()
        .is_some_and(|argument_id| argument_is_collection_literal(ctx, argument_id));
    force_expand
        && dynamic_arguments.len() > 1
        && has_trailing_collection_argument
        && !layout_cache.has_block_callback_argument
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !has_boundary_comments
        && !separator_facts.has_trailing_collection_comment_signal
}

pub(crate) fn call_argument_layout(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    single_argument_force_expand: bool,
    force_expand_single_multiline_with_static_arguments: bool,
    force_expand_single_collection_for_type_binary_callee: bool,
    has_boundary_comments: bool,
) -> CallArgumentLayout {
    let layout_cache = call_argument_layout_cache(ctx, call_node_id, dynamic_arguments);
    let has_call_infix_annotations = layout_cache.has_call_infix_annotations;
    let has_any_argument_annotation = layout_cache.has_any_argument_annotation;

    // phase: comment and separator facts
    let comment_facts = call_comment_phase_facts(
        ctx,
        call_node_id,
        dynamic_arguments,
        has_any_argument_annotation,
        has_call_infix_annotations,
    );
    let has_line_comment_annotations = comment_facts.has_line_comment_annotations;
    let has_prefix_line_comment_annotations = comment_facts.has_prefix_line_comment_annotations;
    let separator_facts = &comment_facts.separator_facts;

    // phase: single plain separator comments should route through comment-expanded rendering
    if dynamic_arguments.len() == 1 && separator_facts.use_single_plain_separator_comment_layout {
        ctx.increment_counter(
            "call.arguments.path.comment_expanded.single_plain_separator",
            1,
        );
        return build_comment_expanded_call_argument_layout(
            ctx,
            dynamic_arguments,
            separator_facts,
        );
    }

    if let Some(layout) = try_single_argument_inline_layout(
        ctx,
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
        && call_has_react_hook_like_callback_deps_array(ctx, dynamic_arguments)
    {
        ctx.increment_counter("call.arguments.path.react_hook_like_inline", 1);
        return CallArgumentLayout::InlineAll;
    }

    // leading callback plus short tail calls can stay inline
    if call_has_leading_block_callback_with_simple_tail(ctx, call_node_id, dynamic_arguments) {
        ctx.increment_counter("call.arguments.path.leading_block_callback_inline", 1);
        return CallArgumentLayout::InlineAll;
    }

    // phase: explicit comment-expanded multiline layout
    if dynamic_arguments.len() > 1
        && (has_line_comment_annotations || has_prefix_line_comment_annotations)
    {
        ctx.increment_counter("call.arguments.path.comment_expanded", 1);
        return build_comment_expanded_call_argument_layout(
            ctx,
            dynamic_arguments,
            separator_facts,
        );
    }

    // phase: expansion and force-expand
    let expansion_facts = call_expansion_phase_facts(
        ctx,
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
    if let Some(layout) = try_hug_last_inline_layout(
        ctx,
        call_node_id,
        dynamic_arguments,
        has_boundary_comments,
        has_line_comment_annotations,
        expansion,
        force_expand,
    ) {
        return layout;
    }

    // phase: trailing collection patterns can force expanded list rendering
    if should_use_trailing_collection_expanded_layout(
        ctx,
        dynamic_arguments,
        layout_cache,
        force_expand,
        has_any_argument_annotation,
        has_line_comment_annotations,
        has_boundary_comments,
        separator_facts,
    ) {
        ctx.increment_counter("call.arguments.path.trailing_collection_expanded", 1);
        return CallArgumentLayout::TrailingCollectionExpanded;
    }

    // fall back to default list rules
    ctx.increment_counter("call.arguments.path.list_default", 1);
    build_default_list_call_argument_layout(
        ctx,
        dynamic_arguments,
        force_expand,
        has_any_argument_annotation,
        has_line_comment_annotations,
        separator_facts,
    )
}

/// Return whether call arguments should force hug-last inline layout.
pub(crate) fn should_force_hug_last_inline(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    trailing_collection_argument: bool,
) -> bool {
    let can_force_hug_test_like_callback = dynamic_arguments.len() == 2
        && argument_is_string_like(ctx, dynamic_arguments[0])
        && (argument_is_lambda_expression(ctx, dynamic_arguments[1])
            || argument_is_function_expression(ctx, dynamic_arguments[1]));
    let can_force_hug_reference_callback_with_collection_tail = dynamic_arguments.len() >= 3
        && trailing_collection_argument
        && argument_is_reference_like(ctx, dynamic_arguments[0]);
    let can_force_hug_simple_block_lambda_tail = dynamic_arguments.len() <= 3
        && dynamic_arguments
            .last()
            .is_some_and(|argument_id| is_block_lambda_argument(ctx, *argument_id));

    let force_hug_test_like_callback = can_force_hug_test_like_callback
        && call_callee_has_test_like_member_name(ctx, call_node_id);
    let force_hug_reference_callback_with_collection_tail =
        can_force_hug_reference_callback_with_collection_tail
            && dynamic_arguments
                .iter()
                .skip(1)
                .take(dynamic_arguments.len().saturating_sub(2))
                .any(|argument_id| {
                    argument_is_lambda_expression(ctx, *argument_id)
                        || argument_is_function_expression(ctx, *argument_id)
                });
    let force_hug_simple_block_lambda_tail = can_force_hug_simple_block_lambda_tail
        && leading_arguments_are_compact_simple_unannotated(ctx, dynamic_arguments)
        && leading_arguments_are_compact_callback_tail_candidates(ctx, dynamic_arguments);

    force_hug_test_like_callback
        || force_hug_reference_callback_with_collection_tail
        || force_hug_simple_block_lambda_tail
}

/// Return whether a call matches the react hook callback-plus-deps pattern.
pub(crate) fn call_has_react_hook_like_callback_deps_array(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() < 2 || dynamic_arguments.len() > 3 {
        return false;
    }

    let callback_index = if dynamic_arguments.len() == 2 { 0 } else { 1 };
    let deps_index = callback_index + 1;

    if dynamic_arguments.len() == 3 && !argument_is_identifier_reference(ctx, dynamic_arguments[0])
    {
        return false;
    }

    argument_is_zero_parameter_block_lambda(ctx, dynamic_arguments[callback_index])
        && argument_is_array_literal(ctx, dynamic_arguments[deps_index])
}

/// Return whether one argument is a string or template literal.
fn argument_is_string_like(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(ctx.tree, argument_id);
    let value_id = transparent_inner_expression(ctx, value_id);

    matches!(
        ctx.tree.get(value_id),
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    )
}

/// Return whether an argument is a single-segment identifier path.
fn argument_is_identifier_reference(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(ctx.tree, argument_id);
    let value_id = transparent_inner_expression(ctx, value_id);

    matches!(
        ctx.tree.get(value_id),
        Expression::Path {
            path,
            static_arguments: None
        } if path.segments.len() == 1
    )
}

/// Return whether an argument is a zero-parameter block lambda.
fn argument_is_zero_parameter_block_lambda(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_block_callback(ctx, argument_id) {
        return false;
    }

    let value_id = argument_value_id(ctx.tree, argument_id);
    let value_id = transparent_inner_expression(ctx, value_id);
    let Expression::Declaration(declaration_id) = ctx.tree.get(value_id) else {
        return false;
    };
    let Declaration::Function { signature, .. } = ctx.tree.get(*declaration_id) else {
        return false;
    };

    signature.kind == FunctionKind::Lambda && signature.dynamic_parameters.is_empty()
}

/// Return whether a callee ends in a test style member name.
fn call_callee_has_test_like_member_name(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = ctx.tree.get(call_node_id) else {
        return false;
    };

    let mut current_id = *left;
    loop {
        current_id = transparent_inner_expression(ctx, current_id);
        match ctx.tree.get(current_id) {
            Expression::Member { name, .. } | Expression::PrivateMember { name, .. } => {
                let name = ctx.strings.get(*name);
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
                let name = ctx.strings.get(*last_segment);
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
