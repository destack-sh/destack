use super::super::*;
use super::classify::*;
use crate::timing::tags;
use crate::{
    CachedCallArgumentExpansionProfile, CachedCallArgumentExpansionProfiles,
    CachedCallArgumentLayoutClass,
};
use destack_fir::write;

// call argument layout thresholds
const NON_LAST_BLOCK_CALLBACK_COUNT_TARGET: usize = 1;
const NON_LAST_BLOCK_CALLBACK_MIN_INDEX: usize = 1;
const FIRST_BLOCK_CALLBACK_COLLECTION_TAIL_ARGUMENT_COUNT: usize = 2;
const MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT: usize = 2;
const MULTILINE_FUNCTION_COMPOSITION_MIN_ARGUMENTS: usize = 3;
const CALL_ARGUMENT_DELIMITER_WIDTH: usize = 2;

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

/// Store regular and chain expansion results for one call expression.
#[derive(Clone, Copy)]
struct CallArgumentExpansionProfiles {
    /// The regular call formatting expansion profile.
    regular: CallArgumentExpansionProfile,
    /// Whether chain formatting should force expansion.
    chain_force_expand: bool,
}

impl From<CachedCallArgumentExpansionProfiles> for CallArgumentExpansionProfiles {
    fn from(cached: CachedCallArgumentExpansionProfiles) -> Self {
        Self {
            regular: CallArgumentExpansionProfile::from(cached.regular),
            chain_force_expand: cached.chain_force_expand,
        }
    }
}

