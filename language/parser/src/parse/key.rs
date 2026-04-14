use crate::{ParseError, ParseResult, Parser};
use destack_ast::{Key, Keyword, LiteralType, Name, NodeType, ScalarLiteral, TokenSpan, TokenType};
use destack_core::StringId;
use destack_source::Span;

impl Parser {
    /// Peek an identifier.
    #[inline]
    pub fn peek_identifier(&mut self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Identifier)
    }

    /// Return true when the next token is an identifier.
    #[inline]
    pub fn peek_identifier_is(&mut self) -> bool {
        self.peek_is(TokenType::Identifier)
    }

    /// Eat an identifier.
    #[inline]
    pub fn eat_identifier(&mut self) -> ParseResult<StringId> {
        let (string_id, _) = self.eat_identifier_with_span()?;
        Ok(string_id)
    }

    /// Eat an identifier and return both the identifier and its span.
    #[inline]
    pub fn eat_identifier_with_span(&mut self) -> ParseResult<(StringId, Span)> {
        let index = self.pos_index();
        let token = *self.eat_token(TokenType::Identifier)?;
        let has_escape = self.identifier_has_escape_for_index(index);

        // reject escaped keywords in JS/TS
        if has_escape && (self.language.is_javascript() || self.language.is_typescript()) {
            let raw = self.file.span_str(token.span);
            if self.identifier_is_escaped_keyword(raw)
                || self.identifier_has_disallowed_escape_code_point(raw)
            {
                return Err(ParseError::unexpected(token.span));
            }
        }

        let cached = self.identifier_for_index(index);
        let string_id = cached.unwrap_or_else(|| {
            let raw = self.file.span_str(token.span);
            self.strings.intern(raw)
        });
        Ok((string_id, token.span))
    }

    /// Eat a binding identifier.
    #[inline]
    pub fn eat_binding_identifier(&mut self) -> ParseResult<StringId> {
        let (string_id, _) = self.eat_binding_identifier_with_span()?;
        Ok(string_id)
    }

    /// Eat a binding identifier and return both the identifier and its span.
    #[inline]
    pub fn eat_binding_identifier_with_span(&mut self) -> ParseResult<(StringId, Span)> {
        self.eat_identifier_with_span()
    }

    /// Eat a string literal and return both the content and its span.
    #[inline]
    pub fn eat_string_literal_with_span(&mut self) -> ParseResult<(StringId, Span)> {
        let token = *self.peek_string_literal()?;
        let content = self.get_string_literal_str(token).to_owned();
        let string_id = self.strings.intern(&content);
        self.bump();
        Ok((string_id, token.span))
    }

    /// Peek an identifier that matches a given string.
    #[inline]
    pub fn peek_identifier_str(&mut self, string: &str) -> ParseResult<&TokenSpan> {
        if self.peek_identifier_str_is(string) {
            self.peek_token(TokenType::Identifier)
        } else {
            Err(ParseError::expected(
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

        self.identifier_equals_at(self.pos_index(), string)
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
    pub fn eat_identifier_str(&mut self, string: &str) -> ParseResult<StringId> {
        let span = *self.peek_identifier_str(string)?;
        if let Some(id) = self.identifier_for_index(self.pos_index()) {
            self.bump();
            return Ok(id);
        }
        let string = self.file.get_span_str(span.span).unwrap_or_default();
        let string_id = self.strings.intern(string);
        self.bump();
        Ok(string_id)
    }

    /// Eat a name maybe.
    #[inline]
    pub fn eat_name_maybe(&mut self) -> ParseResult<Option<Name>> {
        if self.peek_name_is() {
            Ok(Some(self.eat_name()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a name maybe, returning both the name and its span.
    #[inline]
    pub fn eat_name_maybe_with_span(&mut self) -> ParseResult<Option<(Name, Span)>> {
        if self.peek_name_is() {
            Ok(Some(self.eat_name_with_span()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a tree literal identifier (`kebab-case` as `kebabCase`, namespaces allowed).
    #[inline]
    pub fn eat_tree_literal_identifier(&mut self) -> ParseResult<StringId> {
        let (string_id, _) = self.eat_tree_literal_identifier_with_span()?;
        Ok(string_id)
    }

    /// Eat a tree literal identifier and return both the identifier and its span.
    #[inline]
    pub fn eat_tree_literal_identifier_with_span(&mut self) -> ParseResult<(StringId, Span)> {
        let mut identifier = String::new();
        let token = *self.eat_token(TokenType::Identifier)?;
        let mut last_span = token.span;
        let token_part = self.get_token_str(token);

        // disallow escaped identifiers in tree literals
        if token_part.contains('\\') {
            return Err(ParseError::unexpected(token.span));
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
                let token = *self.peek_numeric_literal()?;
                self.bump();
                token
            } else {
                *self.eat_token(TokenType::Identifier)?
            };
            let token_part = self.get_token_str(token);

            // disallow escaped identifiers in tree literals
            if token.token.ty == TokenType::Identifier && token_part.contains('\\') {
                return Err(ParseError::unexpected(token.span));
            }
            last_span = token.span;

            if is_kebab {
                // keep numeric kebab segments as is, uppercase identifier segments
                if token.token.ty == TokenType::Literal {
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
        let string_id = self.strings.intern(identifier);
        Ok((string_id, last_span))
    }

    /// Peek a string literal.
    #[inline]
    pub fn peek_string_literal(&mut self) -> ParseResult<&TokenSpan> {
        let token = *self.peek_token(TokenType::Literal)?;
        match token.token.literal {
            Some(LiteralType::String {
                is_terminated: true,
                has_invalid_escape: false,
            }) => self.peek_token(TokenType::Literal),
            Some(LiteralType::Character { is_terminated, .. })
                if is_terminated
                    && (self.language.is_typescript() || self.language.is_javascript()) =>
            {
                self.peek_token(TokenType::Literal)
            }
            _ => Err(ParseError::expected(token.span, TokenType::Literal)),
        }
    }

    /// Return true when the next token is a valid string literal.
    #[inline]
    pub fn peek_string_literal_is(&mut self) -> bool {
        if self.peek_token_type() != TokenType::Literal {
            return false;
        }

        let Some(token) = self.token_at(self.pos_index()) else {
            return false;
        };

        matches!(
            token.token.literal,
            Some(LiteralType::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ) || matches!(token.token.literal, Some(LiteralType::Character { is_terminated: true, .. })
        if self.language.is_typescript() || self.language.is_javascript())
    }

    /// Get the content of a string literal (without surrounding quotes).
    #[inline]
    pub fn get_string_literal_str(&self, token: TokenSpan) -> &str {
        let token_str = self.get_token_str(token);

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
    pub fn peek_next_string_literal(&mut self) -> ParseResult<&TokenSpan> {
        let token = *self.peek_next_token(TokenType::Literal)?;
        if token.token.ty == TokenType::Literal {
            match token.token.literal {
                Some(LiteralType::String {
                    is_terminated: true,
                    has_invalid_escape: false,
                }) => self.peek_next_token(TokenType::Literal),
                Some(LiteralType::Character { is_terminated, .. })
                    if is_terminated
                        && (self.language.is_typescript() || self.language.is_javascript()) =>
                {
                    self.peek_next_token(TokenType::Literal)
                }
                _ => Err(ParseError::expected(token.span, TokenType::Literal)),
            }
        } else {
            Err(ParseError::expected(token.span, TokenType::Literal))
        }
    }

    /// Return true when the next token after current is a valid string literal.
    #[inline]
    pub fn peek_next_string_literal_is(&mut self) -> bool {
        if self.peek_next_token_type() != TokenType::Literal {
            return false;
        }

        let next_index = self.index_for_next();
        let Some(token) = self.token_at(next_index) else {
            return false;
        };

        matches!(
            token.token.literal,
            Some(LiteralType::String {
                is_terminated: true,
                has_invalid_escape: false,
            })
        ) || matches!(token.token.literal, Some(LiteralType::Character { is_terminated: true, .. })
        if self.language.is_typescript() || self.language.is_javascript())
    }

    /// Peek a numeric literal (int or float, for object keys).
    #[inline]
    pub fn peek_numeric_literal(&mut self) -> ParseResult<&TokenSpan> {
        let token = self.peek()?;
        if token.token.ty == TokenType::Literal {
            match token.token.literal {
                Some(LiteralType::Int { .. }) | Some(LiteralType::Float { .. }) => Ok(token),
                _ => Err(ParseError::expected(token.span, TokenType::Literal)),
            }
        } else {
            Err(ParseError::expected(token.span, TokenType::Literal))
        }
    }

    /// Return true when the next token is a numeric literal.
    #[inline]
    pub fn peek_numeric_literal_is(&mut self) -> bool {
        if self.peek_token_type() != TokenType::Literal {
            return false;
        }

        let Some(token) = self.token_at(self.pos_index()) else {
            return false;
        };

        matches!(
            token.token.literal,
            Some(LiteralType::Int { .. }) | Some(LiteralType::Float { .. })
        )
    }

    /// Peek a name (like `x` or `"Content-Type"`).
    #[inline]
    pub fn peek_name(&mut self) -> ParseResult<()> {
        if self.peek_name_is() {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Return true when the next token is a name.
    #[inline]
    pub fn peek_name_is(&mut self) -> bool {
        self.peek_identifier_is() || self.peek_string_literal_is()
    }

    /// Peek a next name (like `x` or `"Content-Type"`).
    #[inline]
    pub fn peek_next_name(&mut self) -> ParseResult<()> {
        if self.peek_next_name_is() {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek_next()?.span))
        }
    }

    /// Return true when the next token after current is a name.
    #[inline]
    pub fn peek_next_name_is(&mut self) -> bool {
        self.peek_next_is(TokenType::Identifier) || self.peek_next_string_literal_is()
    }

    /// Eat a name (like `x` or `"Content-Type"`).
    #[inline]
    pub fn eat_name(&mut self) -> ParseResult<Name> {
        let (name, _span) = self.eat_name_with_span()?;
        Ok(name)
    }

    /// Eat a name and return both the name and its span.
    #[inline]
    pub fn eat_name_with_span(&mut self) -> ParseResult<(Name, Span)> {
        // regular identifier
        if self.peek_is(TokenType::Identifier) {
            let (name, span) = self.eat_identifier_with_span()?;
            Ok((Name::Identifier(name), span))
        }
        // string identifier
        else if self.peek_string_literal_is() {
            let token = *self.peek_string_literal()?;
            let content = self.get_string_literal_str(token).to_owned();
            let string_id = self.strings.intern(&content);
            self.bump();
            Ok((Name::String(string_id), token.span))
        }
        // error
        else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Peek a name or a dynamic key.
    #[inline]
    pub fn peek_key(&mut self) -> ParseResult<()> {
        if self.peek_key_is() {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
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

        let Some(token) = self.token_at(self.pos_index()) else {
            return false;
        };

        matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
    }

    /// Eat a key name and return both the parsed name and its span.
    #[inline]
    fn eat_key_name_with_span(&mut self) -> ParseResult<(Name, Span)> {
        // identifier or string key name
        if self.peek_name_is() {
            return self.eat_name_with_span();
        }

        // boolean identifier name key
        if self.peek_boolean_name_literal_is() {
            let token = *self.eat()?;
            let raw = self.get_token_str(token).to_owned();
            let string_id = self.strings.intern(&raw);
            return Ok((Name::Identifier(string_id), token.span));
        }

        // invalid key name
        Err(ParseError::unexpected(self.peek()?.span))
    }

    /// Return true when the next tokens start a private hash key.
    #[inline]
    fn peek_private_hash_key_is(&mut self) -> bool {
        if !self.options.allows_private_hash_key()
            || !(self.language.is_javascript()
                || self.language.is_typescript()
                || self.language.is_destack())
            || !self.peek_is(TokenType::Hash)
            || !self.peek_next_is(TokenType::Identifier)
        {
            return false;
        }

        // require the hash and identifier to be adjacent
        let hash_index = self.pos_index();
        let ident_index = hash_index + 1;
        self.tokens_are_adjacent(hash_index, ident_index)
    }

    /// Eat a name or a dynamic key.
    #[inline]
    pub fn eat_key(&mut self) -> ParseResult<Key> {
        // private hash key
        if self.peek_private_hash_key_is() {
            self.bump(); // eat #
            let name = self.eat_identifier()?;
            Ok(Key::Private(name))
        }
        // key name
        else if self.peek_key_name_is() {
            let (name, _span) = self.eat_key_name_with_span()?;
            Ok(Key::Name(name))
        }
        // numeric key
        else if self.peek_numeric_literal_is() {
            let (string_id, _span) = self.eat_numeric_key_name_with_span()?;
            Ok(Key::Name(Name::Number(string_id)))
        }
        // dynamic key
        else if self.peek_is(TokenType::OpenBracket) {
            self.bump(); // eat open bracket
            self.eat_newlines_maybe()?;

            // typed index signatures are parsed by the dedicated type property entrypoint
            let has_named_type_head = self.peek_is(TokenType::Identifier)
                && (self.peek_next_is(TokenType::Colon)
                    || self.is_token_after_newlines(self.pos(), TokenType::Colon));
            if has_named_type_head {
                let colon_index = self.next_non_newline_index_from(self.pos_index() + 1);
                let colon_span = self
                    .token_ref_at(colon_index)
                    .map(|token| token.span)
                    .unwrap_or(self.peek()?.span);
                Err(ParseError::unexpected(colon_span))
            }
            // expression
            else {
                let key = self.eat_expression(
                    self.options
                        .not_in_position()
                        .not_in_left_precedence()
                        .not_in_sequence_expression(),
                )?;
                self.eat_newlines_maybe()?;
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
                Ok(Key::Expression(key))
            }
        }
        // error
        else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a name or a dynamic key maybe.
    pub fn eat_key_maybe(&mut self) -> ParseResult<Option<Key>> {
        if self.peek_key_is() {
            Ok(Some(self.eat_key()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a name or a dynamic key, returning both the key and its span.
    pub fn eat_key_with_span(&mut self) -> ParseResult<(Key, Span)> {
        let start = self.mark_span();
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
        // numeric key
        else if self.peek_numeric_literal_is() {
            let (string_id, span) = self.eat_numeric_key_name_with_span()?;
            Ok((Key::Name(Name::Number(string_id)), span))
        }
        // dynamic key
        else if self.peek_is(TokenType::OpenBracket) {
            self.bump(); // eat open bracket
            self.eat_newlines_maybe()?;

            // typed index signatures are parsed by the dedicated type property entrypoint
            let has_named_type_head = self.peek_is(TokenType::Identifier)
                && (self.peek_next_is(TokenType::Colon)
                    || self.is_token_after_newlines(self.pos(), TokenType::Colon));
            if has_named_type_head {
                let colon_index = self.next_non_newline_index_from(self.pos_index() + 1);
                let colon_span = self
                    .token_ref_at(colon_index)
                    .map(|token| token.span)
                    .unwrap_or(self.peek()?.span);
                Err(ParseError::unexpected(colon_span))
            }
            // expression
            else {
                let key = self.eat_expression(
                    self.options
                        .not_in_position()
                        .not_in_left_precedence()
                        .not_in_sequence_expression(),
                )?;
                self.eat_newlines_maybe()?;
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
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a name or a dynamic key maybe, returning both the key and its span.
    pub fn eat_key_maybe_with_span(&mut self) -> ParseResult<Option<(Key, Span)>> {
        if self.peek_key_is() {
            Ok(Some(self.eat_key_with_span()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a numeric key token and return the raw key text with its span.
    fn eat_numeric_key_name_with_span(&mut self) -> ParseResult<(StringId, Span)> {
        // capture the original numeric token text for key identity
        let token = *self.peek_numeric_literal()?;
        let key_string = self.file.span_str(token.span).to_string();

        // parse and validate numeric literal grammar in key position
        // keep the original raw token text, object key identity is source text not normalized value
        let numeric_literal = self.eat_scalar_literal()?;
        if !matches!(
            numeric_literal,
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) | ScalarLiteral::Bigint(_)
        ) {
            return Err(ParseError::unexpected(token.span));
        }

        let key_name = self.strings.intern(key_string);
        Ok((key_name, token.span))
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{Expression, IfCondition, IfKind, Key, ScalarLiteral};
    use destack_source::LanguageType;

    use crate::tests::TestParser;
    use crate::{assert_expression_path, assert_node, assert_string};

    #[test]
    fn test_reject_key_named_type_expression_with_multiline_type() {
        let mut test = TestParser::new(
            r#"[key:
    | string
    | number]"#,
        );
        let mut parser = test.prepare();
        let error = parser.eat_key_with_span().unwrap_err();
        assert_eq!(parser.get_span_str(error.leaf_span()), ":");
    }

    #[test]
    fn test_reject_key_named_type_expression_with_newlines_before_colon_and_close_bracket() {
        let mut test = TestParser::new(
            r#"[key
:
string
]"#,
        );
        let mut parser = test.prepare();
        let error = parser.eat_key_with_span().unwrap_err();
        assert_eq!(parser.get_span_str(error.leaf_span()), ":");
    }

    /// Reject computed keys with sequence expressions in javascript.
    #[test]
    fn test_reject_key_computed_sequence_expression_javascript() {
        // source: [a,b]
        let mut test = TestParser::new_with_options("[a,b]", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_key_with_span().unwrap_err();

        // ,
        assert_eq!(parser.get_span_str(error.leaf_span()), ",");
    }

    /// Reject legacy octal numeric keys in javascript.
    #[test]
    fn test_reject_legacy_octal_numeric_key_javascript() {
        // source: 021
        let mut test = TestParser::new_with_options("021", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_key_with_span().unwrap_err();

        // 021
        assert_eq!(parser.get_span_str(error.leaf_span()), "021");
    }

    /// Parse computed keys with ternaries even when outer left precedence is set.
    #[test]
    fn test_parse_key_computed_ternary_with_outer_left_precedence() {
        let mut test = TestParser::new_with_options(
            "[hasCjsFormat ? 'module' : 'import']",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.options = parser.options.in_left_precedence(1);

        let (key, _span) = parser.eat_key_with_span().unwrap();

        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
        assert!(matches!(key, Key::Expression(_)));
        assert_node!(parser.tree, match key { Key::Expression(key) => key, _ => unreachable!() }, Expression::If { kind, condition, then_expression, else_expression } => {
            assert_eq!(*kind, IfKind::Ternary);
            assert_node!(condition, IfCondition::Expression { condition } => {
                assert_expression_path!(parser, parser.tree.get(*condition), "hasCjsFormat");
            });
            assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
                assert_string!(parser, *string, "module");
            });
            assert_node!(parser.tree, else_expression.expect("expected else"), Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
                assert_string!(parser, *string, "import");
            });
        });
    }
}
