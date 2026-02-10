use super::super::*;
use super::classify::*;
use crate::timing::tags;
use crate::{
    CachedCallArgumentExpansionProfile, CachedCallArgumentExpansionProfiles,
    CachedCallArgumentFacts,
};
use destack_fir::write;

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
) -> CallArgumentFacts {
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
        if !has_line_comment_annotations
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

/// Return call argument facts with per-call caching.
fn resolve_call_argument_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> CallArgumentFacts {
    if let Some(cached) = context.cached_call_argument_facts(call_node_id) {
        context.increment_counter("profile.call_arguments.layout.cache.hits", 1);
        return CallArgumentFacts::from(cached);
    }

    context.increment_counter("profile.call_arguments.layout.cache.misses", 1);
    let facts = collect_call_argument_facts(context, dynamic_arguments);
    context.cache_call_argument_facts(call_node_id, CachedCallArgumentFacts::from(facts));

    facts
}

/// Return single argument expansion facts without scanning all arguments.
fn collect_single_call_argument_facts(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> (bool, bool) {
    let has_line_comment_annotations = context.has_annotation(argument_id)
        && argument_has_line_comment_annotation(context, argument_id);
    let trailing_collection_argument = argument_is_collection_literal(context, argument_id);

    (has_line_comment_annotations, trailing_collection_argument)
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
                && (argument_has_line_comment_annotation(context, argument_id)
                    || argument_has_prefix_line_comment_annotation(context, argument_id)
                    || argument_has_multiline_prefix_annotation(context, argument_id)
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

    // many short unannotated argument lists can skip the full expansion classifier
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
                    },
                )
        });
    if can_use_simple_multi_argument_fast_path {
        context.increment_counter("profile.call_arguments.layout.simple_fast_path", 1);
        return CallArgumentExpansionProfiles {
            regular: CallArgumentExpansionProfile {
                force_expand: false,
                has_call_infix_annotations,
                trailing_collection_argument: false,
            },
            chain_force_expand: false,
        };
    }

    let argument_facts = resolve_call_argument_facts(context, call_node_id, dynamic_arguments);
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
        call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments);
    let should_expand_for_block_callback = has_block_callback_argument
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
    let force_expand_multiline_function_composition = context.node_has_newline(call_node_id)
        && dynamic_arguments.len() >= 3
        && has_any_function_argument
        && !has_spread_argument;
    let force_expand_complex = argument_facts.has_complex_non_callback_argument;

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
            has_call_infix_annotations,
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

/// Write an inline comma-separated call argument list.
fn write_inline_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (index, argument_id) in dynamic_arguments.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [*argument_id])?;
    }
    write!(f, [token(")")])?;

    Ok(())
}

/// Return whether a call can use the no-annotation inline fast path.
fn call_arguments_use_no_annotation_inline_fast_path(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    line_width: usize,
    has_call_infix_annotations: bool,
    arguments_are_multiline_in_source: bool,
) -> bool {
    dynamic_arguments.len() > 1
        && !has_call_infix_annotations
        && !arguments_are_multiline_in_source
        && expression_source_len(context, call_node_id) <= line_width
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
                    },
                )
        })
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
    /// Whether call arguments are multiline in source.
    arguments_are_multiline_in_source: bool,
    /// Whether the single argument should force expansion.
    single_argument_force_expand: bool,
}

