use destack_ast as ast;
use destack_ast::TokenType;
use destack_source::FileId;
use destack_workspace::{Repository, Revision};

use crate::core::{AstQueryContext, with_ast_query_for_file};

/// Check whether a character can start an identifier.
pub(crate) fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

/// Check whether a character can continue an identifier.
pub(crate) fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

/// Extract the identifier token at a given offset.
pub(crate) fn token_at_offset(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
) -> Option<String> {
    // resolve the token text at the cursor
    let token_text = token_text_at_offset(repository, revision, file_id, offset)?;

    // ensure the token is an identifier
    let token = token_span_at_offset(repository, revision, file_id, offset)?;
    if token.token.ty != TokenType::Identifier {
        return None;
    }

    Some(token_text)
}

/// Extract the non-trivia token text at a given offset.
pub(crate) fn token_text_at_offset(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
) -> Option<String> {
    // find the token at the cursor
    let token = token_span_at_offset(repository, revision, file_id, offset)?;
    // read the source content
    let file = repository.file(revision, file_id).ok().flatten()?;
    let content = file.text();
    if content.is_empty() {
        return None;
    }

    // slice the token text from the source
    let span = token.span;
    let text = content.get(span.start as usize..span.end as usize)?;

    Some(text.to_string())
}

/// Find the non-trivia token span that contains the offset.
pub(crate) fn token_span_at_offset(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
) -> Option<ast::TokenSpan> {
    with_ast_query_for_file(repository, revision, file_id, |ast| {
        token_span_at_offset_in_ast(ast, offset)
    })?
}

/// Find the token span that contains the offset.
fn token_span_at_offset_in_ast(ast: AstQueryContext<'_>, offset: u32) -> Option<ast::TokenSpan> {
    // track the last token starting before the offset
    let mut candidate = None;

    // walk tokens in order to find the containing span
    for token in ast.tokens() {
        if token.span.file != ast.file_id() {
            continue;
        }

        if is_trivia_token(token.token.ty) {
            continue;
        }

        if token.span.contains(offset) {
            return Some(*token);
        }

        if token.span.start > offset {
            break;
        }

        candidate = Some(*token);
    }

    // allow cursor at the end of a token span
    let candidate = candidate?;
    if candidate.span.end == offset {
        return Some(candidate);
    }

    None
}

/// Check whether a token type is trivia.
fn is_trivia_token(token: TokenType) -> bool {
    matches!(
        token,
        TokenType::Whitespace
            | TokenType::Newline
            | TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
            | TokenType::End
    )
}

/// Check if a string is a simple identifier.
pub(crate) fn is_simple_identifier(text: &str) -> bool {
    // check the first character
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    // validate the first character
    let first_ok = is_identifier_start(first);
    if !first_ok {
        return false;
    }

    // ensure the rest are valid identifier characters
    chars.all(is_identifier_continue)
}
