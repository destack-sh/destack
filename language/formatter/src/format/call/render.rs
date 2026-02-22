use crate::format::analysis::timing::tags;
use crate::format::call::arguments::{
    format_call_arguments, format_multiline_call_argument_list_with_last_separator_line_comment,
    format_single_plain_argument_with_separator_line_comment,
    single_argument_separator_line_comment_fact, write_argument_without_separator_line_comment,
    write_separator_line_comment_after_comma,
};
use crate::format::call::facts::{
    argument_is_plain_call_argument, call_arguments_preserve_blank_line_between,
    write_inline_call_argument_list, write_plain_call_argument, write_plain_call_argument_or_node,
};
use crate::format::call::layout::{
    CallArgumentCommentExpandedLayout, CallArgumentDefaultListLayout, CallArgumentLayoutDecision,
    CallArgumentLayoutRenderState,
};
use crate::format::expression::{
    Argument, DestackFormatter, Expression, FormatResult, GroupId, LocalNodeId, TrailingComma,
    block_indent, empty_line, format_block_of_properties, format_static_argument_list, format_with,
    group, hard_line_break, if_group_breaks, list_like, soft_block_indent,
    soft_line_break_or_space, space, token,
};
use destack_ast::PostfixPosition;
use destack_fir::format::{Buffer, Format};
use destack_fir::write;

/// Format a call expression.
#[inline]
pub(crate) fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f.context().timing_scope(tags::FORMAT_EXPRESSION_CALL);

    if let Expression::Call {
        position,
        left,
        static_arguments,
        dynamic_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(static_arguments) = static_arguments {
            format_static_argument_list(f, static_arguments)?;
        }

        format_call_arguments(f, node_id, dynamic_arguments)?;
    } else {
        debug_assert!(false, "unexpected expression kind for call formatter");
    }
    Ok(())
}

/// Format an instantiation expression.
#[inline]
pub(crate) fn format_instantiation_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Instantiation {
        left,
        static_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        format_static_argument_list(f, static_arguments)?;
    } else {
        debug_assert!(
            false,
            "unexpected expression kind for instantiation formatter"
        );
    }
    Ok(())
}

/// Write one single argument wrapped in call parentheses.
pub(crate) fn write_single_call_argument_inline_wrapped<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    write_plain_call_argument_or_node(f, argument_id)?;
    write!(f, [token(")")])
}

/// Write one trailing collection argument while forcing its value expression to expand.
fn write_collection_argument_with_expanded_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    let write_expanded_value =
        |f: &mut DestackFormatter<'ast, '_>, value: LocalNodeId<Expression>| -> FormatResult<()> {
            match f.context().tree.get(value) {
                Expression::ObjectExpression {
                    ty: None,
                    properties,
                } => write!(
                    f,
                    [
                        token("{"),
                        hard_line_break(),
                        block_indent(&format_with(|f| format_block_of_properties(
                            f, properties, ","
                        ))),
                        hard_line_break(),
                        token("}")
                    ]
                ),
                Expression::ArrayExpression { elements } => {
                    let mut list = list_like("[", "]", ",", elements);
                    list.should_expand(true);
                    write!(f, [list])
                }
                _ => write!(f, [group(&value).should_expand(true)]),
            }
        };

    match f.context().tree.get(argument_id) {
        Argument::Named {
            modifiers: None,
            name,
            value,
        } => {
            write!(f, [*name, token(":"), space()])?;
            write_expanded_value(f, *value)
        }
        Argument::Labeled {
            modifiers: None,
            label,
            value,
        } => {
            write!(f, [*label, token(":"), space()])?;
            write_expanded_value(f, *value)
        }
        Argument::Positional {
            modifiers: None,
            value,
        } => write_expanded_value(f, *value),
        Argument::Spread {
            modifiers: None,
            label: None,
            value,
        } => {
            write!(f, [token("...")])?;
            write_expanded_value(f, *value)
        }
        Argument::Spread {
            modifiers: None,
            label: Some(label),
            value,
        } => {
            write!(f, [token("..."), *label, token(":"), space()])?;
            write_expanded_value(f, *value)
        }
        _ => write!(f, [group(&argument_id).should_expand(true)]),
    }
}

/// Format one expanded call argument list with compact leading arguments and a trailing collection.
fn format_trailing_collection_hug_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let Some((&last_argument_id, leading_arguments)) = dynamic_arguments.split_last() else {
        write!(f, [token("("), token(")")])?;
        return Ok(());
    };

    write!(f, [token("(")])?;

    for (index, argument_id) in leading_arguments.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        if all_plain_call_arguments {
            write_plain_call_argument(f, argument_id)?;
        } else {
            write_plain_call_argument_or_node(f, argument_id)?;
        }
    }

    if !leading_arguments.is_empty() {
        write!(f, [token(","), space()])?;
    }

    write_collection_argument_with_expanded_value(f, last_argument_id)?;
    write!(f, [token(")")])?;

    Ok(())
}

