use crate::lex::decode_unicode_escape;
use crate::parse::{AwaitKeyword, YieldKeyword};
use crate::{Parser, ParserError, ParserResult};
use std::str::FromStr;
use tspp_core::StringId;
use tspp_dir::{Keyword, Literal, Name, Token, TokenLiteral, TokenType};
use tspp_source::ByteRange;

impl Parser {
    /// Eat one identifier or boolean member name and its byte range.
    pub(crate) fn eat_member_name_with_range(&mut self) -> ParserResult<(StringId, ByteRange)> {
        if self.peek_is(TokenType::Literal)
            && matches!(
                self.peek_token().literal(),
                Some(TokenLiteral::Boolean { .. })
            )
        {
            let token = self.peek_token_span();
            let name = self.intern_range(token.token.range());
            self.bump();

            return Ok((name, token.token.range()));
        }

        self.eat_identifier_with_range()
    }

    /// Eat an identifier.
    #[inline]
    pub fn eat_identifier(&mut self) -> ParserResult<StringId> {
        let (string_id, _) = self.eat_identifier_with_range()?;
        Ok(string_id)
    }

    /// Eat an identifier and return its string ID and byte range.
    #[inline]
    pub fn eat_identifier_with_range(&mut self) -> ParserResult<(StringId, ByteRange)> {
        let token = self.eat_token(TokenType::Identifier)?;
        let range = token.range();
        let raw = &self.file.text()[range.start as usize..range.end as usize];

        // decode escaped identifiers to the name they spell
        if token.is_identifier_escaped() {
            let Some(decoded) = Self::decode_identifier_unicode_escapes(raw) else {
                return Err(ParserError::unexpected(token));
            };
            if Keyword::from_str(&decoded).is_ok() || decoded.contains('\0') {
                return Err(ParserError::unexpected(token));
            }
            let string_id = self.strings.intern(&decoded);

            return Ok((string_id, range));
        }
        let string_id = self.strings.intern(raw);
        Ok((string_id, range))
    }

    /// Eat a binding identifier and return its string ID and byte range.
    #[inline]
    pub(crate) fn eat_binding_identifier_with_range(
        &mut self,
    ) -> ParserResult<(StringId, ByteRange)> {
        self.report_forbidden_binding_identifier();

        self.eat_identifier_with_range()
    }

    /// Report a contextually reserved binding identifier while preserving its tree shape.
    pub(crate) fn report_forbidden_binding_identifier(&mut self) {
        let keyword = self.peek_keyword();
        let is_forbidden = self.keywords.yield_keyword == YieldKeyword::Forbidden
            && keyword == Some(Keyword::Yield)
            || self.keywords.await_keyword == AwaitKeyword::Forbidden
                && keyword == Some(Keyword::Await);
        if is_forbidden {
            let error = ParserError::unexpected(self.peek_token_span());
            self.report_error(error);
        }
    }

    /// Eat a string literal and return its content ID and byte range.
    #[inline]
    pub fn eat_string_literal_with_range(&mut self) -> ParserResult<(StringId, ByteRange)> {
        let token = self.peek_string_literal()?;
        let content = self.string_literal_str(token).to_owned();
        let string_id = self.strings.intern(&content);
        self.bump();
        Ok((string_id, token.range()))
    }

    /// Peek an identifier that matches a given string.
    #[inline]
    pub fn peek_identifier_str(&self, string: &str) -> ParserResult<Token> {
        if self.peek_identifier_is(string) {
            self.require_token(TokenType::Identifier)
        } else {
            Err(ParserError::expected(
                self.peek_token().range(),
                TokenType::Identifier,
            ))
        }
    }

    /// Return true when the current token is an identifier matching a string.
    #[inline]
    pub fn peek_identifier_is(&self, string: &str) -> bool {
        self.cursor
            .identifier_is(&self.file, self.peek_token(), string)
    }

