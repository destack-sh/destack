use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::Comment;
use destack_fir::format::{Buffer, Format, FormatResult, Formatter, hard_line_break};
use destack_fir::prelude::{
    empty_line, expand_parent, format_with, line_suffix, soft_line_break_or_space, space, text,
};
use destack_fir::write;
use destack_source::Span;

/// Format one raw comment or documentation token.
fn format_comment_like_raw_text<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
    is_block_comment: bool,
) -> FormatResult<()> {
    let is_multiline_comment = raw_comment.contains('\n');

    // render one-line comments with trailing whitespace normalized
    if !is_multiline_comment {
        write!(f, [text(raw_comment.trim_end())])?;
        return Ok(());
    }

    // preserve star-aligned block comments in conventional form
    if is_block_comment && block_comment_is_alignable(raw_comment) {
        format_alignable_block_comment(f, raw_comment)?;
        return Ok(());
    }

    format_multiline_comment_raw(f, raw_comment)
}

/// Format one raw comment.
pub(crate) fn format_raw_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comment: Comment,
) -> FormatResult<()> {
    f.context().comments_mut().increment_printed_count();

    let raw_comment = f.context().comment_raw_text(comment);
    let is_block_comment = comment.is_block();
    format_comment_like_raw_text(f, raw_comment, is_block_comment)
}

/// Write raw leading comments with source-preserving separators.
pub(crate) fn write_raw_leading_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    let source = f.context().source_text();
    let mut comments = comments.iter().copied().peekable();

    while let Some(comment) = comments.next() {
        format_raw_comment(f, comment)?;

        if comment.is_block() {
            match source.lines_after(comment.span.end) {
                0 => {
                    let should_nestle = comments.peek().is_some_and(|next_comment| {
                        should_nestle_adjacent_doc_comments(comment, *next_comment)
                    });

                    if !should_nestle {
                        write!(f, [space()])?;
                    }
                }
                1 => {
                    let lines_before = {
                        let comment_cursor = f.context().comments();
                        source.get_lines_before(comment.span, &comment_cursor)
                    };

                    if lines_before == 0 {
                        write!(f, [soft_line_break_or_space()])?;
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                _ => {
                    write!(f, [empty_line()])?;
                }
            }
        } else {
            match source.lines_after(comment.span.end) {
                0 | 1 => {
                    write!(f, [hard_line_break()])?;
                }
                _ => {
                    write!(f, [empty_line()])?;
                }
            }
        }
    }

    Ok(())
}

/// Write raw trailing comments with upstream-shaped line-suffix behavior.
pub(crate) fn write_raw_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    write_raw_trailing_comments_with_options(f, comments, true)
}

/// Write raw trailing comments without expanding the parent group for line comments.
pub(crate) fn write_raw_trailing_comments_without_parent_expansion<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    write_raw_trailing_comments_with_options(f, comments, false)
}

/// Write raw trailing comments with configurable parent expansion for line comments.
fn write_raw_trailing_comments_with_options<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
    expand_parent_for_line_comments: bool,
) -> FormatResult<()> {
    let source = f.context().source_text();
    let mut total_lines_before = 0usize;
    let mut previous_comment = None;

    for comment in comments.iter().copied() {
        f.context().comments_mut().increment_printed_count();

        let raw_comment = f.context().comment_raw_text(comment);
        let is_block_comment = comment.is_block();
        let lines_before = {
            let comment_cursor = f.context().comments();
            source.get_lines_before(comment.span, &comment_cursor)
        };
        total_lines_before += lines_before;

        let should_nestle = previous_comment.is_some_and(|previous_comment| {
            should_nestle_adjacent_doc_comments(previous_comment, comment)
        });

        if total_lines_before > 0 || previous_comment.is_some_and(Comment::is_line) {
            write!(
                f,
                [line_suffix(&format_with(
                    move |f: &mut DestackFormatter<'ast, '_>| {
                        match lines_before {
                            _ if should_nestle => {}
                            0 => {
                                if previous_comment.is_some_and(Comment::is_line) {
                                    write!(f, [hard_line_break()])?;
                                } else {
                                    write!(f, [space()])?;
                                }
                            }
                            1 => {
                                write!(f, [hard_line_break()])?;
                            }
                            _ => {
                                write!(f, [empty_line()])?;
                            }
                        }

                        format_comment_like_raw_text(f, raw_comment, is_block_comment)
                    }
                ))]
            )?;
        } else {
            let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                if !should_nestle {
                    write!(f, [space()])?;
                }

                format_comment_like_raw_text(f, raw_comment, is_block_comment)
            });

            if comment.is_line() {
                if expand_parent_for_line_comments {
                    write!(f, [line_suffix(&content), expand_parent()])?;
                } else {
                    write!(f, [line_suffix(&content)])?;
                }
            } else {
                write!(f, [content])?;
            }
        }

        previous_comment = Some(comment);
    }

    Ok(())
}

