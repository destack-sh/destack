use crate::{Annotation, DestackFormatContext};
use destack_ast::{LocalNodeId, TokenSpan, TokenType};
use destack_source::Span;

/// Return whether one token type is ignorable trivia for span-adjacent scans.
#[inline]
fn is_ignored_span_neighbor_token(token_type: TokenType) -> bool {
    matches!(token_type, TokenType::Whitespace | TokenType::Newline)
}

/// Return the nearest non-whitespace token before one span.
pub(crate) fn previous_non_whitespace_token_before_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<TokenSpan> {
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.end <= span.start);

    while index > 0 {
        index -= 1;
        let token = tokens[index];
        if is_ignored_span_neighbor_token(token.token.ty) {
            continue;
        }

        return Some(token);
    }

    None
}

/// Return the nearest non-whitespace token after one span.
pub(crate) fn next_non_whitespace_token_after_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<TokenSpan> {
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.start < span.end);

    while let Some(token) = tokens.get(index).copied() {
        if is_ignored_span_neighbor_token(token.token.ty) {
            index += 1;
            continue;
        }

        return Some(token);
    }

    None
}

/// Return the nearest non-whitespace token before one annotation span.
pub(crate) fn previous_non_whitespace_token_before_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<TokenSpan> {
    let span = context.get_annotation_span(annotation_id);
    previous_non_whitespace_token_before_span(context, span)
}

/// Return the nearest non-whitespace token after one annotation span.
pub(crate) fn next_non_whitespace_token_after_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<TokenSpan> {
    let span = context.get_annotation_span(annotation_id);
    next_non_whitespace_token_after_span(context, span)
}

/// Return the first non-whitespace character before a span.
pub(crate) fn previous_non_whitespace_before_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<char> {
    if span.start == 0 {
        return None;
    }

    let head_span = Span::new(span.file, 0, span.start);
    let head_source = context.file.get_span_str(head_span)?;
    head_source
        .chars()
        .rev()
        .find(|character: &char| !character.is_whitespace())
}

/// Return the first non-whitespace character after a span.
pub(crate) fn next_non_whitespace_after_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<char> {
    if span.end >= context.file.len {
        return None;
    }

    let tail_span = Span::new(span.file, span.end, context.file.len);
    let tail_source = context.file.get_span_str(tail_span)?;
    tail_source
        .chars()
        .find(|character: &char| !character.is_whitespace())
}

/// Return the first non-whitespace character before an annotation span.
pub(crate) fn previous_non_whitespace_before_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<char> {
    let span = context.get_annotation_span(annotation_id);
    previous_non_whitespace_before_span(context, span)
}

/// Return the first non-whitespace character after an annotation span.
pub(crate) fn next_non_whitespace_after_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<char> {
    let span = context.get_annotation_span(annotation_id);
    next_non_whitespace_after_span(context, span)
}
