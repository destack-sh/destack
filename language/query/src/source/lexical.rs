use destack_dir as dir;
use destack_source::Span;

use crate::core::DirQueryContext;

/// Check whether a token type is trivia.
pub(crate) fn is_trivia_token(token: dir::TokenType) -> bool {
    matches!(
        token,
        dir::TokenType::Whitespace
            | dir::TokenType::Newline
            | dir::TokenType::LineComment
            | dir::TokenType::BlockComment
            | dir::TokenType::DocLineComment
            | dir::TokenType::DocBlockComment
            | dir::TokenType::End
    )
}

/// Find the previous significant token before or at the cursor.
pub(crate) fn previous_significant_token(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Option<dir::TokenSpan> {
    let mut candidate = None;

    // scan tokens in order for the latest significant token before the offset
    for token in ctx.tokens() {
        // skip tokens from other files
        if token.span.file != ctx.file_id() {
            continue;
        }

        // skip trivia when classifying cursor intent
        if is_trivia_token(token.token.ty) {
            continue;
        }

        // keep the latest token that still ends before the cursor
        if token.span.end <= offset {
            candidate = Some(*token);
            continue;
        }

        // stop once later tokens begin after the cursor
        if token.span.start > offset {
            break;
        }
    }

    candidate
}

/// Find the next significant token after or at the cursor.
pub(crate) fn next_significant_token(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Option<dir::TokenSpan> {
    for token in ctx.tokens() {
        // skip tokens from other files
        if token.span.file != ctx.file_id() {
            continue;
        }

        // skip trivia when classifying cursor intent
        if is_trivia_token(token.token.ty) {
            continue;
        }

        // return the first significant token that starts at or after the cursor
        if token.span.start >= offset {
            return Some(*token);
        }
    }

    None
}

/// Find the significant token span that owns one cursor offset in a query context.
pub(crate) fn token_span_at_cursor_offset(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Option<dir::TokenSpan> {
    let mut candidate = None;

    // scan tokens until the cursor falls inside one token
    for token in ctx.tokens() {
        // skip tokens from other files
        if token.span.file != ctx.file_id() {
            continue;
        }

        // skip trivia when extracting the completion prefix token
        if is_trivia_token(token.token.ty) {
            continue;
        }

        // prefer a token that directly contains the cursor
        if token.span.contains(offset) {
            return Some(*token);
        }

        // stop once tokens begin after the cursor
        if token.span.start > offset {
            break;
        }

        // remember the latest significant token before the cursor
        candidate = Some(*token);
    }

    // allow the cursor at the end of one token span
    let candidate = candidate?;
    if candidate.span.end == offset {
        return Some(candidate);
    }

    None
}

/// Read one token slice from the source text.
pub(crate) fn token_text(source: &str, span: Span) -> Option<&str> {
    source.get(span.start as usize..span.end as usize)
}

/// Resolve the member access dot before the given offset when present.
pub(crate) fn member_access_dot_before_offset(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Option<dir::TokenSpan> {
    // look at the nearest significant token before the cursor
    let previous = previous_significant_token(ctx, offset)?;

    // `value.$0`
    if previous.token.ty == dir::TokenType::Dot {
        return Some(previous);
    }

    // only identifiers can continue one already-started member name
    if previous.token.ty != dir::TokenType::Identifier {
        return None;
    }

    // `value.na$0`
    let dot = previous_significant_token(ctx, previous.span.start)?;
    if dot.token.ty != dir::TokenType::Dot {
        return None;
    }

    Some(dot)
}

/// Resolve the receiver token before one member access dot.
pub(crate) fn receiver_token_before_member_access_dot(
    ctx: DirQueryContext<'_>,
    dot: dir::TokenSpan,
) -> Option<dir::TokenSpan> {
    // the receiver token sits immediately before the dot
    let mut receiver_token = previous_significant_token(ctx, dot.span.start)?;

    // optional chaining inserts `?` before `.`
    if receiver_token.token.ty == dir::TokenType::Maybe {
        receiver_token = previous_significant_token(ctx, receiver_token.span.start)?;
    }

    Some(receiver_token)
}

/// Check whether one token range contains a statement boundary.
pub(crate) fn tokens_between_offsets_include_statement_boundary(
    ctx: DirQueryContext<'_>,
    start: u32,
    end: u32,
) -> bool {
    for token in ctx.tokens() {
        // skip tokens from other files
        if token.span.file != ctx.file_id() {
            continue;
        }

        // ignore tokens that end before the tracked range
        if token.span.end <= start {
            continue;
        }

        // stop once tokens start after the tracked range
        if token.span.start >= end {
            break;
        }

        // statement separators end keyword-owned expression slots
        if matches!(
            token.token.ty,
            dir::TokenType::Newline | dir::TokenType::Semicolon
        ) {
            return true;
        }
    }

    false
}
