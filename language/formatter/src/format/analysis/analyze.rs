use super::call::{
    call_argument_layout_facts_are_simple_multi_unannotated, resolve_call_argument_layout_facts,
};
use super::classify::{
    argument_has_non_blank_annotation, argument_is_collection_literal,
    argument_is_interpolated_template_literal, argument_is_reference_like,
    call_has_leading_block_callback_with_simple_tail, call_has_non_blank_infix_annotation,
    call_has_static_arguments, call_like_has_type_binary_callee,
};
use crate::expression::{
    argument_is_function_expression, argument_is_lambda_expression, argument_value_id,
    expression_inline_width_hint, is_expression_chain, token, transparent_inner_expression,
};
use crate::tree::{
    argument_is_block_callback, argument_is_template_literal, has_multiline_jsx_argument,
};
use crate::{
    CallArgumentExpansionProfileFacts, CallArgumentExpansionProfilesFacts, DestackFormatContext,
    DestackFormatter,
};
use destack_ast::{Argument, Expression, LocalNodeId};
use destack_fir::format::Buffer;
use destack_fir::prelude::{FormatResult, space};
use destack_fir::write;

// call argument layout thresholds
const NON_LAST_BLOCK_CALLBACK_COUNT_TARGET: usize = 1;
const NON_LAST_BLOCK_CALLBACK_MIN_INDEX: usize = 1;
const FIRST_BLOCK_CALLBACK_COLLECTION_TAIL_ARGUMENT_COUNT: usize = 2;
const MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT: usize = 2;
const FUNCTION_COMPOSITION_MIN_ARGUMENTS: usize = 3;

/// Store derived call argument expansion flags.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentExpansionProfile {
    /// The final force expand decision.
    pub(crate) force_expand: bool,
    /// Whether the call has a non blank infix annotation.
    pub(crate) has_call_infix_annotations: bool,
    /// Whether the last argument is a collection literal.
    pub(crate) trailing_collection_argument: bool,
}

impl From<CallArgumentExpansionProfileFacts> for CallArgumentExpansionProfile {
    fn from(cached: CallArgumentExpansionProfileFacts) -> Self {
        Self {
            force_expand: cached.force_expand,
            has_call_infix_annotations: cached.has_call_infix_annotations,
            trailing_collection_argument: cached.trailing_collection_argument,
        }
    }
}

impl From<CallArgumentExpansionProfile> for CallArgumentExpansionProfileFacts {
    fn from(profile: CallArgumentExpansionProfile) -> Self {
        Self {
            force_expand: profile.force_expand,
            has_call_infix_annotations: profile.has_call_infix_annotations,
            trailing_collection_argument: profile.trailing_collection_argument,
        }
    }
}

/// Store regular and chain expansion results for one call expression.
#[derive(Clone, Copy)]
pub(crate) struct CallArgumentExpansionProfiles {
    /// The regular call formatting expansion profile.
    pub(crate) regular: CallArgumentExpansionProfile,
    /// Whether chain formatting should force expansion.
    pub(crate) chain_force_expand: bool,
}

impl From<CallArgumentExpansionProfilesFacts> for CallArgumentExpansionProfiles {
    fn from(cached: CallArgumentExpansionProfilesFacts) -> Self {
        Self {
            regular: CallArgumentExpansionProfile::from(cached.regular),
            chain_force_expand: cached.chain_force_expand,
        }
    }
}

impl From<CallArgumentExpansionProfiles> for CallArgumentExpansionProfilesFacts {
    fn from(profiles: CallArgumentExpansionProfiles) -> Self {
        Self {
            regular: CallArgumentExpansionProfileFacts::from(profiles.regular),
            chain_force_expand: profiles.chain_force_expand,
        }
    }
}

/// Store comment and boundary data for call argument layout decisions.
#[derive(Default)]
pub(crate) struct CallArgumentCommentProfile {
    /// Whether any argument has a line comment annotation.
    pub(crate) has_line_comment_annotations: bool,
    /// Whether any argument has a prefix line comment annotation.
    pub(crate) has_prefix_line_comment_annotations: bool,
}

