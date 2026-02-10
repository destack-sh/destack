use super::*;
use destack_fir::{format_args, write};

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
                chain_call_count > 1 && has_multiline_call,
                chain_call_count,
                chain_has_member_access,
            )
        } else {
            (false, 0, false)
        };
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
                .any(|argument_id| f.context().has_newline(f.context().get_span(argument_id)))
        }),
        Expression::Instantiation {
            static_arguments, ..
        } => static_arguments
            .iter()
            .copied()
            .any(|argument_id| f.context().has_newline(f.context().get_span(argument_id))),
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

    // estimate remaining width if the declarator stayed inline
    let line_width = usize::from(f.context().options.line_width);
    let pattern_span = f.context().get_span(*pattern);
    let pattern_source_len = f.context().span_char_len(pattern_span);
    let type_source_len = ty.map(|ty_id| expression_source_len(f.context(), ty_id));
    let header_source_len = type_source_len.map_or(pattern_source_len, |type_len| {
        pattern_source_len
            .saturating_add(type_len)
            .saturating_add(2)
    });
    let remaining_width = line_width.saturating_sub(header_source_len.saturating_add(3));
    let leading_prefix_len = declarator_leading_prefix_len(f.context(), declarator_id);
    let remaining_width = remaining_width.saturating_sub(leading_prefix_len);

    let value_source_len = expression_source_len(f.context(), *value_id);
    let value_has_prefix_annotation = f.context().has_prefix_annotation(*value_id);
    let value_annotation_len = expression_prefix_annotation_source_len(f.context(), *value_id);
    let value_source_len = value_source_len.saturating_add(value_annotation_len);
    let value_is_long = value_source_len >= remaining_width;
    let estimated_inline_declarator_len = leading_prefix_len
        .saturating_add(header_source_len)
        .saturating_add(3)
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
    let value_is_parenthesized = matches!(value_expr, Expression::Parenthesized { .. });
    let value_has_generic_class_heritage =
        value_has_generic_class_heritage(f.context(), value_inner_id);
    let value_has_existing_operator_break =
        between_span.is_some_and(|span| f.context().has_newline(span));
    let value_has_between_comment =
        between_span.is_some_and(|span| span_has_comment(f.context(), span));
    let value_has_internal_comment = span_has_comment(f.context(), value_span);
    let value_binary_operand_count = match value_inner_expr {
        Expression::Binary { operator, .. } => {
            flatten_binary_expression(tree, value_inner_id, *operator).len()
        }
        _ => 0,
    };
    let value_is_long_binary = value_is_binary
        && value_is_long
        && value_binary_operand_count > 2
        && binary_rhs_prefers_break_after_operator(value_source_len, line_width);
    let inline_declarator_fits = estimated_inline_declarator_len <= line_width;

    // fast path: trivial declarators should stay inline without best fitting variants
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

    // prefer keeping the value on a single line
    let format_inline = format_with(|f| {
        write!(f, [header, space(), token("="), space(), *value_id])?;
        Ok(())
    });
    // expand inline if value is breakable (like let x = [\n ... ])
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

    // expand inline without a fits boundary for chains and binaries
    let format_value_expanded_strict = format_with(|f| {
        write!(
            f,
            [
                header,
                space(),
                token("="),
                space(),
                group(value_id).should_expand(should_force_expand_value),
            ]
        )
    });
    // expand the header (pattern) while keeping value inline
    // e.g., const { a, b } = value becomes:
    // const {
    //     a,
    //     b,
    // } = value;
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
    // expand and indent the value on a new line (last resort)
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

    // for string literals, never break at `=`: let them exceed line width
    if value_is_leading_pipe_type_union {
        write!(f, [format_inline])?;
    } else if is_string_literal {
        if value_is_long && !pattern_breakable {
            write!(f, [format_break_after_operator_for_binary])?;
        } else if pattern_breakable {
            if inline_declarator_fits {
                write!(f, [format_inline])?;
            } else {
                write!(f, [format_header_expanded])?;
            }
        } else {
            write!(f, [format_inline])?;
        }
    } else if value_is_long_binary {
        // comment-heavy logical chains read better with the first operand kept inline
        let prefer_inline_for_internal_binary_comment = value_has_internal_comment
            && !value_has_between_comment
            && !value_has_existing_operator_break;

        // long binary declarators generally break after `=`
        let prefer_operator_break = (value_has_existing_operator_break
            || value_has_between_comment
            || !inline_declarator_fits)
            && !prefer_inline_for_internal_binary_comment;
        if prefer_operator_break {
            write!(f, [format_break_after_operator_for_binary])?;
        } else {
            write!(f, [format_inline])?;
        }
    } else if value_handles_its_own_breaking {
        let source_rhs_has_newline = value_has_newline && allow_source_operator_break_preservation;
        let source_operator_has_newline =
            value_has_existing_operator_break && allow_source_operator_break_preservation;
        let static_argument_operator_break_candidate = value_is_call_like
            && value_has_static_arguments
            && !value_has_multiline_static_argument
            && (!value_is_chain || value_chain_call_count <= 1)
            && !value_has_prefix_annotation
            && !value_has_between_comment
            && estimated_inline_declarator_len >= line_width.saturating_sub(leading_prefix_len);
        let prefer_static_argument_operator_break = static_argument_operator_break_candidate
            && (source_operator_has_newline || value_is_long);
        let preserve_source_rhs_break =
            source_rhs_has_newline && (value_is_binary || value_is_sequence);
        let preserve_source_operator_break =
            source_operator_has_newline && (value_is_chain || value_is_binary || value_is_sequence);
        let has_significant_between_comment = value_has_between_comment
            && (value_is_long
                || estimated_inline_declarator_len.saturating_add(between_source_len)
                    >= line_width.saturating_sub(leading_prefix_len));

        // prefer header expansion before rhs internal expansion
        let value_prefers_operator_break = !value_is_lambda
            && !value_is_declaration
            && (value_has_prefix_annotation
                || preserve_source_rhs_break
                || preserve_source_operator_break
                || prefer_static_argument_operator_break
                || (value_is_chain
                    && value_is_long
                    && value_chain_call_count <= 1
                    && value_chain_has_member_access
                    && !value_is_complex_chain)
                || has_significant_between_comment
                || (value_has_generic_class_heritage && value_is_long));
        let value_is_await_expression = matches!(
            value_expr,
            Expression::Await { .. } | Expression::AwaitMaybe { .. }
        );
        let value_has_multiline_chain_body = value_has_newline && value_is_chain;
        let value_should_lead_with_break =
            (value_is_long && !value_is_await_expression && !value_has_multiline_chain_body)
                || value_has_prefix_annotation
                || preserve_source_operator_break
                || prefer_static_argument_operator_break
                || has_significant_between_comment;

        match pattern_breakable {
            true => {
                let should_try_operator_break_before_header_expand = value_is_long
                    && (value_is_binary || value_is_chain || value_is_call_like || value_is_lambda);
                if value_has_prefix_annotation
                    || has_significant_between_comment
                    || should_try_operator_break_before_header_expand
                {
                    if !inline_declarator_fits {
                        write!(f, [format_break_after_operator_for_binary])?;
                    } else {
                        f.context().record_best_fitting(
                            "best_fitting.expression.core.own_break.pattern_breakable.operator_first",
                            4,
                        );
                        best_fitting![
                            format_inline,
                            format_break_after_operator_for_binary,
                            format_header_expanded,
                            format_value_expanded_strict
                        ]
                        .format(f)?;
                    }
                } else if !inline_declarator_fits {
                    write!(f, [format_header_expanded])?;
                } else {
                    f.context().record_best_fitting(
                        "best_fitting.expression.core.own_break.pattern_breakable.header_first",
                        3,
                    );
                    best_fitting![
                        format_inline,
                        format_header_expanded,
                        format_value_expanded_strict
                    ]
                    .format(f)?;
                }
            }
            false => {
                if value_prefers_operator_break {
                    if value_should_lead_with_break {
                        // when rhs should lead with a break, prefer operator-first layout
                        write!(f, [format_break_after_operator_for_binary])?;
                    } else if inline_declarator_fits {
                        write!(f, [format_inline])?;
                    } else {
                        write!(f, [format_break_after_operator_for_binary])?;
                    }
                } else if inline_declarator_fits {
                    write!(f, [format_inline])?;
                } else {
                    write!(f, [format_value_expanded_strict])?;
                }
            }
        }
    } else {
        match (pattern_breakable, value_breakable) {
            (true, true) => {
                // both sides breakable: prefer inline, then expanded variants
                if !inline_declarator_fits {
                    f.context()
                        .record_best_fitting("best_fitting.expression.core.both_breakable", 3);
                    best_fitting![
                        format_value_expanded,
                        format_header_expanded,
                        format_indented
                    ]
                    .format(f)?;
                } else {
                    f.context()
                        .record_best_fitting("best_fitting.expression.core.both_breakable", 4);
                    best_fitting![
                        format_inline,
                        format_value_expanded,
                        format_header_expanded,
                        format_indented
                    ]
                    .format(f)?;
                }
            }
            (true, false) => {
                let pattern_has_newline = f.context().has_newline(pattern_span);
                let pattern_has_comments_or_annotations = f.context().has_annotation(*pattern)
                    || span_has_comment(f.context(), pattern_span);

                // keep long atomic rhs values inline when possible
                if pattern_has_newline {
                    if !inline_declarator_fits {
                        f.context().record_best_fitting(
                            "best_fitting.expression.core.pattern_breakable.pattern_newline",
                            2,
                        );
                        best_fitting![format_header_expanded, format_indented].format(f)?;
                    } else {
                        f.context().record_best_fitting(
                            "best_fitting.expression.core.pattern_breakable.pattern_newline",
                            3,
                        );
                        best_fitting![format_header_expanded, format_inline, format_indented]
                            .format(f)?;
                    }
                } else if value_is_long && pattern_has_comments_or_annotations {
                    if !inline_declarator_fits {
                        f.context().record_best_fitting(
                            "best_fitting.expression.core.pattern_breakable.long_with_comments",
                            2,
                        );
                        best_fitting![
                            format_break_after_operator_for_binary,
                            format_header_expanded
                        ]
                        .format(f)?;
                    } else {
                        f.context().record_best_fitting(
                            "best_fitting.expression.core.pattern_breakable.long_with_comments",
                            3,
                        );
                        best_fitting![
                            format_inline,
                            format_break_after_operator_for_binary,
                            format_header_expanded
                        ]
                        .format(f)?;
                    }
                } else if inline_declarator_fits {
                    write!(f, [format_inline])?;
                } else {
                    write!(f, [format_header_expanded])?;
                }
            }
            (false, true) => {
                // only force operator break when there is an explicit boundary comment near `=`
                let prefers_operator_break = value_has_between_comment;
                let prefer_declaration_operator_break =
                    value_is_declaration && (value_is_long || value_has_newline);

                if prefers_operator_break || prefer_declaration_operator_break {
                    if inline_declarator_fits
                        && !value_is_long
                        && !value_has_between_comment
                        && !value_has_prefix_annotation
                    {
                        write!(f, [format_inline])?;
                    } else {
                        write!(f, [format_indented])?;
                    }
                } else if (value_is_parenthesized && value_has_newline) || inline_declarator_fits {
                    write!(f, [format_inline])?;
                } else if value_has_newline || value_has_prefix_annotation || value_is_long {
                    write!(f, [format_value_expanded])?;
                } else {
                    write!(f, [format_indented])?;
                }
            }
            (false, false) => {
                if value_has_generic_class_heritage && value_is_long {
                    write!(f, [format_indented])?;
                } else if (value_is_parenthesized && value_has_newline) || inline_declarator_fits {
                    write!(f, [format_inline])?;
                } else {
                    write!(f, [format_indented])?;
                }
            }
        }
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
