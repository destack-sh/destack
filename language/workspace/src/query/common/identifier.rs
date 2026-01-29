use destack_ast as ast;
use destack_ast::TokenType;
use destack_source::{FileContent, FileId};

use crate::Session;
use crate::query::common::{QueryContext, get_module_by_file_id};

/// Check whether a character can start an identifier.
pub(crate) fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

/// Check whether a character can continue an identifier.
pub(crate) fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

/// Extract the identifier token at a given offset.
pub(crate) fn token_at_offset(session: &Session, file_id: FileId, offset: u32) -> Option<String> {
    // resolve query context for token lookup
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // find the token at the cursor
    let token = token_span_at_offset(&ctx, offset)?;
    if token.token.ty != TokenType::Identifier {
        return None;
    }

    // read the source content
    let file = session.files.get(file_id);
    let content = match &file.content {
        FileContent::Text { content } => content.as_str(),
        FileContent::Json { content, .. } => content.as_str(),
        _ => return None,
    };

    // slice the token text from the source
    let span = token.span;
    let text = content.get(span.start as usize..span.end as usize)?;

    Some(text.to_string())
}

/// Find the token span that contains the offset.
fn token_span_at_offset(ctx: &QueryContext<'_>, offset: u32) -> Option<ast::TokenSpan> {
    // track the last token starting before the offset
    let mut candidate = None;

    // walk tokens in order to find the containing span
    for token in &ctx.ast.tokens {
        if token.span.file != ctx.file_id {
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

/// Extract the first identifier from a string.
pub(crate) fn extract_identifier(text: &str) -> Option<String> {
    // scan for the first valid identifier start
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.peek().copied() {
        if is_identifier_start(ch) {
            break;
        }
        chars.next();
    }

    // collect identifier characters
    let mut name = String::new();
    while let Some(ch) = chars.peek().copied() {
        if name.is_empty() {
            if !is_identifier_start(ch) {
                chars.next();
                continue;
            }
            name.push(ch);
            chars.next();
            continue;
        }

        if is_identifier_continue(ch) {
            name.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    if name.is_empty() {
        return None;
    }

    // return the extracted identifier
    Some(name)
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
