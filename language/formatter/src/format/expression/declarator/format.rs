use super::layout::{DeclaratorLayout, DeclaratorLayoutInputs, choose_declarator_layout};
use super::pattern::pattern_has_default_assignment;
use super::value::{
    declarator_value_has_inline_assignment_seam_prefix_comment, value_has_generic_class_heritage,
    value_is_inline_closure_cast_type_binary,
};
use crate::FormatNode;
use crate::expression::{
    Declarator, DestackFormatContext, DestackFormatter, Expression, FormatResult, LocalNodeId,
    NodeTree, Pattern, ScalarLiteral, Span, block_indent, dedent,
    expression_has_multiline_static_type_argument, expression_has_static_type_arguments,
    fits_expanded, flattened_binary_operand_count, format_call_expression,
    format_instantiation_expression, format_with, group, hard_line_break,
    has_line_comment_between_expressions, indent, is_chain_root, is_expression_breakable,
    is_expression_chain, is_lambda_expression, is_pattern_breakable, is_poorly_breakable_chain,
    soft_line_break_or_space, space, span_has_comment, summarize_chain_calls, token,
    transparent_inner_expression,
};
use crate::operator::{is_type_context, union_source_has_leading_pipe};
use destack_fir::format::{Buffer, Format};
use destack_fir::{format_args, write};

// declarator chain and binary thresholds
const COMPLEX_CHAIN_CALL_COUNT_THRESHOLD: usize = 1;
const LONG_BINARY_OPERAND_COUNT_THRESHOLD: usize = 2;

/// Store base expression-shape facts for one declarator value.
#[derive(Clone, Copy)]
struct DeclaratorShapeFacts {
    value_inner_id: LocalNodeId<Expression>,
    pattern_breakable: bool,
    value_breakable: bool,
    value_is_binary: bool,
    value_is_sequence: bool,
    value_is_chain: bool,
    value_is_call_like: bool,
    value_is_lambda: bool,
    value_is_declaration: bool,
    value_handles_its_own_breaking: bool,
    should_force_expand_value: bool,
}

/// Store chain-specific profile facts for one declarator value.
#[derive(Clone, Copy, Default)]
struct DeclaratorChainProfile {
    value_is_complex_chain: bool,
    value_chain_call_count: usize,
    value_chain_has_member_access: bool,
}

/// Store source and inline-layout facts for one declarator.
#[derive(Clone, Copy)]
struct DeclaratorSourceProfile {
    value_has_newline: bool,
    pattern_has_newline: bool,
    pattern_has_default_assignment: bool,
    pattern_has_comments_or_annotations: bool,
    value_is_parenthesized: bool,
    value_has_prefix_annotation_that_forces_break: bool,
    value_has_existing_operator_break: bool,
    value_has_between_comment: bool,
    value_has_internal_comment: bool,
    value_has_line_comment_between_operands: bool,
    value_is_long_binary: bool,
    has_single_chain_call: bool,
    is_string_literal: bool,
    is_template_expression: bool,
    value_is_leading_pipe_type_union: bool,
    value_has_static_type_arguments: bool,
    value_has_multiline_static_type_argument: bool,
    value_has_instantiation_prefix: bool,
    has_significant_between_comment: bool,
    value_is_await_expression: bool,
    value_is_comptime_expression: bool,
    value_has_multiline_chain_body: bool,
}

/// Return whether one chain value has an instantiation in its left prefix.
fn value_chain_has_instantiation_prefix(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = transparent_inner_expression(context, expression_id);

    loop {
        current_id = transparent_inner_expression(context, current_id);

        match context.tree.get(current_id) {
            Expression::Instantiation { .. } => return true,
            Expression::Call { left, .. }
            | Expression::New { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => {
                current_id = *left;
            }
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                current_id = *expression;
            }
            _ => return false,
        }
    }
}

