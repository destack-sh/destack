use super::argument::write_argument_with_following_span_start;
use crate::format::annotation::{
    DanglingIndentMode, FormatDanglingComments, block_infix_annotations,
};
use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::format::file::any_ignore_range_for_nodes;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Argument, Comment, DecoratorPosition, Expression, LocalNodeId, TokenType};
use destack_fir::format::{Buffer, FormatNodes, FormatResult, GroupId};
use destack_fir::prelude::{
    block_indent, empty_line, format_with, group, if_group_breaks, line_suffix_boundary,
    soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::TrailingComma;

/// The separator to emit for one call argument entry.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CallArgumentSeparator {
    /// No separator follows this argument.
    None,

    /// Always emit a comma after this argument.
    Always,

    /// Emit a comma only when the enclosing argument group breaks.
    IfGroupBreaks {
        /// The surrounding argument group id.
        group_id: GroupId,
    },
}

/// Return the source line distance between two byte offsets.
fn line_distance_between_offsets(
    context: &DestackFormatContext<'_>,
    start_offset: u32,
    end_offset: u32,
) -> Option<u32> {
    let (start_line, _) = context.file.get_position(start_offset)?;
    let (end_line, _) = context.file.get_position(end_offset)?;

    end_line.checked_sub(start_line)
}

/// Return whether source text contains an empty line between adjacent arguments.
pub(crate) fn arguments_have_empty_line(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    arguments.windows(2).any(|window| {
        let [current_argument_id, next_argument_id] = window else {
            return false;
        };

        let current_span = context.span(*current_argument_id);
        let next_span = context.span(*next_argument_id);

        line_distance_between_offsets(context, current_span.end, next_span.start)
            .is_some_and(|line_distance| line_distance > 1)
    })
}

/// Return the number of source lines before one call argument, including preceding comments.
pub(crate) fn call_argument_lines_before(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> usize {
    let argument_span = context.span(argument_id);
    let comments = context.comments();

    context
        .source_text()
        .get_lines_before(argument_span, comments)
}

/// Format all call arguments in explicit broken-out layout.
pub(crate) fn format_all_args_broken_out<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _call_span: Span,
    arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
    disallow_trailing_separator: bool,
) -> FormatResult<()> {
    let write_trailing_separator = !disallow_trailing_separator
        && matches!(f.context().options.trailing_comma, TrailingComma::All);

    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                for (index, argument_id) in arguments.iter().copied().enumerate() {
                    if index > 0 {
                        let has_empty_line =
                            call_argument_lines_before(f.context(), argument_id) > 1;

                        if has_empty_line {
                            write!(f, [empty_line()])?;
                        } else {
                            write!(f, [soft_line_break_or_space()])?;
                        }
                    }

                    let following_span_start = arguments
                        .get(index + 1)
                        .map(|argument_id| f.context().span(*argument_id).start)
                        .unwrap_or(0);
                    let separator = if index + 1 != arguments.len() {
                        CallArgumentSeparator::Always
                    } else if write_trailing_separator {
                        CallArgumentSeparator::Always
                    } else {
                        CallArgumentSeparator::None
                    };

                    write_call_argument_in_list(f, argument_id, following_span_start, separator)?;
                }

                Ok(())
            })),
            line_suffix_boundary(),
            token(")")
        ])
        .with_id(Some(group_id))
        .should_expand(true)]
    )
}

/// Format arguments for one long curried call.
pub(crate) fn format_long_curried_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _call_span: Span,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let write_trailing_separator = matches!(f.context().options.trailing_comma, TrailingComma::All);

    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                for (index, argument_id) in arguments.iter().copied().enumerate() {
                    if index > 0 {
                        write!(f, [soft_line_break_or_space()])?;
                    }

                    let following_span_start = arguments
                        .get(index + 1)
                        .map(|argument_id| f.context().span(*argument_id).start)
                        .unwrap_or(0);
                    let separator = if index + 1 != arguments.len() {
                        CallArgumentSeparator::Always
                    } else if write_trailing_separator {
                        CallArgumentSeparator::Always
                    } else {
                        CallArgumentSeparator::None
                    };

                    write_call_argument_in_list(f, argument_id, following_span_start, separator)?;
                }

                Ok(())
            })),
            line_suffix_boundary(),
            token(")")
        ])
        .should_expand(true)]
    )
}

/// Write one call argument entry.
pub(crate) fn write_call_argument_in_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    following_span_start: u32,
    separator: CallArgumentSeparator,
) -> FormatResult<()> {
    write_argument_with_following_span_start(f, argument_id, following_span_start)?;
    write_call_argument_separator(f, separator)?;

    Ok(())
}

/// Write one call-argument separator.
fn write_call_argument_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    separator: CallArgumentSeparator,
) -> FormatResult<()> {
    match separator {
        CallArgumentSeparator::None => {}
        CallArgumentSeparator::Always => {
            write!(f, [token(",")])?;
        }
        CallArgumentSeparator::IfGroupBreaks { group_id } => {
            write!(
                f,
                [if_group_breaks(&token(",")).with_group_id(Some(group_id))]
            )?;
        }
    }

    Ok(())
}