/// Collect one-pass comment data for call arguments.
pub(crate) fn collect_call_argument_comment_profile(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentCommentProfile {
    let mut has_line_comment_annotations = false;
    let mut has_prefix_line_comment_annotations = false;

    for argument_id in dynamic_arguments.iter().copied() {
        let argument_has_annotation = context.has_annotation(argument_id);
        if argument_has_annotation {
            let annotation_profile = context.ensure_argument_annotation_facts(argument_id);
            if !has_line_comment_annotations && annotation_profile.has_line_comment {
                has_line_comment_annotations = true;
            }
            if !has_prefix_line_comment_annotations && annotation_profile.has_prefix_line_comment {
                has_prefix_line_comment_annotations = true;
            }
        }
    }

    CallArgumentCommentProfile {
        has_line_comment_annotations,
        has_prefix_line_comment_annotations,
    }
}

/// Return single argument expansion facts without scanning all arguments.
pub(crate) fn collect_single_call_argument_facts(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> (bool, bool) {
    let has_line_comment_annotations = context
        .ensure_argument_annotation_facts(argument_id)
        .has_line_comment;
    let trailing_collection_argument = argument_is_collection_literal(context, argument_id);
    let has_collection_source_comment = if trailing_collection_argument {
        let argument_span = context.span(argument_id);
        context.has_comment(argument_span)
    } else {
        false
    };

    (
        has_line_comment_annotations || has_collection_source_comment,
        trailing_collection_argument,
    )
}

/// Return whether one argument has callback-blocking line or multiline prefix annotations.
pub(crate) fn argument_has_callback_blocking_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let profile = context.ensure_argument_annotation_facts(argument_id);
    profile.has_line_comment
        || profile.has_prefix_line_comment
        || (context.node_has_newline(argument_id) && profile.has_prefix_annotation)
}

/// Return whether a single static argument call should force expansion.
pub(crate) fn call_force_expand_single_multiline_with_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 1
        || !call_has_static_arguments(context, call_node_id)
        || argument_has_non_blank_annotation(context, dynamic_arguments[0])
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
    dynamic_arguments.len() == 1
        && argument_is_collection_literal(context, dynamic_arguments[0])
        && call_like_has_type_binary_callee(context, call_node_id)
}