impl From<CallArgumentExpansionProfiles> for CachedCallArgumentExpansionProfiles {
    fn from(profiles: CallArgumentExpansionProfiles) -> Self {
        Self {
            regular: CachedCallArgumentExpansionProfile::from(profiles.regular),
            chain_force_expand: profiles.chain_force_expand,
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
    /// Deferred boundary comments keyed by argument index.
    deferred_boundary_prefix_annotations: Option<Vec<Vec<LocalNodeId<Annotation>>>>,
}

/// Collect one-pass comment data for call arguments.
fn collect_call_argument_comment_profile(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    include_deferred_boundary_comments: bool,
) -> CallArgumentCommentProfile {
    let mut has_line_comment_annotations = false;
    let mut has_prefix_line_comment_annotations = false;
    let mut deferred_boundary_prefix_annotations = None;

    for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
        let argument_has_annotation = context.has_annotation(argument_id);
        if argument_has_annotation {
            let annotation_profile = context.argument_annotation_profile(argument_id);
            if !has_line_comment_annotations && annotation_profile.has_line_comment {
                has_line_comment_annotations = true;
            }
            if !has_prefix_line_comment_annotations && annotation_profile.has_prefix_line_comment {
                has_prefix_line_comment_annotations = true;
            }
        }

        // only non-leading arguments can attach deferred boundary comments
        if include_deferred_boundary_comments && index > 0 && argument_has_annotation {
            let deferred_boundary_comments =
                call_argument_inline_boundary_prefix_annotations(context, argument_id);
            if !deferred_boundary_comments.is_empty() {
                let comments_by_index = deferred_boundary_prefix_annotations
                    .get_or_insert_with(|| vec![Vec::new(); dynamic_arguments.len()]);
                comments_by_index[index] = deferred_boundary_comments;
            }
        }
    }

    CallArgumentCommentProfile {
        has_line_comment_annotations,
        has_prefix_line_comment_annotations,
        deferred_boundary_prefix_annotations,
    }
}

/// Return single argument expansion facts without scanning all arguments.
fn collect_single_call_argument_facts(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> (bool, bool) {
    let has_line_comment_annotations = context.has_annotation(argument_id)
        && context
            .argument_annotation_profile(argument_id)
            .has_line_comment;
    let trailing_collection_argument = argument_is_collection_literal(context, argument_id);

    (has_line_comment_annotations, trailing_collection_argument)
}

/// Return whether one argument has callback-blocking line or multiline prefix annotations.
fn argument_has_callback_blocking_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let profile = context.argument_annotation_profile(argument_id);
    profile.has_line_comment
        || profile.has_prefix_line_comment
        || (context.node_has_newline(argument_id) && profile.has_prefix_annotation)
}

/// Return whether a single static argument call should expand.
fn call_force_expand_single_long_with_static_arguments(
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

    let line_width = usize::from(context.options.line_width);
    let call_len = expression_source_len(context, call_node_id);
    call_len > line_width
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

/// Return whether a single argument call should force expanded list layout.
fn single_argument_requires_expanded_list(
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

    let value_span = context.get_span(value_id);
    let compact_value_len = source_min_inline_char_len(context.get_span_str(value_span));
    let line_width = usize::from(context.options.line_width);

    compact_value_len > line_width
}

/// Build regular and chain call expansion profiles for one call expression.
fn build_call_argument_expansion_profiles(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentExpansionProfiles {
    context.increment_counter("profile.call_arguments.layout.builds", 1);
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
        let (has_line_comment_annotations, trailing_collection_argument) =
            collect_single_call_argument_facts(context, argument_id);
        let force_expand_jsx = has_multiline_jsx_argument(context.tree, dynamic_arguments);
        let force_expand_single_commented_callback =
            argument_is_block_callback(context, argument_id)
                && (argument_has_callback_blocking_comment_annotation(context, argument_id)
                    || has_call_infix_annotations);
        let force_expand_single_multiline_argument = context.node_has_newline(argument_id)
            && !argument_is_collection_literal(context, argument_id)
            && !argument_is_lambda_expression(context, argument_id)
            && !argument_is_function_expression(context, argument_id)
            && !argument_is_tree_expression(context, argument_id)
            && !argument_is_template_literal(context, argument_id);
        let force_expand_single_long_with_static_arguments =
            call_force_expand_single_long_with_static_arguments(
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

        let regular_force_expand = force_expand_jsx
            || has_line_comment_annotations
            || force_expand_single_commented_callback
            || force_expand_single_multiline_argument
            || force_expand_single_long_with_static_arguments
            || force_expand_single_chain_argument
            || force_expand_single_collection_for_type_binary_callee
            || has_call_infix_annotations;
        let chain_force_expand = force_expand_jsx
            || has_line_comment_annotations
            || force_expand_single_commented_callback
            || force_expand_single_chain_argument
            || force_expand_single_long_with_static_arguments;

        return CallArgumentExpansionProfiles {
            regular: CallArgumentExpansionProfile {
                force_expand: regular_force_expand,
                has_call_infix_annotations,
                trailing_collection_argument,
            },
            chain_force_expand,
        };
    }

    let layout_class = resolve_call_argument_layout_class(context, call_node_id, dynamic_arguments);

    // compact unannotated argument lists do not require full expansion profiling
    let can_use_simple_multi_argument_fast_path =
        !layout_class.has_call_infix_annotations && layout_class.all_compact_simple_unannotated;
    if can_use_simple_multi_argument_fast_path {
        context.increment_counter("profile.call_arguments.layout.simple_fast_path", 1);
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
        context.increment_counter(
            "profile.call_arguments.layout.fast_path.multiple_function",
            1,
        );
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
    let force_expand_multiline_function_composition = context.node_has_newline(call_node_id)
        && dynamic_arguments.len() >= MULTILINE_FUNCTION_COMPOSITION_MIN_ARGUMENTS
        && has_any_function_argument
        && !has_spread_argument;
    let has_non_complex_force_expand_signal = force_expand_jsx
        || has_line_comment_annotations
        || should_expand_for_block_callback
        || force_expand_multiline_function_composition
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
        || force_expand_multiline_function_composition
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
fn resolve_regular_call_argument_expansion_profile(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentExpansionProfile {
    if let Some(cached) = context.cached_call_argument_expansion_profiles(call_node_id) {
        context.increment_counter("profile.call_arguments.regular.cache.hits", 1);
        let profile = CallArgumentExpansionProfiles::from(cached).regular;
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
    let profiles = build_call_argument_expansion_profiles(context, call_node_id, dynamic_arguments);
    let profile = profiles.regular;
    context.cache_call_argument_expansion_profiles(
        call_node_id,
        CachedCallArgumentExpansionProfiles::from(profiles),
    );
    context.increment_counter(
        if profile.force_expand {
            "profile.call_arguments.regular.force_expand.true"
        } else {
            "profile.call_arguments.regular.force_expand.false"
        },
        1,
    );

    profile
}

/// Resolve chain call argument force-expand decision with per-call caching.
pub(in super::super) fn resolve_chain_call_argument_force_expand(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(cached) = context.cached_call_argument_expansion_profiles(call_node_id) {
        context.increment_counter("profile.call_arguments.chain.cache.hits", 1);
        let force_expand = CallArgumentExpansionProfiles::from(cached).chain_force_expand;
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
    let profiles = build_call_argument_expansion_profiles(context, call_node_id, dynamic_arguments);
    let force_expand = profiles.chain_force_expand;
    context.cache_call_argument_expansion_profiles(
        call_node_id,
        CachedCallArgumentExpansionProfiles::from(profiles),
    );
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

/// Return whether single argument hugged formatting should force expansion.
fn call_should_force_hugged_expand(
    force_expand_single_collection_for_type_binary_callee: bool,
) -> bool {
    force_expand_single_collection_for_type_binary_callee
}

/// Estimate one-line `callee(arg1, arg2)` length for plain call expressions.
fn call_inline_len_without_static_arguments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    call_has_static_arguments: bool,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> Option<usize> {
    let Expression::Call { left, .. } = context.tree.get(call_node_id) else {
        return None;
    };
    if call_has_static_arguments {
        return None;
    }

    let callee_len = expression_source_len(context, *left);
    let arguments_len = arguments_rendered_len(context, dynamic_arguments);
    Some(
        callee_len
            .saturating_add(arguments_len)
            .saturating_add(CALL_ARGUMENT_DELIMITER_WIDTH),
    )
}

/// Write an inline comma-separated call argument list.
fn write_inline_call_argument_list<'ast>(
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
fn argument_is_plain_call_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if let Some(cached) = context.cached_argument_plain_call_argument(argument_id) {
        context.increment_counter("profile.call.arguments.plain.cache.hits", 1);
        return cached;
    }
    context.increment_counter("profile.call.arguments.plain.cache.misses", 1);

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
    context.cache_argument_plain_call_argument(argument_id, is_plain);
    is_plain
}

/// Write one call argument that is known to be plain.
fn write_plain_call_argument<'ast>(
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

/// Write one call argument with a plain fast path and a safe fallback.
fn write_plain_call_argument_or_node<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    if !argument_is_plain_call_argument(f.context(), argument_id) {
        write!(f, [argument_id])?;
        return Ok(());
    }

    write_plain_call_argument(f, argument_id)
}

/// Return whether one argument is compact, unannotated, and simple.
fn argument_is_compact_simple_unannotated(
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
fn call_has_call_chain_parent(
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
struct SingleSimpleArgumentFastPathOptions {
    /// The line width budget.
    line_width: usize,
    /// Whether the call has static type arguments.
    call_has_static_arguments: bool,
    /// Whether the call node has infix annotations.
    has_call_infix_annotations: bool,
    /// Whether the single argument should force expansion.
    single_argument_force_expand: bool,
}

/// Store one-pass call argument shape data shared by layout decisions.
#[derive(Clone, Copy, Debug, Default)]
struct CallArgumentShape {
    /// Whether any dynamic argument has annotations.
    has_any_argument_annotation: bool,
    /// Whether argument source between first and last spans multiple lines.
    is_multiline_in_source: bool,
    /// Whether all dynamic arguments are single-line and unannotated.
    all_single_line_and_unannotated: bool,
}

/// Build one call argument shape from cached layout-class facts.
fn call_argument_shape_from_layout_class(
    layout_class: CachedCallArgumentLayoutClass,
) -> CallArgumentShape {
    CallArgumentShape {
        has_any_argument_annotation: layout_class.has_any_argument_annotation,
        is_multiline_in_source: layout_class.is_multiline_in_source,
        all_single_line_and_unannotated: layout_class.all_single_line_and_unannotated,
    }
}

/// Resolve one cached call argument layout class.
fn resolve_call_argument_layout_class(
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

    if dynamic_arguments.is_empty() {
        let layout_class = CachedCallArgumentLayoutClass {
            has_call_infix_annotations,
            all_single_line_and_unannotated: true,
            all_compact_simple_unannotated: true,
            all_plain_call_arguments: true,
            has_call_chain_parent,
            ..CachedCallArgumentLayoutClass::default()
        };
        context.cache_call_argument_layout_class(call_node_id, layout_class);
        return layout_class;
    }

    let mut has_any_argument_annotation = false;
    let mut all_single_line_and_unannotated = true;
    let mut all_compact_simple_unannotated = true;
    let mut all_plain_call_arguments = true;
    let mut has_line_comment_annotations = false;
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
        let has_annotation = context.has_annotation(argument_id);
        let has_newline = context.node_has_newline(argument_id);
        if has_annotation {
            has_any_argument_annotation = true;
            if !has_line_comment_annotations
                && context
                    .argument_annotation_profile(argument_id)
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
        if index == 0 {
            first_argument_is_block_callback = is_block_callback;
        }
        if is_last_argument {
            trailing_collection_argument = matches!(
                value,
                Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
            );
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
        && call_arguments_force_hug_last_inline(
            context,
            call_node_id,
            dynamic_arguments,
            trailing_collection_argument,
        );
    let is_multiline_in_source = if all_single_line_and_unannotated {
        false
    } else {
        call_arguments_are_multiline_in_source(context, dynamic_arguments)
    };

    let layout_class = CachedCallArgumentLayoutClass {
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
    };
    context.cache_call_argument_layout_class(call_node_id, layout_class);

    layout_class
}

/// Return whether all leading arguments before the last are compact and simple.
fn leading_arguments_are_compact_simple_unannotated(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() <= 1 {
        return true;
    }

    dynamic_arguments
        .iter()
        .copied()
        .take(dynamic_arguments.len().saturating_sub(1))
        .all(|argument_id| argument_is_compact_simple_unannotated(context, argument_id))
}

/// Return whether all leading arguments before the last are compact callback-tail candidates.
fn leading_arguments_are_compact_callback_tail_candidates(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.len() <= 1 {
        return true;
    }

    dynamic_arguments
        .iter()
        .copied()
        .take(dynamic_arguments.len().saturating_sub(1))
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
fn call_arguments_use_single_simple_argument_fast_path(
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
fn call_arguments_use_single_callback_argument_inline(
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
        && (argument_is_lambda_expression(context, argument_id)
            || argument_is_function_expression(context, argument_id))
}

/// Return whether a single simple argument can stay inline.
#[derive(Clone, Copy)]
struct SingleSimpleArgumentInlineOptions {
    /// The call expression node id.
    call_node_id: LocalNodeId<Expression>,
    /// The line width budget.
    line_width: usize,
    /// Whether the call has static type arguments.
    call_has_static_arguments: bool,
    /// Whether single long static arguments force expansion.
    force_expand_single_long_with_static_arguments: bool,
    /// Whether single collection arguments force expansion for type-binary callees.
    force_expand_single_collection_for_type_binary_callee: bool,
    /// Whether the single argument should force expansion.
    single_argument_force_expand: bool,
}

/// Return whether a single simple argument can stay inline.
fn call_arguments_use_single_simple_argument_inline(
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
struct CallArgumentPlannerBaseState {
    /// The configured line width.
    line_width: usize,
    /// Whether the call has static type arguments.
    call_has_static_arguments: bool,
    /// Whether the call has non-blank infix annotations.
    has_call_infix_annotations: bool,
    /// One-pass argument shape facts.
    argument_shape: CallArgumentShape,
}

/// Store shared call argument planning inputs.
#[derive(Clone, Copy)]
struct CallArgumentPlannerState {
    /// The configured line width.
    line_width: usize,
    /// Whether the call has static type arguments.
    call_has_static_arguments: bool,
    /// Whether the call has non-blank infix annotations.
    has_call_infix_annotations: bool,
    /// Whether the single argument should force expanded list layout.
    single_argument_force_expand: bool,
    /// Whether one long single argument with static arguments should force expansion.
    force_expand_single_long_with_static_arguments: bool,
    /// Whether one single collection argument in type-binary callee should force expansion.
    force_expand_single_collection_for_type_binary_callee: bool,
    /// Estimated one-line call length for plain dynamic calls.
    inline_call_len_without_static_arguments: Option<usize>,
    /// One-pass argument shape facts.
    argument_shape: CallArgumentShape,
}

/// Resolve and cache the inline call length estimate for dynamic-only calls.
fn resolve_inline_call_len_without_static_arguments(
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
enum HugLastCallArgumentLayout {
    /// Keep the argument list inline.
    Inline,
    /// Keep the default list-like layout.
    ListDefault,
}

/// Return whether call arguments can use the hug-last policy.
fn can_consider_hug_last_call_arguments(
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
fn call_arguments_force_hug_last_inline(
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

/// Resolve hug-last call argument layout.
fn resolve_hug_last_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    planner_state: CallArgumentPlannerState,
    force_expand: bool,
    trailing_collection_argument: bool,
) -> Option<HugLastCallArgumentLayout> {
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

    if call_arguments_force_hug_last_inline(
        context,
        call_node_id,
        dynamic_arguments,
        trailing_collection_argument,
    ) {
        context.increment_counter("profile.call.arguments.path.hug_last_forced", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    // only hug when deterministic inline fit says yes
    let can_inline_hug_last = !force_expand
        && planner_state.argument_shape.all_single_line_and_unannotated
        && resolve_inline_call_len()
            .is_some_and(|inline_len| inline_len <= planner_state.line_width);
    if can_inline_hug_last {
        context.increment_counter("profile.call.arguments.hug_last.fast_path", 1);
        context.increment_counter("profile.call.arguments.path.hug_last_fast", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    let last_argument_id = dynamic_arguments.last().copied();
    let last_argument_is_collection_literal = last_argument_id
        .is_some_and(|argument_id| argument_is_collection_literal(context, argument_id));
    let last_argument_is_callback_like = last_argument_id.is_some_and(|argument_id| {
        is_block_lambda_argument(context, argument_id)
            || argument_is_function_expression(context, argument_id)
    });
    let can_inline_callback_tail = !force_expand
        && last_argument_is_callback_like
        && leading_arguments_are_compact_callback_tail_candidates(context, dynamic_arguments);
    if can_inline_callback_tail {
        context.increment_counter("profile.call.arguments.hug_last.fallback_inline", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    let can_inline_overflow_tail = !force_expand
        && resolve_inline_call_len()
            .is_some_and(|inline_len| inline_len > planner_state.line_width)
        && (last_argument_is_collection_literal || last_argument_is_callback_like);
    if can_inline_overflow_tail {
        context.increment_counter("profile.call.arguments.hug_last.overflow_inline", 1);
        return Some(HugLastCallArgumentLayout::Inline);
    }

    if trailing_collection_argument
        && leading_arguments_are_compact_simple_unannotated(context, dynamic_arguments)
    {
        context.increment_counter("profile.call.arguments.hug_last.skip_probe.collection", 1);
        return Some(HugLastCallArgumentLayout::ListDefault);
    }

    None
}

/// Store planned post-hugged call argument layouts.
enum CallArgumentLayoutDecision {
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

/// Build shared call argument planning base inputs.
fn build_call_argument_planner_base_state(
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

/// Build full call argument planner state.
fn build_call_argument_planner_state(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    base_state: CallArgumentPlannerBaseState,
    single_argument_force_expand: bool,
    force_expand_single_long_with_static_arguments: bool,
    inline_call_len_without_static_arguments: Option<usize>,
) -> CallArgumentPlannerState {
    let force_expand_single_collection_for_type_binary_callee = if dynamic_arguments.len() == 1 {
        call_force_expand_single_collection_for_type_binary_callee(
            context,
            call_node_id,
            dynamic_arguments,
        )
    } else {
        false
    };

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

/// Decide post-hugged call argument layout.
fn decide_post_hugged_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    planner_state: CallArgumentPlannerState,
) -> CallArgumentLayoutDecision {
    // keep single callback arguments wrapped directly to avoid list-style trailing commas
    let use_single_callback_argument_inline = call_arguments_use_single_callback_argument_inline(
        context,
        dynamic_arguments,
        planner_state.has_call_infix_annotations,
        planner_state.force_expand_single_long_with_static_arguments,
        planner_state.force_expand_single_collection_for_type_binary_callee,
    );
    if use_single_callback_argument_inline {
        context.increment_counter("profile.call.arguments.path.single_callback_inline", 1);
        return CallArgumentLayoutDecision::InlineSingle;
    }

    // keep short single positional arguments inline
    let use_single_simple_argument = call_arguments_use_single_simple_argument_inline(
        context,
        dynamic_arguments,
        SingleSimpleArgumentInlineOptions {
            call_node_id,
            line_width: planner_state.line_width,
            call_has_static_arguments: planner_state.call_has_static_arguments,
            force_expand_single_long_with_static_arguments: planner_state
                .force_expand_single_long_with_static_arguments,
            force_expand_single_collection_for_type_binary_callee: planner_state
                .force_expand_single_collection_for_type_binary_callee,
            single_argument_force_expand: planner_state.single_argument_force_expand,
        },
        planner_state.argument_shape,
        planner_state.inline_call_len_without_static_arguments,
    );
    if use_single_simple_argument {
        context.increment_counter("profile.call.arguments.path.single_simple", 1);
        return CallArgumentLayoutDecision::InlineSingle;
    }

    let has_leading_block_callback_with_simple_tail =
        call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments);
    let use_leading_block_callback_inline = has_leading_block_callback_with_simple_tail
        && expression_source_len(context, call_node_id) <= planner_state.line_width;
    if use_leading_block_callback_inline {
        context.increment_counter(
            "profile.call.arguments.path.leading_block_callback_inline",
            1,
        );
        return CallArgumentLayoutDecision::InlineAll;
    }

    let has_any_argument_annotation = planner_state.argument_shape.has_any_argument_annotation;
    let include_deferred_boundary_comments = dynamic_arguments.len() > 1;
    let comment_profile = if has_any_argument_annotation || planner_state.has_call_infix_annotations
    {
        let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_PROFILE);
        context.increment_counter("profile.call.arguments.comment_profile.calls", 1);
        collect_call_argument_comment_profile(
            context,
            dynamic_arguments,
            include_deferred_boundary_comments,
        )
    } else {
        context.increment_counter(
            "profile.call.arguments.comment_profile.skip_no_annotation",
            1,
        );
        CallArgumentCommentProfile::default()
    };
    let has_line_comment_annotations = comment_profile.has_line_comment_annotations;
    let has_prefix_line_comment_annotations = comment_profile.has_prefix_line_comment_annotations;
    if dynamic_arguments.len() > 1 {
        let has_deferred_boundary_prefix_annotations = comment_profile
            .deferred_boundary_prefix_annotations
            .as_ref()
            .is_some_and(|comments_by_index| {
                comments_by_index
                    .iter()
                    .any(|comments| !comments.is_empty())
            });
        if has_line_comment_annotations
            || has_prefix_line_comment_annotations
            || has_deferred_boundary_prefix_annotations
        {
            context.increment_counter("profile.call.arguments.path.comment_expanded", 1);
            return CallArgumentLayoutDecision::CommentExpanded(comment_profile);
        }
    }

    let expansion_profile = {
        let _timing =
            context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_PROFILE);
        resolve_regular_call_argument_expansion_profile(context, call_node_id, dynamic_arguments)
    };
    let force_expand = expansion_profile.force_expand;
    let has_call_infix_annotations = expansion_profile.has_call_infix_annotations;
    let trailing_collection_argument = expansion_profile.trailing_collection_argument;

    let can_consider_hug_last_argument = can_consider_hug_last_call_arguments(
        context,
        dynamic_arguments,
        has_line_comment_annotations,
        has_call_infix_annotations,
    );
    if can_consider_hug_last_argument {
        let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_HUG_LAST);
        if let Some(layout) = resolve_hug_last_call_argument_layout(
            context,
            call_node_id,
            dynamic_arguments,
            planner_state,
            force_expand,
            trailing_collection_argument,
        ) {
            return match layout {
                HugLastCallArgumentLayout::Inline => CallArgumentLayoutDecision::InlineAll,
                HugLastCallArgumentLayout::ListDefault => {
                    context.increment_counter("profile.call.arguments.path.list_default", 1);
                    CallArgumentLayoutDecision::ListDefault {
                        force_expand,
                        has_line_comment_annotations,
                    }
                }
            };
        }
    }

    context.increment_counter("profile.call.arguments.path.list_default", 1);
    CallArgumentLayoutDecision::ListDefault {
        force_expand,
        has_line_comment_annotations,
    }
}

/// Format default call arguments via direct argument emission for annotation-free lists.
fn format_fast_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let allow_trailing_comma = f.context().options.trailing_comma == TrailingComma::All;
    let body = format_with(|f| {
        if all_plain_call_arguments {
            for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
                if index > 0 {
                    write!(f, [token(","), soft_line_break_or_space()])?;
                }
                write_plain_call_argument(f, argument_id)?;
            }
        } else {
            for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
                if index > 0 {
                    write!(f, [token(","), soft_line_break_or_space()])?;
                }
                write_plain_call_argument_or_node(f, argument_id)?;
            }
        }

        if allow_trailing_comma {
            write!(f, [if_group_breaks(&token(","))])?;
        }

        Ok(())
    });
    let content = format_with(|f| write!(f, [token("("), soft_block_indent(&body), token(")")]));
    group(&content)
        .with_id(Some(group_id))
        .should_expand(force_expand)
        .format(f)
}

/// Format call arguments with the default list formatter.
fn format_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    has_line_comment_annotations: bool,
    has_any_argument_annotation: bool,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let is_single_argument = dynamic_arguments.len() == 1;
    let single_argument_id = is_single_argument.then_some(dynamic_arguments[0]);
    let has_single_template_literal_argument = if is_single_argument {
        single_argument_id
            .is_some_and(|argument_id| argument_is_template_literal(f.context(), argument_id))
    } else {
        false
    };
    let last_argument_has_line_comment = if has_line_comment_annotations {
        dynamic_arguments.last().is_some_and(|argument_id| {
            argument_has_line_comment_annotation(f.context(), *argument_id)
        })
    } else {
        false
    };
    let single_callback_without_leading_prefix = if is_single_argument {
        single_argument_id.is_some_and(|argument_id| {
            (argument_is_lambda_expression(f.context(), argument_id)
                || argument_is_function_expression(f.context(), argument_id))
                && !argument_has_leading_prefix_annotation_outside_span(f.context(), argument_id)
        })
    } else {
        false
    };
    let can_use_plain_default_fast_path = !f.context().has_ignore_directive_markers()
        && dynamic_arguments.len() > 1
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !is_single_argument;

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_LIST_DEFAULT);
    if can_use_plain_default_fast_path {
        f.context()
            .increment_counter("profile.call.arguments.path.list_default_plain_fast", 1);
        return format_fast_default_call_argument_list(
            f,
            group_id,
            dynamic_arguments,
            force_expand,
            all_plain_call_arguments,
        );
    }

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
}

/// Format call arguments with explicit multiline comment expansion.
fn format_comment_expanded_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    comment_profile: &CallArgumentCommentProfile,
) -> FormatResult<()> {
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
                        let deferred_boundary_comments = deferred_boundary_prefix_annotations
                            .and_then(|comments_by_index| comments_by_index.get(index))
                            .map(|annotations| annotations.as_slice())
                            .unwrap_or(&[]);
                        for annotation_id in deferred_boundary_comments {
                            let content = format_with(|f| {
                                write!(f, [space(), *annotation_id])?;
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

    Ok(())
}

/// Format call arguments with list-group awareness.
pub(in super::super) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS);

    // set up the list group id for conditional formatting
    let group_id = f.group_id("call_args");
    let previous_group_id = f.context().current_argument_group_id;
    f.context_mut().current_argument_group_id = Some(group_id);
    let result = format_call_arguments_with_group(f, call_node_id, dynamic_arguments, group_id);
    f.context_mut().current_argument_group_id = previous_group_id;

    result
}

/// Format call arguments with an active list group id.
fn format_single_call_argument_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
    group_id: GroupId,
) -> FormatResult<()> {
    let single_argument = [argument_id];

    // shared single argument planner base state
    let has_call_infix_annotations = call_has_non_blank_infix_annotation(f.context(), call_node_id);
    let has_any_argument_annotation = f.context().has_annotation(argument_id);
    let is_multiline_in_source = f.context().node_has_newline(argument_id);
    let argument_shape = CallArgumentShape {
        has_any_argument_annotation,
        is_multiline_in_source,
        all_single_line_and_unannotated: !has_any_argument_annotation && !is_multiline_in_source,
    };
    let planner_base_state = CallArgumentPlannerBaseState {
        line_width: usize::from(f.context().options.line_width),
        call_has_static_arguments: call_has_static_arguments(f.context(), call_node_id),
        has_call_infix_annotations,
        argument_shape,
    };
    let single_argument_force_expand =
        single_argument_requires_expanded_list(f.context(), &single_argument);
    let inline_call_len_without_static_arguments = if planner_base_state.call_has_static_arguments {
        None
    } else {
        resolve_inline_call_len_without_static_arguments(
            f.context(),
            call_node_id,
            &single_argument,
            planner_base_state.call_has_static_arguments,
        )
    };

    // keep very common single simple arguments wrapped inline
    let use_single_simple_argument_fast_path = call_arguments_use_single_simple_argument_fast_path(
        f.context(),
        &single_argument,
        SingleSimpleArgumentFastPathOptions {
            line_width: planner_base_state.line_width,
            call_has_static_arguments: planner_base_state.call_has_static_arguments,
            has_call_infix_annotations: planner_base_state.has_call_infix_annotations,
            single_argument_force_expand,
        },
        planner_base_state.argument_shape,
        inline_call_len_without_static_arguments,
    );
    if use_single_simple_argument_fast_path {
        f.context()
            .increment_counter("profile.call.arguments.single_simple.fast_path", 1);
        f.context()
            .increment_counter("profile.call.arguments.path.single_simple_fast_path", 1);
        write!(f, [token("(")])?;
        write_plain_call_argument_or_node(f, argument_id)?;
        write!(f, [token(")")])?;
        return Ok(());
    }

    // preserve inline callback single argument rendering
    let force_expand_single_long_with_static_arguments =
        call_force_expand_single_long_with_static_arguments(
            f.context(),
            call_node_id,
            &single_argument,
        );
    let force_expand_single_collection_for_type_binary_callee =
        call_force_expand_single_collection_for_type_binary_callee(
            f.context(),
            call_node_id,
            &single_argument,
        );
    let use_single_callback_argument_inline = call_arguments_use_single_callback_argument_inline(
        f.context(),
        &single_argument,
        planner_base_state.has_call_infix_annotations,
        force_expand_single_long_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
    );
    if use_single_callback_argument_inline {
        f.context()
            .increment_counter("profile.call.arguments.path.single_callback_inline", 1);
        write!(f, [token("(")])?;
        write_plain_call_argument_or_node(f, argument_id)?;
        write!(f, [token(")")])?;
        return Ok(());
    }

    // keep single collection arguments on hugged path when allowed
    let force_hugged_expand =
        call_should_force_hugged_expand(force_expand_single_collection_for_type_binary_callee);
    let can_use_hugged = !argument_has_multiline_prefix_annotation(f.context(), argument_id)
        && !planner_base_state.has_call_infix_annotations
        && !force_expand_single_long_with_static_arguments
        && !argument_is_lambda_expression(f.context(), argument_id)
        && !argument_is_function_expression(f.context(), argument_id);
    if can_use_hugged {
        let used_hugged = format_hugged(
            f,
            &single_argument,
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

    // run profiled single argument planner decision
    let planner_state = build_call_argument_planner_state(
        f.context(),
        call_node_id,
        &single_argument,
        planner_base_state,
        single_argument_force_expand,
        force_expand_single_long_with_static_arguments,
        inline_call_len_without_static_arguments,
    );
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED);
    let layout_decision = {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_DECIDE);
        decide_post_hugged_call_argument_layout(
            f.context(),
            call_node_id,
            &single_argument,
            planner_state,
        )
    };

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_RENDER);
    let all_plain_call_arguments = argument_is_plain_call_argument(f.context(), argument_id);
    match layout_decision {
        CallArgumentLayoutDecision::InlineAll => {
            write_inline_call_argument_list(f, &single_argument, all_plain_call_arguments)
        }
        CallArgumentLayoutDecision::InlineSingle => {
            write!(f, [token("(")])?;
            write_plain_call_argument_or_node(f, argument_id)?;
            write!(f, [token(")")])
        }
        CallArgumentLayoutDecision::CommentExpanded(comment_profile) => {
            format_comment_expanded_call_argument_list(f, &single_argument, &comment_profile)
        }
        CallArgumentLayoutDecision::ListDefault {
            force_expand,
            has_line_comment_annotations,
        } => format_default_call_argument_list(
            f,
            group_id,
            &single_argument,
            force_expand,
            has_line_comment_annotations,
            planner_base_state
                .argument_shape
                .has_any_argument_annotation,
            all_plain_call_arguments,
        ),
    }
}

/// Format call arguments with an active list group id.
fn format_call_arguments_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
) -> FormatResult<()> {
    if dynamic_arguments.len() == 1 {
        return format_single_call_argument_with_group(
            f,
            call_node_id,
            dynamic_arguments[0],
            group_id,
        );
    }

    let layout_class =
        resolve_call_argument_layout_class(f.context(), call_node_id, dynamic_arguments);
    let planner_base_state =
        build_call_argument_planner_base_state(f.context(), call_node_id, layout_class);
    let all_plain_call_arguments = layout_class.all_plain_call_arguments;

    // keep compact, no-annotation lists on a cheap inline or grouped path
    let no_annotation_multi_argument_candidate = dynamic_arguments.len() > 1
        && !planner_base_state.has_call_infix_annotations
        && planner_base_state
            .argument_shape
            .all_single_line_and_unannotated
        && !planner_base_state.argument_shape.is_multiline_in_source;
    let use_no_annotation_multi_argument_fast_path =
        no_annotation_multi_argument_candidate && layout_class.all_compact_simple_unannotated;
    if use_no_annotation_multi_argument_fast_path {
        if expression_source_len(f.context(), call_node_id) <= planner_base_state.line_width {
            f.context()
                .increment_counter("profile.call.arguments.path.no_annotation_inline_fast", 1);
            write_inline_call_argument_list(f, dynamic_arguments, all_plain_call_arguments)?;
        } else {
            f.context()
                .increment_counter("profile.call.arguments.path.no_annotation_grouped_fast", 1);
            let mut list = list_like("(", ")", ",", dynamic_arguments);
            list.with_group_id(Some(group_id)).should_expand(false);
            write!(f, [list])?;
        }
        return Ok(());
    }

    // frequent hug-last callback and collection tails can skip profiled layout
    let use_forced_hug_last_inline_fast_path =
        dynamic_arguments.len() > 1 && layout_class.force_hug_last_inline;
    if use_forced_hug_last_inline_fast_path {
        f.context()
            .increment_counter("profile.call.arguments.path.hug_last_forced", 1);
        write_inline_call_argument_list(f, dynamic_arguments, all_plain_call_arguments)?;
        return Ok(());
    }

    let inline_call_len_without_static_arguments = None;
    let planner_state = build_call_argument_planner_state(
        f.context(),
        call_node_id,
        dynamic_arguments,
        planner_base_state,
        false,
        false,
        inline_call_len_without_static_arguments,
    );
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED);
    let layout_decision = {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_DECIDE);
        decide_post_hugged_call_argument_layout(
            f.context(),
            call_node_id,
            dynamic_arguments,
            planner_state,
        )
    };

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_RENDER);
    match layout_decision {
        CallArgumentLayoutDecision::InlineAll => {
            write_inline_call_argument_list(f, dynamic_arguments, all_plain_call_arguments)
        }
        CallArgumentLayoutDecision::InlineSingle => {
            debug_assert_eq!(dynamic_arguments.len(), 1);
            write!(f, [token("(")])?;
            write_plain_call_argument_or_node(f, dynamic_arguments[0])?;
            write!(f, [token(")")])
        }
        CallArgumentLayoutDecision::CommentExpanded(comment_profile) => {
            format_comment_expanded_call_argument_list(f, dynamic_arguments, &comment_profile)
        }
        CallArgumentLayoutDecision::ListDefault {
            force_expand,
            has_line_comment_annotations,
        } => format_default_call_argument_list(
            f,
            group_id,
            dynamic_arguments,
            force_expand,
            has_line_comment_annotations,
            layout_class.has_any_argument_annotation,
            all_plain_call_arguments,
        ),
    }
}

/// Return whether a call should expand its argument list when formatted in a chain.
pub(in super::super) fn call_arguments_force_expand_for_chain(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(force_expand) = context.cached_call_argument_chain_force_expand(call_node_id) {
        context.increment_counter("profile.call_arguments.chain.fast_cache.hits", 1);
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
    context.increment_counter("profile.call_arguments.chain.fast_cache.misses", 1);

    if dynamic_arguments.is_empty() {
        context.cache_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("profile.call_arguments.chain.fast_false.empty", 1);
        context.increment_counter("profile.call_arguments.chain.force_expand.false", 1);
        return false;
    }

    let layout_class = resolve_call_argument_layout_class(context, call_node_id, dynamic_arguments);
    let can_use_simple_fast_false =
        !layout_class.has_call_infix_annotations && layout_class.all_compact_simple_unannotated;
    if can_use_simple_fast_false {
        context.cache_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("profile.call_arguments.chain.fast_false.simple", 1);
        context.increment_counter("profile.call_arguments.chain.force_expand.false", 1);
        return false;
    }

    let force_expand =
        resolve_chain_call_argument_force_expand(context, call_node_id, dynamic_arguments);
    context.cache_call_argument_chain_force_expand(call_node_id, force_expand);
    force_expand
}