/// Write empty call arguments, preserving infix annotations.
pub(crate) fn write_empty_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let empty_argument_comments = empty_call_argument_comments(f.context(), call_node_id);

    if !empty_argument_comments.is_empty() {
        let comments = empty_argument_comments.as_slice();
        let dangling_comments = FormatDanglingComments::Comments {
            comments,
            indent: DanglingIndentMode::None,
        }
        .with_soft_block_indent();

        write!(f, [token("("), dangling_comments, token(")")])?;
    } else if f.context().has_infix_annotation(call_node_id) {
        let should_expand_multiline =
            empty_call_infix_requires_multiline(f.context(), call_node_id);
        if should_expand_multiline {
            write!(
                f,
                [
                    token("("),
                    block_indent(&block_infix_annotations(f.context(), call_node_id)),
                    token(")")
                ]
            )?;
        } else {
            write!(
                f,
                [
                    token("("),
                    block_infix_annotations(f.context(), call_node_id),
                    token(")")
                ]
            )?;
        }
    } else {
        write!(f, [token("("), token(")")])?;
    }

    Ok(())
}

/// Return comments that belong inside one empty call argument list.
fn empty_call_argument_comments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let call_span = context.span(call_node_id);
    let Some(open_parenthesis_start) =
        context.nth_token_type_start_in_span(call_span, TokenType::OpenParenthesis, 1)
    else {
        return Vec::new();
    };

    context
        .comments()
        .comments_in_range(open_parenthesis_start, call_span.end)
        .to_vec()
}

/// Write call arguments with the direct flat list layout.
pub(crate) fn write_simple_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _call_span: Span,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [token("(")])?;

    for (index, argument_id) in arguments.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [space()])?;
        }

        let following_span_start = arguments
            .get(index + 1)
            .map(|argument_id| f.context().span(*argument_id).start)
            .unwrap_or(0);
        let separator = if index + 1 != arguments.len() {
            CallArgumentSeparator::Always
        } else {
            CallArgumentSeparator::None
        };

        write_call_argument_in_list(f, argument_id, following_span_start, separator)?;
    }

    write!(f, [line_suffix_boundary(), token(")")])
}

/// Return whether one call argument list contains ignored ranges.
pub(crate) fn call_arguments_have_ignored_ranges(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if !context.has_ignore_directive_markers() {
        return false;
    }

    let comment_tokens = context.comment_tokens();
    any_ignore_range_for_nodes(context, arguments, comment_tokens)
}

/// Write call arguments with ignored ranges preserved as raw text.
pub(crate) fn write_ignored_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&separated_entries(
                ",",
                arguments,
                TrailingSeparator::Allowed,
                Some(group_id),
            )),
            line_suffix_boundary(),
            token(")")
        ])
        .with_id(Some(group_id))
        .should_expand(true)]
    )
}

/// Return whether empty call infix annotations should expand across lines.
fn empty_call_infix_requires_multiline(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    ctx.comments().has_comment_in_span(ctx.span(call_node_id))
        || ctx
            .annotation_ids(call_node_id)
            .iter()
            .any(|annotation_id| {
                if ctx.annotation(*annotation_id).position != DecoratorPosition::BlockInfix {
                    return false;
                }

                let annotation_span = ctx.annotation_span(*annotation_id);
                ctx.has_newline(annotation_span)
            })
}

/// Format call arguments with the default list formatter.
pub(crate) fn format_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _call_span: Span,
    group_id: GroupId,
    arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    disallow_trailing_separator: bool,
) -> FormatResult<()> {
    let trailing_separator = if disallow_trailing_separator {
        TrailingSeparator::Omit
    } else {
        match f.context().options.trailing_comma {
            TrailingComma::All => TrailingSeparator::Allowed,
            TrailingComma::Es5 | TrailingComma::None => TrailingSeparator::Omit,
        }
    };

    let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [
                token("("),
                soft_block_indent(&format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                    for (index, argument_id) in arguments.iter().copied().enumerate() {
                        if index > 0 {
                            write!(f, [soft_line_break_or_space()])?;
                        }

                        let following_span_start = arguments
                            .get(index + 1)
                            .map(|argument_id| f.context().span(*argument_id).start)
                            .unwrap_or(0);
                        let separator = if index + 1 != arguments.len() {
                            CallArgumentSeparator::Always
                        } else if trailing_separator == TrailingSeparator::Allowed {
                            CallArgumentSeparator::IfGroupBreaks { group_id }
                        } else if trailing_separator == TrailingSeparator::Mandatory {
                            CallArgumentSeparator::Always
                        } else {
                            CallArgumentSeparator::None
                        };

                        write_call_argument_in_list(
                            f,
                            argument_id,
                            following_span_start,
                            separator,
                        )?;
                    }

                    Ok(())
                })),
                line_suffix_boundary(),
                token(")")
            ]
        )
    });
    let interned = f.intern(&content)?;

    if let Some(element) = interned {
        let should_expand = force_expand || element.will_break();

        write!(
            f,
            [
                group(&format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                    f.write_node(element.clone());
                    Ok(())
                }))
                .with_id(Some(group_id))
                .should_expand(should_expand)
            ]
        )?;
    }

    Ok(())
}
