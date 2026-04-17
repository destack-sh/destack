use super::argument::{write_call_argument_node_body, write_grouped_call_argument};
use crate::format::annotation::{
    block_infix_annotations, format_raw_comment, format_trailing_comments,
    write_raw_leading_comments,
};
use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::format::context::DestackFormatterCommentExt;
use crate::format::declaration::GroupedCallArgumentLayout;
use crate::format::file::any_ignore_range_for_nodes;
use crate::{Decorator, DestackFormatContext, DestackFormatter};
use destack_ast::{Argument, DecoratorPosition, Expression, LocalNodeId, TokenType};
use destack_fir::format::{Buffer, FormatNodes, FormatResult, GroupId};
use destack_fir::prelude::{
    block_indent, empty_line, format_with, group, hard_line_break, if_group_breaks,
    soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};

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

        context
            .source_line_distance(current_span.end, next_span.start)
            .is_some_and(|line_distance| line_distance > 1)
    })
}

/// Return the number of source lines before one call argument, including owned comments.
pub(crate) fn call_argument_lines_before(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> usize {
    let argument_span = context.span(argument_id);
    let comments = context.comments();

    context
        .source_text()
        .get_lines_before(argument_span, &comments)
}

/// Format all call arguments in explicit broken-out layout.
pub(crate) fn format_all_args_broken_out<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_span: destack_source::Span,
    arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
    disallow_trailing_separator: bool,
) -> FormatResult<()> {
    let write_trailing_separator = !disallow_trailing_separator
        && matches!(
            f.context().options.trailing_comma,
            destack_workspace::TrailingComma::All
        );

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
                    write_call_argument_in_list(
                        f,
                        call_span,
                        argument_id,
                        following_span_start,
                        None,
                        index == 0,
                    )?;

                    if index + 1 != arguments.len() {
                        write!(f, [token(",")])?;
                    }
                }

                if write_trailing_separator {
                    write!(f, [token(",")])?;
                }

                Ok(())
            })),
            token(")")
        ])
        .with_id(Some(group_id))
        .should_expand(true)]
    )
}

/// Format arguments for one long curried call.
pub(crate) fn format_long_curried_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_span: destack_source::Span,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let write_trailing_separator = matches!(
        f.context().options.trailing_comma,
        destack_workspace::TrailingComma::All
    );

    write!(
        f,
        [
            token("("),
            soft_block_indent(&format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                for (index, argument_id) in arguments.iter().copied().enumerate() {
                    if index > 0 {
                        write!(f, [token(","), soft_line_break_or_space()])?;
                    }

                    let following_span_start = arguments
                        .get(index + 1)
                        .map(|argument_id| f.context().span(*argument_id).start)
                        .unwrap_or(0);
                    write_call_argument_in_list(
                        f,
                        call_span,
                        argument_id,
                        following_span_start,
                        None,
                        index == 0,
                    )?;
                }

                if write_trailing_separator {
                    write!(f, [token(",")])?;
                }

                Ok(())
            })),
            token(")")
        ]
    )
}

/// Write one call argument entry with list-level trailing comment ownership.
pub(crate) fn write_call_argument_in_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_span: destack_source::Span,
    argument_id: LocalNodeId<Argument>,
    following_span_start: u32,
    grouped_call_argument_layout: Option<GroupedCallArgumentLayout>,
    emit_leading_comments: bool,
) -> FormatResult<()> {
    if emit_leading_comments {
        write_first_call_argument_leading_comments(f, call_span, argument_id)?;
    }

    match grouped_call_argument_layout {
        Some(grouped_call_argument_layout) => {
            write_grouped_call_argument(f, argument_id, grouped_call_argument_layout)?;
        }
        None => {
            write_call_argument_node_body(f, argument_id)?;
        }
    }

    write_call_argument_trailing_comments(f, call_span, argument_id, following_span_start)
}

/// Write raw comments that belong before the first call argument.
fn write_first_call_argument_leading_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_span: destack_source::Span,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    let Some(open_parenthesis_start) =
        f.context()
            .nth_token_type_start_in_span(call_span, TokenType::OpenParenthesis, 1)
    else {
        return Ok(());
    };
    let open_parenthesis_span = destack_source::Span::new(
        call_span.file,
        open_parenthesis_start,
        open_parenthesis_start + 1,
    );
    let comment_start = f
        .context()
        .previous_non_trivia_token_before_span(open_parenthesis_span)
        .map_or(call_span.start, |token| token.span.end);

    let argument_span = f.context().span(argument_id);
    let leading_comments = {
        let comments = f.context().comments();
        comments
            .comments_in_range(comment_start, argument_span.start)
            .to_vec()
    };

    if leading_comments.is_empty() {
        return Ok(());
    }

    write_raw_leading_comments(f, &leading_comments)
}

