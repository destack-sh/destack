use std::borrow::Cow;

use crate::lex::decode_html_entity;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Argument, Expression, Keyword, LiteralType, LocalNodeId, NodeType, NumberBase, Path, Property,
    ScalarLiteral, StringId, TemplateLiteral, TokenSpan, TokenType,
};

impl Parser {
    /// Return true when the current token starts a template literal.
    #[inline]
    pub fn is_template_literal_start(&mut self) -> bool {
        self.peek_is(TokenType::TemplateString) || self.peek_is(TokenType::TemplateStringStart)
    }

    /// Return true when the current token starts a scalar literal.
    #[inline]
    pub fn is_scalar_literal_start(&mut self) -> bool {
        self.peek_is(TokenType::Literal)
    }

    /// Return true when the current token starts a tree literal.
    #[inline]
    pub fn is_tree_literal_start(&mut self) -> bool {
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }

        let pos = self.pos_index();
        let next = self.next_non_newline_index_from_stream(pos + 1);
        let Some(next_token) = self.token_ref_at(next).copied() else {
            return false;
        };
        if next_token.token.ty == TokenType::Divide && !self.options.in_tree_literal {
            return false;
        }
        if !matches!(
            next_token.token.ty,
            TokenType::GreaterThan | TokenType::Divide | TokenType::Identifier
        ) {
            return false;
        }

        if next_token.token.ty == TokenType::Identifier {
            let after_identifier = self.next_non_newline_index_from_stream(next + 1);
            if self
                .token_ref_at(after_identifier)
                .is_some_and(|token| token.token.ty == TokenType::Comma)
            {
                return false;
            }
        }