/// Format plain default call arguments directly when all argument separators are stable.
fn format_plain_default_call_argument_list<'ast>(
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
    call_node_id: LocalNodeId<Expression>,
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout: CallArgumentDefaultListLayout,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_LIST_DEFAULT);

    if layout.use_separator_comment_multiline {
        let used_multiline_separator_layout =
            format_multiline_call_argument_list_with_last_separator_line_comment(
                f,
                call_node_id,
                dynamic_arguments,
            )?;

        debug_assert!(
            used_multiline_separator_layout,
            "separator-comment multiline layout should stay eligible from layout rules",
        );

        f.context().increment_counter(
            "call.arguments.path.list_default.separator_comment_multiline",
            1,
        );

        return Ok(());
    }

    if layout.use_plain_default_short_circuit {
        f.context()
            .increment_counter("call.arguments.path.list_default_plain_short_circuit", 1);

        return format_plain_default_call_argument_list(
            f,
            group_id,
            dynamic_arguments,
            layout.force_expand,
            all_plain_call_arguments,
        );
    }

    let mut list = list_like("(", ")", ",", dynamic_arguments);
    list.with_group_id(Some(group_id))
        .should_expand(layout.force_expand);

    if layout.disallow_trailing_separator {
        list.disallow_trailing_separator();
    }

    if layout.force_trailing_separator {
        list.force_trailing_separator();
    }

    write!(f, [list])
}

/// Format call arguments with explicit multiline comment expansion.
fn format_comment_expanded_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout: CallArgumentCommentExpandedLayout,
) -> FormatResult<()> {
    if layout.use_separator_comment_multiline {
        let used_multiline_separator_layout =
            format_multiline_call_argument_list_with_last_separator_line_comment(
                f,
                call_node_id,
                dynamic_arguments,
            )?;

        debug_assert!(
            used_multiline_separator_layout,
            "separator-comment multiline layout should stay eligible from layout rules",
        );

        f.context().increment_counter(
            "call.arguments.path.comment_expanded.separator_comment_multiline",
            1,
        );

        return Ok(());
    }

    if layout.use_single_plain_separator_comment_layout {
        debug_assert_eq!(
            dynamic_arguments.len(),
            1,
            "single plain separator layout should have one argument",
        );

        let argument_id = dynamic_arguments[0];

        let separator_line_comment_source =
            if argument_is_plain_call_argument(f.context(), argument_id) {
                single_argument_separator_line_comment_fact(f.context(), call_node_id, argument_id)
            } else {
                None
            };

        if let Some(comment_source) = separator_line_comment_source.as_ref() {
            return format_single_plain_argument_with_separator_line_comment(
                f,
                argument_id,
                comment_source,
            );
        }
    }

    let last_argument_separator_line_comment_source =
        dynamic_arguments.last().copied().and_then(|argument_id| {
            single_argument_separator_line_comment_fact(f.context(), call_node_id, argument_id)
        });

    write!(f, [token("("), hard_line_break()])?;

    let format_result = write!(
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
                    if is_last_argument
                        && let Some(comment_source) =
                            last_argument_separator_line_comment_source.as_ref()
                        && write_argument_without_separator_line_comment(f, *argument_id)?
                    {
                        write_separator_line_comment_after_comma(f, comment_source)?;
                        continue;
                    }

                    write!(f, [group(argument_id)])?;

                    if index + 1 < dynamic_arguments.len()
                        || layout.use_trailing_comma
                        || layout.force_trailing_comma_for_separator_comment
                    {
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

/// Render one decided call argument layout.
pub(crate) fn render_call_argument_plan<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    plan: CallArgumentLayoutDecision,
    render_state: CallArgumentLayoutRenderState,
) -> FormatResult<()> {
    match plan {
        CallArgumentLayoutDecision::InlineAll => write_inline_call_argument_list(
            f,
            dynamic_arguments,
            render_state.all_plain_call_arguments,
        ),
        CallArgumentLayoutDecision::InlineSingle => {
            debug_assert_eq!(dynamic_arguments.len(), 1);

            let Some(argument_id) = dynamic_arguments.first().copied() else {
                debug_assert!(false, "single inline layout requires one argument");
                return Ok(());
            };

            write_single_call_argument_inline_wrapped(f, argument_id)
        }
        CallArgumentLayoutDecision::TrailingCollectionExpanded => {
            format_trailing_collection_hug_list(
                f,
                dynamic_arguments,
                render_state.all_plain_call_arguments,
            )
        }
        CallArgumentLayoutDecision::CommentExpanded(layout) => {
            format_comment_expanded_call_argument_list(
                f,
                render_state.call_node_id,
                dynamic_arguments,
                layout,
            )
        }
        CallArgumentLayoutDecision::ListDefault(layout) => format_default_call_argument_list(
            f,
            render_state.call_node_id,
            render_state.group_id,
            dynamic_arguments,
            layout,
            render_state.all_plain_call_arguments,
        ),
    }
}
