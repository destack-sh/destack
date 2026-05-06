use destack_ast as ast;
use destack_source::Span;

use crate::core::AstQueryContext;

/// Check whether a token type is trivia.
pub(crate) fn is_trivia_token(token: ast::TokenType) -> bool {
    matches!(
        token,
        ast::TokenType::Whitespace
            | ast::TokenType::Newline
            | ast::TokenType::LineComment
            | ast::TokenType::BlockComment
            | ast::TokenType::DocLineComment
            | ast::TokenType::DocBlockComment
            | ast::TokenType::End
    )
}

/// Find the previous significant token before or at the cursor.
pub(crate) fn previous_significant_token(
    ast: AstQueryContext<'_>,
    offset: u32,
) -> Option<ast::TokenSpan> {
    let mut candidate = None;

    // scan tokens in order for the latest significant token before the offset
    for token in ast.tokens() {
        // skip tokens from other files
        if token.span.file != ast.file_id() {
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
    ast: AstQueryContext<'_>,
    offset: u32,
) -> Option<ast::TokenSpan> {
    for token in ast.tokens() {
        // skip tokens from other files
        if token.span.file != ast.file_id() {
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
    ast: AstQueryContext<'_>,
    offset: u32,
) -> Option<ast::TokenSpan> {
    let mut candidate = None;

    // scan tokens until the cursor falls inside one token
    for token in ast.tokens() {
        // skip tokens from other files
        if token.span.file != ast.file_id() {
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
    ast: AstQueryContext<'_>,
    offset: u32,
) -> Option<ast::TokenSpan> {
    // look at the nearest significant token before the cursor
    let previous = previous_significant_token(ast, offset)?;

    // `value.$0`
    if previous.token.ty == ast::TokenType::Dot {
        return Some(previous);
    }

    // only identifiers can continue one already-started member name
    if previous.token.ty != ast::TokenType::Identifier {
        return None;
    }

    // `value.na$0`
    let dot = previous_significant_token(ast, previous.span.start)?;
    if dot.token.ty != ast::TokenType::Dot {
        return None;
    }

    Some(dot)
}

/// Resolve the receiver token before one member access dot.
pub(crate) fn receiver_token_before_member_access_dot(
    ast: AstQueryContext<'_>,
    dot: ast::TokenSpan,
) -> Option<ast::TokenSpan> {
    // the receiver token sits immediately before the dot
    let mut receiver_token = previous_significant_token(ast, dot.span.start)?;

    // optional chaining inserts `?` before `.`
    if receiver_token.token.ty == ast::TokenType::Maybe {
        receiver_token = previous_significant_token(ast, receiver_token.span.start)?;
    }

    Some(receiver_token)
}

/// Check whether one token range contains a statement boundary.
pub(crate) fn tokens_between_offsets_include_statement_boundary(
    ast: AstQueryContext<'_>,
    start: u32,
    end: u32,
) -> bool {
    for token in ast.tokens() {
        // skip tokens from other files
        if token.span.file != ast.file_id() {
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
            ast::TokenType::Newline | ast::TokenType::Semicolon
        ) {
            return true;
        }
    }

    false
}
