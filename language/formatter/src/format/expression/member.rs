use crate::format::chain::receiver_is_await_wrapped;
use crate::format::expression::{
    DestackFormatter, Expression, FormatResult, LocalNodeId, NodeTree, ParenthesizedUnwrapMode,
    PostfixPosition, Span, StringId, block_indent, format_static_argument_list, format_with, group,
    indent, line_postfix_boundary, should_parenthesize_index_expression,
    should_unwrap_parenthesized, soft_line_break, span_has_comment, token,
    write_postfix_base_expression,
};
use destack_fir::format::Buffer;
use destack_fir::{format_args, write};

/// Return whether one member receiver ends with static instantiation arguments.
fn expression_has_trailing_static_instantiation(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        Expression::Path {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Parenthesized { expression } => {
            expression_has_trailing_static_instantiation(tree, *expression)
        }
        _ => false,
    }
}

/// Format one member receiver, adding wrapper parentheses when static instantiation tails need grouping.
fn format_member_receiver<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    receiver_id: LocalNodeId<Expression>,
    should_wrap_for_static_instantiation: bool,
) -> FormatResult<()> {
    if should_wrap_for_static_instantiation {
        write!(f, [token("(")])?;
        write_postfix_base_expression(f, receiver_id)?;
        write!(f, [token(")")])?;
        return Ok(());
    }

    write_postfix_base_expression(f, receiver_id)
}

/// Format a member expression.
pub(crate) fn format_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    match f.context().tree.get(node_id) {
        Expression::Member {
            left,
            name,
            static_arguments,
        } => {
            let left = if let Expression::Parenthesized { expression } = f.context().tree.get(*left)
                && should_unwrap_parenthesized(
                    f.context(),
                    *left,
                    *expression,
                    ParenthesizedUnwrapMode::MemberObject,
                ) {
                *expression
            } else {
                *left
            };
            let is_breakable_member_receiver = matches!(
                f.context().tree.get(left),
                Expression::Call { .. } | Expression::Instantiation { .. }
            ) && !receiver_is_await_wrapped(f.context(), left);
            let should_wrap_receiver_for_static_instantiation =
                expression_has_trailing_static_instantiation(f.context().tree, left);

            if is_breakable_member_receiver {
                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| {
                            format_member_receiver(
                                f,
                                left,
                                should_wrap_receiver_for_static_instantiation,
                            )
                        }),
                        indent(&format_with(|f| {
                            write!(f, [soft_line_break(), token("."), *name])?;
                            if let Some(static_arguments) = static_arguments {
                                format_static_argument_list(f, static_arguments)?;
                            }
                            Ok(())
                        }))
                    ])]
                )?;
            } else {
                format_member_receiver(f, left, should_wrap_receiver_for_static_instantiation)?;
                write!(f, [token(".")])?;
                write!(f, [*name])?;
                if let Some(static_arguments) = static_arguments {
                    format_static_argument_list(f, static_arguments)?;
                }
            }
        }
        Expression::PrivateMember {
            left,
            name,
            static_arguments,
        } => {
            let left = if let Expression::Parenthesized { expression } = f.context().tree.get(*left)
                && should_unwrap_parenthesized(
                    f.context(),
                    *left,
                    *expression,
                    ParenthesizedUnwrapMode::MemberObject,
                ) {
                *expression
            } else {
                *left
            };
            let is_breakable_member_receiver = matches!(
                f.context().tree.get(left),
                Expression::Call { .. } | Expression::Instantiation { .. }
            ) && !receiver_is_await_wrapped(f.context(), left);
            let should_wrap_receiver_for_static_instantiation =
                expression_has_trailing_static_instantiation(f.context().tree, left);

            if is_breakable_member_receiver {
                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| {
                            format_member_receiver(
                                f,
                                left,
                                should_wrap_receiver_for_static_instantiation,
                            )
                        }),
                        indent(&format_with(|f| {
                            write!(f, [soft_line_break(), token("."), token("#"), *name])?;
                            if let Some(static_arguments) = static_arguments {
                                format_static_argument_list(f, static_arguments)?;
                            }
                            Ok(())
                        }))
                    ])]
                )?;
            } else {
                format_member_receiver(f, left, should_wrap_receiver_for_static_instantiation)?;
                write!(f, [token("."), token("#"), *name])?;
                if let Some(static_arguments) = static_arguments {
                    format_static_argument_list(f, static_arguments)?;
                }
            }
        }
        _ => {
            debug_assert!(false, "unexpected expression kind for member formatter");
        }
    }
    Ok(())
}

/// Format a type index expression without considering chaining.
#[inline]
pub(crate) fn format_type_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    index: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let needs_parentheses = matches!(
        f.context().tree.get(left),
        Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
    );
    if needs_parentheses {
        write!(f, [token("("), left, token(")")])?;
    } else {
        write!(f, [left])?;
    }
    write!(f, [token("["), index, token("]")])?;
    Ok(())
}

/// Format a type template literal expression.
pub(crate) fn format_type_template_literal<'ast>(
    strings: &[StringId],
    spans: &[LocalNodeId<Expression>],
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(strings.len(), spans.len().saturating_add(1));

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    for (span, segment) in spans.iter().zip(string_segments) {
        let should_expand_span = span_has_comment(f.context(), f.context().span(*span));

        write!(
            f,
            [
                token("${"),
                group(span).should_expand(should_expand_span),
                line_postfix_boundary(),
                token("}"),
                *segment,
            ]
        )?;
    }

    write!(f, [token("`")])
}

/// Format an index expression without considering chaining.
#[inline]
pub(crate) fn format_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Index {
        position,
        left,
        index,
    } = f.context().tree.get(node_id)
    {
        write_postfix_base_expression(f, *left)?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(index) = index {
            let should_parenthesize = should_parenthesize_index_expression(f.context(), *index);
            let left_span = f.context().span(*left);
            let index_span = f.context().span(*index);
            let has_break_after_open = left_span.file == index_span.file
                && left_span.end < index_span.start
                && f.context().has_newline(Span::new(
                    left_span.file,
                    left_span.end,
                    index_span.start,
                ));
            let should_break_index = has_break_after_open
                || f.context().has_newline(index_span)
                || f.context().has_annotation(*index);

            if should_break_index {
                write!(
                    f,
                    [group(&format_with(|f| {
                        write!(f, [token("[")])?;
                        if should_parenthesize {
                            write!(
                                f,
                                [block_indent(&format_args![token("("), *index, token(")")])]
                            )?;
                        } else {
                            write!(f, [block_indent(index)])?;
                        }
                        write!(f, [token("]")])
                    }))
                    .should_expand(true)]
                )?;
            } else if should_parenthesize {
                write!(f, [token("["), token("("), *index, token(")"), token("]")])?;
            } else {
                write!(f, [token("["), *index, token("]")])?;
            }
        } else {
            write!(f, [token("[]")])?;
        }
    } else {
        debug_assert!(false, "unexpected expression kind for index formatter");
    }
    Ok(())
}
