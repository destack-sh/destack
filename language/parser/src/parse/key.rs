use crate::{ParseError, ParseResult, Parser};
use destack_ast::{Key, LiteralType, Name, TokenSpan, TokenType};
use destack_source::StringId;

impl Parser {
    /// Peek an identifier.
    #[inline]
    pub fn peek_identifier(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Identifier)
    }

    /// Eat an identifier.
    #[inline]
    pub fn eat_identifier(&mut self) -> ParseResult<StringId> {
        let token = *self.eat_token(TokenType::Identifier)?;
        let string_id = self.strings.intern(self.get_token_str(token));
        Ok(string_id)
    }

    /// Peek an identifier that matches a given string.
    #[inline]
    pub fn peek_identifier_str(&self, string: &str) -> ParseResult<&TokenSpan> {
        let span = self.peek_token(TokenType::Identifier)?;
        if self.get_token_str(*span) == string {
            Ok(span)
        } else {
            Err(ParseError::expected(span.span, TokenType::Identifier))
        }
    }

    /// Eat an identifier that matches a given string.
    #[inline]
    pub fn eat_identifier_str(&mut self, string: &str) -> ParseResult<StringId> {
        let span = self.peek_identifier_str(string)?;
        let string_id = self.strings.intern(self.get_token_str(*span));
        self.bump();
        Ok(string_id)
    }

    /// Eat an identifier or a wildcard maybe.
    #[inline]
    pub fn eat_identifier_or_wildcard_maybe(&mut self) -> ParseResult<Option<StringId>> {
        if self.peek_token(TokenType::Wildcard).is_ok() {
            self.bump();
            Ok(None)
        } else if self.peek_identifier().is_ok() {
            Ok(Some(self.eat_identifier()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a name maybe.
    #[inline]
    pub fn eat_name_maybe(&mut self) -> ParseResult<Option<Name>> {
        if self.peek_name().is_ok() {
            Ok(Some(self.eat_name()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a tree literal identifier (`kebab-case` as `kebabCase`).
    #[inline]
    pub fn eat_tree_literal_identifier(&mut self) -> ParseResult<StringId> {
        let mut identifier = String::new();
        loop {
            let token = *self.eat_token(TokenType::Identifier)?;
            let token_part = self.get_token_str(token);

            // uppercase first letter (except at start)
            if identifier.is_empty() {
                identifier.push_str(token_part);
            } else {
                identifier.push_str(&token_part[0..1].to_uppercase());
                identifier.push_str(&token_part[1..]);
            }

            if self.peek_token(TokenType::Subtract).is_ok() {
                self.bump();
            } else {
                break;
            }
        }
        let string_id = self.strings.intern(identifier);
        Ok(string_id)
    }

    /// Peek a string literal.
    #[inline]
    pub fn peek_string_literal(&self) -> ParseResult<&TokenSpan> {
        let token = self.peek()?;
        if token.token.ty == TokenType::Literal
            && token.token.literal
                == Some(LiteralType::String {
                    is_terminated: true,
                    has_invalid_escape: false,
                })
        {
            Ok(token)
        } else {
            Err(ParseError::expected(token.span, TokenType::Literal))
        }
    }

    /// Peek a next string literal.
    #[inline]
    pub fn peek_next_string_literal(&self) -> ParseResult<&TokenSpan> {
        let token = self.peek_next_token(TokenType::Literal)?;
        if token.token.ty == TokenType::Literal
            && token.token.literal
                == Some(LiteralType::String {
                    is_terminated: true,
                    has_invalid_escape: false,
                })
        {
            Ok(token)
        } else {
            Err(ParseError::expected(token.span, TokenType::Literal))
        }
    }

    /// Peek a name (like `x` or `"Content-Type"`).
    #[inline]
    pub fn peek_name(&self) -> ParseResult<()> {
        if self.peek_token(TokenType::Identifier).is_ok() || self.peek_string_literal().is_ok() {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Peek a next name (like `x` or `"Content-Type"`).
    #[inline]
    pub fn peek_next_name(&self) -> ParseResult<()> {
        if self.peek_next_token(TokenType::Identifier).is_ok()
            || self.peek_next_string_literal().is_ok()
        {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek_next()?.span))
        }
    }

    /// Eat a name (like `x` or `"Content-Type"`).
    #[inline]
    pub fn eat_name(&mut self) -> ParseResult<Name> {
        // regular identifier
        if self.peek_token(TokenType::Identifier).is_ok() {
            Ok(Name::Identifier(self.eat_identifier()?))
        }
        // string identifier
        else if self.peek_string_literal().is_ok() {
            let token = self.peek_string_literal()?;
            let token_str = self.get_token_str(*token);
            let token_str = &token_str[1..token_str.len() - 1];
            let string_id = self.strings.intern(token_str);
            self.bump();
            Ok(Name::String(string_id))
        }
        // error
        else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Peek a name or a dynamic key.
    #[inline]
    pub fn peek_key(&self) -> ParseResult<()> {
        if self.peek_name().is_ok() || self.peek_token(TokenType::OpenBracket).is_ok() {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a name or a dynamic key.
    #[inline]
    pub fn eat_key(&mut self) -> ParseResult<Key> {
        if self.peek_name().is_ok() {
            Ok(Key::Name(self.eat_name()?))
        } else if self.peek_token(TokenType::OpenBracket).is_ok() {
            self.bump(); // eat open bracket
            // name: type
            if self.peek_token(TokenType::Identifier).is_ok()
                && self.peek_next_token(TokenType::Colon).is_ok()
            {
                let name = self.eat_identifier()?;
                self.bump(); // eat colon
                let key_type =
                    self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
                self.eat_token(TokenType::CloseBracket)?;
                Ok(Key::NamedExpression {
                    name,
                    key: key_type,
                })
            }
            // expression
            else {
                let key = self.eat_expression()?;
                self.eat_token(TokenType::CloseBracket)?;
                Ok(Key::Expression(key))
            }
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a name or a dynamic key maybe.
    #[inline]
    pub fn eat_key_maybe(&mut self) -> ParseResult<Option<Key>> {
        if self.peek_key().is_ok() {
            Ok(Some(self.eat_key()?))
        } else {
            Ok(None)
        }
    }
}
