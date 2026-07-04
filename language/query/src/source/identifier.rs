use destack_dir as dir;

use super::lexical::is_trivia_token;
use crate::ModuleQueryContext;

/// Check whether a character can start an identifier.
pub(crate) fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

/// Check whether a character can continue an identifier.
pub(crate) fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

impl ModuleQueryContext<'_> {
    /// Extract the identifier token at one offset.
    pub(crate) fn token_at_offset(&self, offset: u32) -> Option<String> {
        let token_text = self.token_text_at_offset(offset)?;

        // require an identifier token
        let token = self.token_span_at_offset(offset)?;
        if token.token.ty() != dir::TokenType::Identifier {
            return None;
        }

        Some(token_text)
    }

    /// Extract the non-trivia token text at one offset.
    pub(crate) fn token_text_at_offset(&self, offset: u32) -> Option<String> {
        let token = self.token_span_at_offset(offset)?;

        // read source text for the token span
        let file = self.source_file();
        let content = file.text();
        let span = token.span;
        let text = content
            .get(span.start as usize..span.end as usize)
            .unwrap_or_else(|| panic!("invalid token source range: {span:?}"));

        Some(text.to_string())
    }
}

impl ModuleQueryContext<'_> {
    /// Find the non-trivia token span that contains the offset.
    pub(crate) fn token_span_at_offset(&self, offset: u32) -> Option<dir::TokenSpan> {
        let mut candidate = None;

        // walk tokens in order to find the containing span
        for token in self.tokens() {
            if token.span.file != self.file_id() {
                continue;
            }

            if is_trivia_token(token.token.ty()) {
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

        let candidate = candidate?;
        if candidate.span.end == offset {
            return Some(candidate);
        }

        None
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

    // ensure the rest are valid identifier characters
    chars.all(is_identifier_continue)
}
