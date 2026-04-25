use std::collections::HashMap;

use crate::format::annotation::{FormatTrailingComments, write_comment_slice};
use crate::format::file::{ignore_ranges_for_nodes, write_ignored_span};
use crate::{DestackFormatContext, FormatNode};
use destack_ast::{Comment, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenSpan, TokenType};
use destack_fir::format::{FormatResult, GroupId};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

/// The trailing separator mode for one separated entry list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrailingSeparator {
    /// Omit the trailing separator.
    Omit,
    /// Allow the trailing separator when the group breaks.
    Allowed,
    /// Require the trailing separator.
    Mandatory,
}

/// One formatted entry in a separated list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FormatSeparatedElement<T: Node + Clone> {
    element: LocalNodeId<T>,
    next_element: Option<LocalNodeId<T>>,
    is_last: bool,
    separator: &'static str,
    trailing_separator: TrailingSeparator,
    group_id: Option<GroupId>,
}

impl<'ast, T> Format<DestackFormatContext<'ast>> for FormatSeparatedElement<T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T>,
{
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        write!(f, [self.element])?;

        let element_span = f.context().span(self.element);
        let element_anchor_end = f
            .context()
            .last_non_trivia_token_in_span(element_span)
            .map_or(element_span.end, |token| token.span.end);
        let next_element_start = self.next_element.map(|next_element| {
            let next_element_span = f.context().span(next_element);

            f.context()
                .first_non_trivia_token_in_span(next_element_span)
                .map_or(next_element_span.start, |token| token.span.start)
        });
        let source_separator = separator_token_after_element(
            f.context(),
            element_span,
            next_element_start,
            self.separator,
        );
        let next_following_start = next_element_start
            .or_else(|| {
                source_separator.and_then(|separator| {
                    f.context()
                        .next_non_trivia_token_after_span(separator.span)
                        .map(|token| token.span.start)
                })
            })
            .or_else(|| {
                f.context()
                    .next_non_trivia_token_after_span(element_span)
                    .map(|token| token.span.start)
            })
            .unwrap_or(element_span.end);
        let following_start =
            list_element_following_start(f.context(), source_separator, next_following_start);
        let gap_comments =
            gap_comments_after_element(f.context(), element_anchor_end, following_start);
        let element_tail_comments =
            element_tail_comments(f.context(), element_anchor_end, element_span);
        let (comments_before_separator, comments_after_separator) =
            split_gap_comments_around_separator(&gap_comments, source_separator);

        if !gap_comments.is_empty() {
            let leading_comments = if source_separator.is_some() {
                comments_before_separator
            } else {
                &[][..]
            };
            let trailing_comments = if source_separator.is_some() {
                comments_after_separator
            } else {
                gap_comments.as_slice()
            };
            let separator_trailing_comment_count =
                separator_trailing_comment_count(f.context(), trailing_comments);
            let trailing_comments = &trailing_comments[..separator_trailing_comment_count];

            if !leading_comments.is_empty() {
                write_comment_slice(f, leading_comments)?;
            }

            let separator_precedes_trailing_comments = source_separator.is_some()
                || trailing_comments
                    .first()
                    .is_some_and(|comment| comment.is_line());

            if separator_precedes_trailing_comments {
                if self.is_last && source_separator.is_some() {
                    write_immediate_trailing_separator(
                        f,
                        self.separator,
                        self.trailing_separator,
                        self.group_id,
                    )?;
                } else {
                    write_separator_token(
                        f,
                        self.separator,
                        self.is_last,
                        self.trailing_separator,
                        self.group_id,
                    )?;
                }

                if !trailing_comments.is_empty() {
                    write!(f, [FormatTrailingComments::Comments(trailing_comments)])?;
                }
            } else {
                write_comment_slice(f, trailing_comments)?;

                if self.is_last && source_separator.is_none() {
                    write_immediate_trailing_separator(
                        f,
                        self.separator,
                        self.trailing_separator,
                        self.group_id,
                    )?;
                } else {
                    write_separator_token(
                        f,
                        self.separator,
                        self.is_last,
                        self.trailing_separator,
                        self.group_id,
                    )?;
                }
            }

            return Ok(());
        }

