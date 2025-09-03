use destack_language_token::{TokenSpan, TokenType};

use crate::{Doc, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Peek a doc comment (incl. `///` or `/**`).
    pub fn peek_doc(&self) -> ParseResult<&TokenSpan> {
        if let Ok(doc_line_comment) = self.peek_token(TokenType::DocLineComment) {
            Ok(doc_line_comment)
        } else {
            self.peek_token(TokenType::DocBlockComment)
        }
    }

    /// Eat a doc comment (incl. `///` or `/**`).
    /// Successive doc comments are concatenated.
    ///
    /// Examples:
    /// ```
    /// /// Simple doc.
    ///
    /// /// This is a doc comment
    /// /// And it will be merged with this one
    ///
    /// /** inline doc comment */ /** followed by another (also merged) */
    /// ```
    pub fn eat_doc(&mut self) -> ParseResult<NodeId<Doc>> {
        todo!()
    }
}