        true
    }

    /// Peek a scalar literal token.
    #[inline]
    pub fn peek_scalar_literal(&mut self) -> ParseResult<&TokenSpan> {
        if self.peek_is(TokenType::Literal) {
            Ok(self.peek()?)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a scalar literal and return its value.
    ///
    /// Examples:
    /// ```
    /// true
    /// false
    /// 1
    /// 1n
    /// 1.0
    /// 0x1234
    /// "Hello, world!"
    /// 'a'
    /// b'a'
    /// b"abc
    /// /abc/
    /// /abc/g
    /// ```
    pub fn eat_scalar_literal(&mut self) -> ParseResult<ScalarLiteral> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        let literal_span = *self.eat()?;
        let Some(body) = literal_span.token.literal else {
            return Err(ParseError::unexpected(literal_span.span));
        };
        let has_invalid_numeric_suffix =
            self.numeric_literal_has_invalid_js_ts_suffix(literal_span);
        let literal_str = self.file.span_str(literal_span.span);

        match body {
            // boolean literal
            LiteralType::Boolean { value } => Ok(ScalarLiteral::Boolean(value)),

            // int literal
            LiteralType::Int {
                base,
                is_empty,
                is_bigint,
            } => {
                if is_empty {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // JS/TS reject legacy leading-zero decimal forms
                if (self.language.is_javascript() || self.language.is_typescript())
                    && self.int_literal_is_invalid_js_ts(literal_str, base, is_bigint)
                {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // reject legacy octal literals without an explicit 0o/0O prefix
                if self.int_literal_is_legacy_octal(literal_str, base, is_bigint) {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // JS/TS reject invalid digits for binary, octal, and hexadecimal literals
                if (self.language.is_javascript() || self.language.is_typescript())
                    && self.int_literal_has_invalid_digits(literal_str, base, is_bigint)
                {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // JS/TS require a separator after numeric literals before identifier starts
                if has_invalid_numeric_suffix {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // strip underscores for parsing
                let content: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                // strip bigint suffix
                let content = if is_bigint {
                    Cow::Borrowed(content.trim_end_matches("n"))
                } else {
                    content
                };

                // handle base-specific prefixes
                let digits = match base {
                    NumberBase::Decimal => content,
                    NumberBase::Binary => {
                        Cow::Borrowed(self.strip_radix_prefix(&content, NumberBase::Binary))
                    }
                    NumberBase::Octal => {
                        Cow::Borrowed(self.strip_radix_prefix(&content, NumberBase::Octal))
                    }
                    NumberBase::Hexadecimal => {
                        Cow::Borrowed(self.strip_radix_prefix(&content, NumberBase::Hexadecimal))
                    }
                };

                // parse with saturation to avoid overflow errors in parsing
                let parsed = self.parse_int_literal_saturating(&digits, base);

                // int
                if is_bigint {
                    Ok(ScalarLiteral::Bigint(parsed))
                } else {
                    Ok(ScalarLiteral::Integer(parsed))
                }
            }

            // float literal
            LiteralType::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // JS/TS reject legacy leading-zero decimal forms
                if (self.language.is_javascript() || self.language.is_typescript())
                    && self.float_literal_is_invalid_js_ts(literal_str)
                {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // JS/TS require a separator after numeric literals before identifier starts
                if has_invalid_numeric_suffix {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                let content: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                content
                    .parse::<f64>()
                    .map(ScalarLiteral::Float)
                    .map_err(|_| {
                        ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // character literal (ignore quotes)
            LiteralType::Character {
                is_terminated,
                is_html_entity,
            } => {
                if is_html_entity {
                    decode_html_entity(literal_str)
                        .map(ScalarLiteral::Character)
                        .ok_or_else(|| {
                            ParseError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        })
                } else {
                    if !is_terminated {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    let content = literal_str.trim_start_matches('\'').trim_end_matches('\'');
                    content
                        .chars()
                        .next()
                        .map(ScalarLiteral::Character)
                        .ok_or_else(|| {
                            ParseError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        })
                }
            }

            // string literal (ignore quotes)
            // supports both '...' and "..." delimited string literals
            LiteralType::String {
                is_terminated,
                has_invalid_escape,
            } => {
                if !is_terminated || has_invalid_escape {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = {
                    // "..." or '...'
                    if literal_str.len() >= 2
                        && ((literal_str.starts_with('"') && literal_str.ends_with('"'))
                            || (literal_str.starts_with('\'') && literal_str.ends_with('\'')))
                    {
                        &literal_str[1..literal_str.len() - 1]
                    }
                    // unexpected (error elsewhere)
                    else {
                        literal_str
                    }
                };

                let string_id = self.strings.intern(content);
                Ok(ScalarLiteral::String(string_id))
            }

            // regex string literal (ignore quotes)
            LiteralType::RegexString { has_flags } => {
                // regex literals require a closing slash
                if !literal_str.starts_with('/') {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // regex without flags
                if !has_flags {
                    if !literal_str.ends_with('/') || literal_str.len() < 2 {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }

                    let content = &literal_str[1..literal_str.len() - 1];
                    if self.contains_js_line_terminator(content) {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    let string_id = self.strings.intern(content);
                    Ok(ScalarLiteral::RegexString {
                        content: string_id,
                        flags: None,
                    })
                }
                // regex with flags
                else {
                    let Some(last_slash_index) = literal_str.rfind('/') else {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    };
                    if last_slash_index == 0 {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }

                    let content = &literal_str[1..last_slash_index];
                    let flags = &literal_str[last_slash_index + 1..];
                    if flags.is_empty() || self.contains_js_line_terminator(content) {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    if !self.regex_flags_are_valid(flags) {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    if !self.regex_unicode_escapes_are_valid(content, flags) {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    let string_id = self.strings.intern(content);
                    let flags_id = self.strings.intern(flags);
                    Ok(ScalarLiteral::RegexString {
                        content: string_id,
                        flags: Some(flags_id),
                    })
                }
            }

            // tree text content (raw text inside tree literals, like JSX text)
            LiteralType::TreeString => {
                let string_id = self.strings.intern(literal_str);
                Ok(ScalarLiteral::String(string_id))
            }
        }
    }

    /// Parse an integer literal with saturation for values outside i64.
    fn parse_int_literal_saturating(&self, digits: &str, base: NumberBase) -> i64 {
        // select radix for the literal base
        let radix = match base {
            NumberBase::Decimal => 10,
            NumberBase::Binary => 2,
            NumberBase::Octal => 8,
            NumberBase::Hexadecimal => 16,
        };

        // accumulate digits with overflow detection
        let mut value: u128 = 0;
        let max_value = i64::MAX as u128;
        for ch in digits.chars() {
            let Some(digit) = ch.to_digit(radix) else {
                return i64::MAX;
            };
            let digit = digit as u128;
            let next = value
                .checked_mul(radix as u128)
                .and_then(|value| value.checked_add(digit));
            let Some(next) = next else {
                return i64::MAX;
            };
            if next > max_value {
                return i64::MAX;
            }
            value = next;
        }

        // finalize with saturation
        value as i64
    }

    /// Strip a radix prefix from an integer literal body.
    fn strip_radix_prefix<'a>(&self, literal: &'a str, base: NumberBase) -> &'a str {
        match base {
            NumberBase::Decimal => literal,
            NumberBase::Binary => literal
                .strip_prefix("0b")
                .or_else(|| literal.strip_prefix("0B"))
                .unwrap_or(literal),
            NumberBase::Octal => literal
                .strip_prefix("0o")
                .or_else(|| literal.strip_prefix("0O"))
                .unwrap_or(literal),
            NumberBase::Hexadecimal => literal
                .strip_prefix("0x")
                .or_else(|| literal.strip_prefix("0X"))
                .unwrap_or(literal),
        }
    }

    /// Return true when an int literal uses legacy JS/TS leading-zero syntax.
    fn int_literal_is_invalid_js_ts(
        &self,
        literal: &str,
        base: NumberBase,
        is_bigint: bool,
    ) -> bool {
        if base != NumberBase::Decimal {
            return false;
        }

        let body = if is_bigint {
            literal.trim_end_matches('n')
        } else {
            literal
        };
        let bytes = body.as_bytes();

        if bytes.len() < 2 || bytes[0] != b'0' {
            return false;
        }

        let second = bytes[1] as char;
        second.is_ascii_digit() || second == '_'
    }

    /// Return true when an int literal uses a legacy octal form.
    fn int_literal_is_legacy_octal(
        &self,
        literal: &str,
        base: NumberBase,
        is_bigint: bool,
    ) -> bool {
        if base != NumberBase::Octal {
            return false;
        }

        let body = if is_bigint {
            literal.trim_end_matches('n')
        } else {
            literal
        };

        body.starts_with('0') && !body.starts_with("0o") && !body.starts_with("0O")
    }

    /// Return true when an int literal contains digits that are invalid for its base.
    fn int_literal_has_invalid_digits(
        &self,
        literal: &str,
        base: NumberBase,
        is_bigint: bool,
    ) -> bool {
        let body = if is_bigint {
            literal.trim_end_matches('n')
        } else {
            literal
        };

        let digits = self.strip_radix_prefix(body, base);
        if digits.is_empty() {
            return true;
        }

        let radix = match base {
            NumberBase::Decimal => 10,
            NumberBase::Binary => 2,
            NumberBase::Octal => 8,
            NumberBase::Hexadecimal => 16,
        };

        for character in digits.chars() {
            if character == '_' {
                continue;
            }
            if character.to_digit(radix).is_none() {
                return true;
            }
        }

        false
    }

    /// Return true when a float literal uses legacy JS/TS leading-zero syntax.
    fn float_literal_is_invalid_js_ts(&self, literal: &str) -> bool {
        let bytes = literal.as_bytes();
        if bytes.len() < 2 || bytes[0] != b'0' {
            return false;
        }

        let second = bytes[1] as char;
        second.is_ascii_digit() || second == '_'
    }

    /// Return true when a JS/TS numeric literal is immediately followed by an identifier.
    fn numeric_literal_has_invalid_js_ts_suffix(&mut self, literal: TokenSpan) -> bool {
        if !(self.language.is_javascript() || self.language.is_typescript()) {
            return false;
        }

        let Ok(next) = self.peek() else {
            return false;
        };
        if next.token.ty != TokenType::Identifier {
            return false;
        }

        next.span.start == literal.span.end
    }

    /// Peek a template literal.
    #[inline]
    pub fn peek_template_literal(&mut self) -> ParseResult<&TokenSpan> {
        if self.peek_is(TokenType::TemplateString) || self.peek_is(TokenType::TemplateStringStart) {
            Ok(self.peek()?)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a template literal. The path (i.e. tag) must be passed in explicitly.
    ///
    /// Examples:
    /// ```
    /// `hello`
    /// `hello ${name}`
    /// `SELECT * FROM users`
    /// `${stmt}`
    /// `SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    pub fn eat_template_literal(&mut self) -> ParseResult<TemplateLiteral> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        let (strings, arguments) =
            self.eat_template_literal_parts(|parser| parser.eat_template_literal_argument())?;

        if arguments.is_empty() && strings.len() == 1 {
            Ok(TemplateLiteral::String { string: strings[0] })
        } else {
            Ok(TemplateLiteral::InterpolatedString { strings, arguments })
        }
    }

    /// Eat a type template literal.
    ///
    /// Examples:
    /// ```
    /// `${K}`
    /// `foo-${Bar}`
    /// ```
    pub fn eat_type_template_literal_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        let start = self.mark();
        let (strings, spans) = self.eat_template_literal_parts(|parser| {
            parser.with_options(parser.options.not_in_position().in_type(), |parser| {
                parser.eat_expression(parser.options)
            })
        })?;

        let expression = Expression::TypeTemplateLiteral { strings, spans };
        Ok(self.tree.insert(expression, self.get_span_from(&start)))
    }

    /// Eat the parts of a template literal.
    fn eat_template_literal_parts<T>(
        &mut self,
        mut parse_span: impl FnMut(&mut Parser) -> ParseResult<T>,
    ) -> ParseResult<(Vec<StringId>, Vec<T>)> {
        let next = *self.eat()?;
        let next_str = self.file.span_str(next.span);

        // template string without interpolation
        if next.token.ty == TokenType::TemplateString {
            let string = next_str.trim_start_matches('`').trim_end_matches('`');
            let string_id = self.strings.intern(string);
            return Ok((vec![string_id], Vec::new()));
        }
        // template string with interpolation
        if next.token.ty == TokenType::TemplateStringStart {
            let mut strings: Vec<StringId> = Vec::new();
            let mut spans: Vec<T> = Vec::new();

            // start: remove ` prefix and ${ suffix
            let string = next_str
                .strip_prefix('`')
                .and_then(|s| s.strip_suffix("${"))
                .unwrap_or("");
            let string_id = self.strings.intern(string);
            strings.push(string_id);

            // eat until the end
            while !self.peek_is(TokenType::TemplateStringEnd) {
                // string
                if self.peek_is(TokenType::TemplateStringMiddle) {
                    let token = *self.eat()?;
                    let token_str = self.file.span_str(token.span);
                    // remove } prefix and ${ suffix
                    let string = token_str
                        .strip_prefix('}')
                        .and_then(|s| s.strip_suffix("${"))
                        .unwrap_or("");
                    let string_id = self.strings.intern(string);
                    strings.push(string_id);
                }
                // span
                else {
                    self.eat_newlines_maybe()?;
                    let span = parse_span(self)?;
                    self.eat_newlines_maybe()?;
                    if !self.peek_is(TokenType::TemplateStringMiddle)
                        && !self.peek_is(TokenType::TemplateStringEnd)
                    {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }
                    spans.push(span);
                }
            }

            // end: remove } prefix and ` suffix
            let token = *self.eat_token(TokenType::TemplateStringEnd)?;
            let token_str = self.file.span_str(token.span);
            let string = token_str
                .strip_prefix('}')
                .and_then(|s| s.strip_suffix('`'))
                .unwrap_or("");
            let string_id = self.strings.intern(string);
            strings.push(string_id);

            return Ok((strings, spans));
        }

        Err(ParseError::unexpected(next.span))
    }

    /// Eat a template literal interpolation argument.
    ///
    /// Template literal interpolations parse as full expressions (no named args).
    pub fn eat_template_literal_argument(&mut self) -> ParseResult<LocalNodeId<Argument>> {
        let start = self.mark();

        let value = self.with_options(
            self.options
                .not_in_position()
                .not_in_tree_literal()
                .not_in_left_precedence(),
            |parser| parser.eat_expression(parser.options),
        )?;

        let argument_id = self.tree.insert(
            Argument::Positional {
                modifiers: None,
                value,
            },
            self.get_span_from(&start),
        );

        Ok(argument_id)
    }

    /// Eat an array literal (including the surrounding brackets).
    pub fn eat_array_literal(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        self.eat_token(TokenType::OpenBracket)?;
        self.eat_newlines_maybe()?;
        let elements = if self.peek_is(TokenType::CloseBracket) {
            vec![]
        } else {
            let element_options = self.options.not_in_position().not_in_left_precedence();
            self.with_options(element_options, |parser| {
                parser.eat_sequence_literal_body(None, TokenType::CloseBracket)
            })?
        };
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseBracket)?;
        Ok(elements)
    }

    /// Eat the body of a sequence literal (excluding the surrounding parenthesis).
    /// Only positional and spread elements are allowed (no named elements).
    pub fn eat_sequence_literal_body(
        &mut self,
        first_element: Option<LocalNodeId<Argument>>,
        close_token: TokenType,
    ) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let mut elements = Vec::new();
        if let Some(first) = first_element {
            elements.push(first);
        }
        // track whether we expect an element (at start or after comma)
        let mut expect_element = first_element.is_none();
        while self.has_more_tokens() {
            // allow trailing newlines before the closing token
            if self.peek_is(TokenType::Newline)
                && self.is_token_after_newlines(self.pos(), close_token)
            {
                break;
            }
            // stop at closing parenthesis (trailing commas are allowed, no hole)
            if self.peek_token_type() == close_token {
                break;
            }
            // consume comma separators, including newline then comma
            let has_comma_separator = self.peek_comma_is()
                || self.peek_is(TokenType::Newline)
                    && self.is_token_after_newlines(self.pos(), TokenType::Comma);
            if has_comma_separator {
                let start = self.mark();

                // leading hole: if we expected an element but got separator instead
                if expect_element {
                    let stub = self
                        .tree
                        .insert(Expression::Stub, self.get_span_from(&start));
                    let hole = self.tree.insert(
                        Argument::Positional {
                            modifiers: None,
                            value: stub,
                        },
                        self.get_span_from(&start),
                    );
                    elements.push(hole);
                }

                // consume optional newlines before comma and then the comma itself
                self.eat_newlines_maybe()?;
                self.eat_item_stop_with_newlines()?;
                expect_element = true;
                continue;
            }

            // consume newline separators
            if self.peek_is(TokenType::Newline) {
                self.bump(); // eat newline
                self.eat_newlines_maybe()?;
                expect_element = true;
                continue;
            }

            // keep eating elements (positional/spread only)
            let element = self
                .eat_positional_argument()
                .for_node_type(NodeType::Argument)?;
            elements.push(element);
            expect_element = false;
        }
        Ok(elements)
    }

    /// Eat an object literal (including the surrounding braces).
    ///
    /// Examples:
    /// ```
    /// { }
    /// { a: 1, b }
    /// { a(x): void }
    pub fn eat_object_literal(&mut self) -> ParseResult<Vec<LocalNodeId<Property>>> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;
        // object literal properties are always expression properties, not variant members
        let mut property_options = self.options;
        property_options.in_variant = false;
        let old_options = self.options;
        self.options = property_options;
        let properties = self.eat_properties();
        self.options = old_options;
        let properties = properties?;

        // JS/TS object shorthand only supports identifier names
        if (self.language.is_javascript() || self.language.is_typescript()) && !self.options.in_type
        {
            self.validate_object_literal_shorthand_keys(&properties)?;
        }

        self.eat_token(TokenType::CloseBrace)?;
        Ok(properties)
    }

    /// Peek whether `<...>` starts a generic arrow in tree literal positions.
    #[cfg(test)]
    pub(super) fn peek_tree_generic_arrow(&mut self) -> bool {
        // only disambiguate when tree literals are enabled
        if !self.language.supports_jsx() {
            return false;
        }

        // type or static contexts do not use tree literal parsing
        if self.options.in_type || self.options.in_static {
            return false;
        }

        // require disambiguators only when ambiguity must be rejected
        let require_tree_disambiguator = self.options.disallow_ambiguous_tree_literal;
        self.peek_generic_arrow_after_type_parameters(require_tree_disambiguator)
    }

    /// Peek whether `<...>(...)` forms a generic arrow function signature.
    pub(super) fn peek_generic_arrow_after_type_parameters(
        &mut self,
        require_tree_disambiguator: bool,
    ) -> bool {
        // require `<` at the current position
        if self.peek_token(TokenType::LessThan).is_err() {
            return false;
        }

        // require an identifier in the type parameter list
        let has_identifier = self.peek_next_is(TokenType::Identifier);

        // allow multiline identifiers in generic parameter lists
        let has_multiline_identifier = if self.peek_next_is(TokenType::Newline) {
            let mut pos = self.pos() as usize;
            loop {
                self.ensure_token(pos + 1);
                let Some(token) = self.tokens().get(pos + 1) else {
                    break;
                };
                if token.token.ty != TokenType::Newline {
                    break;
                }
                pos += 1;
            }
            self.tokens()
                .get(pos + 1)
                .is_some_and(|token| token.token.ty == TokenType::Identifier)
        } else {
            false
        };
        if !has_identifier && !has_multiline_identifier {
            return false;
        }

        // find the closing `>` for the type parameter list and track TSX disambiguators
        let mut angle_depth: usize = if self.has_split_token(TokenType::LessThan) {
            1
        } else {
            0
        };
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut has_tree_disambiguator = false;
        let mut pos = self.pos() as usize;
        let mut close_pos = None;
        loop {
            self.ensure_token(pos);
            let Some(token) = self.tokens().get(pos) else {
                break;
            };
            match token.token.ty {
                TokenType::LessThan => angle_depth += 1,
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft => angle_depth += 2,
                TokenType::GreaterThan => {
                    angle_depth = angle_depth.saturating_sub(1);
                    if angle_depth == 0 {
                        close_pos = Some(pos as u32);
                        break;
                    }
                }
                TokenType::OpenParenthesis => paren_depth += 1,
                TokenType::CloseParenthesis => paren_depth = paren_depth.saturating_sub(1),
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenType::Comma => {
                    if angle_depth == 1
                        && paren_depth == 0
                        && bracket_depth == 0
                        && brace_depth == 0
                    {
                        has_tree_disambiguator = true;
                    }
                }
                TokenType::Assign => {
                    if angle_depth == 1
                        && paren_depth == 0
                        && bracket_depth == 0
                        && brace_depth == 0
                    {
                        has_tree_disambiguator = true;
                    }
                }
                _ => {}
            }
            if angle_depth == 1
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && self.keyword_for_index(pos) == Some(Keyword::Extends)
            {
                has_tree_disambiguator = true;
            }
            pos += 1;
        }
        let Some(close_pos) = close_pos else {
            return false;
        };
        if require_tree_disambiguator && !has_tree_disambiguator {
            return false;
        }

        // skip newlines after the type parameters
        let after_close_pos = {
            let mut pos = close_pos as usize;
            loop {
                self.ensure_token(pos + 1);
                let Some(token) = self.tokens().get(pos + 1) else {
                    break;
                };
                if token.token.ty != TokenType::Newline {
                    break;
                }
                pos += 1;
            }
            pos as u32
        };

        // require `(` after the type parameters
        self.ensure_token(after_close_pos as usize + 1);
        let Some(after_close) = self.tokens().get(after_close_pos as usize + 1) else {
            return false;
        };
        if after_close.token.ty != TokenType::OpenParenthesis {
            return false;
        }

        // find the closing `)` for the parameters
        let Ok(parenthesis_close) = self.find_matching_close(
            Some(after_close_pos + 1),
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        ) else {
            return false;
        };

        // skip newlines after the parameter list
        let after_parenthesis_pos = {
            let mut pos = parenthesis_close as usize;
            loop {
                self.ensure_token(pos + 1);
                let Some(token) = self.tokens().get(pos + 1) else {
                    break;
                };
                if token.token.ty != TokenType::Newline {
                    break;
                }
                pos += 1;
            }
            pos as u32
        };

        // require `:` or `=>` after the parameters
        self.ensure_token(after_parenthesis_pos as usize + 1);
        let Some(after_parenthesis) = self.tokens().get(after_parenthesis_pos as usize + 1) else {
            return false;
        };
        matches!(
            after_parenthesis.token.ty,
            TokenType::Colon | TokenType::Arrow | TokenType::ArrowWide
        )
    }

    /// Peek a tree literal (including the `<` and `>` tokens).
    #[inline]
    pub fn peek_tree_literal(&mut self) -> ParseResult<()> {
        let mark = self.mark();
        let result = (|| {
            if !self.peek_is(TokenType::LessThan) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
            let unexpected_span = self.peek()?.span;

            // prime opening tag mode for tree literal lookahead
            if !self.in_tree_literal() || self.in_tree_attribute_expression() {
                self.enter_tree_opening_tag();
            }

            // skip newlines after `<`
            let mut pos = self.pos_index();
            loop {
                self.ensure_token(pos + 1);
                let Some(token) = self.tokens().get(pos + 1) else {
                    break;
                };
                if token.token.ty != TokenType::Newline {
                    break;
                }
                pos += 1;
            }

            // next token after `<` (and newlines)
            let next = *self
                .token_ref_at(pos + 1)
                .ok_or(ParseError::unexpected(unexpected_span))?;
            // closing tags should only appear inside tree content
            if next.token.ty == TokenType::Divide && !self.in_tree_literal() {
                return Err(ParseError::unexpected(unexpected_span));
            }
            if !matches!(
                next.token.ty,
                TokenType::GreaterThan | TokenType::Divide | TokenType::Identifier
            ) {
                return Err(ParseError::unexpected(unexpected_span));
            }

            // exclude generic arrow function disambiguation: <T,>(...)
            if next.token.ty == TokenType::Identifier {
                let mut comma_pos = pos + 1;
                loop {
                    self.ensure_token(comma_pos + 1);
                    let Some(token) = self.tokens().get(comma_pos + 1) else {
                        break;
                    };
                    if token.token.ty != TokenType::Newline {
                        break;
                    }
                    comma_pos += 1;
                }
                if let Some(token) = self.token_ref_at(comma_pos + 1)
                    && token.token.ty == TokenType::Comma
                {
                    return Err(ParseError::unexpected(unexpected_span));
                }
            }

            Ok(())
        })();

        self.rewind(mark);

        result
    }

    /// Skip whitespace-only tree string tokens (TSX content whitespace).
    /// JSX semantics ignore whitespace-only text between elements (Babel/TypeScript behavior).
    /// See: https://github.com/facebook/jsx/issues/19
    fn skip_tree_whitespace(&mut self) -> ParseResult<bool> {
        let mut skipped = false;
        loop {
            let token = *self.peek()?;
            // skip newlines
            if token.token.ty == TokenType::Newline {
                self.bump();
                skipped = true;
                continue;
            }
            // skip whitespace-only tree strings (entire tokens, not content inside strings)
            if token.token.ty == TokenType::Literal
                && token.token.literal == Some(LiteralType::TreeString)
            {
                let content = self.get_span_str(token.span);
                if content.trim().is_empty() {
                    self.bump();
                    skipped = true;
                    continue;
                }
            }
            break;
        }
        Ok(skipped)
    }

    /// Eat a tree literal (including the `<` and `>` tokens).
    ///
    /// Examples:
    /// ```
    /// <Entity />
    /// <Entity a=1 test />
    /// <Level level=1>
    ///     <Entity name="Alfred" />
    ///     <Entity>2</Entity>
    ///     "some text"
    ///     ..someChildren.map(child => <Entity name={child.name} />)
    /// </Level>
    /// ```
    pub fn eat_tree_literal(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        let start = self.mark();
        self.eat_token(TokenType::LessThan)?;
        if !self.in_tree_literal()
            || self.in_tree_attribute_expression()
            || !self.options.in_tree_literal
        {
            self.enter_tree_opening_tag();
        }
        self.eat_newlines_maybe()?;

        // left
        let mut path_name_span = None;
        let path: Option<Path> = if self.peek_is(TokenType::Identifier) {
            let (path, name_span) = self.eat_tree_literal_path_with_last_span()?;

            // jsx namespace names cannot be followed by member access
            if self.tree_literal_path_has_namespace_member(&path) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            path_name_span = Some(name_span);
            Some(path)
        } else {
            None
        };
        self.eat_newlines_maybe()?;
        let header_start = self.mark();

        // static arguments on the tag (TypeScript/TSX generic JSX components)
        let static_arguments = if path.is_some()
            && (self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft))
        {
            Some(
                self.with_options(self.options.not_in_tree_literal(), |parser| {
                    parser.eat_static_arguments()
                })?,
            )
        } else {
            None
        };
        self.eat_newlines_maybe()?;

        // header (arguments separated by `=`)
        let arguments: Option<Vec<LocalNodeId<Argument>>> = {
            self.skip_tree_whitespace()?;
            // fragment without arguments
            if self.peek_is(TokenType::Divide) || self.peek_is(TokenType::GreaterThan) {
                None
            }
            // fragment with arguments
            else {
                let mut arguments: Vec<LocalNodeId<Argument>> = vec![];
                while self.has_more_tokens() {
                    self.skip_tree_whitespace()?;
                    if self.peek_is(TokenType::Divide) || self.peek_is(TokenType::GreaterThan) {
                        break;
                    }
                    let argument = self.with_options(
                        self.options.not_in_position().in_tree_literal(),
                        |parser| parser.eat_tree_literal_argument(),
                    )?;
                    arguments.push(argument);
                    self.eat_newlines_maybe()?;
                }
                Some(arguments)
            }
        };
        self.eat_newlines_maybe()?;

        // body (either />, or > with child elements)
        let elements: Option<Vec<LocalNodeId<Argument>>> = {
            // fragment without children (/>)
            if self.peek_is(TokenType::Divide) {
                self.bump(); // eat /
                self.eat_token(TokenType::GreaterThan)?; // eat >
                None
            }
            // fragment with children (>)
            else {
                self.eat_token(TokenType::GreaterThan)?; // eat >
                self.skip_tree_whitespace()?; // skip whitespace-only tree content

                // eat children until closing fragment
                let mut elements: Vec<LocalNodeId<Argument>> = vec![];
                let mut found_closing = false;
                while self.has_more_tokens() {
                    // skip whitespace before checking for closing tag
                    self.skip_tree_whitespace()?;

                    // stop at closing fragment (</)
                    if self.peek_is(TokenType::LessThan) {
                        let slash_index = self.next_non_newline_index_from(self.pos_index() + 1);
                        let has_slash_after = self
                            .token_ref_at(slash_index)
                            .is_some_and(|token| token.token.ty == TokenType::Divide);
                        let closes_fragment = if has_slash_after {
                            let after_slash = self.next_non_newline_index_from(slash_index + 1);
                            self.token_ref_at(after_slash)
                                .is_some_and(|token| token.token.ty == TokenType::GreaterThan)
                        } else {
                            false
                        };

                        if !has_slash_after {
                            // not a closing tag
                        } else {
                            // close fragment for fragment literals
                            if path.is_none() && closes_fragment {
                                self.bump(); // eat <
                                self.eat_newlines_maybe()?;
                                self.bump(); // eat /
                                self.eat_newlines_maybe()?;
                                self.bump(); // eat >
                                found_closing = true;
                                break;
                            }

                            // fragment close is invalid for non fragment tags
                            if path.is_some() && closes_fragment {
                                return Err(ParseError::unexpected(self.peek()?.span));
                            }

                            // named closing tag is invalid for fragment literals
                            if path.is_none() {
                                return Err(ParseError::unexpected(self.peek()?.span));
                            }

                            // check if closing fragment has same path
                            if let Some(path) = &path {
                                self.bump(); // eat <
                                self.eat_newlines_maybe()?;
                                self.bump(); // eat /
                                self.eat_newlines_maybe()?;
                                let closing_path = self.eat_tree_literal_path()?;

                                // jsx namespace names cannot be followed by member access
                                if self.tree_literal_path_has_namespace_member(&closing_path) {
                                    return Err(ParseError::unexpected(self.peek()?.span));
                                }

                                if closing_path == *path {
                                    self.eat_token(TokenType::GreaterThan)?;
                                    found_closing = true;
                                    break;
                                }
                                return Err(ParseError::unexpected(self.peek()?.span));
                            }
                        }
                    }

                    // keep eating child elements
                    // NOTE #Robustness: uses statement position so {expr} parses as block (expression container)
                    let element = self.with_options(
                        self.options
                            .not_in_position()
                            .in_tree_literal()
                            .in_statement_position(),
                        |parser| parser.eat_tree_argument(),
                    )?;
                    elements.push(element);
                    self.skip_tree_whitespace()?; // skip whitespace-only tree content
                }

                if !found_closing {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                Some(elements)
            }
        };

        // tree literal
        let left = path.as_ref().map(|path| {
            let expression_id = self.tree.insert(
                Expression::Path {
                    path: path.clone(),
                    static_arguments: static_arguments.clone(),
                },
                self.get_span_between(&start, &header_start),
            );
            if let Some(name_span) = path_name_span {
                self.tree.set_main_span(expression_id, name_span);
            }
            expression_id
        });
        let expression = Expression::TreeExpression {
            left,
            arguments,
            elements,
        };
        Ok(self.tree.insert(expression, self.get_span_from(&start)))
    }

    /// Validate shorthand object literal keys in JS/TS.
    fn validate_object_literal_shorthand_keys(
        &self,
        properties: &[LocalNodeId<Property>],
    ) -> ParseResult<()> {
        for property_id in properties {
            let property = self.tree.get(*property_id);
            let Property::Field {
                key,
                value,
                default,
                ..
            } = property
            else {
                continue;
            };

            // fields with explicit values are always valid
            if value.is_some() || default.is_some() {
                continue;
            }

            // shorthand keys must be identifiers
            let is_identifier_shorthand = matches!(
                key,
                Some(destack_ast::Key::Name(destack_ast::Name::Identifier(_)))
            );
            if is_identifier_shorthand {
                continue;
            }

            return Err(ParseError::unexpected(self.tree.get_span(*property_id)));
        }

        Ok(())
    }

    /// Return true when a tree literal path combines namespace and member syntax.
    fn tree_literal_path_has_namespace_member(&self, path: &Path) -> bool {
        if path.segments.len() <= 1 {
            return false;
        }

        path.segments
            .iter()
            .any(|segment| self.strings.get(*segment).contains(':'))
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, BinaryOperator, Declaration, Expression, FloatType, FunctionKind, IfCondition,
        IfKind, IntType, Name, Parameter, ScalarLiteral, TemplateLiteral, TypeBinaryOperator,
        TypeLiteral,
    };
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    /// Parse integer literals in various formats.
    #[test]
    fn test_parse_integer_literal() {
        let mut test = TestParser::new("1 731 0x1234 2n");
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(1)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(731)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(0x1234)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Bigint(2)
        );
    }

    /// Parse integer literals with uppercase radix prefixes.
    #[test]
    fn test_parse_integer_literal_uppercase_radix_prefixes() {
        let mut test = TestParser::new_with_options("0B101 0O77 0Xff", LanguageType::TypeScript);
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(5)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(63)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(255)
        );
    }

    /// Parse large integer literals without overflow errors.
    #[test]
    fn test_parse_integer_literal_saturating() {
        let mut test = TestParser::new("9999999999999999999999999 9999999999999999999999999n");
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(i64::MAX)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Bigint(i64::MAX)
        );
    }

    /// Parse scientific notation and decimal floats.2
    #[test]
    fn test_parse_float_literal() {
        let mut test = TestParser::new("10e37 1.0");
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Float(1.0e38)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Float(1.0)
        );
    }

    /// Parse true and false literals.
    #[test]
    fn test_parse_boolean_literal() {
        let mut test = TestParser::new("true false");
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Boolean(true)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Boolean(false)
        );
    }

    /// Parse string literal.
    #[test]
    fn test_parse_string_literal() {
        let mut test = TestParser::new(r#""hello" 'hi there'"#);
        let mut parser = test.prepare();

        let literal = parser.eat_scalar_literal().unwrap();
        assert_string!(
            parser,
            match literal {
                ScalarLiteral::String(id) => id,
                other => panic!("expected string literal, got {other:?}"),
            },
            "hello"
        );

        let literal = parser.eat_scalar_literal().unwrap();
        assert_string!(
            parser,
            match literal {
                ScalarLiteral::String(id) => id,
                other => panic!("expected string literal, got {other:?}"),
            },
            "hi there"
        );
    }

    /// Parse a single quoted line separator character literal.
    #[test]
    fn test_parse_single_quoted_line_separator_character_literal() {
        // source: ('\u{2028}')
        let mut test = TestParser::new("('\u{2028}')");
        let mut parser = test.prepare();

        parser
            .eat_token(destack_ast::TokenType::OpenParenthesis)
            .unwrap();
        let literal = parser.eat_scalar_literal().unwrap();
        assert!(matches!(literal, ScalarLiteral::Character('\u{2028}')));
    }

    /// Parse a single quoted paragraph separator character literal.
    #[test]
    fn test_parse_single_quoted_paragraph_separator_character_literal() {
        // source: ('\u{2029}')
        let mut test = TestParser::new("('\u{2029}')");
        let mut parser = test.prepare();

        parser
            .eat_token(destack_ast::TokenType::OpenParenthesis)
            .unwrap();
        let literal = parser.eat_scalar_literal().unwrap();
        assert!(matches!(literal, ScalarLiteral::Character('\u{2029}')));
    }

    /// Parse a regex string literal.
    #[test]
    fn test_parse_regex_string_literal() {
        let mut test = TestParser::new("/abc/\n/abc/g");
        let mut parser = test.prepare();

        // /abc/
        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::RegexString { content, flags } => {
                assert_string!(parser, content, "abc");
                assert!(flags.is_none());
            }
            other => panic!("expected regex string literal, got {other:?}"),
        }
        parser.eat_newline().unwrap();

        // /abc/g
        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::RegexString { content, flags } => {
                assert_string!(parser, content, "abc");
                assert_string!(parser, flags.unwrap(), "g");
            }
            other => panic!("expected regex string literal, got {other:?}"),
        }
    }

    /// Reject unterminated regex literals.
    #[test]
    fn test_reject_unterminated_regex_literal() {
        // source: /42
        let mut test = TestParser::new("/42");
        let mut parser = test.prepare();

        let result = parser.eat_scalar_literal();
        assert!(result.is_err());
    }

    /// Reject regex literals with raw line terminators.
    #[test]
    fn test_reject_regex_literal_with_line_terminator() {
        // source: /test
        // /
        let mut test = TestParser::new("/test\n/");
        let mut parser = test.prepare();

        let result = parser.eat_scalar_literal();
        assert!(result.is_err());
    }

    /// Parse a template string literal.
    #[test]
    fn test_parse_template_literal() {
        let mut test = TestParser::new(
            r#"
`hello`
`hello ${name}`
`${stmt}`
`${start}${middle}${end}`
`SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`
"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // `hello`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::String { string: template } => {
                // hello
                assert_string!(parser, template, "hello");
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `hello ${name}`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(arguments.len(), 1);
                assert_eq!(strings.len(), 2);
                // hello
                assert_string!(parser, strings[0], "hello ");
                // name
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "name");
                });
                //
                assert_string!(parser, strings[1], "");
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `${stmt}`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(strings.len(), 2);
                assert_eq!(arguments.len(), 1);
                // empty start & empty end
                assert_string!(parser, strings[0], "");
                assert_string!(parser, strings[1], "");
                // stmt
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "stmt");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `${start}${middle}${end}`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(strings.len(), 4);
                assert_eq!(arguments.len(), 3);
                // empty string before & after each argument
                assert_string!(parser, strings[0], "");
                assert_string!(parser, strings[1], "");
                assert_string!(parser, strings[2], "");
                assert_string!(parser, strings[3], "");
                // start
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "start");
                });
                // middle
                assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "middle");
                });
                // end
                assert_node!(parser.tree, arguments[2], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "end");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(arguments.len(), 2);
                assert_eq!(strings.len(), 3);
                // SELECT * FROM users WHERE name =
                assert_string!(parser, strings[0], "SELECT * FROM users WHERE name = ");
                // name
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "name");
                });
                // AND age >
                assert_string!(parser, strings[1], " AND age > ");
                // group.age()
                assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Call { left, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "group.age");
                    });
                });
                // LIMIT 10
                assert_string!(parser, strings[2], " LIMIT 10");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    /// Parse template interpolation with a TypeScript `as` cast.
    #[test]
    fn test_parse_template_literal_as_cast_expression() {
        let mut test =
            TestParser::new_with_options("`${type as string}`", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let literal = parser.eat_template_literal().unwrap();

        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(strings.len(), 2);
                assert_eq!(arguments.len(), 1);
                assert_string!(parser, strings[0], "");
                assert_string!(parser, strings[1], "");

                assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::TypeBinary { left, operator, right } => {
                        assert_eq!(*operator, TypeBinaryOperator::Cast);
                        assert_expression_path!(parser, parser.tree.get(*left), "type");
                        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::String));
                    });
                });
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    /// Parse template literals with escaped `${` text before interpolation.
    #[test]
    fn test_parse_template_literal_with_escaped_interpolation_prefix() {
        let mut test = TestParser::new(r"`\${${value}}`");
        let mut parser = test.prepare();
        let literal = parser.eat_template_literal().unwrap();

        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(strings.len(), 2);
                assert_eq!(arguments.len(), 1);
                assert_string!(parser, strings[0], r"\${");
                assert_string!(parser, strings[1], "}");

                assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "value");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn test_parse_type_literal() {
        let mut test = TestParser::new("int32 uint8 float boolean symbol unique symbol");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Int(IntType::Arbitrary {
                width: Some(8),
                is_signed: false
            })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Float(FloatType { width: None })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Boolean
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Symbol
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::UniqueSymbol
        ));
    }

    #[test]
    fn test_parse_array_literal() {
        let mut test = TestParser::new("[1, 2]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 2);
        // 1
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
        // 2
        assert_node!(
            parser.tree,
            elements[1],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            }
        );
    }

    #[test]
    fn test_parse_sparse_array_middle_hole() {
        let mut test = TestParser::new("[1, , 3]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 3);
        // 1
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
        // hole (stub)
        assert_node!(
            parser.tree,
            elements[1],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Stub);
            }
        );
        // 3
        assert_node!(
            parser.tree,
            elements[2],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            }
        );
    }

    #[test]
    fn test_parse_sparse_array_leading_hole() {
        let mut test = TestParser::new("[, 1]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 2);
        // hole (stub)
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Stub);
            }
        );
        // 1
        assert_node!(
            parser.tree,
            elements[1],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
    }

    #[test]
    fn test_parse_sparse_array_trailing_hole() {
        let mut test = TestParser::new("[1, ]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        // trailing comma without hole is allowed
        assert_eq!(elements.len(), 1);
        // 1
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
    }

    #[test]
    fn test_parse_array_literal_with_newline_prefixed_comma_separator() {
        let mut test = TestParser::new("[1\n, 2]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 2);

        // 1
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );

        // 2
        assert_node!(
            parser.tree,
            elements[1],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            }
        );
    }

    #[test]
    fn test_parse_tree_fragment() {
        let mut test = TestParser::new("<A/>");
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        // <A/>
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            // A
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert!(arguments.is_none());
            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_kebab_tag() {
        let mut test =
            TestParser::new_with_options("<amp-something />", LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        // <amp-something />
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "ampSomething");
            assert!(arguments.is_none());
            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_arguments() {
        // pure TSX: numeric values need {}, boolean flags are implicit true
        let mut test = TestParser::new("<A a={1} annoying-bee={2} c={3} flag />");
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert_eq!(arguments.as_ref().unwrap().len(), 4);
            // a={1}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // annoying-bee={2}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "annoyingBee");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
            // c={3}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
            // flag (implicit true)
            assert_node!(parser.tree, arguments.as_ref().unwrap()[3], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });

            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_arguments_and_child() {
        // pure TSX syntax
        let mut test = TestParser::new(
            r"
<Tooltip
    title={true}
    flag
    something-else={false}
>
    {true}
</Tooltip>
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Tooltip");
            assert_eq!(arguments.as_ref().unwrap().len(), 3);
            // title={true}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "title");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // flag (implicit true)
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // something-else={false}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "somethingElse");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
            });

            assert!(elements.is_some());
            // {true} child expression
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
        });
    }

    #[test]
    fn test_parse_tree_nested_deep() {
        // pure TSX: children must be elements or {expression}
        let mut test = TestParser::new(
            r"
<A>
    <B>
        <C>
            <D/>
            {2}
        </C>
    </B>
</A>
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            // <B>
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "B");
                    assert!(arguments.is_none());
                    assert!(elements.is_some());
                    // <C>
                    assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "C");
                            assert!(arguments.is_none());
                            assert!(elements.is_some());
                            // <D/>
                            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                                    assert_expression_path!(parser, parser.tree.get(*left), "D");
                                    assert!(arguments.is_none());
                                    assert!(elements.is_none());
                                });
                            });
                            // {2}
                            assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Positional { modifiers: _, value } => {
                                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                            });
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_tree_in_parenthesis() {
        let mut test = TestParser::new(
            r#"
(
    <div className="font-semibold">
        <Link subtle to={1}>
            {2}
        </Link>
    </div>
)
        "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expression, Expression::Parenthesized { expression } => {
            // <div className="font-semibold">
            assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                assert_expression_path!(parser, parser.tree.get(*left), "div");
                assert!(arguments.is_some());
                assert_eq!(arguments.as_ref().unwrap().len(), 1);
                // className="font-semibold"
                assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                    // className
                    assert_string!(parser, *name, "className");
                    // font-semibold
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "font-semibold");
                    });
                });

                assert!(elements.is_some());
                assert_eq!(elements.as_ref().unwrap().len(), 1);
                // <Link subtle to={1}>
                assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                        // Link
                        assert_expression_path!(parser, parser.tree.get(*left), "Link");
                        assert!(arguments.is_some());
                        assert_eq!(arguments.as_ref().unwrap().len(), 2);
                        // subtle
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "subtle");
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                        });
                        // to={1} - TSX: {} is expression container, value is just 1
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "to");
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                        });

                        assert!(elements.is_some());
                        assert_eq!(elements.as_ref().unwrap().len(), 1);
                        // {2} - TSX: {} is expression container, value is just 2
                        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                        });
                    });
                });
            });
        });
    }

    /// Parse tree literal with static arguments on the tag.
    #[test]
    fn test_parse_tree_with_static_arguments() {
        let mut test = TestParser::new_with_options(
            r#"<Component<any>></Component>"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Component");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Any));
                });
            });
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 0);
        });
    }

    /// Parse static arguments containing a shift-left-like generic arrow.
    #[test]
    fn test_parse_static_arguments_with_shift_left_generic_arrow() {
        let mut test =
            TestParser::new_with_options(r#"<<T>(v: T) => void>"#, LanguageType::TypeScriptXml);
        let mut parser = test.prepare();

        // parse the static arguments
        let static_arguments = parser.eat_static_arguments().unwrap();
        assert_eq!(static_arguments.len(), 1);

        // verify the generic arrow argument shape
        assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert!(body.is_none());
                    let generics = signature.generics.as_ref().expect("expected generics");
                    let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                    assert_eq!(static_parameters.len(), 1);
                    assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: None, default: None, .. } => {
                        assert_string!(parser, *name, "T");
                    });
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                        assert_string!(parser, *name, "v");
                        assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                            assert_path!(parser, *path, "T");
                        });
                    });
                    assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
                });
            });
        });
    }

    /// Parse tree literal with shift-left-like static arguments on the tag.
    #[test]
    fn test_parse_tree_with_shift_left_static_arguments() {
        let mut test = TestParser::new_with_options(
            r#"<Component<<T>(v: T) => void> />"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();

        // parse the tree literal
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Component");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                            assert_eq!(signature.kind, FunctionKind::Lambda);
                            assert!(body.is_none());
                        });
                    });
                });
            });
            assert!(arguments.is_none());
            assert!(elements.is_none());
        });
    }

    /// Parse a TSX tree literal with static arguments and multiline attributes.
    #[test]
    fn test_parse_tree_with_static_arguments_and_multiline_attributes() {
        let mut test = TestParser::new_with_options(
            r#"<Tags<ValueTagData>
  defaultValue={value}
/>"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_tree_literal().unwrap();

        assert_node!(parser.tree, expression_id, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Tags");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "ValueTagData");
                });
            });

            let arguments = arguments.as_ref().expect("expected attributes");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "defaultValue");
                assert_expression_path!(parser, parser.tree.get(*value), "value");
            });

            assert!(elements.is_none());
        });
    }

    /// Reject ambiguous TSX generic arrows without disambiguators.
    #[test]
    fn test_peek_tree_literal_ambiguous_tsx_generic_arrow() {
        let mut test = TestParser::new_with_options("<T>(x: T) => x", LanguageType::TypeScriptXml);
        let mut parser = test.prepare();

        // ambiguous TSX generics are rejected without disambiguators
        assert!(!parser.peek_tree_generic_arrow());
    }

    /// Reject tree literal parsing for disambiguated TSX generic arrows.
    #[test]
    fn test_peek_tree_literal_disambiguated_tsx_generic_arrow() {
        let mut test = TestParser::new_with_options("<T,>(x: T) => x", LanguageType::TypeScriptXml);
        let mut parser = test.prepare();

        // disambiguators should allow generic arrow parsing
        assert!(parser.peek_tree_generic_arrow());
        assert!(parser.peek_tree_literal().is_err());
    }

    /// Recognize TSX generic arrows with extends disambiguators.
    #[test]
    fn test_peek_tree_generic_arrow_tsx_with_extends() {
        let mut test =
            TestParser::new_with_options("<T extends Foo>(x: T) => x", LanguageType::TypeScriptXml);
        let mut parser = test.prepare();

        // extends should disambiguate in tsx
        assert!(parser.peek_tree_generic_arrow());
    }

    /// Check generic arrow disambiguation in JSX without TypeScript.
    #[test]
    fn test_peek_tree_generic_arrow_jsx_without_typescript() {
        let mut test = TestParser::new_with_options("<div>() => {}", LanguageType::JavaScriptXml);
        let mut parser = test.prepare();

        // ambiguous JSX can still be seen as a generic arrow by lookahead
        assert!(parser.peek_tree_generic_arrow());
        assert!(parser.peek_tree_literal().is_ok());
    }

    #[test]
    fn test_parse_tree_with_text_content() {
        let mut test = TestParser::new(r#"<h4>Tool: {part.toolName}</h4>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "h4");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 2);
            // Tool:
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(_)));
            });
            // {part.toolName}
            assert!(matches!(
                parser.tree.get(elements.as_ref().unwrap()[1]),
                Argument::Positional { .. }
            ));
        });
    }

    #[test]
    fn test_parse_nested_tree_with_text_content() {
        // <div><h4>Tool: {x}</h4></div>
        let mut test = TestParser::new(r#"<div><h4>Tool: {x}</h4></div>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
            // nested <h4>...</h4>
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { .. });
            });
        });
    }

    #[test]
    fn test_parse_tree_with_attribute_and_children() {
        // <div key={index}><h4>Tool: {x}</h4></div>
        let mut test = TestParser::new(r#"<div key={index}><h4>Tool: {x}</h4></div>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
        });
    }

    /// Tree fragment containing a callback that returns nested tree literals.
    #[test]
    fn test_parse_tree_fragment_with_nested_callback() {
        let mut test = TestParser::new(r#"<>{x.map(() => (<div><h4>T: {y}</h4></div>))}</>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, arguments, elements } => {
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
        });
    }

    /// Tree literal with JSX comment syntax {/* */}.
    /// The comment is filtered out, leaving an empty expression container.
    #[test]
    fn test_parse_tree_with_comment_container() {
        let mut test = TestParser::new(r#"<div>{/* comment */}</div>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(_), arguments, elements } => {
            assert!(arguments.is_none());
            assert!(elements.is_some());
        });
    }

    /// Parse tree fragment with comments between the angle brackets.
    #[test]
    fn test_parse_tree_fragment_with_comments() {
        let mut test = TestParser::new_with_options(
            "<\n// comment\n/* comment */\n>\n</>",
            LanguageType::JavaScriptXml,
        );
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, arguments, elements } => {
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert!(elements.as_ref().unwrap().is_empty());
        });
    }

    /// Parse tree fragment with a closing tag that has trivia before the slash.
    #[test]
    fn test_parse_tree_fragment_closing_with_trivia() {
        let mut test =
            TestParser::new_with_options("<>\n< /* comment */ / >", LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, arguments, elements } => {
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert!(elements.as_ref().unwrap().is_empty());
        });
    }

    /// Parse tree literal with namespace tag and attribute.
    #[test]
    fn test_parse_tree_with_namespace_tag() {
        let mut test =
            TestParser::new_with_options(r#"<Foo:Bar n:foo="bar" />"#, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Foo:Bar");
            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "n:foo");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(_)));
            });
            assert!(elements.is_none());
        });
    }

    /// Ternary with tree literal containing && inside expression container.
    /// Regression test for fresh_expression() fix - ensures left_precedence is cleared.
    #[test]
    fn test_parse_ternary_with_and_in_tree() {
        // Just the ternary part, without leading condition
        let mut test = TestParser::new(r#"a ? <>{y && <E />}</> : null"#);
        let mut parser = test.prepare();
        let expr = parser.eat_expression(parser.options).unwrap();
        // a ? ... : null -> If with IfKind::Ternary
        assert_node!(parser.tree, expr, Expression::If { kind, condition, then_expression, else_expression } => {
            assert_eq!(*kind, IfKind::Ternary);
            // condition: a
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Path { .. });
            // consequence: <>{y && <E />}</>
            assert_node!(parser.tree, *then_expression, Expression::TreeExpression { left: None, arguments, elements } => {
                assert!(arguments.is_none());
                assert!(elements.is_some());
                assert_eq!(elements.as_ref().unwrap().len(), 1);
                // {y && <E />} - the && expression
                assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Binary { .. });
                });
            });
            // alternative: null
            assert!(else_expression.is_some());
            assert_node!(parser.tree, else_expression.unwrap(), Expression::TypeLiteral(TypeLiteral::Null));
        });
    }

    /// Nested tree literal in attribute expression container.
    /// Regression test for from_content fix in TreeExpressionEntry.
    #[test]
    fn test_parse_nested_tree_in_attribute() {
        let mut test = TestParser::new(r#"<Button icon={<Icon />} />"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Button");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            // icon={<Icon />}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "icon");
                // value is <Icon />
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(inner_left), arguments: inner_args, elements: inner_elems } => {
                    assert_expression_path!(parser, parser.tree.get(*inner_left), "Icon");
                    assert!(inner_args.is_none());
                    assert!(inner_elems.is_none());
                });
            });
            assert!(elements.is_none());
        });
    }

    /// Parse multiline tsx attribute expression containers before a tag close.
    #[test]
    fn test_parse_multiline_tsx_attribute_expression_before_tag_close() {
        let mut test = TestParser::new_with_options(
            r#"<PopoverProvider
  popover={
    <TooltipContent>
      <Picker />
    </TooltipContent>
  }
>
  <PopoverTrigger />
</PopoverProvider>"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();

        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "PopoverProvider");

            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "popover");
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), elements, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "TooltipContent");
                    let elements = elements.as_ref().expect("expected tooltip children");
                    assert_eq!(elements.len(), 1);
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "Picker");
                            assert!(arguments.is_none());
                            assert!(elements.is_none());
                        });
                    });
                });
            });

            let elements = elements.as_ref().expect("expected provider children");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "PopoverTrigger");
                    assert!(arguments.is_none());
                    assert!(elements.is_none());
                });
            });
        });
    }

    #[test]
    fn test_parse_tsx_attribute_tree_with_nested_map_before_tag_close() {
        let mut test = TestParser::new_with_options(
            r#"<PopoverProvider
  popover={
    <TooltipContent>
      {presets.length > 0 && (
        <Swatches>
          {presets.map((preset, index: number) => (
            <SwatchColor
              key={`${preset?.value || index}-${index}`}
              onClick={() => preset && updateValue(preset.value || '')}
            />
          ))}
        </Swatches>
      )}
    </TooltipContent>
  }
>
  <PopoverTrigger style={{ margin: 4 }} />
</PopoverProvider>"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();

        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "PopoverProvider");

            let arguments = arguments.as_ref().expect("expected provider arguments");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "popover");
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "TooltipContent");
                });
            });

            let elements = elements.as_ref().expect("expected provider children");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "PopoverTrigger");
                    let arguments = arguments.as_ref().expect("expected trigger arguments");
                    let has_style_argument = arguments.iter().any(|argument| {
                        matches!(
                            parser.tree.get(*argument),
                            Argument::Named { name: Name::Identifier(name), value, .. }
                                if parser.strings.get(*name) == "style"
                                    && matches!(parser.tree.get(*value), Expression::ObjectExpression { .. })
                        )
                    });
                    assert!(has_style_argument);
                });
            });
        });
    }

    /// Tree fragment text in attribute expression containers parses as tree string.
    #[test]
    fn test_parse_tree_fragment_text_in_attribute_expression() {
        let mut test = TestParser::new_with_options(
            r#"<Show when={shouldShow()} fallback={<>off</>}><>{props.children}</></Show>"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Show");
            let arguments = arguments.as_ref().expect("expected arguments");
            assert!(arguments.len() >= 2);
            assert_node!(parser.tree, arguments[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "fallback");
                assert_node!(parser.tree, *value, Expression::TreeExpression { left, elements, .. } => {
                    assert!(left.is_none());
                    let elements = elements.as_ref().expect("expected fragment elements");
                    assert_eq!(elements.len(), 1);
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                            assert_string!(parser, *string_id, "off");
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_tree_attribute_spread_with_cast() {
        let mut test = TestParser::new_with_options(
            r#"<WrappedComponent {...(this.props as P & DependentProps)} {...this.state} />"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "WrappedComponent");
            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 2);
            // {...(this.props as P & DependentProps)}
            assert_node!(parser.tree, arguments[0], Argument::Spread { modifiers: _, label: None, value } => {
                assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::TypeBinary { operator, .. } => {
                        assert_eq!(*operator, TypeBinaryOperator::Cast);
                    });
                });
            });
            // {...this.state}
            assert_node!(parser.tree, arguments[1], Argument::Spread { modifiers: _, label: None, value } => {
                assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                    assert_node!(parser.tree, *left, Expression::This);
                    assert_string!(parser, *name, "state");
                });
            });
            assert!(elements.is_none());
        });
    }

    /// Deeply nested tree literals in attributes.
    /// Regression test for from_content fix with multiple nesting levels.
    #[test]
    fn test_parse_deeply_nested_tree_in_attr() {
        let mut test =
            TestParser::new(r#"<Outer title={<div><Button icon={<Icon />} /></div>} />"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Outer");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            // title={<div>...</div>}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "title");
                // <div><Button icon={<Icon />} /></div>
                assert_node!(parser.tree, *value, Expression::TreeExpression { elements: div_elems, .. } => {
                    assert!(div_elems.is_some());
                    assert_eq!(div_elems.as_ref().unwrap().len(), 1);
                });
            });
            assert!(elements.is_none());
        });
    }

    /// Object literal inside attribute expression container.
    #[test]
    fn test_parse_tree_attr_object_literal() {
        let mut test = TestParser::new(r#"<Rive style={{width: 400, height: 400}} />"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Rive");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            // style={{width: 400, height: 400}}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "style");
                // {width: 400, height: 400}
                assert_node!(parser.tree, *value, Expression::ObjectExpression { .. });
            });
            assert!(elements.is_none());
        });
    }

    /// Multiline tree literal with expression container and sibling elements.
    #[test]
    fn test_parse_multiline_tree_with_siblings() {
        let code = "<div>\n\t{x}\n\t<form onClick={() => {}}></form>\n</div>";
        let mut test = TestParser::new(code);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 2);
            // {x}
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Path { .. });
            });
            // <form onClick={() => {}}></form>
            assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(form_left), arguments: form_args, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*form_left), "form");
                    assert!(form_args.is_some());
                    assert_eq!(form_args.as_ref().unwrap().len(), 1);
                });
            });
        });
    }

    /// Logical && pattern inside tree content.
    #[test]
    fn test_parse_tree_with_logical_and() {
        let mut test = TestParser::new(r#"<div>{x && <span/>}</div>"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
            // {x && <span/>}
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Binary { left: bin_left, right: bin_right, .. } => {
                    // x
                    assert_node!(parser.tree, *bin_left, Expression::Path { .. });
                    // <span/>
                    assert_node!(parser.tree, *bin_right, Expression::TreeExpression { .. });
                });
            });
        });
    }

    /// Parse relational operators inside tree expression containers.
    #[test]
    fn test_parse_tree_expression_container_relational() {
        let mut test = TestParser::new(r#"<div>{a < b}</div>"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
            // {a < b}
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, left: bin_left, right: bin_right, .. } => {
                    assert_eq!(*operator, BinaryOperator::LessThan);
                    assert_node!(parser.tree, *bin_left, Expression::Path { .. });
                    assert_node!(parser.tree, *bin_right, Expression::Path { .. });
                });
            });
        });
    }

    /// Parse generic calls inside tree expression containers in TSX.
    #[test]
    fn test_parse_tree_expression_container_generic_call() {
        let mut test =
            TestParser::new_with_options(r#"<div>{foo<T>(x)}</div>"#, LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
            // {foo<T>(x)}
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "T");
                    });
                    assert_eq!(dynamic_arguments.len(), 1);
                    assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { modifiers: _, value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "x");
                    });
                });
            });
        });
    }

    /// Parse logical and with an inline tree containing attributes and text.
    #[test]
    fn test_parse_tree_expression_container_logical_and_inline_tree_with_text() {
        let mut test = TestParser::new_with_options(
            r#"<div>{errors.Checkbox && <p id="Checkbox">Checkbox Error</p>}</div>"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();

        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            let elements = elements.as_ref().expect("expected div children");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::And);
                    assert_node!(parser.tree, *right, Expression::TreeExpression { left: Some(right_left), arguments: Some(arguments), elements: Some(right_elements), .. } => {
                        assert_expression_path!(parser, parser.tree.get(*right_left), "p");
                        assert_eq!(arguments.len(), 1);
                        assert_eq!(right_elements.len(), 1);
                        assert_node!(parser.tree, right_elements[0], Argument::Positional { value, .. } => {
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(value)) => {
                                assert_string!(parser, *value, "Checkbox Error");
                            });
                        });
                    });
                });
            });
        });
    }

    /// Tree literal should parse after a closing class block on a new line.
    #[test]
    fn test_parse_tree_after_class_block_newline() {
        let mut test = TestParser::new_with_options(
            "class C extends D<T> {}\n<C/>",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();

        // class declaration
        let class_expr = parser.try_eat_statement_expression().unwrap();
        assert_node!(parser.tree, class_expr, Expression::Declaration(_));

        // statement boundary
        parser.eat_statement_stop_with_newlines().unwrap();

        // tree literal expression
        let tree_expr = parser.try_eat_statement_expression().unwrap();
        assert_node!(parser.tree, tree_expr, Expression::TreeExpression { .. });
    }

    /// Valid template literal with interpolation should parse correctly.
    #[test]
    fn test_parse_template_literal_valid() {
        let mut test = TestParser::new("`hello ${name}!`");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Valid template literal without interpolation.
    #[test]
    fn test_parse_template_literal_plain() {
        let mut test = TestParser::new("`hello world`");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Empty template literal.
    #[test]
    fn test_parse_template_literal_empty() {
        let mut test = TestParser::new("``");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Template literal with only interpolation `${foo}`.
    #[test]
    fn test_parse_template_literal_only_interpolation() {
        let mut test = TestParser::new("`${foo}`");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Template literal with adjacent interpolations.
    #[test]
    fn test_parse_template_literal_adjacent_interpolations() {
        let mut test = TestParser::new("`${a}${b}${c}`");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Template literal interpolation should allow optional chaining.
    #[test]
    fn test_parse_template_literal_optional_chain() {
        let mut test = TestParser::new(r#"`value ${theme?.activeColor}`"#);
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Template literal interpolation should allow ternary expressions.
    #[test]
    fn test_parse_template_literal_ternary() {
        let mut test = TestParser::new(r#"`value ${mode === "dark" ? "dark" : "light"}`"#);
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Parse JSX text that includes `=` and `>=` after opening tags.
    #[test]
    fn test_parse_jsx_text_with_equals() {
        let cases = [
            (r#"<div className={styles.foo}>=</div>"#, "="),
            (r#"<div className={styles.foo} >=</div>"#, "="),
            (r#"<div>=</div>"#, "="),
            (r#"<div >=</div>"#, "="),
        ];

        // parse each case and verify the text node
        for (input, expected_text) in cases {
            let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
            let mut parser = test.prepare();
            let expression = parser.eat_tree_literal().unwrap();
            assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
                let elements = elements.as_ref().expect("expected elements");
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, expected_text);
                    });
                });
            });
        }
    }

    /// Parse JSX fragments containing text and comparisons after closing tags.
    #[test]
    fn test_parse_jsx_fragment_with_equals() {
        let text_cases = [(r#"<>=x</>"#, "=x"), (r#"<span>=x</span>"#, "=x")];

        // parse text cases inside fragments and elements
        for (input, expected_text) in text_cases {
            let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
            let mut parser = test.prepare();
            let expression = parser.eat_tree_literal().unwrap();
            assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
                let elements = elements.as_ref().expect("expected elements");
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, expected_text);
                    });
                });
            });
        }

        let operator_cases = [
            (r#"<>x</>>=1"#, BinaryOperator::GreaterThanOrEqual),
            (r#"<span>x</span>>=1"#, BinaryOperator::GreaterThanOrEqual),
        ];

        // parse operator cases where >= follows a closing tag
        for (input, expected_operator) in operator_cases {
            let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
            let mut parser = test.prepare();
            let expression = parser.eat_expression(parser.options).unwrap();
            assert_node!(parser.tree, expression, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, expected_operator);
                assert_node!(parser.tree, *left, Expression::TreeExpression { .. });
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        }
    }

    /// Parse JSX fragments with text between child elements in arrays.
    #[test]
    fn test_parse_jsx_fragment_equals_in_array() {
        let input = r#"<Y
    elements={[
        <>
            <span>x</span>=
            <br />
        </>,
        true
    ]}
/>
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();

        // ensure the array includes a fragment and a boolean
        assert_node!(parser.tree, expression, Expression::TreeExpression { arguments, .. } => {
            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "elements");
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TreeExpression { left: None, .. });
                    });
                });
            });
        });
    }

    /// Parse ternary expressions that return JSX elements inside expression containers.
    #[test]
    fn test_parse_jsx_ternary_expression_container() {
        let input = r#"<div>{isLoading ? <div>loading</div> : <div>done</div>}</div>"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();

        // verify the ternary expression container
        assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
            let elements = elements.as_ref().expect("expected elements");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::If { kind, .. } => {
                    assert_eq!(*kind, IfKind::Ternary);
                });
            });
        });
    }

    /// Parse ternary expressions that return TSX elements inside expression containers.
    #[test]
    fn test_parse_tsx_ternary_expression_container() {
        let input = r#"<div>{isLoading ? <div>loading</div> : <div>done</div>}</div>"#;
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();

        // verify the ternary expression container
        assert_node!(parser.tree, expression, Expression::TreeExpression { elements, .. } => {
            let elements = elements.as_ref().expect("expected elements");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::If { kind, .. } => {
                    assert_eq!(*kind, IfKind::Ternary);
                });
            });
        });
    }

    /// Parse tsx siblings after map callback returning a parenthesized tree literal.
    #[test]
    fn test_parse_tsx_after_parenthesized_tree_in_expression_container() {
        let input = r#"<div>
  {items.map((item) => (
    <option>{item}</option>
  ))}
  <button />
</div>"#;

        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");

            let elements = elements.as_ref().expect("expected elements");
            assert_eq!(elements.len(), 2);

            assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Call { dynamic_arguments, .. } => {
                    assert_eq!(dynamic_arguments.len(), 1);
                    assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                                assert_eq!(signature.kind, FunctionKind::Lambda);

                                let body = body.expect("expected lambda body");
                                assert_node!(parser.tree, body, Expression::Parenthesized { expression } => {
                                    assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                                        assert_expression_path!(parser, parser.tree.get(*left), "option");
                                        assert!(arguments.is_none());

                                        let elements = elements.as_ref().expect("expected option children");
                                        assert_eq!(elements.len(), 1);
                                        assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                                            assert_expression_path!(parser, parser.tree.get(*value), "item");
                                        });
                                    });
                                });
                            });
                        });
                    });
                });
            });

            assert_node!(parser.tree, elements[1], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "button");
                    assert!(arguments.is_none());
                    assert!(elements.is_none());
                });
            });
        });
    }

    /// Parse ternary fragments with text fallback in tsx.
    #[test]
    fn test_parse_tsx_ternary_fragment_with_text_fallback() {
        let input = "shouldShow ? <>{children}</> : <>off</>";
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expression, Expression::If { kind, condition, then_expression, else_expression } => {
            assert_eq!(*kind, IfKind::Ternary);
            assert_node!(condition, IfCondition::Expression { condition } => {
                assert_expression_path!(parser, parser.tree.get(*condition), "shouldShow");
            });

            assert_node!(parser.tree, *then_expression, Expression::TreeExpression { left, elements, .. } => {
                assert!(left.is_none());
                let elements = elements.as_ref().expect("expected then elements");
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "children");
                });
            });

            let else_expression = else_expression.expect("expected else expression");
            assert_node!(parser.tree, else_expression, Expression::TreeExpression { left, elements, .. } => {
                assert!(left.is_none());
                let elements = elements.as_ref().expect("expected else elements");
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "off");
                    });
                });
            });
        });
    }

    /// Parse tsx fragment text nodes with standalone colon content.
    #[test]
    fn test_parse_tsx_fragment_with_colon_text_node() {
        let input = r#"<code>{value && <>:</>}</code>"#;
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();

        // verify logical-and fragment text parsing
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "code");
            let elements = elements.as_ref().expect("expected code children");
            assert_eq!(elements.len(), 1);

            // verify the right side of `value && ...`
            assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::And);
                    assert_node!(parser.tree, *right, Expression::TreeExpression { left: None, elements, .. } => {
                        let elements = elements.as_ref().expect("expected fragment children");
                        assert_eq!(elements.len(), 1);
                        assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                                assert_string!(parser, *string_id, ":");
                            });
                        });
                    });
                });
            });
        });
    }

    /// Parse logical and with a fragment that contains a nested ternary tree literal.
    #[test]
    fn test_parse_tsx_logical_and_fragment_with_nested_ternary_tree() {
        let input = r#"<div>{condition && <>{show ? <Box /> : null}</>}</div>"#;
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();

        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            let elements = elements.as_ref().expect("expected div children");
            assert_eq!(elements.len(), 1);

            assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::And);
                    assert_node!(parser.tree, *right, Expression::TreeExpression { left: None, elements, .. } => {
                        let elements = elements.as_ref().expect("expected fragment children");
                        assert_eq!(elements.len(), 1);
                        assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                            assert_node!(parser.tree, *value, Expression::If { kind, .. } => {
                                assert_eq!(*kind, IfKind::Ternary);
                            });
                        });
                    });
                });
            });
        });
    }

    /// Reject tree literal namespace and member combinations during parse.
    #[test]
    fn test_reject_tree_literal_namespace_member_path_parse_error() {
        let mut test = TestParser::new_with_options("<a.b:c />", LanguageType::JavaScriptXml);
        let mut parser = test.prepare();

        let error = parser
            .eat_tree_literal()
            .expect_err("expected parse failure for namespace member path");
        assert_eq!(parser.get_span_str(error.leaf_span()), "/");
    }

    /// Parse JSX elements after newline-terminated let and var declarations.
    #[test]
    fn test_parse_jsx_after_let_newline() {
        let cases = [
            "let x\n<Comp></Comp>",
            "let x\n\n<Comp></Comp>",
            "let x;\n<Comp></Comp>",
            "var x\n<Comp></Comp>",
            "var x;\n<Comp></Comp>",
            "{ foo: 'test' }\n<Comp></Comp>",
            "function test1() {}\n<Comp></Comp>",
        ];

        // ensure top-level newline allows tree literals after declarations
        for input in cases {
            let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
            let mut parser = test.prepare();
            let expressions = parser.parse();
            assert_eq!(expressions.len(), 2);
            let tree_expression = match parser.tree.get(expressions[1]) {
                Expression::Statement(expression_id) => *expression_id,
                _ => expressions[1],
            };
            assert_node!(
                parser.tree,
                tree_expression,
                Expression::TreeExpression { .. }
            );
        }

        let input = r#"
function x() {
    let x
    <div />
}
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // ensure tree literals follow let statements inside blocks
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
                let body = body.expect("expected body");
                assert_node!(parser.tree, body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 2);
                    let tree_expression = match parser.tree.get(block.expressions[1]) {
                        Expression::Statement(expression_id) => *expression_id,
                        _ => block.expressions[1],
                    };
                    assert_node!(parser.tree, tree_expression, Expression::TreeExpression { .. });
                });
            });
        });

        let input = r#"