        if self.is_last && source_separator.is_none() && !element_tail_comments.is_empty() {
            write_immediate_trailing_separator(
                f,
                self.separator,
                self.trailing_separator,
                self.group_id,
            )?;
        } else {
            write_separator_token(
                f,
                self.separator,
                self.is_last,
                self.trailing_separator,
                self.group_id,
            )?;
        }

        Ok(())
    }
}

/// Return the next non-trivia start after one list element's separator, when present.
fn list_element_following_start(
    context: &DestackFormatContext<'_>,
    source_separator: Option<TokenSpan>,
    next_following_start: u32,
) -> u32 {
    let Some(source_separator) = source_separator else {
        return next_following_start;
    };

    context
        .next_non_trivia_token_after_span(source_separator.span)
        .map_or(source_separator.span.end, |token| token.span.start)
}

/// Write one separator token according to list position and trailing-separator mode.
fn write_separator_token<'ast>(
    f: &mut Formatter<'_, DestackFormatContext<'ast>>,
    separator: &'static str,
    is_last: bool,
    trailing_separator: TrailingSeparator,
    group_id: Option<GroupId>,
) -> FormatResult<()> {
    if is_last {
        match trailing_separator {
            TrailingSeparator::Allowed => {
                write!(
                    f,
                    [if_group_breaks(&token(separator)).with_group_id(group_id)]
                )?;
            }
            TrailingSeparator::Mandatory => {
                write!(f, [token(separator)])?;
            }
            TrailingSeparator::Omit => {}
        }
    } else {
        write!(f, [token(separator)])?;
    }

    Ok(())
}

/// Write one trailing separator immediately after same-line trailing comments.
fn write_immediate_trailing_separator<'ast>(
    f: &mut Formatter<'_, DestackFormatContext<'ast>>,
    separator: &'static str,
    trailing_separator: TrailingSeparator,
    group_id: Option<GroupId>,
) -> FormatResult<()> {
    match trailing_separator {
        TrailingSeparator::Allowed => {
            write!(
                f,
                [if_group_breaks(&token(separator)).with_group_id(group_id)]
            )?;
        }
        TrailingSeparator::Mandatory => {
            write!(f, [token(separator)])?;
        }
        TrailingSeparator::Omit => {}
    }

    Ok(())
}

/// Return one source separator token after a list element, if present.
fn separator_token_after_element(
    context: &DestackFormatContext<'_>,
    element_span: Span,
    next_element_start: Option<u32>,
    separator: &str,
) -> Option<TokenSpan> {
    let separator_token_type = separator_token_type(separator)?;
    let last_token_in_element = context.last_non_trivia_token_in_span(element_span);
    if let Some(token) = last_token_in_element
        && token.token.ty == separator_token_type
    {
        return Some(token);
    }

    let separator_token = context.next_non_trivia_token_after_span(element_span)?;
    if separator_token.token.ty != separator_token_type {
        return None;
    }

    if let Some(next_element_start) = next_element_start
        && separator_token.span.end > next_element_start
    {
        return None;
    }

    Some(separator_token)
}

/// Return comments between one element and the next following token start.
fn gap_comments_after_element(
    context: &DestackFormatContext<'_>,
    gap_start: u32,
    gap_end: u32,
) -> Vec<Comment> {
    if gap_start >= gap_end {
        return Vec::new();
    }

    context
        .comments()
        .comments_in_range(gap_start, gap_end)
        .to_vec()
}

/// Return comments that still sit inside one element tail.
fn element_tail_comments(
    context: &DestackFormatContext<'_>,
    anchor_end: u32,
    element_span: Span,
) -> Vec<Comment> {
    if anchor_end >= element_span.end {
        return Vec::new();
    }

    context
        .comments()
        .comments_in_range(anchor_end, element_span.end)
        .to_vec()
}

/// Split one gap comment slice around one source separator token.
fn split_gap_comments_around_separator(
    comments: &[Comment],
    separator_token: Option<TokenSpan>,
) -> (&[Comment], &[Comment]) {
    let Some(separator_token) = separator_token else {
        return (&[][..], comments);
    };

    let split_index = comments
        .iter()
        .position(|comment| comment.span.start >= separator_token.span.end)
        .unwrap_or(comments.len());

    comments.split_at(split_index)
}

