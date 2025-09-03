use destack_language_token::{SourceFile, Span, Token, TokenSpan, TokenType};

use crate::{NodeTree, ParseError, ParseResult, PathPool, StringPool};

const DEFAULT_EOF_TOKEN_SPAN: TokenSpan = TokenSpan {
    span: Span { start: 0, end: 0 },
    token: Token::eof(),
};

/// A parser for the Destack Language.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored.
#[derive(Debug)]
pub struct Parser<'a> {
    /// The file we're parsing.
    pub(crate) file: SourceFile<'a>,
    /// The tokens to parse.
    pub(crate) tokens: &'a [TokenSpan],
    /// The EOF token (the actual last token or a fake placeholder one if empty).
    eof_token: TokenSpan,

    /// The string pool.
    pub(crate) strings: StringPool,
    /// The path pool.
    pub(crate) paths: PathPool,
    /// The Node tree.
    pub(crate) tree: NodeTree,

    /// The current position in the tokens.
    pos: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser.
    pub fn new(file: SourceFile<'a>, tokens: &'a [TokenSpan]) -> Self {
        let eof_token = *tokens.last().unwrap_or(&DEFAULT_EOF_TOKEN_SPAN);
        let strings = StringPool::new();
        let paths = PathPool::new();
        let tree = NodeTree::new();
        Self {
            file,
            tokens,
            strings,
            paths,
            tree,
            pos: 0,
            eof_token,
        }
    }

    /// Gets a mark of the current position.
    #[inline]
    pub fn mark(&self) -> ParserMark {
        ParserMark { pos: self.pos }
    }

    /// Rewind the position to the given mark.
    #[inline]
    pub fn rewind(&mut self, mark: ParserMark) {
        self.pos = mark.pos;
    }

    /// Get a mark and return the span of the current position.
    #[inline]
    pub fn get_span_from(&self, mark: ParserMark) -> Span {
        let start_token = self.tokens[mark.pos];
        let end_token = self.tokens[self.pos - 1]; // pos is lookahead
        Span {
            start: start_token.span.start,
            end: end_token.span.end,
        }
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'a str {
        &self.file.content[span.start as usize..span.end as usize]
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &'a str {
        &self.file.content[token.span.start as usize..token.span.end as usize]
    }

    /// Get the current Token.
    #[inline]
    pub fn prev(&self) -> Option<&TokenSpan> {
        self.tokens.get(self.pos)
    }

    /// Peek the next Token or error.
    #[inline]
    pub fn peek(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos)
            .ok_or(ParseError::SyntaxError(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 1)
            .ok_or(ParseError::SyntaxError(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 2)
            .ok_or(ParseError::SyntaxError(self.eof_token.span))
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat_next(&mut self) -> ParseResult<&TokenSpan> {
        if self.pos < self.tokens.len() {
            let next = &self.tokens[self.pos];
            self.pos += 1;
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(self.eof_token.span))
        }
    }

    /// Bump the position.
    #[inline]
    pub fn bump(&mut self) {
        self.pos += 1;
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(next.span))
        }
    }

    /// Peek the next next token.
    #[inline]
    pub fn peek_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(next.span))
        }
    }

    /// Eat a token.
    #[inline]
    pub fn eat_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let current = self.eat_next()?;
        if current.token.r#type == token_type {
            Ok(current)
        } else {
            Err(ParseError::SyntaxError(current.span))
        }
    }

    /// Peek a colon.
    #[inline]
    pub fn peek_colon(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Colon)
    }

    /// Eat a colon.
    #[inline]
    pub fn eat_colon(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Colon)
    }

    /// Peek a semicolon.
    #[inline]
    pub fn peek_semicolon(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Semicolon)
    }

    /// Eat a semicolon.
    #[inline]
    pub fn eat_semicolon(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Semicolon)
    }

    /// Peek a comma.
    #[inline]
    pub fn peek_comma(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Comma)
    }

    /// Eat a comma.
    #[inline]
    pub fn eat_comma(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Comma)
    }

    /// Peek a newline.
    #[inline]
    pub fn peek_newline(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Newline)
    }

    /// Eat a newline.
    #[inline]
    pub fn eat_newline(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Newline)
    }

    /// Eat 0 or more newlines.
    #[inline]
    pub fn eat_newlines_maybe(&mut self) -> ParseResult<()> {
        while self.peek_newline().is_ok() {
            self.eat_newline()?;
        }
        Ok(())
    }

    /// Peek an item stop (comma or newline).
    #[inline]
    pub fn peek_item_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(newline) = self.peek_newline() {
            Ok(newline)
        } else {
            self.peek_comma()
        }
    }

    /// Eat an item stop (comma or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_item_stop(&mut self) -> ParseResult<()> {
        if self.peek_newline().is_ok() {
            self.eat_newline()?;
        } else {
            self.eat_comma()?;
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek a statement stop (semicolon or newline).
    #[inline]
    pub fn peek_statement_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(newline) = self.peek_newline() {
            Ok(newline)
        } else {
            self.peek_semicolon()
        }
    }

    /// Eat a statement stop (semicolon or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_statement_stop(&mut self) -> ParseResult<()> {
        if self.peek_newline().is_ok() {
            self.eat_newline()?;
        } else {
            self.eat_semicolon()?;
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParserMark {
    /// The token position.
    pos: usize,
}
