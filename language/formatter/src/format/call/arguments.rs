use crate::format::analysis::scan::{
    next_non_whitespace_token_after_annotation, previous_non_whitespace_token_before_span,
};
use crate::format::analysis::timing::tags;
use crate::format::call::facts::{
    argument_has_callback_blocking_comment_annotation, argument_has_multiline_prefix_annotation,
    argument_has_separator_line_comment_annotation, argument_is_inline_closure_cast_object,
    argument_is_interpolated_template_literal, argument_is_plain_call_argument,
    call_arguments_have_boundary_comments, call_arguments_preserve_blank_line_between,
    call_force_expand_single_collection_for_type_binary_callee,
    call_force_expand_single_multiline_with_static_arguments, call_has_non_blank_infix_annotation,
    call_has_static_arguments, call_should_force_hugged_expand,
    resolve_chain_call_argument_force_expand, single_argument_requires_expanded_list,
    write_plain_call_argument, write_plain_call_argument_or_node,
};
use crate::format::call::layout::{
    CallArgumentLayoutBaseState, CallArgumentLayoutRenderState, CallArgumentShape,
    SingleSimpleArgumentShortCircuitOptions, build_call_argument_layout_base_state,
    build_call_argument_layout_state, call_argument_layout_facts_are_simple_multi_unannotated,
    call_arguments_use_single_callback_argument_inline,
    call_arguments_use_single_simple_argument_short_circuit,
    decide_post_hugged_call_argument_layout, resolve_call_argument_layout_facts,
};
use crate::format::call::render::{
    render_call_argument_plan, write_single_call_argument_inline_wrapped,
};
use crate::format::directive::any_ignore_range_for_nodes;
use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, Declaration, DestackFormatContext, DestackFormatter,
    Expression, FormatResult, FunctionKind, GroupId, HugOptions, LocalNodeId, Span, TokenType,
    argument_is_function_expression, argument_is_lambda_expression, argument_value_id,
    block_indent, empty_line, format_hugged, format_with, group, hard_line_break, list_like, space,
    token, transparent_inner_expression,
};
use destack_ast::{Comment, CommentStyle, TokenSpan};
use destack_fir::format::Buffer;
use destack_fir::write;

/// Write one single callback argument with a trailing separator wrap.
pub(crate) fn write_single_callback_argument_wrapped_with_trailing_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    write_plain_call_argument_or_node(f, argument_id)?;
    write!(f, [token(","), hard_line_break(), token(")")])
}

/// Return whether single callback rendering should keep hugging with a trailing separator wrap.
fn callback_argument_has_call_chain_body(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };
    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = context.tree.get(*declaration_id)
    else {
        return false;
    };
    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    let mut expression_id = transparent_inner_expression(context, *body_id);
    let mut saw_call_like = false;
    let mut saw_chain_member = false;
    loop {
        match context.tree.get(expression_id) {
            Expression::Call { left, .. } | Expression::Instantiation { left, .. } => {
                saw_call_like = true;
                expression_id = transparent_inner_expression(context, *left);
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => {
                saw_chain_member = true;
                expression_id = transparent_inner_expression(context, *left);
            }
            _ => break,
        }
    }

    saw_call_like && saw_chain_member
}

/// Return whether single callback rendering should keep hugging with a trailing separator wrap.
pub(crate) fn single_callback_argument_prefers_trailing_separator_wrap(
    context: &DestackFormatContext<'_>,
    _call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    callback_argument_has_call_chain_body(context, argument_id)
}