/// Return the trailing comment count that stays with one separator line.
fn separator_trailing_comment_count(
    context: &DestackFormatContext<'_>,
    comments: &[Comment],
) -> usize {
    let mut count = 0usize;

    for comment in comments.iter().copied() {
        count += 1;

        if comment.is_line()
            || context.span_has_newline_before_next_non_whitespace_token(comment.span)
        {
            break;
        }
    }

    count
}

/// An iterator over formatted separated elements.
pub(crate) struct FormatSeparatedIter<I, T: Node + Clone> {
    next: Option<LocalNodeId<T>>,
    inner: I,
    separator: &'static str,
    trailing_separator: TrailingSeparator,
    group_id: Option<GroupId>,
}

impl<I, T> FormatSeparatedIter<I, T>
where
    T: Node + Clone,
    I: Iterator<Item = LocalNodeId<T>>,
{
    /// Create a new separated iterator for one element stream.
    pub(crate) fn new(inner: I, separator: &'static str) -> Self {
        Self {
            next: None,
            inner,
            separator,
            trailing_separator: TrailingSeparator::Omit,
            group_id: None,
        }
    }

    /// Set the trailing separator mode.
    pub(crate) fn with_trailing_separator(mut self, trailing_separator: TrailingSeparator) -> Self {
        self.trailing_separator = trailing_separator;
        self
    }

    /// Set the group id used by conditional trailing separators.
    pub(crate) fn with_group_id(mut self, group_id: Option<GroupId>) -> Self {
        self.group_id = group_id;
        self
    }
}

impl<I, T> Iterator for FormatSeparatedIter<I, T>
where
    T: Node + Clone,
    I: Iterator<Item = LocalNodeId<T>>,
{
    type Item = FormatSeparatedElement<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let element = self.next.take().or_else(|| self.inner.next())?;
        self.next = self.inner.next();
        let is_last = self.next.is_none();

        Some(FormatSeparatedElement {
            element,
            next_element: self.next,
            is_last,
            separator: self.separator,
            trailing_separator: self.trailing_separator,
            group_id: self.group_id,
        })
    }
}

