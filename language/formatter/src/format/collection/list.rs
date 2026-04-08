use crate::format::annotation::format_raw_comment;
use std::collections::HashMap;

use destack_fir::format::{FormatResult, GroupId};

use crate::format::directive::{ignore_ranges_for_nodes, write_ignored_span};
use crate::{DestackFormatContext, FormatNode};
use destack_ast::{Comment, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenType};
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
pub(crate) struct FormatSeparatedElement<T: Node> {
    element: LocalNodeId<T>,
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
        let mut separator_comment_start = None;

        if self.is_last {
            match self.trailing_separator {
                TrailingSeparator::Allowed => {
                    // source trailing separator comments require the separator shell
                    separator_comment_start = separator_comment_start_after_element(
                        f.context(),
                        element_span,
                        self.separator,
                    );

                    let trailing_comments = separator_comment_start
                        .map(|separator_start| {
                            raw_comments_after_separator_start(f.context(), separator_start)
                        })
                        .unwrap_or_default();
                    if trailing_comments.is_empty() {
                        separator_comment_start = None;
                        write!(
                            f,
                            [
                                if_group_breaks(&token(self.separator))
                                    .with_group_id(self.group_id)
                            ]
                        )?;
                    } else {
                        write!(f, [token(self.separator)])?;
                    }
                }
                TrailingSeparator::Mandatory => {
                    write!(f, [token(self.separator)])?;
                    separator_comment_start = separator_comment_start_after_element(
                        f.context(),
                        element_span,
                        self.separator,
                    );
                }
                TrailingSeparator::Omit => {}
            }
        } else {
            write!(f, [token(self.separator)])?;
            separator_comment_start =
                separator_comment_start_after_element(f.context(), element_span, self.separator);
        }

        if let Some(separator_comment_start) = separator_comment_start {
            let trailing_comments =
                raw_comments_after_separator_start(f.context(), separator_comment_start);
            for comment in trailing_comments {
                write!(f, [space()])?;
                format_raw_comment(f, comment)?;
            }
        }

        Ok(())
    }
}

/// Return the raw comment scan start after one source separator for a list element.
fn separator_comment_start_after_element(
    context: &DestackFormatContext<'_>,
    element_span: Span,
    separator: &str,
) -> Option<u32> {
    let separator_token_type = separator_token_type(separator)?;
    let last_token_in_element = context.last_non_trivia_token_in_span(element_span);
    if let Some(token) = last_token_in_element
        && token.token.ty == separator_token_type
    {
        return Some(token.span.end);
    }

    let separator_token = context.next_non_trivia_token_after_span(element_span)?;
    (separator_token.token.ty == separator_token_type).then_some(separator_token.span.end)
}

/// Return raw end-of-line comments that follow one separator token.
fn raw_comments_after_separator_start(
    context: &DestackFormatContext<'_>,
    separator_start: u32,
) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut start = separator_start;

    for comment in context.comments().comments_after(start).iter().copied() {
        if !context.range_contains_only_horizontal_whitespace(start, comment.span.start) {
            break;
        }

        comments.push(comment);
        if comment.is_line()
            || context.span_has_newline_before_next_non_whitespace_token(comment.span)
        {
            break;
        }

        start = comment.span.end;
    }

    comments
}

/// An iterator over formatted separated elements.
pub(crate) struct FormatSeparatedIter<I, T: Node> {
    next: Option<LocalNodeId<T>>,
    inner: I,
    separator: &'static str,
    trailing_separator: TrailingSeparator,
    group_id: Option<GroupId>,
}

impl<I, T> FormatSeparatedIter<I, T>
where
    T: Node,
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
    T: Node,
    I: Iterator<Item = LocalNodeId<T>>,
{
    type Item = FormatSeparatedElement<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let element = self.next.take().or_else(|| self.inner.next())?;
        self.next = self.inner.next();
        let is_last = self.next.is_none();

        Some(FormatSeparatedElement {
            element,
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

            if needs_separator && !starts_with_separator {
                write!(f, [token(separator), soft_line_break_or_space()])?;
            }

            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            needs_separator = !ends_with_separator;
            continue;
        }

        if needs_separator {
            write!(f, [token(separator), soft_line_break_or_space()])?;
        }

        write!(f, [*element_id])?;
        needs_separator = true;
    }

    Ok(needs_separator)
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
    let Some(separator_token) = separator_token_type(separator) else {
        return false;
    };

    context
        .last_non_trivia_token_in_span(range_span)
        .is_some_and(|token| token.token.ty == separator_token)
}