/// Return whether a call should expand its argument list when formatted in a chain.
pub(crate) fn call_arguments_force_expand_for_chain(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(force_expand) = context.lookup_call_argument_chain_force_expand(call_node_id) {
        context.increment_counter("call.arguments.chain.simple_cache.hits", 1);
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
    context.increment_counter("call.arguments.chain.simple_cache.misses", 1);

    if dynamic_arguments.is_empty() {
        context.store_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("call.arguments.chain.simple_false.empty", 1);
        context.increment_counter("call.arguments.chain.force_expand.false", 1);
        return false;
    }

    let layout_facts = resolve_call_argument_layout_facts(context, call_node_id, dynamic_arguments);
    let should_bypass_simple_false = dynamic_arguments.len() == 1
        && single_argument_requires_expanded_list(context, dynamic_arguments);
    let can_use_simple_false =
        call_argument_layout_facts_are_simple_multi_unannotated(layout_facts)
            && !should_bypass_simple_false;
    if can_use_simple_false {
        context.store_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("call.arguments.chain.simple_false.simple", 1);
        context.increment_counter("call.arguments.chain.force_expand.false", 1);
        return false;
    }

    let force_expand =
        resolve_chain_call_argument_force_expand(context, call_node_id, dynamic_arguments);
    context.store_call_argument_chain_force_expand(call_node_id, force_expand);
    force_expand
}

/// Format a non-empty multi-argument call through profiled layout and render.
pub(crate) fn format_profiled_multi_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
) -> FormatResult<()> {
    // multi argument path: scan facts, decide plan, then render
    let layout_facts =
        resolve_call_argument_layout_facts(f.context(), call_node_id, dynamic_arguments);
    let layout_base_state =
        build_call_argument_layout_base_state(f.context(), call_node_id, layout_facts);
    let all_plain_call_arguments = layout_facts.all_plain_call_arguments;
    let has_boundary_comments =
        call_arguments_have_boundary_comments(f.context(), call_node_id, dynamic_arguments);

    let layout_state = build_call_argument_layout_state(
        f.context(),
        call_node_id,
        dynamic_arguments,
        layout_base_state,
        false,
        false,
    );
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED);
    let plan = {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_DECIDE);
        decide_post_hugged_call_argument_layout(
            f.context(),
            call_node_id,
            dynamic_arguments,
            layout_state,
            has_boundary_comments,
        )
    };

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_RENDER);
    let render_state = CallArgumentLayoutRenderState {
        call_node_id,
        group_id,
        all_plain_call_arguments,
    };
    render_call_argument_plan(f, dynamic_arguments, plan, render_state)
}

/// Format one single call argument with an active list group id.
pub(crate) fn format_single_call_argument_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
    group_id: GroupId,
) -> FormatResult<()> {
    let facts = build_single_argument_facts(f.context(), call_node_id, argument_id);
    let path = resolve_single_argument_format_path(f.context(), call_node_id, argument_id, &facts);

    if let Some(completed) =
        try_format_single_argument_simple_path(f, argument_id, group_id, &facts, path)?
    {
        return Ok(completed);
    }

    format_single_argument_with_profiled_layout(f, call_node_id, argument_id, group_id, &facts)
}

/// Store single-argument facts used by simple-path and layout-policy selection.
pub(crate) struct SingleArgumentFacts {
    /// The single argument as a fixed-size slice payload.
    pub(crate) single_argument: [LocalNodeId<Argument>; 1],
    /// Whether boundary comments exist between call tokens and argument tokens.
    pub(crate) has_boundary_comments: bool,
    /// One separator line comment source for a plain argument when present.
    pub(crate) separator_line_comment_source: Option<SeparatorLineCommentSource>,
    /// Shared layout base state for the profiled fallback.
    pub(crate) layout_base_state: CallArgumentLayoutBaseState,
    /// Whether the argument has any attached annotation signal.
    pub(crate) has_any_argument_annotation: bool,
    /// Whether single-argument expanded layout is forced.
    pub(crate) single_argument_force_expand: bool,
    /// Whether static arguments force multiline for this single argument.
    pub(crate) force_expand_single_multiline_with_static_arguments: bool,
    /// Whether type-binary callee shape forces collection expansion.
    pub(crate) force_expand_single_collection_for_type_binary_callee: bool,
    /// Whether callback hugging is blocked by annotation comments.
    pub(crate) has_hug_blocking_comment_annotation: bool,
}

impl SingleArgumentFacts {
    /// Return short-circuit options for the simple single-argument branch.
    fn short_circuit_options(&self) -> SingleSimpleArgumentShortCircuitOptions {
        SingleSimpleArgumentShortCircuitOptions {
            call_has_static_arguments: self.layout_base_state.call_has_static_arguments,
            has_call_infix_annotations: self.layout_base_state.has_call_infix_annotations,
            single_argument_force_expand: self.single_argument_force_expand,
        }
    }

    /// Return whether the argument can use the simple short-circuit branch.
    fn can_use_simple_short_circuit(&self, context: &DestackFormatContext<'_>) -> bool {
        call_arguments_use_single_simple_argument_short_circuit(
            context,
            &self.single_argument,
            self.short_circuit_options(),
            self.layout_base_state.argument_shape,
        ) && !self.has_boundary_comments
    }

