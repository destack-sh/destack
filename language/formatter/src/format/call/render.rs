use super::decide::trailing_collection_argument_has_comment_signal;
use super::separator::{
    format_multiline_call_argument_list_with_last_separator_line_comment,
    format_single_plain_argument_with_separator_line_comment,
    single_argument_separator_line_comment_fact, write_argument_without_separator_line_comment,
    write_separator_line_comment_after_comma,
};
use crate::analysis::timing::tags;
use crate::analysis::{
    CallArgumentLayoutDecision, CallArgumentLayoutRenderState,
    argument_has_separator_line_comment_annotation, argument_is_collection_literal,
    argument_is_interpolated_template_literal, argument_is_plain_call_argument,
    call_arguments_preserve_blank_line_between, write_inline_call_argument_list,
    write_plain_call_argument, write_plain_call_argument_or_node,
};
use crate::expression::{
    Argument, DestackFormatter, Expression, FormatResult, GroupId, LocalNodeId, TrailingComma,
    argument_is_template_literal, block_indent, empty_line, format_block_of_properties,
    format_with, group, hard_line_break, if_group_breaks, list_like, soft_block_indent,
    soft_line_break_or_space, space, token,
};
use destack_fir::format::{Buffer, Format};
use destack_fir::write;

/// Format default call arguments via direct argument emission for annotation-free lists.
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

/// Format call arguments with the default list formatter.
fn format_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    has_line_comment_annotations: bool,
    has_any_argument_annotation: bool,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let is_single_argument = dynamic_arguments.len() == 1;
    let single_argument_id = if is_single_argument {
        Some(dynamic_arguments[0])
    } else {
        None
    };
    let has_single_template_literal_argument = if is_single_argument {
        single_argument_id
            .is_some_and(|argument_id| argument_is_template_literal(f.context(), argument_id))
    } else {
        false
    };
    let trailing_collection_has_comment_signal =
        trailing_collection_argument_has_comment_signal(f.context(), dynamic_arguments);
    let single_argument_has_separator_line_comment_annotation = if is_single_argument {
        single_argument_id.is_some_and(|argument_id| {
            argument_has_separator_line_comment_annotation(f.context(), argument_id)
        })
    } else {
        false
    };
    let last_argument_has_separator_line_comment_annotation = dynamic_arguments
        .last()
        .copied()
        .is_some_and(|argument_id| {
            argument_has_separator_line_comment_annotation(f.context(), argument_id)
        });
    let can_use_plain_default_short_circuit = !f.context().has_ignore_directive_markers()
        && dynamic_arguments.len() > 1
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !trailing_collection_has_comment_signal
        && !is_single_argument;

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_LIST_DEFAULT);
    if last_argument_has_separator_line_comment_annotation
        && format_multiline_call_argument_list_with_last_separator_line_comment(
            f,
            call_node_id,
            dynamic_arguments,
        )?
    {
        f.context().increment_counter(
            "call.arguments.path.list_default.separator_comment_multiline",
            1,
        );
        return Ok(());
    }

    if can_use_plain_default_short_circuit {
        f.context()
            .increment_counter("call.arguments.path.list_default_plain_short_circuit", 1);
        return format_plain_default_call_argument_list(
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
    if trailing_collection_has_comment_signal {
        list.disallow_trailing_separator();
    }
    if last_argument_has_separator_line_comment_annotation {
        list.force_trailing_separator();
    }
    if single_argument_has_separator_line_comment_annotation {
        list.force_trailing_separator();
    }

    write!(f, [list])
}

/// Format call arguments with explicit multiline comment expansion.
fn format_comment_expanded_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if format_multiline_call_argument_list_with_last_separator_line_comment(
        f,
        call_node_id,
        dynamic_arguments,
    )? {
        f.context().increment_counter(
            "call.arguments.path.comment_expanded.separator_comment_multiline",
            1,
        );
        return Ok(());
    }

    if dynamic_arguments.len() == 1 {
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

    let has_trailing_collection_comment =
        trailing_collection_argument_has_comment_signal(f.context(), dynamic_arguments);
    let has_single_argument_separator_line_comment_annotation = dynamic_arguments.len() == 1
        && argument_has_separator_line_comment_annotation(f.context(), dynamic_arguments[0]);
    let has_last_argument_separator_line_comment_annotation = dynamic_arguments
        .last()
        .copied()
        .is_some_and(|argument_id| {
            argument_has_separator_line_comment_annotation(f.context(), argument_id)
        });
    let last_argument_separator_line_comment_source =
        dynamic_arguments.last().copied().and_then(|argument_id| {
            single_argument_separator_line_comment_fact(f.context(), call_node_id, argument_id)
        });
    let use_trailing_comma = f.context().options.trailing_comma == TrailingComma::All
        && !has_trailing_collection_comment
        && !has_single_argument_separator_line_comment_annotation;
    let force_trailing_comma_for_separator_comment =
        has_last_argument_separator_line_comment_annotation
            && last_argument_separator_line_comment_source.is_none();

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
                        || use_trailing_comma
                        || force_trailing_comma_for_separator_comment
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

/// Write one single argument wrapped in call parentheses.
pub(super) fn write_single_call_argument_inline_wrapped<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    write_plain_call_argument_or_node(f, argument_id)?;
    write!(f, [token(")")])
}

/// Render one decided call argument layout.
pub(super) fn render_call_argument_plan<'ast>(
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
        CallArgumentLayoutDecision::CommentExpanded(_comment_profile) => {
            format_comment_expanded_call_argument_list(
                f,
                render_state.call_node_id,
                dynamic_arguments,
            )
        }
        CallArgumentLayoutDecision::ListDefault {
            force_expand,
            has_line_comment_annotations,
        } => format_default_call_argument_list(
            f,
            render_state.call_node_id,
            render_state.group_id,
            dynamic_arguments,
            force_expand,
            has_line_comment_annotations,
            render_state.has_any_argument_annotation,
            render_state.all_plain_call_arguments,
        ),
    }
}
