use destack_language_token::{TokenSpan, TokenType};

use crate::{Keyword, ParseError, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a keyword.
    pub fn eat_keyword(&mut self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        self.peek_keyword(keyword)?;
        self.eat_token(TokenType::Identifier)
    }

    /// Peek a keyword.
    #[inline]
    pub fn peek_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::UnexpectedToken(current.span))
        } else {
            Ok(current)
        }
    }

    /// Peek the next keyword.
    #[inline]
    pub fn peek_next_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::UnexpectedToken(current.span))
        } else {
            Ok(current)
        }
    }

    /// Peek the next next keyword.
    #[inline]
    pub fn peek_next_next_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::UnexpectedToken(current.span))
        } else {
            Ok(current)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Keyword, Parser};

    #[test]
    fn test_keyword() {
        let input = "public";
        let tokens = tokenize_semantic(input);
        let parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let result = parser.peek_keyword(Keyword::Public);
        assert!(result.is_ok());
        let result = parser.peek_keyword(Keyword::Module);
        assert!(result.is_err());
    }
}