    /// Return whether the argument can use the hugged branch.
    fn can_use_hugged(
        &self,
        context: &DestackFormatContext<'_>,
        argument_id: LocalNodeId<Argument>,
    ) -> bool {
        !self.has_boundary_comments
            && !self.has_any_argument_annotation
            && !argument_has_multiline_prefix_annotation(context, argument_id)
            && !self.has_hug_blocking_comment_annotation
            && !self.layout_base_state.has_call_infix_annotations
            && !self.force_expand_single_multiline_with_static_arguments
            && !argument_is_lambda_expression(context, argument_id)
            && !argument_is_function_expression(context, argument_id)
            && !argument_is_interpolated_template_literal(context, argument_id)
    }
}

/// Store the selected single-argument formatting branch.
enum SingleArgumentFormatPath {
    /// Render one plain argument with separator line comments owned by the list seam.
    SeparatorComment,
    /// Keep one simple argument inline with wrapped parens.
    SimpleShortCircuit,
    /// Keep inline closure-cast object arguments inline for call-then-chain seams.
    InlineClosureCastObject,
    /// Keep callback argument inline and optionally use trailing separator wrapping.
    CallbackInline {
        /// Whether callback inline path should emit the trailing separator wrapper.
        use_trailing_separator_wrap: bool,
    },
    /// Attempt hugging for one eligible single argument.
    Hugged {
        /// Whether hugging should force expand mode.
        force_hugged_expand: bool,
    },
    /// Fall back to profiled layout and render.
    Profiled,
}

/// Resolve one single-argument formatting path from precomputed facts.
fn resolve_single_argument_format_path(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
    facts: &SingleArgumentFacts,
) -> SingleArgumentFormatPath {
    if facts.separator_line_comment_source.is_some() {
        return SingleArgumentFormatPath::SeparatorComment;
    }

    if facts.can_use_simple_short_circuit(context) {
        return SingleArgumentFormatPath::SimpleShortCircuit;
    }

    if !facts.has_boundary_comments && argument_is_inline_closure_cast_object(context, argument_id)
    {
        return SingleArgumentFormatPath::InlineClosureCastObject;
    }

    let use_single_callback_argument_inline = call_arguments_use_single_callback_argument_inline(
        context,
        &facts.single_argument,
        facts.layout_base_state.has_call_infix_annotations,
        facts.force_expand_single_multiline_with_static_arguments,
        facts.force_expand_single_collection_for_type_binary_callee,
    ) && !facts.has_boundary_comments;
    if use_single_callback_argument_inline {
        let use_trailing_separator_wrap = single_callback_argument_prefers_trailing_separator_wrap(
            context,
            call_node_id,
            argument_id,
        );
        return SingleArgumentFormatPath::CallbackInline {
            use_trailing_separator_wrap,
        };
    }

    if facts.can_use_hugged(context, argument_id) {
        return SingleArgumentFormatPath::Hugged {
            force_hugged_expand: call_should_force_hugged_expand(
                facts.force_expand_single_collection_for_type_binary_callee,
            ),
        };
    }

    SingleArgumentFormatPath::Profiled
}

