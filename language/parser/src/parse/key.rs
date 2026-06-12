use crate::{Parser, ParserError, ParserResult};
use destack_core::StringId;
use destack_dir::{
    Key, Keyword, Name, NodeType, ScalarLiteral, TokenLiteral, TokenSpan, TokenType,
};
use destack_source::Span;

impl Parser {
    /// Peek an identifier.
    #[inline]
    pub fn peek_identifier(&mut self) -> ParserResult<TokenSpan> {
        self.peek_token(TokenType::Identifier)
    }

    /// Return true when the next token is an identifier.
    #[inline]
    pub fn peek_identifier_is(&mut self) -> bool {
        self.peek_is(TokenType::Identifier)
    }

    /// Eat an identifier.
    #[inline]
    pub fn eat_identifier(&mut self) -> ParserResult<StringId> {
        let (string_id, _) = self.eat_identifier_with_span()?;
        Ok(string_id)
    }

    /// Eat an identifier and return both the identifier and its span.
    #[inline]
    pub fn eat_identifier_with_span(&mut self) -> ParserResult<(StringId, Span)> {
        let token = self.eat_token(TokenType::Identifier)?;
        let raw = self.file.span_str(token.span);
        let has_escape = raw.as_bytes().contains(&b'\\');

        // reject escaped keywords in typed and untyped identifier forms
        if has_escape
            && (self.language.is_javascript() || self.language.is_typescript())
            && (self.identifier_is_escaped_keyword(raw)
                || self.identifier_has_disallowed_escape_code_point(raw))
        {
            return Err(ParserError::unexpected(token.span));
        }

        let string_id = self.strings.intern(raw);
        Ok((string_id, token.span))
    }

    /// Eat a binding identifier and return both the identifier and its span.
    #[inline]
    pub fn eat_binding_identifier_with_span(&mut self) -> ParserResult<(StringId, Span)> {
        self.eat_identifier_with_span()
    }

    /// Eat a string literal and return both the content and its span.
    #[inline]
    pub fn eat_string_literal_with_span(&mut self) -> ParserResult<(StringId, Span)> {
        let token = self.peek_string_literal()?;
        let content = self.get_string_literal_str(token).to_owned();
        let string_id = self.strings.intern(&content);
        self.bump();
        Ok((string_id, token.span))
    }

