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

/// Read one token slice from the source text.
pub(crate) fn token_text(source: &str, span: Span) -> Option<&str> {
    source.get(span.start as usize..span.end as usize)
}

impl DirQueryContext<'_> {
    /// Find the previous significant token before or at the cursor.
    pub(crate) fn previous_significant_token(self, offset: u32) -> Option<dir::TokenSpan> {
        let mut candidate = None;

        // scan tokens in order for the latest significant token before the offset
        for token in self.tokens() {
            if token.span.file != self.file_id() {
                continue;
            }

            if is_trivia_token(token.token.ty()) {
                continue;
            }

            if token.span.end <= offset {
                candidate = Some(*token);
                continue;
            }

            if token.span.start > offset {
                break;
            }
        }

        candidate
    }

    /// Find the next significant token after or at the cursor.
    pub(crate) fn next_significant_token(self, offset: u32) -> Option<dir::TokenSpan> {
        // scan tokens in order for the first significant token after the offset
        for token in self.tokens() {
            if token.span.file != self.file_id() {
                continue;
            }

            if is_trivia_token(token.token.ty()) {
                continue;
            }

            if token.span.start >= offset {
                return Some(*token);
            }
        }

        None
    }

    /// Find the significant token span that owns one cursor offset in a query context.
    pub(crate) fn token_span_at_cursor_offset(self, offset: u32) -> Option<dir::TokenSpan> {
        let mut candidate = None;

        // scan tokens until the cursor falls inside one token
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

    /// Resolve the member access dot before the given offset when present.
    pub(crate) fn member_access_dot_before_offset(self, offset: u32) -> Option<dir::TokenSpan> {
        let previous = self.previous_significant_token(offset)?;

        // `value.$0`
        if previous.token.ty() == dir::TokenType::Dot {
            return Some(previous);
        }

        // only identifiers can continue one already started member name
        if previous.token.ty() != dir::TokenType::Identifier {
            return None;
        }

        let dot = self.previous_significant_token(previous.span.start)?;
        if dot.token.ty() != dir::TokenType::Dot {
            return None;
        }

        Some(dot)
    }

    /// Resolve the receiver token before one member access dot.
    pub(crate) fn receiver_token_before_member_access_dot(
        self,
        dot: dir::TokenSpan,
    ) -> Option<dir::TokenSpan> {
        let mut receiver_token = self.previous_significant_token(dot.span.start)?;

        // optional chaining inserts `?` before `.`
        if receiver_token.token.ty() == dir::TokenType::Maybe {
            receiver_token = self.previous_significant_token(receiver_token.span.start)?;
        }

        Some(receiver_token)
    }

    /// Check whether one token range contains a statement boundary.
    pub(crate) fn tokens_between_offsets_include_statement_boundary(
        self,
        start: u32,
        end: u32,
    ) -> bool {
        // scan non trivia tokens between the two offsets
        for token in self.tokens() {
            if token.span.file != self.file_id() {
                continue;
            }

            if token.span.end <= start {
                continue;
            }

            if token.span.start >= end {
                break;
            }

            if matches!(
                token.token.ty(),
                dir::TokenType::Newline | dir::TokenType::Semicolon
            ) {
                return true;
            }
        }

        false
    }
}