/// Return whether a call can use the single-argument fast path before heavy profiling.
fn call_arguments_use_single_simple_argument_fast_path(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    options: SingleSimpleArgumentFastPathOptions,
) -> bool {
    if dynamic_arguments.len() != 1
        || options.call_has_static_arguments
        || options.has_call_infix_annotations
        || options.arguments_are_multiline_in_source
        || options.single_argument_force_expand
    {
        return false;
    }

    if call_inline_len_without_static_arguments(context, call_node_id, dynamic_arguments)
        .is_none_or(|inline_len| inline_len > options.line_width)
    {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    if context.has_annotation(argument_id) || context.node_has_newline(argument_id) {
        return false;
    }

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
    !argument_has_line_comment_annotation(context, argument_id)
        && !argument_has_prefix_line_comment_annotation(context, argument_id)
        && !argument_has_multiline_prefix_annotation(context, argument_id)
        && (argument_is_lambda_expression(context, argument_id)
            || argument_is_function_expression(context, argument_id))
}

/// Return whether a single simple argument can stay inline.
#[derive(Clone, Copy)]
struct SingleSimpleArgumentInlineOptions {
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
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    options: SingleSimpleArgumentInlineOptions,
) -> bool {
    if dynamic_arguments.len() != 1
        || options.force_expand_single_long_with_static_arguments
        || options.force_expand_single_collection_for_type_binary_callee
        || options.single_argument_force_expand
    {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    if argument_has_non_blank_annotation(context, argument_id)
        || context.node_has_newline(argument_id)
    {
        return false;
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);
    let value = context.tree.get(value_id);
    let inline_len = if options.call_has_static_arguments {
        expression_source_len(context, call_node_id)
    } else {
        call_inline_len_without_static_arguments(context, call_node_id, dynamic_arguments)
            .unwrap_or(usize::MAX)
    };

    is_trivial_expression(context.tree, value) && inline_len <= options.line_width
}

/// Format call arguments with list-group awareness.
pub(in super::super) fn format_call_arguments<'ast>(
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
        let single_argument_force_expand =
            single_argument_requires_expanded_list(f.context(), dynamic_arguments);
        let use_no_annotation_inline_fast_path = call_arguments_use_no_annotation_inline_fast_path(
            f.context(),
            call_node_id,
            dynamic_arguments,
            line_width,
            has_call_infix_annotations,
            arguments_are_multiline_in_source,
        );
        if use_no_annotation_inline_fast_path {
            f.context()
                .increment_counter("profile.call.arguments.path.no_annotation_inline_fast", 1);
            write_inline_call_argument_list(f, dynamic_arguments)?;
            return Ok(());
        }

        // very common single argument calls can short-circuit before heavier layout profiling
        let use_single_simple_argument_fast_path =
            call_arguments_use_single_simple_argument_fast_path(
                f.context(),
                call_node_id,
                dynamic_arguments,
                SingleSimpleArgumentFastPathOptions {
                    line_width,
                    call_has_static_arguments,
                    has_call_infix_annotations,
                    arguments_are_multiline_in_source,
                    single_argument_force_expand,
                },
            );
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
            );
        let force_expand_single_collection_for_type_binary_callee =
            call_force_expand_single_collection_for_type_binary_callee(
                f.context(),
                call_node_id,
                dynamic_arguments,
            );

        let force_hugged_expand =
            call_should_force_hugged_expand(force_expand_single_collection_for_type_binary_callee);

        // try hugged format for single object/array arguments
        let can_use_hugged = if dynamic_arguments.len() == 1 {
            !argument_has_multiline_prefix_annotation(f.context(), dynamic_arguments[0])
                && !has_call_infix_annotations
                && !force_expand_single_long_with_static_arguments
                && !argument_is_lambda_expression(f.context(), dynamic_arguments[0])
                && !argument_is_function_expression(f.context(), dynamic_arguments[0])
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

        // keep single callback arguments wrapped directly to avoid list-style trailing commas
        let use_single_callback_argument_inline =
            call_arguments_use_single_callback_argument_inline(
                f.context(),
                dynamic_arguments,
                has_call_infix_annotations,
                force_expand_single_long_with_static_arguments,
                force_expand_single_collection_for_type_binary_callee,
            );
        if use_single_callback_argument_inline {
            f.context()
                .increment_counter("profile.call.arguments.path.single_callback_inline", 1);
            write!(f, [token("("), dynamic_arguments[0], token(")")])?;
            return Ok(());
        }

        // keep short single positional arguments inline
        let use_single_simple_argument = call_arguments_use_single_simple_argument_inline(
            f.context(),
            call_node_id,
            dynamic_arguments,
            SingleSimpleArgumentInlineOptions {
                line_width,
                call_has_static_arguments,
                force_expand_single_long_with_static_arguments,
                force_expand_single_collection_for_type_binary_callee,
                single_argument_force_expand,
            },
        );

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
            && expression_source_len(f.context(), call_node_id) <= line_width;

        if use_leading_block_callback_inline {
            f.context().increment_counter(
                "profile.call.arguments.path.leading_block_callback_inline",
                1,
            );
            write_inline_call_argument_list(f, dynamic_arguments)?;
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
            let hug_last_format =
                format_with(|f| write_inline_call_argument_list(f, dynamic_arguments));

            if force_hug_last_argument {
                f.context()
                    .increment_counter("profile.call.arguments.path.hug_last_forced", 1);
                hug_last_format.format(f)?;
            } else {
                let line_width = usize::from(f.context().options.line_width);
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
                                },
                            )
                    });
                if all_arguments_are_single_line_and_unannotated
                    && inline_call_len.is_some_and(|inline_len| inline_len <= line_width)
                {
                    f.context()
                        .increment_counter("profile.call.arguments.hug_last.fast_path", 1);
                    f.context()
                        .increment_counter("profile.call.arguments.path.hug_last_fast", 1);
                    hug_last_format.format(f)?;
                } else if last_argument_is_collection_literal
                    && leading_arguments_are_compact_simple
                {
                    f.context().increment_counter(
                        "profile.call.arguments.hug_last.skip_probe.collection",
                        1,
                    );
                    list_format.format(f)?;
                } else if inline_call_len.is_some_and(|inline_len| inline_len > line_width) {
                    f.context().increment_counter(
                        "profile.call.arguments.hug_last.skip_probe_overflow",
                        1,
                    );
                    list_format.format(f)?;
                } else if let Some(inline_len) = inline_call_len {
                    if inline_len <= line_width {
                        f.context()
                            .increment_counter("profile.call.arguments.hug_last.deterministic", 1);
                        hug_last_format.format(f)?;
                    } else {
                        f.context().increment_counter(
                            "profile.call.arguments.hug_last.deterministic_list",
                            1,
                        );
                        list_format.format(f)?;
                    }
                } else {
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

/// Return whether a call should expand its argument list when formatted in a chain.
pub(in super::super) fn call_arguments_force_expand_for_chain(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.is_empty() {
        context.increment_counter("profile.call_arguments.chain.fast_false.empty", 1);
        context.increment_counter("profile.call_arguments.chain.force_expand.false", 1);
        return false;
    }

    resolve_chain_call_argument_force_expand(context, call_node_id, dynamic_arguments)
}