/// Build single-argument facts once for simple-path and profiled branches.
pub(crate) fn build_single_argument_facts(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> SingleArgumentFacts {
    let single_argument = [argument_id];
    let has_boundary_comments =
        call_arguments_have_boundary_comments(context, call_node_id, &single_argument);
    let separator_line_comment_source = if argument_is_plain_call_argument(context, argument_id) {
        single_argument_separator_line_comment_fact(context, call_node_id, argument_id)
    } else {
        None
    };

    let has_call_infix_annotations = call_has_non_blank_infix_annotation(context, call_node_id);
    let argument_annotation_facts = context.ensure_argument_annotation_facts(argument_id);
    let has_any_argument_annotation = argument_annotation_facts.has_prefix_annotation
        || argument_annotation_facts.has_comment
        || argument_annotation_facts.has_line_comment
        || argument_annotation_facts.has_prefix_line_comment;
    let is_multiline_in_source = context.node_has_newline(argument_id);
    let argument_shape = CallArgumentShape {
        has_any_argument_annotation,
        is_multiline_in_source,
        all_single_line_and_unannotated: !has_any_argument_annotation && !is_multiline_in_source,
    };

    let layout_base_state = CallArgumentLayoutBaseState {
        call_has_static_arguments: call_has_static_arguments(context, call_node_id),
        has_call_infix_annotations,
        argument_shape,
    };

    let single_argument_force_expand =
        single_argument_requires_expanded_list(context, &single_argument);
    let force_expand_single_multiline_with_static_arguments =
        call_force_expand_single_multiline_with_static_arguments(
            context,
            call_node_id,
            &single_argument,
        );
    let force_expand_single_collection_for_type_binary_callee =
        call_force_expand_single_collection_for_type_binary_callee(
            context,
            call_node_id,
            &single_argument,
        );

    let has_hug_blocking_comment_annotation =
        argument_has_callback_blocking_comment_annotation(context, argument_id);

    SingleArgumentFacts {
        single_argument,
        has_boundary_comments,
        separator_line_comment_source,
        layout_base_state,
        has_any_argument_annotation,
        single_argument_force_expand,
        force_expand_single_multiline_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
        has_hug_blocking_comment_annotation,
    }
}

/// Try single-argument simple paths and return `Some(())` when one branch completes output.
fn try_format_single_argument_simple_path<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    group_id: GroupId,
    facts: &SingleArgumentFacts,
    path: SingleArgumentFormatPath,
) -> FormatResult<Option<()>> {
    match path {
        SingleArgumentFormatPath::SeparatorComment => {
            let comment_source = facts
                .separator_line_comment_source
                .as_ref()
                .expect("separator comment source should exist for separator path");
            format_single_plain_argument_with_separator_line_comment(
                f,
                argument_id,
                comment_source,
            )?;
            Ok(Some(()))
        }
        SingleArgumentFormatPath::SimpleShortCircuit => {
            f.context()
                .increment_counter("call.arguments.single_simple.short_circuit", 1);
            f.context()
                .increment_counter("call.arguments.path.single_simple.short_circuit", 1);
            write_single_call_argument_inline_wrapped(f, argument_id)?;
            Ok(Some(()))
        }
        SingleArgumentFormatPath::InlineClosureCastObject => {
            f.context()
                .increment_counter("call.arguments.path.inline_closure_cast_object", 1);
            write_single_call_argument_inline_wrapped(f, argument_id)?;
            Ok(Some(()))
        }
        SingleArgumentFormatPath::CallbackInline {
            use_trailing_separator_wrap,
        } => {
            f.context()
                .increment_counter("call.arguments.path.single_callback_inline", 1);
            if use_trailing_separator_wrap {
                f.context().increment_counter(
                    "call.arguments.path.single_callback_inline.trailing_wrap",
                    1,
                );
                write_single_callback_argument_wrapped_with_trailing_separator(f, argument_id)?;
            } else {
                write_single_call_argument_inline_wrapped(f, argument_id)?;
            }
            Ok(Some(()))
        }
        SingleArgumentFormatPath::Hugged {
            force_hugged_expand,
        } => {
            let used_hugged = format_hugged(
                f,
                &facts.single_argument,
                HugOptions::CALL,
                Some(group_id),
                force_hugged_expand,
            )?;
            if used_hugged {
                f.context()
                    .increment_counter("call.arguments.path.hugged", 1);
                return Ok(Some(()));
            }

            Ok(None)
        }
        SingleArgumentFormatPath::Profiled => Ok(None),
    }
}

/// Format one single argument through profiled layout and render fallback.
fn format_single_argument_with_profiled_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
    group_id: GroupId,
    facts: &SingleArgumentFacts,
) -> FormatResult<()> {
    let layout_state = build_call_argument_layout_state(
        f.context(),
        call_node_id,
        &facts.single_argument,
        facts.layout_base_state,
        facts.single_argument_force_expand,
        facts.force_expand_single_multiline_with_static_arguments,
    );

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED);
    let plan = {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_DECIDE);
        decide_post_hugged_call_argument_layout(
            f.context(),
            call_node_id,
            &facts.single_argument,
            layout_state,
            facts.has_boundary_comments,
        )
    };

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_RENDER);
    let render_state = CallArgumentLayoutRenderState {
        call_node_id,
        group_id,
        all_plain_call_arguments: argument_is_plain_call_argument(f.context(), argument_id),
    };

    render_call_argument_plan(f, &facts.single_argument, plan, render_state)
}