/// Collect base shape facts for one declarator value.
fn collect_declarator_shape_facts(
    context: &DestackFormatContext<'_>,
    tree: &NodeTree,
    pattern_id: LocalNodeId<Pattern>,
    value_id: LocalNodeId<Expression>,
) -> DeclaratorShapeFacts {
    let value_expr = tree.get(value_id);
    let value_inner_id = transparent_inner_expression(context, value_id);
    let value_inner_expr = tree.get(value_inner_id);
    let value_is_binary = matches!(value_inner_expr, Expression::Binary { .. });
    let value_is_sequence = matches!(value_inner_expr, Expression::SequenceExpression { .. });
    let value_is_chain_root = is_chain_root(tree, value_inner_id);
    let value_is_chain = is_expression_chain(tree, value_inner_id) || value_is_chain_root;
    let value_is_poor_chain = value_is_chain && is_poorly_breakable_chain(context, value_inner_id);
    let value_is_call_like = matches!(
        value_inner_expr,
        Expression::Call { .. } | Expression::New { .. } | Expression::Instantiation { .. }
    );
    let value_is_lambda = is_lambda_expression(context, value_inner_id);
    let value_is_declaration = matches!(value_inner_expr, Expression::Declaration(_));
    let value_handles_its_own_breaking = value_is_binary
        || value_is_sequence
        || value_is_chain
        || value_is_call_like
        || value_is_lambda
        || value_is_declaration;

    DeclaratorShapeFacts {
        value_inner_id,
        pattern_breakable: is_pattern_breakable(tree, pattern_id),
        value_breakable: is_expression_breakable(tree, value_expr),
        value_is_binary,
        value_is_sequence,
        value_is_chain,
        value_is_call_like,
        value_is_lambda,
        value_is_declaration,
        value_handles_its_own_breaking,
        should_force_expand_value: is_expression_breakable(tree, value_expr)
            && !value_is_poor_chain,
    }
}

