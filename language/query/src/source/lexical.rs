use tspp_dir as dir;
use tspp_source::{FileId, Span};

use crate::{ModuleQueryContext, QueryError, QueryResult};

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
pub(crate) fn token_text(source: &str, span: Span) -> QueryResult<&str> {
    let text = source
        .get(span.start as usize..span.end as usize)
        .ok_or(QueryError::invalid(format!("source span: {span:?}")))?;

    Ok(text)
}

impl ModuleQueryContext<'_> {
    /// Find the previous significant token before or at the cursor.
    pub(crate) fn previous_significant_token(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<dir::TokenSpan>> {
        let mut candidate = None;

        // scan tokens in order for the latest significant token before the offset
        for token in self.tokens(file_id)? {
            if is_trivia_token(token.token.ty()) {
                continue;
            }

            if token.span.end <= offset {
                candidate = Some(token);
                continue;
            }

            if token.span.start > offset {
                break;
            }
        }

        Ok(candidate)
    }

    /// Resolve the member access dot before the given offset when present.
    pub(crate) fn member_access_dot_before_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<dir::TokenSpan>> {
        let Some(previous) = self.previous_significant_token(file_id, offset)? else {
            return Ok(None);
        };

        // `value.$0`
        if previous.token.ty() == dir::TokenType::Dot {
            return Ok(Some(previous));
        }

        // only identifiers can continue one already started member name
        if previous.token.ty() != dir::TokenType::Identifier {
            return Ok(None);
        }

        let Some(dot) = self.previous_significant_token(file_id, previous.span.start)? else {
            return Ok(None);
        };
        if dot.token.ty() != dir::TokenType::Dot {
            return Ok(None);
        }

        Ok(Some(dot))
    }

    /// Resolve the receiver token before one member access dot.
    pub(crate) fn receiver_token_before_member_access_dot(
        &self,
        dot: dir::TokenSpan,
    ) -> QueryResult<Option<dir::TokenSpan>> {
        let file_id = dot.span.file;
        let Some(mut receiver_token) = self.previous_significant_token(file_id, dot.span.start)?
        else {
            return Ok(None);
        };

        // optional chaining inserts `?` before `.`
        if receiver_token.token.ty() == dir::TokenType::Maybe {
            let Some(previous) =
                self.previous_significant_token(file_id, receiver_token.span.start)?
            else {
                return Ok(None);
            };
            receiver_token = previous;
        }

        Ok(Some(receiver_token))
    }
}
