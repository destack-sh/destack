use super::*;
use destack_fir::{format_args, write};

// declarator inline width constants
const DECLARATOR_TYPE_SEPARATOR_INLINE_WIDTH: usize = 2;
const DECLARATOR_ASSIGNMENT_SEPARATOR_INLINE_WIDTH: usize = 3;

// declarator chain and binary thresholds
const COMPLEX_CHAIN_CALL_COUNT_THRESHOLD: usize = 1;
const LONG_BINARY_OPERAND_COUNT_THRESHOLD: usize = 2;

/// Enumerate declarator layout outcomes.
#[derive(Clone, Copy)]
enum DeclaratorLayout {
    Inline,
    BreakAfterOperator,
    ValueExpanded,
    HeaderExpanded,
    Indented,
}

/// Store normalized declarator layout selection inputs.
#[derive(Clone, Copy)]
struct DeclaratorLayoutInputs {
    pattern_breakable: bool,
    value_breakable: bool,
    value_is_long: bool,
    inline_declarator_fits: bool,
    value_is_leading_pipe_type_union: bool,
    is_string_literal: bool,
    value_is_long_binary: bool,
    value_has_internal_comment: bool,
    value_has_between_comment: bool,
    value_has_existing_operator_break: bool,
    value_handles_its_own_breaking: bool,
    value_has_prefix_annotation: bool,
    prefer_static_argument_operator_break: bool,
    value_is_chain: bool,
    has_single_chain_call: bool,
    value_chain_has_member_access: bool,
    value_is_complex_chain: bool,
    has_significant_between_comment: bool,
    value_has_generic_class_heritage: bool,
    value_is_lambda: bool,
    value_is_declaration: bool,
    value_is_await_expression: bool,
    value_has_multiline_chain_body: bool,
    value_is_sequence: bool,
    value_has_line_comment_between_operands: bool,
    value_is_call_like: bool,
    value_has_newline: bool,
    pattern_has_newline: bool,
    pattern_has_default_assignment: bool,
    pattern_has_comments_or_annotations: bool,
    value_is_parenthesized: bool,
}