/// Return whether a single argument call should force expanded list layout.
pub(crate) fn single_argument_requires_expanded_list(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() != 1 {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    if !is_expression_chain(context.tree, value_id) {
        return false;
    }

    context.node_has_newline(value_id)
}

/// Build regular and chain call expansion profiles for one call expression.
pub(crate) fn build_call_argument_expansion_profiles(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentExpansionProfiles {
    context.increment_counter("call.arguments.layout.builds", 1);
    let has_call_infix_annotations = call_has_non_blank_infix_annotation(context, call_node_id);

    if dynamic_arguments.is_empty() {
        return CallArgumentExpansionProfiles {
            regular: CallArgumentExpansionProfile {
                force_expand: false,
                has_call_infix_annotations,
                trailing_collection_argument: false,
            },
            chain_force_expand: false,
        };
    }

    if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];
        let ensure_argument_annotation_facts =
            context.ensure_argument_annotation_facts(argument_id);
        let (has_line_comment_annotations, trailing_collection_argument) =
            collect_single_call_argument_facts(context, argument_id);
        let force_expand_jsx = has_multiline_jsx_argument(context.tree, dynamic_arguments);
        let force_expand_single_commented_callback =
            argument_is_block_callback(context, argument_id)
                && (argument_has_callback_blocking_comment_annotation(context, argument_id)
                    || has_call_infix_annotations);
        // keep non-new single multiline arguments expanded for stable call wrapping
        let argument_value_id = argument_value_id(context.tree, argument_id);
        let argument_value_id = transparent_inner_expression(context, argument_value_id);
        let argument_value_is_tree_expression = matches!(
            context.tree.get(argument_value_id),
            Expression::TreeExpression { .. }
        );
        let force_expand_single_multiline_argument =
            !is_expression_chain(context.tree, call_node_id)
                && context.node_has_newline(argument_id)
                && !argument_is_collection_literal(context, argument_id)
                && !argument_is_lambda_expression(context, argument_id)
                && !argument_is_function_expression(context, argument_id)
                && !argument_value_is_tree_expression
                && (!argument_is_template_literal(context, argument_id)
                    || argument_is_interpolated_template_literal(context, argument_id));
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
            ensure_argument_annotation_facts.has_prefix_line_comment;

        let regular_force_expand = force_expand_jsx
            || has_line_comment_annotations
            || force_expand_single_commented_callback
            || force_expand_single_multiline_argument
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

        return CallArgumentExpansionProfiles {
            regular: CallArgumentExpansionProfile {
                force_expand: regular_force_expand,
                has_call_infix_annotations,
                trailing_collection_argument,
            },
            chain_force_expand,
        };
    }

    let layout_class = resolve_call_argument_layout_facts(context, call_node_id, dynamic_arguments);

    // compact unannotated argument lists do not require full expansion profiling
    let can_use_simple_multi_argument_short_circuit =
        call_argument_layout_facts_are_simple_multi_unannotated(layout_class);
    if can_use_simple_multi_argument_short_circuit {
        context.increment_counter("call.arguments.layout.simple_short_circuit", 1);
        return CallArgumentExpansionProfiles {
            regular: CallArgumentExpansionProfile {
                force_expand: false,
                has_call_infix_annotations: layout_class.has_call_infix_annotations,
                trailing_collection_argument: false,
            },
            chain_force_expand: false,
        };
    }

    let has_line_comment_annotations = layout_class.has_line_comment_annotations;
    let force_expand_jsx = has_multiline_jsx_argument(context.tree, dynamic_arguments);
    let trailing_collection_argument = layout_class.trailing_collection_argument;
    let has_block_callback_argument = layout_class.has_block_callback_argument;
    let last_argument_is_block_callback = layout_class.last_argument_is_block_callback;
    let first_argument_is_block_callback = layout_class.first_argument_is_block_callback;
    let has_non_trivial_non_callback_argument = layout_class.has_non_trivial_non_callback_argument;
    let non_last_block_callback_count = layout_class.non_last_block_callback_count;
    let non_last_block_callback_index = layout_class.non_last_block_callback_index;
    let allow_non_last_block_callback_with_collection_tail = !last_argument_is_block_callback
        && trailing_collection_argument
        && non_last_block_callback_count == NON_LAST_BLOCK_CALLBACK_COUNT_TARGET
        && non_last_block_callback_index
            .is_some_and(|index| index >= NON_LAST_BLOCK_CALLBACK_MIN_INDEX)
        && argument_is_reference_like(context, dynamic_arguments[0])
        && !has_non_trivial_non_callback_argument;
    let force_expand_first_block_callback_with_collection_tail = dynamic_arguments.len()
        == FIRST_BLOCK_CALLBACK_COLLECTION_TAIL_ARGUMENT_COUNT
        && first_argument_is_block_callback
        && trailing_collection_argument;
    let has_leading_block_callback_with_simple_tail =
        call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments);
    let should_expand_for_block_callback = has_block_callback_argument
        && (force_expand_first_block_callback_with_collection_tail
            || has_non_trivial_non_callback_argument
            || (!last_argument_is_block_callback
                && !allow_non_last_block_callback_with_collection_tail
                && !has_leading_block_callback_with_simple_tail));
    let arrow_argument_count = layout_class.arrow_argument_count;
    let function_argument_count = layout_class.function_argument_count;
    let has_any_function_argument = arrow_argument_count > 0 || function_argument_count > 0;
    let has_multiple_function_arguments = arrow_argument_count
        >= MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT
        || function_argument_count >= MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT;
    if has_multiple_function_arguments {
        context.increment_counter("call.arguments.layout.multiple_function_short_circuit", 1);
        return CallArgumentExpansionProfiles {
            regular: CallArgumentExpansionProfile {
                force_expand: true,
                has_call_infix_annotations,
                trailing_collection_argument,
            },
            chain_force_expand: true,
        };
    }

    let has_spread_argument = layout_class.has_spread_argument;
    let force_expand_function_composition = dynamic_arguments.len()
        >= FUNCTION_COMPOSITION_MIN_ARGUMENTS
        && has_any_function_argument
        && !has_spread_argument;
    let has_non_complex_force_expand_signal = force_expand_jsx
        || has_line_comment_annotations
        || should_expand_for_block_callback
        || force_expand_function_composition
        || has_call_infix_annotations;
    let force_expand_complex = if has_non_complex_force_expand_signal {
        false
    } else {
        layout_class.has_complex_non_callback_argument
    };

    let regular_force_expand = force_expand_jsx
        || force_expand_complex
        || has_line_comment_annotations
        || should_expand_for_block_callback
        || has_multiple_function_arguments
        || force_expand_function_composition
        || has_call_infix_annotations;
    let chain_force_expand = force_expand_jsx
        || force_expand_complex
        || has_line_comment_annotations
        || should_expand_for_block_callback
        || has_multiple_function_arguments;

    CallArgumentExpansionProfiles {
        regular: CallArgumentExpansionProfile {
            force_expand: regular_force_expand,
            has_call_infix_annotations: layout_class.has_call_infix_annotations,
            trailing_collection_argument,
        },
        chain_force_expand,
    }
}