/// Write trailing raw comments owned by one call argument.
fn write_call_argument_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_span: destack_source::Span,
    argument_id: LocalNodeId<Argument>,
    following_span_start: u32,
) -> FormatResult<()> {
    let argument_span = f.context().span(argument_id);
    write!(
        f,
        [format_trailing_comments(
            call_span,
            argument_span,
            following_span_start,
        )]
    )
}

/// Write empty call arguments, preserving infix annotations.
pub(crate) fn write_empty_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let empty_argument_comments = empty_call_argument_comments(f.context(), call_node_id);

    if !empty_argument_comments.is_empty() {
        let should_expand_multiline = empty_argument_comments.iter().any(|comment| {
            comment.is_line() || comment.preceded_by_newline() || comment.followed_by_newline()
        });

        if should_expand_multiline {
            write!(
                f,
                [
                    token("("),
                    block_indent(&format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                        write_empty_call_argument_comments(f, &empty_argument_comments)
                    })),
                    token(")")
                ]
            )?;
        } else {
            write!(
                f,
                [
                    token("("),
                    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                        write_empty_call_argument_comments(f, &empty_argument_comments)
                    }),
                    token(")")
                ]
            )?;
        }
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

/// Return raw comments that belong inside one empty call argument list.
fn empty_call_argument_comments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> Vec<destack_ast::Comment> {
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

/// Write raw comments that belong inside one empty call argument list.
fn write_empty_call_argument_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[destack_ast::Comment],
) -> FormatResult<()> {
    if comments.is_empty() {
        return Ok(());
    }

    let source = f.context().source_text();

    for (index, comment) in comments.iter().copied().enumerate() {
        format_raw_comment(f, comment)?;

        let Some(next_comment) = comments.get(index + 1).copied() else {
            continue;
        };

        let lines_after = source.lines_after(comment.span.end);
        let lines_before = {
            let comment_cursor = f.context().comments();
            source.get_lines_before(next_comment.span, &comment_cursor)
        };

        if lines_before > 1 || lines_after > 1 {
            write!(f, [empty_line()])?;
        } else if comment.is_line()
            || next_comment.preceded_by_newline()
            || comment.followed_by_newline()
            || lines_after == 1
        {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Write call arguments with the direct flat list layout.
pub(crate) fn write_simple_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_span: destack_source::Span,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [token("(")])?;

    for (index, argument_id) in arguments.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        let following_span_start = arguments
            .get(index + 1)
            .map(|argument_id| f.context().span(*argument_id).start)
            .unwrap_or(0);
        write_call_argument_in_list(
            f,
            call_span,
            argument_id,
            following_span_start,
            None,
            index == 0,
        )?;
    }

    write!(f, [token(")")])
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
    let has_raw_comment_in_call = {
        let call_span = ctx.span(call_node_id);
        let comments = ctx.comments();
        comments.has_comment_in_span(call_span)
    };

    has_raw_comment_in_call
        || ctx
            .annotation_ids(call_node_id)
            .iter()
            .any(|annotation_id| {
                let annotation = ctx.annotation(*annotation_id);
                if annotation.position != DecoratorPosition::BlockInfix {
                    return false;
                }

                let annotation_span = ctx.annotation_span(*annotation_id);
                if ctx.has_newline(annotation_span) {
                    return true;
                }

                match annotation {
                    Decorator { .. } => false,
                }
            })
}

/// Format call arguments with the default list formatter.
pub(crate) fn format_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_span: destack_source::Span,
    group_id: GroupId,
    arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    disallow_trailing_separator: bool,
) -> FormatResult<()> {
    let trailing_separator = if disallow_trailing_separator {
        TrailingSeparator::Omit
    } else {
        match f.context().options.trailing_comma {
            destack_workspace::TrailingComma::All => TrailingSeparator::Allowed,
            destack_workspace::TrailingComma::Es5 | destack_workspace::TrailingComma::None => {
                TrailingSeparator::Omit
            }
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
                            write!(f, [token(","), soft_line_break_or_space()])?;
                        }

                        let following_span_start = arguments
                            .get(index + 1)
                            .map(|argument_id| f.context().span(*argument_id).start)
                            .unwrap_or(0);
                        write_call_argument_in_list(
                            f,
                            call_span,
                            argument_id,
                            following_span_start,
                            None,
                            index == 0,
                        )?;
                    }

                    match trailing_separator {
                        TrailingSeparator::Allowed => {
                            write!(
                                f,
                                [if_group_breaks(&token(",")).with_group_id(Some(group_id))]
                            )?;
                        }
                        TrailingSeparator::Mandatory => {
                            write!(f, [token(",")])?;
                        }
                        TrailingSeparator::Omit => {}
                    }

                    Ok(())
                })),
                token(")")
            ]
        )
    });
    let interned = f.intern_with_comment_snapshot_after(Some(call_span.start), &content)?;

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
