use crate::{Annotation, DestackFormatContext};
use destack_ast::{Keyword, LocalNodeId, TokenSpan, TokenType};
use destack_source::Span;

/// Return whether one token type is ignorable trivia for span-adjacent scans.
#[inline]
fn is_ignored_span_neighbor_token(token_type: TokenType) -> bool {
    matches!(token_type, TokenType::Whitespace | TokenType::Newline)
}

/// Return whether one token type is ignorable trivia, including comments.
#[inline]
fn is_ignored_span_trivia_token(token_type: TokenType) -> bool {
    is_ignored_span_neighbor_token(token_type)
        || matches!(
            token_type,
            TokenType::LineComment
                | TokenType::BlockComment
                | TokenType::DocLineComment
                | TokenType::DocBlockComment
        )
}

/// Map one token type to one punctuation character for trivia-boundary checks.
#[inline]
fn token_type_to_boundary_character(token_type: TokenType) -> Option<char> {
    match token_type {
        TokenType::Colon => Some(':'),
        TokenType::Semicolon => Some(';'),
        TokenType::Comma => Some(','),
        TokenType::Dot => Some('.'),
        TokenType::Assign => Some('='),
        TokenType::Maybe => Some('?'),
        TokenType::ElementwiseAnd => Some('&'),
        TokenType::ElementwiseOr => Some('|'),
        TokenType::Divide
        | TokenType::LineComment
        | TokenType::BlockComment
        | TokenType::DocLineComment
        | TokenType::DocBlockComment => Some('/'),
        TokenType::OpenParenthesis => Some('('),
        TokenType::CloseParenthesis => Some(')'),
        TokenType::OpenBrace => Some('{'),
        TokenType::CloseBrace => Some('}'),
        TokenType::OpenBracket => Some('['),
        TokenType::CloseBracket => Some(']'),
        TokenType::LessThan => Some('<'),
        TokenType::GreaterThan => Some('>'),
        _ => None,
    }
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

/// Return the Nth non-trivia token that intersects one span.
pub(crate) fn nth_non_trivia_token_in_span(
    context: &DestackFormatContext<'_>,
    span: Span,
    nth: usize,
) -> Option<TokenSpan> {
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.end <= span.start);
    let mut seen = 0usize;

    while let Some(token) = tokens.get(index).copied() {
        if token.span.start >= span.end {
            break;
        }

        index += 1;
        if is_ignored_span_trivia_token(token.token.ty) {
            continue;
        }

        if seen == nth {
            return Some(token);
        }
        seen += 1;
    }

    None
}

/// Return the first non-trivia token that intersects one span.
#[inline]
pub(crate) fn first_non_trivia_token_in_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<TokenSpan> {
    nth_non_trivia_token_in_span(context, span, 0)
}

/// Return the last non-trivia token that intersects one span.
pub(crate) fn last_non_trivia_token_in_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<TokenSpan> {
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.start < span.end);

    while index > 0 {
        index -= 1;
        let token = tokens[index];
        if token.span.end <= span.start {
            break;
        }
        if is_ignored_span_trivia_token(token.token.ty) {
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
    let span = context.annotation_span(annotation_id);
    previous_non_whitespace_token_before_span(context, span)
}

/// Return the nearest non-whitespace token after one annotation span.
pub(crate) fn next_non_whitespace_token_after_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<TokenSpan> {
    let span = context.annotation_span(annotation_id);
    next_non_whitespace_token_after_span(context, span)
}

/// Return the first non-whitespace character before a span.
pub(crate) fn previous_non_whitespace_before_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<char> {
    previous_non_whitespace_token_before_span(context, span)
        .and_then(|token| token_type_to_boundary_character(token.token.ty))
}

/// Return the first non-whitespace character after a span.
pub(crate) fn next_non_whitespace_after_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<char> {
    next_non_whitespace_token_after_span(context, span)
        .and_then(|token| token_type_to_boundary_character(token.token.ty))
}

/// Return the first non-whitespace character before an annotation span.
pub(crate) fn previous_non_whitespace_before_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<char> {
    let span = context.annotation_span(annotation_id);
    previous_non_whitespace_before_span(context, span)
}

/// Return the first non-whitespace character after an annotation span.
pub(crate) fn next_non_whitespace_after_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<char> {
    let span = context.annotation_span(annotation_id);
    next_non_whitespace_after_span(context, span)
}

/// Return whether one identifier token matches one keyword.
pub(crate) fn token_is_keyword(
    context: &DestackFormatContext<'_>,
    token: TokenSpan,
    keyword: Keyword,
) -> bool {
    context
        .token_keyword(token)
        .is_some_and(|parsed| parsed == keyword)
}
