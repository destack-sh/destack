use crate::parse::context::{AwaitContext, ExpressionContext, FunctionContext, YieldContext};
use crate::{Parser, ParserError, ParserResult};
use destack_core::StringId;
use destack_dir::{
    Key, Keyword, Name, NodeType, ScalarLiteral, TokenLiteral, TokenSpan, TokenType,
};
use destack_source::ByteRange;
use std::str::FromStr;

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
        let raw = self.file.span_str(token.span);

        // validate escaped identifiers once after tokenization
        if token.token.is_identifier_escaped() {
            let Some(decoded) = Self::decode_identifier_unicode_escapes(raw) else {
                return Err(ParserError::unexpected(token));
            };
            if Keyword::from_str(&decoded).is_ok() || decoded.contains('\0') {
                return Err(ParserError::unexpected(token));
            }
        }
        let string_id = self.strings.intern(raw);
        Ok((string_id, token.token.range()))
    }

    /// Eat a binding identifier and return its string ID and byte range.
    #[inline]
    pub(crate) fn eat_binding_identifier_with_range(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<(StringId, ByteRange)> {
        self.report_forbidden_binding_identifier(function);

        self.eat_identifier_with_range()
    }

    /// Report a contextually reserved binding identifier while preserving its tree shape.
    pub(crate) fn report_forbidden_binding_identifier(&mut self, function: FunctionContext) {
        let keyword = self.peek_keyword();
        let is_forbidden = function.yield_context == YieldContext::Forbidden
            && keyword == Some(Keyword::Yield)
            || function.await_context == AwaitContext::Forbidden && keyword == Some(Keyword::Await);
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
        Ok((string_id, token.token.range()))
    }

    /// Peek an identifier that matches a given string.
    #[inline]
    pub fn peek_identifier_str(&self, string: &str) -> ParserResult<TokenSpan> {
        if self.peek_identifier_is(string) {
            self.require_token(TokenType::Identifier)
        } else {
            Err(ParserError::expected(
                self.peek_token().range(),
                TokenType::Identifier,
            ))
        }
    }

    /// Return true when the next token is an identifier matching a string.
    #[inline]
    pub fn peek_identifier_is(&self, string: &str) -> bool {
        if !self.peek_is(TokenType::Identifier) {
            return false;
        }

        self.peek_token_str() == string
    }

    /// Decode Unicode escapes in one identifier.
    fn decode_identifier_unicode_escapes(raw: &str) -> Option<String> {
        if !raw.contains('\\') {
            return Some(raw.to_string());
        }

        let mut decoded = String::with_capacity(raw.len());
        let mut chars = raw.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch != '\\' {
                decoded.push(ch);
                continue;
            }

            if chars.next() != Some('u') {
                return None;
            }

            let value = if matches!(chars.peek(), Some('{')) {
                chars.next();
                let mut value: u32 = 0;
                let mut digits = 0_usize;
                let mut significant_digits = 0_usize;
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        break;
                    }
                    let digit = next.to_digit(16)?;
                    digits += 1;
                    if digit != 0 || significant_digits > 0 {
                        significant_digits += 1;
                        if significant_digits > 6 {
                            return None;
                        }
                        value = value.checked_mul(16)?.checked_add(digit)?;
                    }
                    chars.next();
                }
                if digits == 0 || chars.next() != Some('}') {
                    return None;
                }
                value
            } else {
                let mut value: u32 = 0;
                for _ in 0..4 {
                    let next = chars.next()?;
                    let digit = next.to_digit(16)?;
                    value = value.checked_mul(16)?.checked_add(digit)?;
                }
                value
            };

            let decoded_char = char::from_u32(value)?;
            decoded.push(decoded_char);
        }

        Some(decoded)
    }

    /// Eat an identifier that matches a given string.
    #[inline]
    pub fn eat_identifier_str(&mut self, string: &str) -> ParserResult<StringId> {
        let span = self.peek_identifier_str(string)?;
        let string = self.file.span_str(span.span);
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
        let first_range = token.token.range();
        let mut last_range = first_range;
        let token_part = self.token_span_str(token);

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
            let token_part = self.token_span_str(token);

            // disallow escaped identifiers in tree literals
            if token.token.ty() == TokenType::Identifier && token_part.contains('\\') {
                return Err(ParserError::unexpected(token));
            }
            last_range = token.token.range();

            if is_kebab {
                // keep numeric kebab segments as is, uppercase identifier segments
                if token.token.ty() == TokenType::Literal {
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
    pub fn peek_string_literal(&self) -> ParserResult<TokenSpan> {
        let token = self.require_token(TokenType::Literal)?;
        match token.token.literal() {
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            }) => self.require_token(TokenType::Literal),
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

    /// Get the content of a string literal (without surrounding quotes).
    #[inline]
    pub fn string_literal_str(&self, token: TokenSpan) -> &str {
        let token_str = self.token_span_str(token);

        // keep the literal content stable even when the closing quote is missing
        if let Some(content) = token_str.strip_prefix('"') {
            return content.strip_suffix('"').unwrap_or(content);
        }

        if let Some(content) = token_str.strip_prefix('\'') {
            return content.strip_suffix('\'').unwrap_or(content);
        }

        token_str
    }

    /// Peek a numeric literal (int or float, for object keys).
    #[inline]
    pub fn peek_numeric_literal(&self) -> ParserResult<TokenSpan> {
        let token = self.peek_token_span();
        if token.token.ty() == TokenType::Literal {
            match token.token.literal() {
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
            Ok((Name::String(string_id), token.token.range()))
        }
        // error
        else {
            Err(ParserError::unexpected(self.peek_token_span()))
        }
    }

    /// Return true when the next token can start a key.
    #[inline]
    pub fn peek_key_start(&self) -> bool {
        self.peek_key_name_start()
            || self.peek_is(TokenType::OpenBracket)
            || self.peek_numeric_literal_start()
    }

    /// Return true when the next token can be used as a key name.
    #[inline]
    fn peek_key_name_start(&self) -> bool {
        self.peek_name_start() || self.peek_boolean_name_start()
    }

    /// Return true when the next token is a boolean literal key.
    #[inline]
    fn peek_boolean_name_start(&self) -> bool {
        if self.peek_token_type() != TokenType::Literal {
            return false;
        }

        matches!(
            self.peek_token().literal(),
            Some(TokenLiteral::Boolean { .. })
        )
    }

    /// Eat a key name and return its value and byte range.
    #[inline]
    fn eat_key_name_with_range(&mut self) -> ParserResult<(Name, ByteRange)> {
        // identifier or string key name
        if self.peek_name_start() {
            return self.eat_name_with_range();
        }

        // boolean identifier name key
        if self.peek_boolean_name_start() {
            let token = self.eat();
            let raw = self.token_span_str(token).to_owned();
            let string_id = self.strings.intern(&raw);
            return Ok((Name::Identifier(string_id), token.token.range()));
        }

        // invalid key name
        Err(ParserError::unexpected(self.peek_token_span()))
    }

    /// Eat a name or dynamic key and return its value and byte range.
    pub(crate) fn eat_key_with_range(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<(Key, ByteRange)> {
        let start = self.mark_parse_start();
        // key name
        if self.peek_key_name_start() {
            let (name, range) = self.eat_key_name_with_range()?;
            Ok((Key::Name(name), range))
        }
        // index key
        else if self.peek_numeric_literal_start() {
            let (index, range) = self.eat_index_key_with_range()?;
            Ok((Key::Name(Name::Index(index)), range))
        }
        // dynamic key
        else if self.peek_is(TokenType::OpenBracket) {
            self.bump();

            // typed index signatures are parsed by the dedicated type property entrypoint
            let colon_range = if self.peek_is(TokenType::Identifier)
                && self.peek_token_type_at(1) == TokenType::Colon
            {
                Some(self.peek_token_at(1).range())
            } else {
                None
            };
            if let Some(colon_range) = colon_range {
                Err(ParserError::unexpected(colon_range))
            }
            // expression
            else {
                let key = self.parse_expression(ExpressionContext {
                    function,
                    ..ExpressionContext::default()
                })?;
                self.eat_close_token_or_recover_missing_with(
                    TokenType::CloseBracket,
                    NodeType::Expression,
                    |_, token_type| {
                        Self::is_close_delimiter_boundary_token(token_type)
                            || matches!(
                                token_type,
                                TokenType::Colon | TokenType::Maybe | TokenType::OpenParenthesis
                            )
                    },
                )?;
                Ok((Key::Expression(key), self.range_since(&start)))
            }
        }
        // error
        else {
            Err(ParserError::unexpected(self.peek_token_span()))
        }
    }

    /// Eat a name or dynamic key when present and return its byte range.
    pub(crate) fn eat_key_with_range_if_present(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<Option<(Key, ByteRange)>> {
        if self.peek_key_start() {
            Ok(Some(self.eat_key_with_range(function)?))
        } else {
            Ok(None)
        }
    }

    /// Eat an integer key token and return its index with its byte range.
    pub(in crate::parse) fn eat_index_key_with_range(
        &mut self,
    ) -> ParserResult<(usize, ByteRange)> {
        let token = self.peek_numeric_literal()?;

        // parse direct index keys through the literal grammar
        let numeric_literal = self.parse_scalar_literal()?;
        let ScalarLiteral::Integer(index) = numeric_literal else {
            return Err(ParserError::unexpected(token));
        };

        let index = usize::try_from(index).map_err(|_| ParserError::unexpected(token))?;

        Ok((index, token.token.range()))
    }
}