/// Collect chain profile facts for one declarator value.
fn collect_declarator_chain_profile(
    context: &DestackFormatContext<'_>,
    tree: &NodeTree,
    value_inner_id: LocalNodeId<Expression>,
    value_is_chain: bool,
) -> DeclaratorChainProfile {
    if !value_is_chain {
        return DeclaratorChainProfile::default();
    }

    let mut chain = Vec::new();
    let mut current = value_inner_id;
    loop {
        chain.push(current);
        let next = match tree.get(current) {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => Some(*left),
            _ => None,
        };

        if let Some(next_id) = next {
            current = next_id;
        } else {
            break;
        }
    }
    chain.reverse();

    let value_chain_has_member_access = chain.iter().copied().any(|expression_id| {
        matches!(
            tree.get(expression_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
    });
    let call_summaries = summarize_chain_calls(context, &chain);
    let value_chain_call_count = call_summaries.len();
    let has_multiline_call = call_summaries
        .iter()
        .any(|summary| summary.has_multiline_argument);
    let value_is_complex_chain =
        value_chain_call_count > COMPLEX_CHAIN_CALL_COUNT_THRESHOLD && has_multiline_call;

    DeclaratorChainProfile {
        value_is_complex_chain,
        value_chain_call_count,
        value_chain_has_member_access,
    }
}

/// Collect source and inline-layout profile facts for one declarator.
#[allow(clippy::too_many_arguments)]
fn collect_declarator_source_profile(
    context: &DestackFormatContext<'_>,
    tree: &NodeTree,
    pattern_id: LocalNodeId<Pattern>,
    ty: Option<LocalNodeId<Expression>>,
    value_id: LocalNodeId<Expression>,
    value_inner_id: LocalNodeId<Expression>,
    value_expr: &Expression,
    value_inner_expr: &Expression,
    shape: DeclaratorShapeFacts,
    chain: DeclaratorChainProfile,
    value_is_inline_closure_cast_type_binary: bool,
) -> DeclaratorSourceProfile {
    let pattern_span = context.span(pattern_id);

    let value_has_prefix_annotation = context.has_prefix_annotation(value_id);
    let value_has_assignment_seam_inline_prefix_comment =
        declarator_value_has_inline_assignment_seam_prefix_comment(context, value_id);
    let value_has_prefix_annotation_that_forces_break = value_has_prefix_annotation
        && !value_is_inline_closure_cast_type_binary
        && !value_has_assignment_seam_inline_prefix_comment;

    let header_end = ty
        .map(|type_id| context.span(type_id).end)
        .unwrap_or(pattern_span.end);
    let value_span = context.span(value_id);
    let between_span = if header_end < value_span.start {
        Some(Span::new(value_span.file, header_end, value_span.start))
    } else {
        None
    };
    let value_has_newline = context.has_newline(value_span);
    let pattern_has_newline = context.has_newline(pattern_span);
    let pattern_has_default_assignment = pattern_has_default_assignment(tree, pattern_id);
    let pattern_has_comments_or_annotations =
        context.has_annotation(pattern_id) || span_has_comment(context, pattern_span);
    let value_is_parenthesized = matches!(value_expr, Expression::Parenthesized { .. });
    let value_has_existing_operator_break =
        between_span.is_some_and(|span| context.has_newline(span));
    let value_has_between_comment =
        between_span.is_some_and(|span| span_has_comment(context, span));
    let value_has_internal_comment = span_has_comment(context, value_span);
    let value_has_line_comment_between_operands = match value_inner_expr {
        Expression::Binary { left, right, .. } => {
            has_line_comment_between_expressions(context, *left, *right)
        }
        _ => false,
    };
    let value_binary_operand_count = match value_inner_expr {
        Expression::Binary { operator, .. } => {
            flattened_binary_operand_count(tree, value_inner_id, *operator)
        }
        _ => 0,
    };
    let value_is_long_binary =
        shape.value_is_binary && value_binary_operand_count > LONG_BINARY_OPERAND_COUNT_THRESHOLD;
    let has_single_chain_call = chain.value_chain_call_count <= COMPLEX_CHAIN_CALL_COUNT_THRESHOLD;
    let is_string_literal = matches!(
        value_inner_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    );
    let is_template_expression = matches!(value_inner_expr, Expression::TemplateExpression { .. });
    let value_is_leading_pipe_type_union = is_type_context(context, value_inner_id)
        && union_source_has_leading_pipe(context, value_inner_id);
    let value_has_static_type_arguments =
        expression_has_static_type_arguments(context, value_inner_id);
    let value_has_multiline_static_type_argument =
        expression_has_multiline_static_type_argument(context, value_inner_id);
    let value_has_instantiation_prefix =
        shape.value_is_chain && value_chain_has_instantiation_prefix(context, value_inner_id);
    let has_significant_between_comment = value_has_between_comment;
    let value_is_await_expression = matches!(
        value_expr,
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    );
    let value_is_comptime_expression = matches!(value_expr, Expression::Comptime { .. });
    let value_has_multiline_chain_body = value_has_newline && shape.value_is_chain;

    DeclaratorSourceProfile {
        value_has_newline,
        pattern_has_newline,
        pattern_has_default_assignment,
        pattern_has_comments_or_annotations,
        value_is_parenthesized,
        value_has_prefix_annotation_that_forces_break,
        value_has_existing_operator_break,
        value_has_between_comment,
        value_has_internal_comment,
        value_has_line_comment_between_operands,
        value_is_long_binary,
        has_single_chain_call,
        is_string_literal,
        is_template_expression,
        value_is_leading_pipe_type_union,
        value_has_static_type_arguments,
        value_has_multiline_static_type_argument,
        value_has_instantiation_prefix,
        has_significant_between_comment,
        value_is_await_expression,
        value_is_comptime_expression,
        value_has_multiline_chain_body,
    }
}

/// Build normalized layout inputs from declarator profiles.
fn build_declarator_layout_inputs(
    shape: DeclaratorShapeFacts,
    chain: DeclaratorChainProfile,
    source: DeclaratorSourceProfile,
    value_has_generic_class_heritage: bool,
    value_is_inline_closure_cast_type_binary: bool,
) -> DeclaratorLayoutInputs {
    DeclaratorLayoutInputs {
        pattern_breakable: shape.pattern_breakable,
        value_breakable: shape.value_breakable,
        value_is_leading_pipe_type_union: source.value_is_leading_pipe_type_union,
        value_is_inline_closure_cast_type_binary,
        is_string_literal: source.is_string_literal,
        value_is_long_binary: source.value_is_long_binary,
        value_has_internal_comment: source.value_has_internal_comment,
        value_has_between_comment: source.value_has_between_comment,
        value_has_existing_operator_break: source.value_has_existing_operator_break,
        value_handles_its_own_breaking: shape.value_handles_its_own_breaking,
        value_has_prefix_annotation_that_forces_break: source
            .value_has_prefix_annotation_that_forces_break,
        value_is_chain: shape.value_is_chain,
        has_single_chain_call: source.has_single_chain_call,
        value_chain_has_member_access: chain.value_chain_has_member_access,
        value_is_complex_chain: chain.value_is_complex_chain,
        value_has_static_type_arguments: source.value_has_static_type_arguments,
        value_has_multiline_static_type_argument: source.value_has_multiline_static_type_argument,
        value_has_instantiation_prefix: source.value_has_instantiation_prefix,
        has_significant_between_comment: source.has_significant_between_comment,
        value_has_generic_class_heritage,
        value_is_lambda: shape.value_is_lambda,
        value_is_declaration: shape.value_is_declaration,
        is_template_expression: source.is_template_expression,
        value_is_await_expression: source.value_is_await_expression,
        value_is_comptime_expression: source.value_is_comptime_expression,
        value_has_multiline_chain_body: source.value_has_multiline_chain_body,
        value_is_sequence: shape.value_is_sequence,
        value_has_line_comment_between_operands: source.value_has_line_comment_between_operands,
        value_is_call_like: shape.value_is_call_like,
        value_has_newline: source.value_has_newline,
        pattern_has_newline: source.pattern_has_newline,
        pattern_has_default_assignment: source.pattern_has_default_assignment,
        pattern_has_comments_or_annotations: source.pattern_has_comments_or_annotations,
        value_is_parenthesized: source.value_is_parenthesized,
    }
}

/// Format a declarator (pattern, optional type, optional value).
pub(in crate::format::expression) fn format_declarator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    tree: &NodeTree,
    declarator_id: LocalNodeId<Declarator>,
) -> FormatResult<()> {
    f.context()
        .increment_counter("profile.declarator.layout.builds", 1);

    let declarator = tree.get(declarator_id);
    let Declarator { pattern, ty, value } = declarator;

    // header: pattern + optional type
    let header = format_with(|f| {
        write!(f, [pattern])?;
        if let Some(ty_id) = ty {
            write!(f, [token(":"), space(), ty_id])?;
        }
        Ok(())
    });

    let Some(value_id) = value else {
        write!(f, [header])?;
        return Ok(());
    };

    let value_expr = tree.get(*value_id);
    let shape = collect_declarator_shape_facts(f.context(), tree, *pattern, *value_id);
    let value_inner_id = shape.value_inner_id;
    let value_inner_expr = tree.get(value_inner_id);
    let chain =
        collect_declarator_chain_profile(f.context(), tree, value_inner_id, shape.value_is_chain);
    let value_is_inline_closure_cast_type_binary =
        value_is_inline_closure_cast_type_binary(f.context(), *value_id);
    let value_has_generic_class_heritage =
        value_has_generic_class_heritage(f.context(), shape.value_inner_id);
    let source = collect_declarator_source_profile(
        f.context(),
        tree,
        *pattern,
        *ty,
        *value_id,
        shape.value_inner_id,
        value_expr,
        value_inner_expr,
        shape,
        chain,
        value_is_inline_closure_cast_type_binary,
    );
    let layout_inputs = build_declarator_layout_inputs(
        shape,
        chain,
        source,
        value_has_generic_class_heritage,
        value_is_inline_closure_cast_type_binary,
    );

    // layout fragments
    let format_inline = format_with(|f| {
        write!(f, [header, space(), token("="), space(), *value_id])?;
        Ok(())
    });

    // expand inline if value is breakable
    let format_value_expanded = format_with(|f| {
        write!(
            f,
            [
                header,
                space(),
                token("="),
                space(),
                fits_expanded(&group(value_id).should_expand(shape.should_force_expand_value)),
            ]
        )
    });

    // expand the header while keeping value inline
    let format_header_expanded = format_with(|f| {
        write!(
            f,
            [
                fits_expanded(&group(&header).should_expand(true)),
                space(),
                token("="),
                space(),
                *value_id,
            ]
        )
    });

    // expand and indent the value
    let format_indented = format_with(|f| {
        group(&format_args![
            header,
            space(),
            token("="),
            block_indent(value_id)
        ])
        .format(f)
    });
    let value_has_instantiation_prefix = source.value_has_instantiation_prefix;
    let format_break_after_operator_for_binary =
        format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let value_has_prefix_annotation = f.context().has_prefix_annotation(*value_id);
            let format_value_without_chain = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let can_format_call_without_chain = !f.context().has_annotation(*value_id)
                    && !f.context().has_annotation(value_inner_id)
                    && shape.value_is_chain;
                if !can_format_call_without_chain {
                    write!(f, [*value_id])?;
                    return Ok(());
                }

                match f.context().tree.get(value_inner_id) {
                    Expression::Call { .. } => {
                        format_call_expression(f, value_inner_id)?;
                    }
                    Expression::Instantiation { .. } => {
                        format_instantiation_expression(f, value_inner_id)?;
                    }
                    _ => {
                        write!(f, [*value_id])?;
                    }
                }
                Ok(())
            });

            let should_force_break_after_operator = shape.value_is_sequence;
            let break_after_operator = if should_force_break_after_operator {
                hard_line_break()
            } else {
                soft_line_break_or_space()
            };

            if value_has_prefix_annotation
                || shape.value_is_sequence
                || value_has_instantiation_prefix
            {
                write!(
                    f,
                    [group(&format_args![
                        header,
                        space(),
                        token("="),
                        indent(&format_args![
                            break_after_operator,
                            format_value_without_chain
                        ])
                    ])]
                )
            } else {
                let dedented_value = dedent(&format_value_without_chain);
                write!(
                    f,
                    [group(&format_args![
                        header,
                        space(),
                        token("="),
                        indent(&format_args![break_after_operator, dedented_value])
                    ])]
                )
            }
        });
    let layout = choose_declarator_layout(layout_inputs);

    match layout {
        DeclaratorLayout::Inline => write!(f, [format_inline])?,
        DeclaratorLayout::BreakAfterOperator => {
            write!(f, [format_break_after_operator_for_binary])?
        }
        DeclaratorLayout::ValueExpanded => write!(f, [format_value_expanded])?,
        DeclaratorLayout::HeaderExpanded => write!(f, [format_header_expanded])?,
        DeclaratorLayout::Indented => write!(f, [format_indented])?,
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declarator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        format_declarator(f, f.context().tree, node_id)?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}