/// Format call arguments with an active list group id.
pub(crate) fn format_call_arguments_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
) -> FormatResult<()> {
    // empty argument lists can still carry boundary infix annotations
    if dynamic_arguments.is_empty() {
        return format_empty_call_arguments(f, call_node_id);
    }

    // ignore ranges: route through list_like so raw span preservation stays consistent
    if try_format_ignore_range_call_arguments(f, dynamic_arguments, group_id)? {
        return Ok(());
    }

    // single argument path has dedicated short-circuit and hugging logic
    if dynamic_arguments.len() == 1 {
        return format_single_call_argument_with_group(
            f,
            call_node_id,
            dynamic_arguments[0],
            group_id,
        );
    }

    format_profiled_multi_call_arguments(f, call_node_id, dynamic_arguments, group_id)
}

/// Format an empty call argument list, preserving infix annotations when present.
fn format_empty_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if f.context().has_infix_annotation(call_node_id) {
        let should_expand_multiline =
            empty_call_infix_requires_multiline(f.context(), call_node_id);
        if should_expand_multiline {
            write!(
                f,
                [
                    token("("),
                    block_indent(&f.context().block_infix_annotations(call_node_id)),
                    token(")")
                ]
            )?;
        } else {
            write!(
                f,
                [
                    token("("),
                    f.context().block_infix_annotations(call_node_id),
                    token(")")
                ]
            )?;
        }
    } else {
        write!(f, [token("("), token(")")])?;
    }

    Ok(())
}

/// Try formatting call arguments through the ignore-range list writer.
fn try_format_ignore_range_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
) -> FormatResult<bool> {
    if !f.context().has_ignore_directive_markers() {
        return Ok(false);
    }

    let comment_tokens = f.context().comment_tokens();
    let has_ignore_ranges =
        any_ignore_range_for_nodes(f.context(), dynamic_arguments, comment_tokens);
    if !has_ignore_ranges {
        return Ok(false);
    }

    f.context()
        .increment_counter("call.arguments.path.ignore_ranges_list_like", 1);
    let mut list = list_like("(", ")", ",", dynamic_arguments);
    list.with_group_id(Some(group_id)).force_expand();
    write!(f, [list])?;

    Ok(true)
}

/// Return whether empty call infix annotations should expand across lines.
fn empty_call_infix_requires_multiline(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(call_node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                if annotation.position() != AnnotationPosition::BlockInfix {
                    return false;
                }

                let annotation_span = context.annotation_span(*annotation_id);
                if context.has_newline(annotation_span) {
                    return true;
                }

                match annotation {
                    Annotation::Comment { node, .. } => {
                        let comment = context.tree.get::<Comment>(node);
                        comment.style == CommentStyle::Slash
                    }
                    Annotation::Blank { .. } | Annotation::Doc { .. } => true,
                    Annotation::Decorator { .. } => false,
                }
            })
        })
        .unwrap_or(false)
}

/// Format call arguments with list-group awareness.
pub(crate) fn format_call_arguments<'ast>(
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

/// Store one separator line comment and whether it started on its own source line.
pub(crate) struct SeparatorLineCommentSource {
    /// The separator comment node ids in source order.
    pub(crate) comment_ids: Vec<LocalNodeId<Comment>>,
    /// Whether the comment starts on its own line after the separator comma.
    pub(crate) is_own_line: bool,
}

/// Return one source comma token that owns one separator slash comment seam.
fn separator_line_comment_preceding_comma(
    context: &DestackFormatContext<'_>,
    annotation_span: Span,
) -> Option<TokenSpan> {
    let mut previous_token = previous_non_whitespace_token_before_span(context, annotation_span);

    while let Some(token) = previous_token {
        if token.token.ty == TokenType::Comma {
            return Some(token);
        }

        if matches!(
            token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            previous_token = previous_non_whitespace_token_before_span(context, token.span);
            continue;
        }

        return None;
    }

    None
}