class Foo {}
<>
<Comp></Comp>
<Comp></Comp>
</>
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // ensure fragments after classes parse with multiple children
        assert_eq!(expressions.len(), 2);
        let tree_expression = match parser.tree.get(expressions[1]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => expressions[1],
        };
        assert_node!(parser.tree, tree_expression, Expression::TreeExpression { left, elements, .. } => {
            assert!(left.is_none());
            let elements = elements.as_ref().expect("expected elements");
            assert_eq!(elements.len(), 2);
        });
    }

    /// Parse JSX elements after return with newline termination.
    #[test]
    fn test_parse_jsx_after_return_newline() {
        let input = r#"
function test() {
    return
    <Comp />
}
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // ensure return is terminated and tree literal follows
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
                let body = body.expect("expected body");
                assert_node!(parser.tree, body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 2);
                    let return_expression = match parser.tree.get(block.expressions[0]) {
                        Expression::Statement(expression_id) => *expression_id,
                        _ => block.expressions[0],
                    };
                    assert_node!(parser.tree, return_expression, Expression::Return { value } => {
                        assert!(value.is_none());
                    });
                    let tree_expression = match parser.tree.get(block.expressions[1]) {
                        Expression::Statement(expression_id) => *expression_id,
                        _ => block.expressions[1],
                    };
                    assert_node!(parser.tree, tree_expression, Expression::TreeExpression { .. });
                });
            });
        });
    }

    /// Parse return parenthesized tree literal containing logical-and fragment with long text.
    #[test]
    fn test_parse_return_parenthesized_tree_with_logical_fragment_long_text() {
        let input = r#"
function app() {
  return (
    <Box>
      {obj.alpha.size > 0 && <>
        <Text wrap={`wrap`}>
          Because of those files having been modified, the following workspaces may need to be released again (note that private workspaces are also shown here, because even though they won't be published, releasing them will allow us to flag their dependents for potential re-release):
        </Text>
      </>}
    </Box>
  );
}
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
                let body = body.expect("expected function body");
                assert_node!(parser.tree, body, Expression::Block(block_id) => {
                    let block = parser.tree.get(*block_id);
                    assert_eq!(block.expressions.len(), 1);
                    let return_expression = match parser.tree.get(block.expressions[0]) {
                        Expression::Statement(expression_id) => *expression_id,
                        _ => block.expressions[0],
                    };
                    assert_node!(parser.tree, return_expression, Expression::Return { value } => {
                        let value = value.expect("expected return value");
                        assert_node!(parser.tree, value, Expression::Parenthesized { expression } => {
                            assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), elements, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "Box");
                                let elements = elements.as_ref().expect("expected box children");
                                assert_eq!(elements.len(), 1);
                            });
                        });
                    });
                });
            });
        });
    }

    /// Reject JSX-looking input when it continues an expression across a newline.
    #[test]
    fn test_parse_jsx_after_expression_newline_is_error() {
        let input = "x\n<Comp />";
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();

        // reject JSX after expression newline
        let result = parser.eat_expression(parser.options);
        assert!(result.is_err());
    }
}