/// Choose the declarator layout from normalized inputs.
fn choose_declarator_layout(inputs: DeclaratorLayoutInputs) -> DeclaratorLayout {
    // keep leading pipe unions inline at the declarator level
    if inputs.value_is_leading_pipe_type_union {
        return DeclaratorLayout::Inline;
    }

    // keep string rhs mostly inline
    if inputs.is_string_literal {
        if inputs.value_is_long && !inputs.pattern_breakable {
            return DeclaratorLayout::BreakAfterOperator;
        }

        if inputs.pattern_breakable {
            return if inputs.inline_declarator_fits {
                DeclaratorLayout::Inline
            } else {
                DeclaratorLayout::HeaderExpanded
            };
        }

        return DeclaratorLayout::Inline;
    }

    // declaration rhs values with prefix trivia should break directly after `=`
    if inputs.value_is_declaration && inputs.value_has_prefix_annotation {
        return DeclaratorLayout::BreakAfterOperator;
    }

    // binary rhs operator break policy
    if inputs.value_is_long_binary {
        let prefer_inline_for_internal_binary_comment = inputs.value_has_internal_comment
            && !inputs.value_has_between_comment
            && !inputs.value_has_existing_operator_break;
        let prefer_operator_break = (inputs.value_has_existing_operator_break
            || inputs.value_has_between_comment
            || !inputs.inline_declarator_fits)
            && !prefer_inline_for_internal_binary_comment;

        return if prefer_operator_break {
            DeclaratorLayout::BreakAfterOperator
        } else {
            DeclaratorLayout::Inline
        };
    }

    // chain and binary values that already own their line breaking
    if inputs.value_handles_its_own_breaking {
        // self-breaking rhs policy: avoid source-preserving break heuristics, they cause idempotence flips
        let value_prefers_operator_break = !inputs.value_is_lambda
            && !inputs.value_is_declaration
            && (inputs.value_has_prefix_annotation
                || inputs.prefer_static_argument_operator_break
                || (inputs.value_is_chain
                    && inputs.value_is_long
                    && inputs.has_single_chain_call
                    && inputs.value_chain_has_member_access
                    && !inputs.value_is_complex_chain)
                || inputs.has_significant_between_comment
                || (inputs.value_has_generic_class_heritage && inputs.value_is_long));
        let value_should_lead_with_break = (inputs.value_is_long
            && !inputs.value_is_await_expression
            && !inputs.value_has_multiline_chain_body)
            || inputs.value_has_prefix_annotation
            || inputs.prefer_static_argument_operator_break
            || inputs.has_significant_between_comment;

        if inputs.pattern_breakable {
            if inputs.value_has_prefix_annotation || inputs.has_significant_between_comment {
                return DeclaratorLayout::BreakAfterOperator;
            }

            let should_break_after_operator_for_long_rhs = !inputs.inline_declarator_fits
                && !inputs.pattern_has_newline
                && ((inputs.value_is_call_like && inputs.pattern_has_default_assignment)
                    || inputs.value_is_sequence
                    || inputs.value_has_line_comment_between_operands);
            if should_break_after_operator_for_long_rhs {
                return DeclaratorLayout::BreakAfterOperator;
            }

            return DeclaratorLayout::Inline;
        }

        if value_prefers_operator_break {
            if value_should_lead_with_break {
                return DeclaratorLayout::BreakAfterOperator;
            }

            return DeclaratorLayout::Inline;
        }

        let should_break_after_operator_for_long_rhs = !inputs.inline_declarator_fits
            && (inputs.value_is_sequence || inputs.value_has_line_comment_between_operands);
        if should_break_after_operator_for_long_rhs {
            return DeclaratorLayout::BreakAfterOperator;
        }

        return DeclaratorLayout::Inline;
    }

    // fallback matrix for non self-breaking values
    match (inputs.pattern_breakable, inputs.value_breakable) {
        (true, true) => {
            if !inputs.inline_declarator_fits {
                DeclaratorLayout::ValueExpanded
            } else {
                DeclaratorLayout::Inline
            }
        }
        (true, false) => {
            if inputs.pattern_has_newline {
                DeclaratorLayout::HeaderExpanded
            } else if inputs.value_is_long && inputs.pattern_has_comments_or_annotations {
                if !inputs.inline_declarator_fits {
                    DeclaratorLayout::BreakAfterOperator
                } else {
                    DeclaratorLayout::Inline
                }
            } else if inputs.inline_declarator_fits {
                DeclaratorLayout::Inline
            } else {
                DeclaratorLayout::HeaderExpanded
            }
        }
        (false, true) => {
            let prefers_operator_break = inputs.value_has_between_comment;
            let prefer_declaration_operator_break =
                inputs.value_is_declaration && (inputs.value_is_long || inputs.value_has_newline);

            if prefers_operator_break || prefer_declaration_operator_break {
                if inputs.inline_declarator_fits
                    && !inputs.value_is_long
                    && !inputs.value_has_between_comment
                    && !inputs.value_has_prefix_annotation
                {
                    DeclaratorLayout::Inline
                } else {
                    DeclaratorLayout::Indented
                }
            } else if (inputs.value_is_parenthesized && inputs.value_has_newline)
                || inputs.inline_declarator_fits
            {
                DeclaratorLayout::Inline
            } else if inputs.value_has_newline
                || inputs.value_has_prefix_annotation
                || inputs.value_is_long
            {
                DeclaratorLayout::ValueExpanded
            } else {
                DeclaratorLayout::Indented
            }
        }
        (false, false) => {
            if inputs.value_has_generic_class_heritage && inputs.value_is_long {
                DeclaratorLayout::Indented
            } else if (inputs.value_is_parenthesized && inputs.value_has_newline)
                || inputs.inline_declarator_fits
            {
                DeclaratorLayout::Inline
            } else {
                DeclaratorLayout::Indented
            }
        }
    }
}