/// Write raw comments with direct source-preserving separators.
pub(crate) fn write_raw_comment_slice<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    let source = f.context().source_text();
    let mut previous_comment = None;

    for comment in comments.iter().copied() {
        f.context().comments_mut().increment_printed_count();

        let raw_comment = f.context().comment_raw_text(comment);
        let is_block_comment = comment.is_block();
        let lines_before = {
            let comment_cursor = f.context().comments();
            source.get_lines_before(comment.span, &comment_cursor)
        };
        let should_nestle = previous_comment.is_some_and(|previous_comment| {
            should_nestle_adjacent_doc_comments(previous_comment, comment)
        });

        match lines_before {
            _ if should_nestle => {}
            0 => {
                if previous_comment.is_some_and(Comment::is_line) {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [space()])?;
                }
            }
            1 => {
                write!(f, [hard_line_break()])?;
            }
            _ => {
                write!(f, [empty_line()])?;
            }
        }

        format_comment_like_raw_text(f, raw_comment, is_block_comment)?;
        previous_comment = Some(comment);
    }

    Ok(())
}

/// Return one trailing comment formatter for one node relationship.
#[inline]
pub(crate) const fn format_trailing_comments(
    enclosing_span: Span,
    preceding_span: Span,
    following_span_start: u32,
) -> FormatTrailingComments<'static> {
    FormatTrailingComments::Node((
        enclosing_span,
        preceding_span,
        following_span_start,
        following_span_start,
    ))
}

/// Return one trailing comment formatter that stops before one explicit boundary.
#[inline]
pub(crate) const fn format_trailing_comments_before_boundary(
    enclosing_span: Span,
    preceding_span: Span,
    boundary_start: u32,
    following_span_start: u32,
) -> FormatTrailingComments<'static> {
    FormatTrailingComments::Node((
        enclosing_span,
        preceding_span,
        boundary_start,
        following_span_start,
    ))
}

/// Return one trailing comment formatter for one explicit comment slice.
#[inline]
pub(crate) const fn format_trailing_comment_slice(
    comments: &[Comment],
) -> FormatTrailingComments<'_> {
    FormatTrailingComments::Comments(comments)
}

/// Format trailing comments for one node relationship.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FormatTrailingComments<'a> {
    /// The enclosing span, preceding span, boundary start, and following sibling start.
    Node((Span, Span, u32, u32)),
    /// One explicit trailing comment slice.
    Comments(&'a [Comment]),
}

impl<'a> Format<DestackFormatContext<'a>> for FormatTrailingComments<'_> {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'a>>) -> FormatResult<()> {
        match self {
            Self::Node((enclosing_span, preceding_span, boundary_start, following_span_start)) => {
                let comments = {
                    let comments = f.context().comments();
                    comments
                        .get_trailing_comments(
                            *enclosing_span,
                            *preceding_span,
                            *boundary_start,
                            *following_span_start,
                        )
                        .to_vec()
                };

                if comments.is_empty() {
                    return Ok(());
                }

                write_raw_trailing_comments(f, &comments)
            }
            Self::Comments(comments) => {
                if comments.is_empty() {
                    return Ok(());
                }

                write_raw_trailing_comments(f, comments)
            }
        }
    }
}

/// Format one multiline raw comment with explicit line breaks.
fn format_multiline_comment_raw<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
) -> FormatResult<()> {
    let mut lines = raw_comment.lines();
    let Some(first_line) = lines.next() else {
        return Ok(());
    };

    write!(f, [text(first_line.trim_end_matches('\r').trim_end())])?;

    let remaining_lines = lines.collect::<Vec<_>>();
    let common_indent = common_multiline_comment_indent(&remaining_lines);
    for line in remaining_lines {
        let line = line.trim_end_matches('\r');
        let line = if common_indent == 0 {
            line
        } else {
            let mut end_index = 0;
            for byte in line.as_bytes().iter().take(common_indent) {
                if !matches!(*byte, b' ' | b'\t') {
                    break;
                }

                end_index += 1;
            }

            &line[end_index..]
        };
        write!(f, [hard_line_break(), text(line)])?;
    }

    Ok(())
}

/// Return whether two adjacent doc comments should stay nestled together.
fn should_nestle_adjacent_doc_comments(current: Comment, next: Comment) -> bool {
    current.is_jsdoc()
        && next.is_jsdoc()
        && current.is_multiline_block()
        && next.is_multiline_block()
        && current.span.end == next.span.start
}

/// Return the common leading indentation width for multiline comment lines.
fn common_multiline_comment_indent(lines: &[&str]) -> usize {
    let mut common_indent = usize::MAX;

    for line in lines {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        let line_indent = line
            .as_bytes()
            .iter()
            .take_while(|byte| matches!(**byte, b' ' | b'\t'))
            .count();
        common_indent = common_indent.min(line_indent);
    }

    if common_indent == usize::MAX {
        return 0;
    }

    common_indent
}

/// Format one alignable multiline block comment.
fn format_alignable_block_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
) -> FormatResult<()> {
    let mut lines = raw_comment.lines();
    let Some(first_line) = lines.next() else {
        return Ok(());
    };

    write!(f, [text(first_line.trim_end_matches('\r').trim_end())])?;
    for line in lines {
        let trimmed_line = line.trim_end_matches('\r').trim();
        let normalized_line = if let Some(prefix) = trimmed_line.strip_suffix("*/") {
            let prefix = prefix.trim_end();
            if prefix.is_empty() {
                "*/".to_string()
            } else {
                format!("{prefix} */")
            }
        } else {
            trimmed_line.to_string()
        };
        write!(f, [hard_line_break(), space(), text(&normalized_line)])?;
    }

    Ok(())
}

/// Return whether one multiline block comment is alignable on `*` prefixes.
fn block_comment_is_alignable(raw_comment: &str) -> bool {
    raw_comment
        .lines()
        .skip(1)
        .all(|line| line.trim_start_matches('\r').trim_start().starts_with('*'))
}
