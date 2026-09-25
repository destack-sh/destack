use tspp_dir as dir;
use tspp_source::FileId;

use super::lexical::is_trivia_token;
use crate::{ModuleQueryContext, QueryResult};

/// Check whether a character can start an identifier.
pub(crate) fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

/// Check whether a character can continue an identifier.
pub(crate) fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

impl ModuleQueryContext<'_> {
    /// Find the non-trivia token span that contains the offset.
    pub(crate) fn token_span_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<dir::TokenSpan>> {
        let mut candidate = None;

        // walk tokens in order to find the containing span
        for token in self.tokens(file_id)? {
            if is_trivia_token(token.token.ty()) {
                continue;
            }

            if token.span.contains(offset) {
                return Ok(Some(token));
            }

            if token.span.start > offset {
                break;
            }

            candidate = Some(token);
        }

        let Some(candidate) = candidate else {
            return Ok(None);
        };
        if candidate.span.end == offset {
            return Ok(Some(candidate));
        }

        Ok(None)
    }
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

    // require identifier characters and exclude language keywords
    chars.all(is_identifier_continue) && text.parse::<dir::Keyword>().is_err()
}