/// Return whether a declaration heritage clause contains static type arguments.
fn declaration_has_generic_heritage(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let heritage = match context.tree.get(declaration_id) {
        Declaration::Struct { heritage, .. }
        | Declaration::Class { heritage, .. }
        | Declaration::Enum { heritage, .. }
        | Declaration::Interface { heritage, .. }
        | Declaration::Extension { heritage, .. } => heritage,
        _ => return false,
    };

    heritage.extends_types.as_ref().is_some_and(|types| {
        types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(context, type_id))
    }) || heritage.implements_types.as_ref().is_some_and(|types| {
        types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(context, type_id))
    })
}

/// Return whether a value expression wraps a class declaration with generic heritage.
fn value_has_generic_class_heritage(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            matches!(context.tree.get(*declaration_id), Declaration::Class { .. })
                && declaration_has_generic_heritage(context, *declaration_id)
        }
        Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. } => {
            value_has_generic_class_heritage(context, *left)
        }
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            value_has_generic_class_heritage(context, *expression)
        }
        _ => false,
    }
}

/// Format a declarator (pattern, optional type, optional value).
pub(super) fn format_declarator<'ast>(
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
    let value_inner_id = transparent_inner_expression(f.context(), *value_id);
    let value_inner_expr = tree.get(value_inner_id);

    // value shape facts
    let pattern_breakable = is_pattern_breakable(tree, *pattern);
    let value_breakable = is_expression_breakable(tree, value_expr);
    let value_is_binary = matches!(value_inner_expr, Expression::Binary { .. });
    let value_is_sequence = matches!(value_inner_expr, Expression::SequenceExpression { .. });
    let value_is_chain_root = is_chain_root(tree, value_inner_id);
    let value_is_chain = is_expression_chain(tree, value_inner_id) || value_is_chain_root;
    let value_is_poor_chain =
        value_is_chain && is_poorly_breakable_chain(f.context(), value_inner_id);
    let value_is_call_like = matches!(
        value_inner_expr,
        Expression::Call { .. } | Expression::New { .. } | Expression::Instantiation { .. }
    );
    let value_is_lambda = is_lambda_expression(f.context(), value_inner_id);
    let value_is_declaration = matches!(value_inner_expr, Expression::Declaration(_));
    let value_handles_its_own_breaking = value_is_binary
        || value_is_sequence
        || value_is_chain
        || value_is_call_like
        || value_is_lambda
        || value_is_declaration;
    let should_force_expand_value = value_breakable && !value_is_poor_chain;

    // chain profile
    let (value_is_complex_chain, value_chain_call_count, value_chain_has_member_access) =
        if value_is_chain {
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

            let chain_has_member_access = chain.iter().copied().any(|expression_id| {
                matches!(
                    tree.get(expression_id),
                    Expression::Member { .. } | Expression::PrivateMember { .. }
                )
            });
            let call_summaries = summarize_chain_calls(f.context(), &chain);
            let chain_call_count = call_summaries.len();
            let has_multiline_call = call_summaries
                .iter()
                .any(|summary| summary.has_multiline_argument);
            (
                chain_call_count > COMPLEX_CHAIN_CALL_COUNT_THRESHOLD && has_multiline_call,
                chain_call_count,
                chain_has_member_access,
            )
        } else {
            (false, 0, false)
        };

    // call argument profile
    let value_is_multiline_call_like = match value_inner_expr {
        Expression::Call {
            static_arguments,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            static_arguments,
            dynamic_arguments,
            ..
        } => {
            dynamic_arguments
                .iter()
                .copied()
                .any(|argument_id| argument_forces_multiline(f.context(), argument_id))
                || static_arguments.as_ref().is_some_and(|arguments| {
                    arguments
                        .iter()
                        .copied()
                        .any(|argument_id| argument_forces_multiline(f.context(), argument_id))
                })
        }
        Expression::Instantiation {
            static_arguments, ..
        } => static_arguments
            .iter()
            .copied()
            .any(|argument_id| argument_forces_multiline(f.context(), argument_id)),
        _ => false,
    };
    let value_has_multiline_static_argument = match value_inner_expr {
        Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        } => static_arguments.as_ref().is_some_and(|arguments| {
            arguments
                .iter()
                .copied()
                .any(|argument_id| f.context().node_has_newline(argument_id))
        }),
        Expression::Instantiation {
            static_arguments, ..
        } => static_arguments
            .iter()
            .copied()
            .any(|argument_id| f.context().node_has_newline(argument_id)),
        _ => false,
    };
    let value_has_static_arguments = match value_inner_expr {
        Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        _ => false,
    };
    let allow_source_operator_break_preservation =
        !(value_is_complex_chain || (value_is_multiline_call_like && !value_has_static_arguments));

    // width budget
    let line_width = usize::from(f.context().options.line_width);
    let pattern_span = f.context().get_span(*pattern);
    let pattern_source_len = f.context().span_char_len(pattern_span);
    let type_source_len = ty.map(|ty_id| expression_source_len(f.context(), ty_id));
    let header_source_len = type_source_len.map_or(pattern_source_len, |type_len| {
        pattern_source_len
            .saturating_add(type_len)
            .saturating_add(DECLARATOR_TYPE_SEPARATOR_INLINE_WIDTH)
    });
    let remaining_width = line_width.saturating_sub(
        header_source_len.saturating_add(DECLARATOR_ASSIGNMENT_SEPARATOR_INLINE_WIDTH),
    );
    let leading_prefix_len = declarator_leading_prefix_len(f.context(), declarator_id);
    let remaining_width = remaining_width.saturating_sub(leading_prefix_len);

    let value_source_len = expression_source_len(f.context(), *value_id);
    let value_has_prefix_annotation = f.context().has_prefix_annotation(*value_id);
    let value_annotation_len = expression_prefix_annotation_source_len(f.context(), *value_id);
    let value_source_len = value_source_len.saturating_add(value_annotation_len);
    let value_is_long = value_source_len >= remaining_width;
    let estimated_inline_declarator_len = leading_prefix_len
        .saturating_add(header_source_len)
        .saturating_add(DECLARATOR_ASSIGNMENT_SEPARATOR_INLINE_WIDTH)
        .saturating_add(value_source_len);
    let header_end = ty
        .map(|type_id| f.context().get_span(type_id).end)
        .unwrap_or(pattern_span.end);
    let value_span = f.context().get_span(*value_id);
    let between_span = if header_end < value_span.start {
        Some(Span::new(value_span.file, header_end, value_span.start))
    } else {
        None
    };
    let between_source_len = between_span
        .map(|span| f.context().span_char_len(span))
        .unwrap_or(0);
    let value_has_newline = f.context().has_newline(value_span);
    let pattern_has_newline = f.context().has_newline(pattern_span);
    let pattern_source = f.context().get_span_str(pattern_span);
    let pattern_has_default_assignment = pattern_source.contains('=');
    let pattern_has_comments_or_annotations =
        f.context().has_annotation(*pattern) || span_has_comment(f.context(), pattern_span);
    let value_is_parenthesized = matches!(value_expr, Expression::Parenthesized { .. });
    let value_has_generic_class_heritage =
        value_has_generic_class_heritage(f.context(), value_inner_id);
    let value_has_existing_operator_break =
        between_span.is_some_and(|span| f.context().has_newline(span));
    let value_has_between_comment =
        between_span.is_some_and(|span| span_has_comment(f.context(), span));
    let value_has_internal_comment = span_has_comment(f.context(), value_span);
    let value_has_line_comment_between_operands = match value_inner_expr {
        Expression::Binary { left, right, .. } => {
            line_comment_between_expressions(f.context(), *left, *right).is_some()
        }
        _ => false,
    };
    let value_binary_operand_count = match value_inner_expr {
        Expression::Binary { operator, .. } => {
            flattened_binary_operand_count(tree, value_inner_id, *operator)
        }
        _ => 0,
    };
    let value_is_long_binary = value_is_binary
        && value_is_long
        && value_binary_operand_count > LONG_BINARY_OPERAND_COUNT_THRESHOLD
        && binary_rhs_prefers_break_after_operator(value_source_len, line_width);
    let inline_declarator_fits = estimated_inline_declarator_len <= line_width;
    let has_single_chain_call = value_chain_call_count <= COMPLEX_CHAIN_CALL_COUNT_THRESHOLD;

    // fast path
    let simple_inline_declarator = !pattern_breakable
        && !value_breakable
        && !value_is_long
        && !value_has_newline
        && !value_has_prefix_annotation
        && !value_has_between_comment
        && !value_has_generic_class_heritage
        && !f.context().has_annotation(*pattern)
        && !f.context().has_annotation(*value_id)
        && !f.context().has_annotation(value_inner_id);
    if simple_inline_declarator {
        write!(f, [header, space(), token("="), space(), *value_id])?;
        return Ok(());
    }

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
                fits_expanded(&group(value_id).should_expand(should_force_expand_value)),
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
    let format_break_after_operator_for_binary =
        format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let value_has_prefix_annotation = f.context().has_prefix_annotation(*value_id);
            let format_value_without_chain = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let can_format_call_without_chain = !f.context().has_annotation(*value_id)
                    && !f.context().has_annotation(value_inner_id)
                    && value_is_chain;
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

            if value_has_prefix_annotation || value_is_sequence {
                write!(
                    f,
                    [
                        header,
                        space(),
                        token("="),
                        indent(&format_args![hard_line_break(), format_value_without_chain])
                    ]
                )
            } else {
                let dedented_value = dedent(&format_value_without_chain);
                write!(
                    f,
                    [
                        header,
                        space(),
                        token("="),
                        indent(&format_args![hard_line_break(), dedented_value])
                    ]
                )
            }
        });
    let is_string_literal = matches!(
        value_inner_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    );
    let value_is_leading_pipe_type_union = is_type_context(f.context(), value_inner_id)
        && union_source_has_leading_pipe(f.context(), value_inner_id);
    let source_operator_has_newline =
        value_has_existing_operator_break && allow_source_operator_break_preservation;
    let static_argument_operator_break_candidate = value_is_call_like
        && value_has_static_arguments
        && !value_has_multiline_static_argument
        && (!value_is_chain || has_single_chain_call)
        && !value_has_prefix_annotation
        && !value_has_between_comment
        && estimated_inline_declarator_len >= line_width.saturating_sub(leading_prefix_len);
    let prefer_static_argument_operator_break =
        static_argument_operator_break_candidate && (source_operator_has_newline || value_is_long);
    let has_significant_between_comment = value_has_between_comment
        && (value_is_long
            || estimated_inline_declarator_len.saturating_add(between_source_len)
                >= line_width.saturating_sub(leading_prefix_len));
    let value_is_await_expression = matches!(
        value_expr,
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    );
    let value_has_multiline_chain_body = value_has_newline && value_is_chain;

    let layout = choose_declarator_layout(DeclaratorLayoutInputs {
        pattern_breakable,
        value_breakable,
        value_is_long,
        inline_declarator_fits,
        value_is_leading_pipe_type_union,
        is_string_literal,
        value_is_long_binary,
        value_has_internal_comment,
        value_has_between_comment,
        value_has_existing_operator_break,
        value_handles_its_own_breaking,
        value_has_prefix_annotation,
        prefer_static_argument_operator_break,
        value_is_chain,
        has_single_chain_call,
        value_chain_has_member_access,
        value_is_complex_chain,
        has_significant_between_comment,
        value_has_generic_class_heritage,
        value_is_lambda,
        value_is_declaration,
        value_is_await_expression,
        value_has_multiline_chain_body,
        value_is_sequence,
        value_has_line_comment_between_operands,
        value_is_call_like,
        value_has_newline,
        pattern_has_newline,
        pattern_has_default_assignment,
        pattern_has_comments_or_annotations,
        value_is_parenthesized,
    });

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