/// Return one separator slash comment annotation source payload.
fn separator_line_comment_annotation_fact(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<(LocalNodeId<Comment>, bool)> {
    let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
        return None;
    };

    if !matches!(
        position,
        AnnotationPosition::LinePrefix
            | AnnotationPosition::LinePostfix
            | AnnotationPosition::LinePostfixBoundary
            | AnnotationPosition::BlockPostfix
    ) {
        return None;
    }

    let comment = context.tree.get::<Comment>(node);
    if comment.style != CommentStyle::Slash {
        return None;
    }

    let annotation_span = context.annotation_span(annotation_id);
    let preceding_comma = separator_line_comment_preceding_comma(context, annotation_span);
    let following_token = next_non_whitespace_token_after_annotation(context, annotation_id);
    let following_separator =
        following_token.is_some_and(|token| token.token.ty == TokenType::Comma);
    let following_close_brace =
        following_token.is_some_and(|token| token.token.ty == TokenType::CloseBrace);

    if preceding_comma.is_none() && !following_separator {
        return None;
    }

    if preceding_comma.is_some() && following_close_brace && !following_separator {
        return None;
    }

    let is_own_line = separator_line_comment_is_own_line(
        context,
        annotation_id,
        annotation_span,
        preceding_comma,
        following_separator,
    );

    Some((node, is_own_line))
}

/// Return whether one separator comment starts on its own source line.
fn separator_line_comment_is_own_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    annotation_span: Span,
    preceding_comma: Option<TokenSpan>,
    following_separator: bool,
) -> bool {
    if let Some(separator_token) = preceding_comma {
        let before_comment_span = Span::new(
            annotation_span.file,
            separator_token.span.end,
            annotation_span.start,
        );

        return context.has_newline(before_comment_span);
    }

    if following_separator {
        let Some(separator_token) =
            next_non_whitespace_token_after_annotation(context, annotation_id)
        else {
            return false;
        };

        let after_comment_span = Span::new(
            annotation_span.file,
            annotation_span.end,
            separator_token.span.start,
        );
        return context.has_newline(after_comment_span);
    }

    false
}

/// Return whether one annotation is a separator line comment.
fn annotation_is_separator_line_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    separator_line_comment_annotation_fact(context, annotation_id).is_some()
}

/// Return one separator comment source from one annotation list and filter.
fn separator_line_comment_fact_from_annotations<F>(
    context: &DestackFormatContext<'_>,
    annotations: &[LocalNodeId<Annotation>],
    mut annotation_allowed: F,
) -> Option<SeparatorLineCommentSource>
where
    F: FnMut(LocalNodeId<Annotation>) -> bool,
{
    for (index, annotation_id) in annotations.iter().copied().enumerate() {
        if !annotation_allowed(annotation_id) {
            continue;
        }

        let Some((comment_id, is_own_line)) =
            separator_line_comment_annotation_fact(context, annotation_id)
        else {
            continue;
        };

        let mut comment_ids = vec![comment_id];
        for next_annotation_id in annotations.iter().skip(index + 1).copied() {
            if !annotation_allowed(next_annotation_id) {
                break;
            }

            if matches!(
                context.annotation(next_annotation_id),
                Annotation::Blank { .. }
            ) {
                continue;
            }

            let Some((next_comment_id, _)) =
                separator_line_comment_annotation_fact(context, next_annotation_id)
            else {
                break;
            };

            comment_ids.push(next_comment_id);
        }

        return Some(SeparatorLineCommentSource {
            comment_ids,
            is_own_line,
        });
    }

    None
}

/// Return one separator line comment source attached to one argument.
pub(crate) fn single_argument_separator_line_comment_fact(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> Option<SeparatorLineCommentSource> {
    let resolve_from_annotations = |annotations: &[LocalNodeId<Annotation>]| {
        separator_line_comment_fact_from_annotations(context, annotations, |_| true)
    };

    if let Some(annotations) = context.annotations(argument_id)
        && let Some(comment_source) = resolve_from_annotations(&annotations)
    {
        return Some(comment_source);
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_annotation_source = context
        .annotations(value_id)
        .and_then(|annotations| resolve_from_annotations(&annotations));
    if value_annotation_source.is_some() {
        return value_annotation_source;
    }

    let argument_span = context.span(argument_id);
    let value_span = context.span(value_id);
    let seam_start = value_span.end;
    let call_span = context.span(call_node_id);
    if value_span.file != call_span.file || seam_start >= call_span.end {
        return None;
    }

    let call_annotation_source = context.annotations(call_node_id).and_then(|annotations| {
        separator_line_comment_fact_from_annotations(context, &annotations, |annotation_id| {
            let annotation_span = context.annotation_span(annotation_id);
            annotation_span.file == argument_span.file
                && annotation_span.start >= seam_start
                && annotation_span.end <= call_span.end
        })
    });
    if call_annotation_source.is_some() {
        return call_annotation_source;
    }

    None
}

/// Return whether an argument has non-separator postfix or infix annotations.
fn argument_has_non_separator_postfix_or_infix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .visit_annotations(argument_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let position = context.annotation(*annotation_id).position();
                let is_postfix_or_infix = matches!(
                    position,
                    AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                );
                is_postfix_or_infix
                    && !matches!(context.annotation(*annotation_id), Annotation::Blank { .. })
                    && !annotation_is_separator_line_comment(context, *annotation_id)
            })
        })
        .unwrap_or(false)
}