    /// Decode Unicode escapes in one identifier.
    fn decode_identifier_unicode_escapes(raw: &str) -> Option<String> {
        let mut decoded = String::with_capacity(raw.len());
        let mut characters = raw.chars();
        while let Some(character) = characters.next() {
            // copy plain identifier text through
            if character != '\\' {
                decoded.push(character);
                continue;
            }

            // identifiers allow unicode escapes only
            if characters.next() != Some('u') {
                return None;
            }
            decoded.push(decode_unicode_escape(&mut characters).ok()?);
        }

        Some(decoded)
    }

    /// Eat an identifier that matches a given string.
    #[inline]
    pub fn eat_identifier_str(&mut self, string: &str) -> ParserResult<StringId> {
        let token = self.peek_identifier_str(string)?;
        let range = token.range();
        let string = &self.file.text()[range.start as usize..range.end as usize];
        let string_id = self.strings.intern(string);
        self.bump();
        Ok(string_id)
    }

    /// Eat a name when present and return its byte range.
    #[inline]
    pub fn eat_name_with_range_if_present(&mut self) -> ParserResult<Option<(Name, ByteRange)>> {
        if self.peek_name_start() {
            Ok(Some(self.eat_name_with_range()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a tree literal identifier (`kebab-case` as `kebabCase`, namespaces allowed).
    #[inline]
    pub fn eat_tree_literal_identifier(&mut self) -> ParserResult<StringId> {
        let (string_id, _) = self.eat_tree_literal_identifier_with_range()?;
        Ok(string_id)
    }

    /// Eat a tree literal identifier and return its string ID and byte range.
    #[inline]
    pub fn eat_tree_literal_identifier_with_range(
        &mut self,
    ) -> ParserResult<(StringId, ByteRange)> {
        let mut identifier = String::new();
        let token = self.eat_token(TokenType::Identifier)?;
        let first_range = token.range();
        let mut last_range = first_range;
        let token_part = self.token_str(token);

        // disallow escaped identifiers in tree literals
        if token_part.contains('\\') {
            return Err(ParserError::unexpected(token));
        }

        // retain the first segment as written
        identifier.push_str(token_part);

        loop {
            // scan kebab-case or namespace separators
            let is_kebab =
                if self.peek_is(TokenType::Subtract) || self.peek_is(TokenType::Decrement) {
                    self.bump();
                    true
                } else if self.peek_is(TokenType::Colon) {
                    self.bump();
                    false
                } else {
                    break;
                };

            // kebab segments allow numeric suffixes, like panose-1
            let token = if is_kebab && self.peek_numeric_literal_start() {
                let token = self.peek_numeric_literal()?;
                self.bump();
                token
            } else {
                self.eat_token(TokenType::Identifier)?
            };
            let token_part = self.token_str(token);

            // disallow escaped identifiers in tree literals
            if token.ty() == TokenType::Identifier && token_part.contains('\\') {
                return Err(ParserError::unexpected(token));
            }
            last_range = token.range();

            if is_kebab {
                // keep numeric kebab segments as is, uppercase identifier segments
                if token.ty() == TokenType::Literal {
                    identifier.push_str(token_part);
                } else {
                    let mut chars = token_part.chars();
                    if let Some(first) = chars.next() {
                        identifier.extend(first.to_uppercase());
                        identifier.push_str(chars.as_str());
                    }
                }
            } else {
                identifier.push(':');
                identifier.push_str(token_part);
            }
        }
        let string_id = self.strings.intern(&identifier);
        let range = ByteRange {
            start: first_range.start,
            end: last_range.end,
        };

        Ok((string_id, range))
    }

    /// Peek a string literal.
    #[inline]
    pub fn peek_string_literal(&self) -> ParserResult<Token> {
        let token = self.require_token(TokenType::Literal)?;
        match token.literal() {
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            }) => Ok(token),
            _ => Err(ParserError::expected(token, TokenType::Literal)),
        }
    }

    /// Return true when the next token is a valid string literal.
    #[inline]
    pub fn peek_string_literal_start(&self) -> bool {
        if self.peek_token_type() != TokenType::Literal {
            return false;
        }

        matches!(
            self.peek_token().literal(),
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        )
    }

    /// Return the content of a string literal without surrounding quotes.
    #[inline]
    pub fn string_literal_str(&self, token: Token) -> &str {
        let token_str = self.token_str(token);

        // keep the literal content stable even when the closing quote is missing
        if let Some(content) = token_str.strip_prefix('"') {
            return content.strip_suffix('"').unwrap_or(content);
        }

        if let Some(content) = token_str.strip_prefix('\'') {
            return content.strip_suffix('\'').unwrap_or(content);
        }

        token_str
    }

    /// Peek a numeric literal.
    #[inline]
    pub fn peek_numeric_literal(&self) -> ParserResult<Token> {
        let token = self.peek_token();
        if token.ty() == TokenType::Literal {
            match token.literal() {
                Some(TokenLiteral::Int { .. }) | Some(TokenLiteral::Float { .. }) => Ok(token),
                _ => Err(ParserError::expected(token, TokenType::Literal)),
            }
        } else {
            Err(ParserError::expected(token, TokenType::Literal))
        }
    }

    /// Return true when the next token is a numeric literal.
    #[inline]
    pub fn peek_numeric_literal_start(&self) -> bool {
        if self.peek_token_type() != TokenType::Literal {
            return false;
        }

        matches!(
            self.peek_token().literal(),
            Some(TokenLiteral::Int { .. }) | Some(TokenLiteral::Float { .. })
        )
    }

    /// Return true when the next token is a name.
    #[inline]
    pub fn peek_name_start(&self) -> bool {
        self.peek_is(TokenType::Identifier) || self.peek_string_literal_start()
    }

    /// Eat a name and return its value and byte range.
    pub fn eat_name_with_range(&mut self) -> ParserResult<(Name, ByteRange)> {
        // regular identifier
        if self.peek_is(TokenType::Identifier) {
            let (name, range) = self.eat_identifier_with_range()?;
            Ok((Name::Identifier(name), range))
        }
        // string identifier
        else if self.peek_string_literal_start() {
            let token = self.peek_string_literal()?;
            let content = self.string_literal_str(token).to_owned();
            let string_id = self.strings.intern(&content);
            self.bump();
            Ok((Name::String(string_id), token.range()))
        }
        // error
        else {
            Err(ParserError::unexpected(self.peek_token_span()))
        }
    }

    /// Return whether the next token can start a property name.
    #[inline]
    pub fn peek_property_name_start(&self) -> bool {
        self.peek_name_start() || self.peek_numeric_literal_start()
    }

    /// Eat a property name and return its value and byte range.
    pub(crate) fn eat_property_name_with_range(&mut self) -> ParserResult<(Name, ByteRange)> {
        // identifier or string name
        if self.peek_name_start() {
            return self.eat_name_with_range();
        }

        // numeric index
        if self.peek_numeric_literal_start() {
            let (index, range) = self.eat_index_name_with_range()?;

            return Ok((Name::Index(index), range));
        }

        // invalid name
        Err(ParserError::unexpected(self.peek_token_span()))
    }

    /// Eat a property name when present and return its byte range.
    pub(crate) fn eat_property_name_with_range_if_present(
        &mut self,
    ) -> ParserResult<Option<(Name, ByteRange)>> {
        if self.peek_property_name_start() {
            Ok(Some(self.eat_property_name_with_range()?))
        } else {
            Ok(None)
        }
    }

    /// Eat an integer name and return its index with its byte range.
    pub(in crate::parse) fn eat_index_name_with_range(
        &mut self,
    ) -> ParserResult<(usize, ByteRange)> {
        let token = self.peek_numeric_literal()?;

        // parse the integer name through the literal parser
        let numeric_literal = self.parse_scalar_literal()?;
        let Literal::Integer(index) = numeric_literal else {
            return Err(ParserError::unexpected(token));
        };

        let index = usize::try_from(index).map_err(|_| ParserError::unexpected(token))?;

        Ok((index, token.range()))
    }
}