    /// Peek an identifier that matches a given string.
    #[inline]
    pub fn peek_identifier_str(&mut self, string: &str) -> ParserResult<TokenSpan> {
        if self.peek_identifier_str_is(string) {
            self.peek_token(TokenType::Identifier)
        } else {
            Err(ParserError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Return true when the next token is an identifier matching a string.
    #[inline]
    pub fn peek_identifier_str_is(&mut self, string: &str) -> bool {
        if !self.peek_identifier_is() {
            return false;
        }

        self.current_token_str() == string
    }

    // identifier keyword check with unicode escape decoding
    fn identifier_is_escaped_keyword(&self, raw: &str) -> bool {
        let Some(decoded) = self.decode_identifier_unicode_escapes(raw) else {
            return false;
        };
        <Keyword as std::str::FromStr>::from_str(&decoded).is_ok()
    }

    // reject escaped identifiers that decode to disallowed code points
    fn identifier_has_disallowed_escape_code_point(&self, raw: &str) -> bool {
        let Some(decoded) = self.decode_identifier_unicode_escapes(raw) else {
            return true;
        };

        decoded.chars().any(|character| character == '\0')
    }

    // decode unicode escapes in an identifier into a string
    fn decode_identifier_unicode_escapes(&self, raw: &str) -> Option<String> {
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
        let string = self.file.get_span_str(span.span).unwrap_or_default();
        let string_id = self.strings.intern(string);
        self.bump();
        Ok(string_id)
    }

    /// Eat a name maybe, returning both the name and its span.
    #[inline]
    pub fn eat_name_maybe_with_span(&mut self) -> ParserResult<Option<(Name, Span)>> {
        if self.peek_name_is() {
            Ok(Some(self.eat_name_with_span()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a tree literal identifier (`kebab-case` as `kebabCase`, namespaces allowed).
    #[inline]
    pub fn eat_tree_literal_identifier(&mut self) -> ParserResult<StringId> {
        let (string_id, _) = self.eat_tree_literal_identifier_with_span()?;
        Ok(string_id)
    }

    /// Eat a tree literal identifier and return both the identifier and its span.
    #[inline]
    pub fn eat_tree_literal_identifier_with_span(&mut self) -> ParserResult<(StringId, Span)> {
        let mut identifier = String::new();
        let token = self.eat_token(TokenType::Identifier)?;
        let first_span = token.span;
        let mut last_span = token.span;
        let token_part = self.get_token_span_str(token);

        // disallow escaped identifiers in tree literals
        if token_part.contains('\\') {
            return Err(ParserError::unexpected(token.span));
        }

        // uppercase first letter (except at start)
        if identifier.is_empty() {
            identifier.push_str(token_part);
        } else {
            // uppercase the first character
            let mut chars = token_part.chars();
            if let Some(first) = chars.next() {
                identifier.extend(first.to_uppercase());
                identifier.push_str(chars.as_str());
            }
        }

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
            let token = if is_kebab && self.peek_numeric_literal_is() {
                let token = self.peek_numeric_literal()?;
                self.bump();
                token
            } else {
                self.eat_token(TokenType::Identifier)?
            };
            let token_part = self.get_token_span_str(token);

            // disallow escaped identifiers in tree literals
            if token.token.ty() == TokenType::Identifier && token_part.contains('\\') {
                return Err(ParserError::unexpected(token.span));
            }
            last_span = token.span;

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
        let span = Span::new(first_span.file, first_span.start, last_span.end);

        Ok((string_id, span))
    }

    /// Peek a string literal.
    #[inline]
    pub fn peek_string_literal(&mut self) -> ParserResult<TokenSpan> {
        let token = self.peek_token(TokenType::Literal)?;
        match token.token.literal() {
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            }) => self.peek_token(TokenType::Literal),
            _ => Err(ParserError::expected(token.span, TokenType::Literal)),
        }
    }

    /// Return true when the next token is a valid string literal.
    #[inline]
    pub fn peek_string_literal_is(&mut self) -> bool {
        if self.peek_token_type() != TokenType::Literal {
            return false;
        }

        matches!(
            self.current_token().literal(),
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        )
    }

    /// Get the content of a string literal (without surrounding quotes).
    #[inline]
    pub fn get_string_literal_str(&self, token: TokenSpan) -> &str {
        let token_str = self.get_token_span_str(token);

        // keep the literal content stable even when the closing quote is missing
        if let Some(content) = token_str.strip_prefix('"') {
            return content.strip_suffix('"').unwrap_or(content);
        }

        if let Some(content) = token_str.strip_prefix('\'') {
            return content.strip_suffix('\'').unwrap_or(content);
        }

        token_str
    }

    /// Peek a next string literal.
    #[inline]
    pub fn peek_next_string_literal(&mut self) -> ParserResult<TokenSpan> {
        let token = self.next_token();
        let span = token.span(self.file_id);
        if !token.is(TokenType::Literal) {
            return Err(ParserError::expected(span, TokenType::Literal));
        }

        match token.literal() {
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            }) => Ok(TokenSpan::new(token, self.file_id)),
            _ => Err(ParserError::expected(span, TokenType::Literal)),
        }
    }

    /// Return true when the next token after current is a valid string literal.
    #[inline]
    pub fn peek_next_string_literal_is(&mut self) -> bool {
        let token = self.next_token();
        if !token.is(TokenType::Literal) {
            return false;
        }

        matches!(
            token.literal(),
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        )
    }

    /// Peek a numeric literal (int or float, for object keys).
    #[inline]
    pub fn peek_numeric_literal(&mut self) -> ParserResult<TokenSpan> {
        let token = self.peek()?;
        if token.token.ty() == TokenType::Literal {
            match token.token.literal() {
                Some(TokenLiteral::Int { .. }) | Some(TokenLiteral::Float { .. }) => Ok(token),
                _ => Err(ParserError::expected(token.span, TokenType::Literal)),
            }
        } else {
            Err(ParserError::expected(token.span, TokenType::Literal))
        }
    }

    /// Return true when the next token is a numeric literal.
    #[inline]
    pub fn peek_numeric_literal_is(&mut self) -> bool {
        if self.peek_token_type() != TokenType::Literal {
            return false;
        }

        matches!(
            self.current_token().literal(),
            Some(TokenLiteral::Int { .. }) | Some(TokenLiteral::Float { .. })
        )
    }

    /// Return true when the next token is a name.
    #[inline]
    pub fn peek_name_is(&mut self) -> bool {
        self.peek_identifier_is() || self.peek_string_literal_is()
    }

    /// Return true when the next token after current is a name.
    #[inline]
    pub fn peek_next_name_is(&mut self) -> bool {
        self.token_type_at_offset(1) == TokenType::Identifier || self.peek_next_string_literal_is()
    }

    /// Eat a name and return both the name and its span.
    pub fn eat_name_with_span(&mut self) -> ParserResult<(Name, Span)> {
        // regular identifier
        if self.peek_is(TokenType::Identifier) {
            let (name, span) = self.eat_identifier_with_span()?;
            Ok((Name::Identifier(name), span))
        }
        // string identifier
        else if self.peek_string_literal_is() {
            let token = self.peek_string_literal()?;
            let content = self.get_string_literal_str(token).to_owned();
            let string_id = self.strings.intern(&content);
            self.bump();
            Ok((Name::String(string_id), token.span))
        }
        // error
        else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Return true when the next token can start a key.
    #[inline]
    pub fn peek_key_is(&mut self) -> bool {
        self.peek_private_hash_key_is()
            || self.peek_key_name_is()
            || self.peek_is(TokenType::OpenBracket)
            || self.peek_numeric_literal_is()
    }

    /// Return true when the next token can be used as a key name.
    #[inline]
    fn peek_key_name_is(&mut self) -> bool {
        self.peek_name_is() || self.peek_boolean_name_literal_is()
    }

    /// Return true when the next token is a boolean literal key.
    #[inline]
    fn peek_boolean_name_literal_is(&mut self) -> bool {
        if self.peek_token_type() != TokenType::Literal {
            return false;
        }

        matches!(
            self.current_token().literal(),
            Some(TokenLiteral::Boolean { .. })
        )
    }

    /// Eat a key name and return both the parsed name and its span.
    #[inline]
    fn eat_key_name_with_span(&mut self) -> ParserResult<(Name, Span)> {
        // identifier or string key name
        if self.peek_name_is() {
            return self.eat_name_with_span();
        }

        // boolean identifier name key
        if self.peek_boolean_name_literal_is() {
            let token = self.eat()?;
            let raw = self.get_token_span_str(token).to_owned();
            let string_id = self.strings.intern(&raw);
            return Ok((Name::Identifier(string_id), token.span));
        }

        // invalid key name
        Err(ParserError::unexpected(self.peek()?.span))
    }

    /// Return true when the next tokens start a private hash key.
    #[inline]
    fn peek_private_hash_key_is(&mut self) -> bool {
        if !self.flags.allows_private_hash_key()
            || !(self.language.is_javascript() || self.language.is_typescript())
            || !self.peek_is(TokenType::Hash)
        {
            return false;
        }

        // require the hash and identifier to be adjacent
        let hash_span = self.current_token().span(self.file_id);
        let token = self.next_token();

        token.is(TokenType::Identifier) && hash_span.end == token.start()
    }

    /// Eat a name or a dynamic key, returning both the key and its span.
    pub fn eat_key_with_span(&mut self) -> ParserResult<(Key, Span)> {
        let start = self.span_start();
        // private hash key
        if self.peek_private_hash_key_is() {
            self.bump(); // eat #
            let name = self.eat_identifier()?;
            let span = self.get_span_from(&start);
            Ok((Key::Private(name), span))
        }
        // key name
        else if self.peek_key_name_is() {
            let (name, span) = self.eat_key_name_with_span()?;
            Ok((Key::Name(name), span))
        }
        // index key
        else if self.peek_numeric_literal_is() {
            let (index, span) = self.eat_index_key_with_span()?;
            Ok((Key::Name(Name::Index(index)), span))
        }
        // dynamic key
        else if self.peek_is(TokenType::OpenBracket) {
            self.bump(); // eat open bracket

            // typed index signatures are parsed by the dedicated type property entrypoint
            let colon_span = if self.peek_is(TokenType::Identifier)
                && self.token_type_at_offset(1) == TokenType::Colon
            {
                Some(self.token_at_offset(1).span(self.file_id))
            } else {
                None
            };
            if let Some(colon_span) = colon_span {
                Err(ParserError::unexpected(colon_span))
            }
            // expression
            else {
                let key =
                    self.eat_expression(self.flags.not_in_position().not_in_sequence_expression())?;
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
                Ok((Key::Expression(key), self.get_span_from(&start)))
            }
        }
        // error
        else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a name or a dynamic key maybe, returning both the key and its span.
    pub fn eat_key_maybe_with_span(&mut self) -> ParserResult<Option<(Key, Span)>> {
        if self.peek_key_is() {
            Ok(Some(self.eat_key_with_span()?))
        } else {
            Ok(None)
        }
    }

    /// Eat an integer key token and return its index with its span.
    pub(in crate::parse) fn eat_index_key_with_span(&mut self) -> ParserResult<(usize, Span)> {
        let token = self.peek_numeric_literal()?;

        // parse direct index keys through the literal grammar
        let numeric_literal = self.eat_scalar_literal()?;
        let ScalarLiteral::Integer(index) = numeric_literal else {
            return Err(ParserError::unexpected(token.span));
        };

        let index = usize::try_from(index).map_err(|_| ParserError::unexpected(token.span))?;

        Ok((index, token.span))
    }
}