/// Return whether one argument can render without separator line comment annotations.
pub(crate) fn argument_can_render_without_separator_line_comment(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_has_separator_line_comment_annotation(context, argument_id) {
        return false;
    }

    if argument_has_non_separator_postfix_or_infix_annotation(context, argument_id) {
        return false;
    }

    matches!(
        context.tree.get(argument_id),
        Argument::Positional {
            modifiers: None,
            ..
        }
    )
}

/// Return whether the list can use multiline trailing separator comment rendering.
pub(crate) fn can_format_multiline_call_argument_list_with_last_separator_line_comment(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };
    let Some(_comment_source) =
        single_argument_separator_line_comment_fact(context, call_node_id, last_argument_id)
    else {
        return false;
    };

    argument_can_render_without_separator_line_comment(context, last_argument_id)
}

/// Format a multiline call list with a trailing separator line comment after the last argument.
pub(crate) fn format_multiline_call_argument_list_with_last_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<bool> {
    if !can_format_multiline_call_argument_list_with_last_separator_line_comment(
        f.context(),
        call_node_id,
        dynamic_arguments,
    ) {
        return Ok(false);
    }

    let last_argument_id = dynamic_arguments
        .last()
        .copied()
        .expect("last argument should exist when multiline separator layout is eligible");
    let comment_source =
        single_argument_separator_line_comment_fact(f.context(), call_node_id, last_argument_id)
            .expect(
                "separator comment source should exist when multiline separator layout is eligible",
            );
    if !argument_can_render_without_separator_line_comment(f.context(), last_argument_id) {
        return Ok(false);
    }

    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                    if index > 0 {
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

                    let is_last_argument = index + 1 == dynamic_arguments.len();
                    if is_last_argument {
                        write_argument_without_separator_line_comment(f, *argument_id)?;
                        write_separator_line_comment_after_comma(f, &comment_source)?;
                    } else {
                        write!(f, [group(argument_id), token(",")])?;
                    }
                }
                Ok(())
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token(")")])?;

    Ok(true)
}

/// Write one separator line comment after one argument comma.
pub(crate) fn write_separator_line_comment_after_comma<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    separator_comment_source: &SeparatorLineCommentSource,
) -> FormatResult<()> {
    let Some(first_comment_id) = separator_comment_source.comment_ids.first().copied() else {
        return Ok(());
    };

    if separator_comment_source.is_own_line {
        write!(f, [token(","), hard_line_break(), first_comment_id])?;
        for comment_id in separator_comment_source.comment_ids.iter().skip(1) {
            write!(f, [hard_line_break(), comment_id])?;
        }
    } else {
        write!(f, [token(","), space(), first_comment_id])?;
        for comment_id in separator_comment_source.comment_ids.iter().skip(1) {
            write!(f, [hard_line_break(), comment_id])?;
        }
    }

    Ok(())
}

/// Write one argument without separator line comments that are emitted at list level.
pub(crate) fn write_argument_without_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<bool> {
    if !argument_can_render_without_separator_line_comment(f.context(), argument_id) {
        return Ok(false);
    }

    let Argument::Positional {
        modifiers: None,
        value,
    } = f.context().tree.get(argument_id)
    else {
        return Ok(false);
    };

    write!(
        f,
        [
            f.context().any_prefix_annotations(argument_id),
            group(value)
        ]
    )?;
    Ok(true)
}

/// Format one single plain argument with a separator line comment seam.
pub(crate) fn format_single_plain_argument_with_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    separator_comment_source: &SeparatorLineCommentSource,
) -> FormatResult<()> {
    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                write_plain_call_argument(f, argument_id)?;
                write_separator_line_comment_after_comma(f, separator_comment_source)?;
                Ok(())
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token(")")])?;

    Ok(())
}