/// Resolve regular call argument expansion profile with per-call caching.
pub(crate) fn resolve_regular_call_argument_expansion_profile(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentExpansionProfile {
    if let Some(cached) = context.lookup_call_argument_expansion_facts(call_node_id) {
        context.increment_counter("call.arguments.regular.cache.hits", 1);
        let profile = CallArgumentExpansionProfiles::from(cached).regular;
        context.increment_counter(
            if profile.force_expand {
                "call.arguments.regular.force_expand.true"
            } else {
                "call.arguments.regular.force_expand.false"
            },
            1,
        );
        return profile;
    }

    context.increment_counter("call.arguments.regular.cache.misses", 1);
    let profiles = build_call_argument_expansion_profiles(context, call_node_id, dynamic_arguments);
    let profile = profiles.regular;
    context.store_call_argument_expansion_facts(
        call_node_id,
        CallArgumentExpansionProfilesFacts::from(profiles),
    );
    context.increment_counter(
        if profile.force_expand {
            "call.arguments.regular.force_expand.true"
        } else {
            "call.arguments.regular.force_expand.false"
        },
        1,
    );

    profile
}

/// Resolve chain call argument force-expand decision with per-call caching.
pub(crate) fn resolve_chain_call_argument_force_expand(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(cached) = context.lookup_call_argument_expansion_facts(call_node_id) {
        context.increment_counter("call.arguments.chain.cache.hits", 1);
        let force_expand = CallArgumentExpansionProfiles::from(cached).chain_force_expand;
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
    let profiles = build_call_argument_expansion_profiles(context, call_node_id, dynamic_arguments);
    let force_expand = profiles.chain_force_expand;
    context.store_call_argument_expansion_facts(
        call_node_id,
        CallArgumentExpansionProfilesFacts::from(profiles),
    );
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

/// Return whether single argument hugged formatting should force expansion.
pub(crate) fn call_should_force_hugged_expand(
    force_expand_single_collection_for_type_binary_callee: bool,
) -> bool {
    force_expand_single_collection_for_type_binary_callee
}

/// Return one-line `callee(arg1, arg2)` inline width hint for plain call expressions.
pub(crate) fn call_inline_width_hint_without_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    call_has_static_arguments: bool,
) -> Option<usize> {
    let Expression::Call { .. } = context.tree.get(call_node_id) else {
        return None;
    };

    if call_has_static_arguments {
        return None;
    }

    Some(expression_inline_width_hint(context, call_node_id))
}

/// Write an inline comma-separated call argument list.
pub(crate) fn write_inline_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    if all_plain_call_arguments {
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            write_plain_call_argument(f, *argument_id)?;
        }
    } else {
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            write_plain_call_argument_or_node(f, *argument_id)?;
        }
    }
    write!(f, [token(")")])?;

    Ok(())
}

/// Return whether an argument can be emitted directly without argument-node formatting.
pub(crate) fn argument_is_plain_call_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if let Some(cached) = context.lookup_argument_plain_call_argument(argument_id) {
        context.increment_counter("call.arguments.plain.cache.hits", 1);
        return cached;
    }
    context.increment_counter("call.arguments.plain.cache.misses", 1);

    let is_plain = !context.has_annotation(argument_id)
        && matches!(
            context.tree.get(argument_id),
            Argument::Named {
                modifiers: None,
                ..
            } | Argument::Labeled {
                modifiers: None,
                ..
            } | Argument::Positional {
                modifiers: None,
                ..
            } | Argument::Spread {
                modifiers: None,
                ..
            }
        );
    context.store_argument_plain_call_argument(argument_id, is_plain);
    is_plain
}

/// Write one call argument that is known to be plain.
pub(crate) fn write_plain_call_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    match f.context().tree.get(argument_id) {
        Argument::Named { name, value, .. } => {
            write!(f, [*name, token(":"), space(), *value])?;
        }
        Argument::Labeled { label, value, .. } => {
            write!(f, [*label, token(":"), space(), *value])?;
        }
        Argument::Positional { value, .. } => {
            write!(f, [*value])?;
        }
        Argument::Spread {
            label: Some(label),
            value,
            ..
        } => {
            write!(f, [token("..."), *label, token(":"), space(), *value])?;
        }
        Argument::Spread {
            label: None, value, ..
        } => {
            write!(f, [token("..."), *value])?;
        }
    }

    Ok(())
}

/// Write one call argument with a plain short-circuit and a safe default branch.
pub(crate) fn write_plain_call_argument_or_node<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    if !argument_is_plain_call_argument(f.context(), argument_id) {
        write!(f, [argument_id])?;
        return Ok(());
    }

    write_plain_call_argument(f, argument_id)
}