/// Format separated entries with optional trailing separator handling.
pub(crate) fn separated_entries<'ast, 'e, T>(
    separator: &'static str,
    elements: &'e [LocalNodeId<T>],
    trailing_separator: TrailingSeparator,
    group_id: Option<GroupId>,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T>,
{
    format_with(move |f: &mut Formatter<'_, DestackFormatContext<'ast>>| {
        let has_elements = !elements.is_empty();

        if !has_elements {
            return Ok(());
        }

        let ignore_ranges_by_id = if f.context().has_ignore_directive_markers() {
            let comment_tokens = f.context().comment_tokens();
            let ignore_ranges_by_id =
                ignore_ranges_for_nodes(f.context(), elements, comment_tokens);
            if ignore_ranges_by_id.is_empty() {
                None
            } else {
                Some(ignore_ranges_by_id)
            }
        } else {
            None
        };
        let has_ignore_ranges = ignore_ranges_by_id.is_some();
        let mut needs_trailing_separator = true;
        if let Some(ignore_ranges_by_id) = ignore_ranges_by_id.as_ref() {
            needs_trailing_separator =
                format_list_with_ignored_ranges(f, elements, ignore_ranges_by_id, separator)?;
        } else {
            let entries = FormatSeparatedIter::new(elements.iter().copied(), separator)
                .with_trailing_separator(trailing_separator)
                .with_group_id(group_id);

            f.join_with(&soft_line_break_or_space())
                .entries(entries)
                .finish()?;
        }

        if has_ignore_ranges && needs_trailing_separator {
            match trailing_separator {
                TrailingSeparator::Allowed => {
                    write!(
                        f,
                        [if_group_breaks(&token(separator)).with_group_id(group_id)]
                    )?;
                }
                TrailingSeparator::Mandatory => {
                    write!(f, [token(separator)])?;
                }
                TrailingSeparator::Omit => {}
            }
        }

        Ok(())
    })
}

/// Format a list while preserving any ignore ranges as raw text.
fn format_list_with_ignored_ranges<'ast, T>(
    f: &mut Formatter<'_, DestackFormatContext<'ast>>,
    elements: &[LocalNodeId<T>],
    ignore_ranges: &HashMap<u32, Span>,
    separator: &'static str,
) -> FormatResult<bool>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T>,
{
    let mut skip_until: Option<u32> = None;
    let mut needs_separator = false;

    for element_id in elements {
        let element_span = f.context().span(*element_id);
        if let Some(skip_end) = skip_until {
            if element_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        if let Some(range_span) = ignore_ranges.get(&element_id.id) {
            let starts_with_separator =
                ignored_range_starts_with_separator(f.context(), *range_span, separator);
            let ends_with_separator =
                ignored_range_ends_with_separator(f.context(), *range_span, separator);
            let has_following_element = elements
                .iter()
                .copied()
                .any(|next_element_id| f.context().span(next_element_id).start >= range_span.end);

            if needs_separator && !starts_with_separator {
                write!(f, [token(separator), soft_line_break_or_space()])?;
            }

            write_ignored_span(f, *range_span)?;
            if ends_with_separator && has_following_element {
                write!(f, [soft_line_break_or_space()])?;
            }

            skip_until = Some(range_span.end);
            needs_separator = !ends_with_separator;
            continue;
        }

        if needs_separator {
            write!(f, [token(separator), soft_line_break_or_space()])?;
        }

        let following_span_start =
            list_element_following_span_start(f.context(), elements, element_id);
        let previous_following_span_start = f
            .context_mut()
            .replace_following_span_start(following_span_start);
        let result = write!(f, [*element_id]);
        f.context_mut()
            .replace_following_span_start(previous_following_span_start);
        result?;

        needs_separator = true;
    }

    Ok(needs_separator)
}

/// Return the source start of the next list element after one element.
fn list_element_following_span_start<T>(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<T>],
    element_id: &LocalNodeId<T>,
) -> u32
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
{
    let element_span = context.span(*element_id);
    let next_element = elements
        .iter()
        .copied()
        .find(|next_element_id| context.span(*next_element_id).start > element_span.start);

    next_element.map_or(0, |next_element_id| {
        let next_span = context.span(next_element_id);

        context
            .first_non_trivia_token_in_span(next_span)
            .map_or(next_span.start, |token| token.span.start)
    })
}

/// Return the token type used for one list separator string.
fn separator_token_type(separator: &str) -> Option<TokenType> {
    match separator {
        "," => Some(TokenType::Comma),
        ";" => Some(TokenType::Semicolon),
        ":" => Some(TokenType::Colon),
        "|" => Some(TokenType::ElementwiseOr),
        "&" => Some(TokenType::ElementwiseAnd),
        _ => None,
    }
}

/// Return whether one ignored range starts with a separator token.
fn ignored_range_starts_with_separator(
    context: &DestackFormatContext<'_>,
    range_span: Span,
    separator: &str,
) -> bool {
    let Some(separator_token) = separator_token_type(separator) else {
        return false;
    };

    context
        .first_non_trivia_token_in_span(range_span)
        .is_some_and(|token| token.token.ty == separator_token)
}

/// Return whether one ignored range ends with a separator token.
fn ignored_range_ends_with_separator(
    context: &DestackFormatContext<'_>,
    range_span: Span,
    separator: &str,
) -> bool {
    if ignored_range_ends_with_comment(context, range_span) {
        return true;
    }

    let Some(separator_token) = separator_token_type(separator) else {
        return false;
    };

    context
        .last_non_trivia_token_in_span(range_span)
        .is_some_and(|token| token.token.ty == separator_token)
}

/// Return whether one ignored range already owns a trailing comment.
fn ignored_range_ends_with_comment(context: &DestackFormatContext<'_>, range_span: Span) -> bool {
    let comment_tokens = context.comment_tokens_in_range(range_span.start, range_span.end);
    let Some(comment_token) = comment_tokens.last().copied() else {
        return false;
    };

    context
        .source_text()
        .all_bytes_match(comment_token.span.end, range_span.end, |byte| {
            byte.is_ascii_whitespace()
        })
}
