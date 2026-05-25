use std::borrow::Cow;
use std::str::Chars;

use crate::lex::decode_html_entity;
use crate::parse::TypeMemberBodyMode;
use crate::parse::mode::ContextualLexMode;
use crate::parse::prelude::*;
use crate::parse::scope::ExpressionScope;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

use destack_dir::{
    Argument, Expression, GenericArgument, Keyword, LocalNodeId, NodeType, NumberBase, Path,
    Property, ScalarLiteral, StringId, TemplateLiteral, TokenLiteral, TokenSpan, TokenType,
    TypeExpression, TypeMember,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};
use smallvec::SmallVec;

/// One open tree literal awaiting child elements.
struct OpenTreeLiteral {
    /// The span start for the tree literal.
    start: ParserSpanStart,
    /// The parsed tag path.
    path: Option<Path>,
    /// The source spans for the tag path segments.
    path_segment_spans: Option<SmallVec<[Span; 3]>>,
    /// The parsed generic arguments.
    generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    /// The parsed tag arguments.
    arguments: Option<Vec<LocalNodeId<Argument>>>,
    /// The parsed child elements.
    elements: Vec<LocalNodeId<Argument>>,
    /// The span of the opening tag.
    opening_span: Span,
    /// The lexing mode after this literal closes.
    close_follow_mode: ContextualLexMode,
}

/// The result of parsing one tree literal opening.
enum TreeLiteralOpen {
    /// A complete self-closing tree expression.
    Complete(LocalNodeId<Expression>),
    /// An open tree expression.
    Open(OpenTreeLiteral),
}

impl Parser {
    /// Decode one fixed-width hexadecimal character escape.
    fn decode_fixed_character_escape(characters: &mut Chars<'_>, width: usize) -> Option<char> {
        let mut value = 0u32;

        for _ in 0..width {
            let digit = characters.next()?.to_digit(16)?;
            value = value.checked_mul(16)?.checked_add(digit)?;
        }

        char::from_u32(value)
    }

    /// Decode one braced hexadecimal character escape.
    fn decode_braced_character_escape(characters: &mut Chars<'_>) -> Option<char> {
        let mut value = 0u32;
        let mut digits = 0usize;

        loop {
            let character = characters.next()?;
            if character == '}' {
                return (digits > 0).then(|| char::from_u32(value)).flatten();
            }

            let digit = character.to_digit(16)?;
            value = value.checked_mul(16)?.checked_add(digit)?;
            digits += 1;
        }
    }

    /// Decode one escaped character payload.
    fn decode_character_escape(characters: &mut Chars<'_>) -> Option<char> {
        let escaped = characters.next()?;

        match escaped {
            '0' => Some('\0'),
            'b' => Some('\u{08}'),
            'f' => Some('\u{0C}'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            'v' => Some('\u{0B}'),
            'x' => Self::decode_fixed_character_escape(characters, 2),
            'u' => {
                if characters.as_str().starts_with('{') {
                    characters.next();
                    Self::decode_braced_character_escape(characters)
                } else {
                    Self::decode_fixed_character_escape(characters, 4)
                }
            }
            character if character.is_ascii_digit() => None,
            character => Some(character),
        }
    }

    /// Decode one single-quoted character literal.
    fn decode_character_literal(literal: &str) -> Option<char> {
        if !literal.starts_with('\'') || !literal.ends_with('\'') || literal.len() < 2 {
            return None;
        }

        let content = &literal[1..literal.len() - 1];
        let mut characters = content.chars();
        let character = if content.starts_with('\\') {
            characters.next();
            Self::decode_character_escape(&mut characters)?
        } else {
            characters.next()?
        };

        characters.next().is_none().then_some(character)
    }

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
        self.is_tree_literal_start_at_offset(0)
    }

    /// Return true when an offset token starts a tree literal.
    #[inline]
    pub(crate) fn is_tree_literal_start_at_offset(&mut self, offset: usize) -> bool {
        if self.token_type_at_offset(offset) != TokenType::LessThan {
            return false;
        }

        let next_token_type = self.token_type_at_offset(offset + 1);
        if next_token_type == TokenType::Divide {
            return false;
        }

        let starts_tag_close = Self::starts_type_angle_close(next_token_type);
        if !starts_tag_close
            && !matches!(next_token_type, TokenType::Divide | TokenType::Identifier)
        {
            return false;
        }

        if next_token_type == TokenType::Identifier {
            return self.token_type_at_offset(offset + 2) != TokenType::Comma;
        }

        true
    }

    /// Peek a scalar literal token.
    #[inline]
    pub fn peek_scalar_literal(&mut self) -> ParserResult<&TokenSpan> {
        if self.peek_is(TokenType::Literal) {
            Ok(self.peek()?)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
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
    pub fn eat_scalar_literal(&mut self) -> ParserResult<ScalarLiteral> {
        let literal_span = *self.eat()?;
        let Some(body) = literal_span.token.literal else {
            return Err(ParserError::unexpected(literal_span.span));
        };
        let has_adjacent_identifier_suffix =
            self.numeric_literal_has_adjacent_identifier_suffix(literal_span);
        let literal_str = self.file.span_str(literal_span.span);

        match body {
            // boolean literal
            TokenLiteral::Boolean { value } => Ok(ScalarLiteral::Boolean(value)),

            // int literal
            TokenLiteral::Int {
                base,
                is_empty,
                is_bigint,
            } => {
                if is_empty {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // typed and untyped source forms reject legacy leading-zero decimal forms
                if (self.language.is_javascript() || self.language.is_typescript())
                    && self.int_literal_uses_legacy_leading_zero(literal_str, base, is_bigint)
                {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // reject legacy octal literals without an explicit 0o/0O prefix
                if self.int_literal_is_legacy_octal(literal_str, base, is_bigint) {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // typed and untyped source forms reject invalid digits for prefixed literals
                if (self.language.is_javascript() || self.language.is_typescript())
                    && self.int_literal_has_invalid_digits(literal_str, base, is_bigint)
                {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // typed and untyped source forms require a separator before identifier starts
                if has_adjacent_identifier_suffix {
                    return Err(ParserError::expected_for(
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
            TokenLiteral::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // typed and untyped source forms reject legacy leading-zero decimal forms
                if (self.language.is_javascript() || self.language.is_typescript())
                    && self.float_literal_uses_legacy_leading_zero(literal_str)
                {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // typed and untyped source forms require a separator after numeric literals before identifier starts
                if has_adjacent_identifier_suffix {
                    return Err(ParserError::expected_for(
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
                        ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // html entity character literal
            TokenLiteral::Character {
                is_terminated,
                is_html_entity,
            } => {
                if !is_terminated || !is_html_entity {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                decode_html_entity(literal_str)
                    .map(ScalarLiteral::Character)
                    .ok_or_else(|| {
                        ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // string or Destack character literal
            TokenLiteral::String {
                is_terminated,
                has_invalid_escape,
            } => {
                if !is_terminated || has_invalid_escape {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                if self.language.is_destack() && literal_str.starts_with('\'') {
                    return Self::decode_character_literal(literal_str)
                        .map(ScalarLiteral::Character)
                        .ok_or_else(|| {
                            ParserError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        });
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
            TokenLiteral::RegexString { has_flags } => {
                // regex literals require a closing slash
                if !literal_str.starts_with('/') {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // regex without flags
                if !has_flags {
                    if !literal_str.ends_with('/') || literal_str.len() < 2 {
                        return Err(ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }

                    let content = &literal_str[1..literal_str.len() - 1];
                    if self.contains_regex_line_terminator(content) {
                        return Err(ParserError::expected_for(
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
                        return Err(ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    };
                    if last_slash_index == 0 {
                        return Err(ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }

                    let content = &literal_str[1..last_slash_index];
                    let flags = &literal_str[last_slash_index + 1..];
                    if flags.is_empty() || self.contains_regex_line_terminator(content) {
                        return Err(ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    if !self.regex_flags_are_valid(flags) {
                        return Err(ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    if !self.regex_unicode_escapes_are_valid(content, flags) {
                        return Err(ParserError::expected_for(
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

            // tree text content, raw text inside tree literals
            TokenLiteral::TreeString => {
                let string_id = self.strings.intern(literal_str);
                Ok(ScalarLiteral::String(string_id))
            }
        }
    }

    /// Re-lex and eat the current regex literal.
    pub(crate) fn eat_regex_literal(&mut self) -> ParserResult<ScalarLiteral> {
        if !self.re_lex_regex() {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        self.eat_scalar_literal()
    }

    /// Parse one tree child scalar literal and advance in the requested mode.
    pub(crate) fn eat_tree_child_scalar_expression(
        &mut self,
        follow_mode: ContextualLexMode,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let token = *self.peek()?;
        let Some(body) = token.token.literal else {
            return Err(ParserError::unexpected(token.span));
        };
        let literal_str = self.file.span_str(token.span);
        let scalar_literal = match body {
            TokenLiteral::Character {
                is_terminated,
                is_html_entity,
            } => {
                if !is_terminated || !is_html_entity {
                    return Err(ParserError::expected_for(
                        token.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                decode_html_entity(literal_str)
                    .map(ScalarLiteral::Character)
                    .ok_or_else(|| {
                        ParserError::expected_for(
                            token.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })?
            }
            TokenLiteral::TreeString => {
                let string_id = self.strings.intern(literal_str);
                ScalarLiteral::String(string_id)
            }
            _ => return Err(ParserError::unexpected(token.span)),
        };

        // continue in the owning tree scope
        self.bump_with_contextual_lex_mode(follow_mode);

        Ok(self.insert_node(Expression::ScalarLiteral(scalar_literal), token.span))
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

    /// Return true when an int literal uses legacy leading-zero syntax.
    fn int_literal_uses_legacy_leading_zero(
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

    /// Return true when a float literal uses legacy leading-zero syntax.
    fn float_literal_uses_legacy_leading_zero(&self, literal: &str) -> bool {
        let bytes = literal.as_bytes();
        if bytes.len() < 2 || bytes[0] != b'0' {
            return false;
        }

        let second = bytes[1] as char;
        second.is_ascii_digit() || second == '_'
    }

    /// Return true when a typed or untyped numeric literal is immediately followed by an identifier.
    fn numeric_literal_has_adjacent_identifier_suffix(&mut self, literal: TokenSpan) -> bool {
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
    pub fn peek_template_literal(&mut self) -> ParserResult<&TokenSpan> {
        if self.peek_is(TokenType::TemplateString) || self.peek_is(TokenType::TemplateStringStart) {
            Ok(self.peek()?)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a template literal.
    ///
    /// Examples:
    /// ```
    /// `hello`
    /// `hello ${name}`
    /// `SELECT * FROM users`
    /// `${stmt}`
    /// `SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    pub fn eat_template_literal(&mut self) -> ParserResult<TemplateLiteral> {
        self.eat_template_literal_with_flags(false)
    }

    /// Eat a tagged template literal.
    pub fn eat_tagged_template_literal(&mut self) -> ParserResult<TemplateLiteral> {
        self.eat_template_literal_with_flags(true)
    }

    /// Eat a template literal with parser-mode constraints.
    fn eat_template_literal_with_flags(
        &mut self,
        allow_legacy_octal_escapes: bool,
    ) -> ParserResult<TemplateLiteral> {
        let (strings, arguments) = self
            .eat_template_literal_body(allow_legacy_octal_escapes, |parser| {
                parser.eat_template_literal_argument()
            })?;

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
    pub fn eat_type_template_literal_expression(
        &mut self,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let start = self.span_start();
        let (strings, spans) = self.eat_template_literal_body(false, |parser| {
            // reset outer precedence so interpolation unions parse fully
            let interpolation_ambient_context = parser.flags.with_type(true);
            let interpolation_expression_context = parser.flags.not_in_position();
            parser.eat_type_expression_or_recover_missing(
                parser
                    .flags
                    .with_ambient_context(interpolation_ambient_context)
                    .with_expression_context(interpolation_expression_context),
                NodeType::Expression,
            )
        })?;

        let expression = TypeExpression::TemplateLiteral { strings, spans };
        let expression_id = self.insert_node(expression, self.get_span_from(&start));

        // the template head is the first interpolation head when present
        if let TypeExpression::TemplateLiteral { spans, .. } = self.tree.get(expression_id)
            && let Some(first_span_expression_id) = spans.first()
        {
            let head_span = self.type_expression_head_span(*first_span_expression_id);
            self.tree.set_head_span(expression_id, head_span);
        }

        Ok(expression_id)
    }

    /// Eat a template literal body.
    fn eat_template_literal_body<T>(
        &mut self,
        allow_legacy_octal_escapes: bool,
        mut parse_span: impl FnMut(&mut Parser) -> ParserResult<T>,
    ) -> ParserResult<(Vec<StringId>, Vec<T>)> {
        let next = *self.eat()?;
        let next_str = self.file.span_str(next.span);

        // template string without interpolation
        if next.token.ty == TokenType::TemplateString {
            let string = Self::template_chunk_body(next_str, 1, 1);
            self.validate_template_literal_chunk_maybe(
                next.span,
                string,
                allow_legacy_octal_escapes,
            )?;

            let string_id = self.strings.intern(string);
            return Ok((vec![string_id], Vec::new()));
        }

        // template string with interpolation
        if next.token.ty == TokenType::TemplateStringStart {
            let mut strings: Vec<StringId> = Vec::new();
            let mut spans: Vec<T> = Vec::new();

            // start chunk: remove ` prefix and ${ suffix
            let string = Self::template_chunk_body(next_str, 1, 2);
            self.validate_template_literal_chunk_maybe(
                next.span,
                string,
                allow_legacy_octal_escapes,
            )?;

            let string_id = self.strings.intern(string);
            strings.push(string_id);

            // eat until the end
            while !self.peek_is(TokenType::TemplateStringEnd) {
                // middle chunk: remove } prefix and ${ suffix
                if self.peek_is(TokenType::TemplateStringMiddle) {
                    let token = *self.eat()?;
                    let token_str = self.file.span_str(token.span);
                    let string = Self::template_chunk_body(token_str, 1, 2);
                    self.validate_template_literal_chunk_maybe(
                        token.span,
                        string,
                        allow_legacy_octal_escapes,
                    )?;

                    let string_id = self.strings.intern(string);
                    strings.push(string_id);
                }
                // interpolation expression
                else {
                    let span = parse_span(self)?;
                    if !self.peek_is(TokenType::TemplateStringMiddle)
                        && !self.peek_is(TokenType::TemplateStringEnd)
                    {
                        return Err(ParserError::unexpected(self.peek()?.span));
                    }
                    spans.push(span);
                }
            }

            // end chunk: remove } prefix and ` suffix
            let token = *self.eat_token(TokenType::TemplateStringEnd)?;
            let token_str = self.file.span_str(token.span);
            let string = Self::template_chunk_body(token_str, 1, 1);
            self.validate_template_literal_chunk_maybe(
                token.span,
                string,
                allow_legacy_octal_escapes,
            )?;

            let string_id = self.strings.intern(string);
            strings.push(string_id);

            return Ok((strings, spans));
        }

        Err(ParserError::unexpected(next.span))
    }

    /// Return the body of one lexer-shaped template chunk.
    fn template_chunk_body(token_str: &str, prefix_len: usize, suffix_len: usize) -> &str {
        let end = token_str.len().saturating_sub(suffix_len);

        token_str.get(prefix_len..end).unwrap_or("")
    }

    /// Return true when the template chunk contains legacy octal escapes.
    fn template_chunk_has_legacy_octal_escape(string: &str) -> bool {
        let bytes = string.as_bytes();
        if !bytes.contains(&b'\\') {
            return false;
        }

        let mut index = 0;

        while index < bytes.len() {
            if bytes[index] != b'\\' {
                index += 1;
                continue;
            }

            index += 1;
            if index >= bytes.len() {
                break;
            }

            let escaped = bytes[index];

            // invalid legacy octal: \1 through \9
            if escaped.is_ascii_digit() && escaped != b'0' {
                return true;
            }

            // invalid legacy octal: \0 followed by another digit
            if escaped == b'0' {
                index += 1;
                if index < bytes.len() && bytes[index].is_ascii_digit() {
                    return true;
                }
                continue;
            }

            // skip escaped code unit
            index += 1;
        }

        false
    }

    /// Reject template chunks with legacy octal escapes when the mode does not allow them.
    fn validate_template_literal_chunk_maybe(
        &self,
        span: Span,
        string: &str,
        allow_legacy_octal_escapes: bool,
    ) -> ParserResult<()> {
        if allow_legacy_octal_escapes {
            return Ok(());
        }

        if Self::template_chunk_has_legacy_octal_escape(string) {
            return Err(ParserError::unexpected(span));
        }

        Ok(())
    }

    /// Eat a template literal interpolation argument.
    ///
    /// Template literal interpolations parse as full expressions (no named args).
    pub fn eat_template_literal_argument(&mut self) -> ParserResult<LocalNodeId<Argument>> {
        let start = self.span_start();

        let value = self.eat_template_interpolation_expression()?;

        let argument_id =
            self.insert_node(Argument::Positional { value }, self.get_span_from(&start));

        Ok(argument_id)
    }

    /// Eat one template interpolation expression.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// first, second
    /// condition ? yes : no
    /// ```
    fn eat_template_interpolation_expression(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let flags = self.flags.not_in_position().not_in_tree_literal();
        let scope = ExpressionScope::from_flags(flags);
        let start = self.span_start();

        let expression = if self.flags == flags {
            self.eat_assignment(&start, scope)
                .and_then(|expression| self.eat_sequence_rest(&start, expression, scope))
        } else {
            let outer_flags = self.swap_flags(flags);
            let expression = self
                .eat_assignment(&start, scope)
                .and_then(|expression| self.eat_sequence_rest(&start, expression, scope));
            self.restore_flags(outer_flags);

            expression
        };

        expression
    }

    /// Eat a bracket literal expression including the surrounding brackets.
    pub fn eat_bracket_literal_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_token(TokenType::OpenBracket)?;

        if self.peek_is(TokenType::CloseBracket) {
            self.bump();

            return Ok(self.insert_node(
                Expression::ArrayExpression { elements: vec![] },
                self.get_span_from(start),
            ));
        }

        let expression_context = self.flags.not_in_position().not_in_sequence_expression();
        let flags = self.flags.with_expression_context(expression_context);

        if self.peek_is(TokenType::Comma) {
            let elements = self.with_flags(flags, |parser| {
                parser.eat_sequence_literal_body(None, TokenType::CloseBracket)
            })?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;

            return Ok(self.insert_node(
                Expression::ArrayExpression { elements },
                self.get_span_from(start),
            ));
        }

        if self.language.is_destack() && self.peek_is(TokenType::Semicolon) {
            let value = self.recover_missing_expression_here(NodeType::Expression);
            self.bump();
            let length = self.eat_expression_or_recover_missing(flags, NodeType::Expression)?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;

            return Ok(self.insert_node(
                Expression::FixedArrayExpression { value, length },
                self.get_span_from(start),
            ));
        }

        let first = self.with_flags(flags, |parser| parser.eat_positional_argument())?;
        if self.language.is_destack()
            && self.peek_is(TokenType::Semicolon)
            && let Argument::Positional { value } = self.tree.get(first)
        {
            let value = *value;
            self.bump();
            let length = self.eat_expression_or_recover_missing(flags, NodeType::Expression)?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;

            return Ok(self.insert_node(
                Expression::FixedArrayExpression { value, length },
                self.get_span_from(start),
            ));
        }

        let elements = self.with_flags(flags, |parser| {
            parser.eat_sequence_literal_body(Some(first), TokenType::CloseBracket)
        })?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;

        Ok(self.insert_node(
            Expression::ArrayExpression { elements },
            self.get_span_from(start),
        ))
    }

    /// Eat an array literal (including the surrounding brackets).
    pub fn eat_array_literal(&mut self) -> ParserResult<Vec<LocalNodeId<Argument>>> {
        self.eat_token(TokenType::OpenBracket)?;
        let elements = if self.peek_is(TokenType::CloseBracket) {
            vec![]
        } else {
            let element_expression_context =
                self.flags.not_in_position().not_in_sequence_expression();
            self.with_flags(
                self.flags
                    .with_expression_context(element_expression_context),
                |parser| parser.eat_sequence_literal_body(None, TokenType::CloseBracket),
            )?
        };
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;
        Ok(elements)
    }

    /// Eat the body of a sequence literal (excluding the surrounding parenthesis).
    /// Only positional and spread elements are allowed (no named elements).
    pub fn eat_sequence_literal_body(
        &mut self,
        first_element: Option<LocalNodeId<Argument>>,
        close_token: TokenType,
    ) -> ParserResult<Vec<LocalNodeId<Argument>>> {
        let mut elements = Vec::new();
        if let Some(first) = first_element {
            elements.push(first);
        }
        // track whether we expect an element (at start or after comma)
        let mut expect_element = first_element.is_none();
        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // stop at the closing token (trailing commas are allowed, no hole)
            if token_type == close_token {
                break;
            }

            // consume comma separators
            if token_type == TokenType::Comma {
                let start = self.span_start();

                // leading hole: if we expected an element but got separator instead
                if expect_element {
                    let stub = self
                        .tree
                        .insert(Expression::Stub, self.get_span_from(&start));
                    let hole = self.insert_node(
                        Argument::Positional { value: stub },
                        self.get_span_from(&start),
                    );
                    elements.push(hole);
                }

                // consume optional newlines before comma and then the comma itself
                self.eat_item_stop()?;

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
    pub fn eat_object_literal(&mut self) -> ParserResult<Vec<LocalNodeId<Property>>> {
        self.eat_token(TokenType::OpenBrace)?;
        let properties = self.eat_object_properties()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Expression)?;

        Ok(properties)
    }

    /// Eat one object type literal (including the surrounding braces).
    pub fn eat_type_object_literal(&mut self) -> ParserResult<Vec<LocalNodeId<TypeMember>>> {
        self.eat_token(TokenType::OpenBrace)?;

        let property_ambient_context = self.flags.with_variant(false).with_type(true);
        let flags = self.flags.with_ambient_context(property_ambient_context);
        let properties = if self.flags == flags {
            self.eat_type_members(TypeMemberBodyMode::SignatureOnly)
        } else {
            let old_flags = self.swap_flags(flags);
            let properties = self.eat_type_members(TypeMemberBodyMode::SignatureOnly);
            self.restore_flags(old_flags);

            properties
        };
        let properties = properties?;

        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Expression)?;
        Ok(properties)
    }

    /// Peek whether `<...>(...)` forms a generic arrow function signature.
    pub(super) fn peek_generic_arrow_after_type_parameters(
        &mut self,
        require_tree_disambiguator: bool,
    ) -> bool {
        self.lookahead(|parser| {
            parser.scan_generic_arrow_after_type_parameters(require_tree_disambiguator)
        })
    }

    /// Scan whether `<...>(...)` forms a generic arrow function signature.
    fn scan_generic_arrow_after_type_parameters(
        &mut self,
        require_tree_disambiguator: bool,
    ) -> bool {
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }

        if self.next_token_type() != TokenType::Identifier {
            return false;
        }

        let Some(has_tree_disambiguator) = self.scan_generic_arrow_type_parameters() else {
            return false;
        };

        if require_tree_disambiguator && !has_tree_disambiguator {
            return false;
        }

        if !self.scan_generic_arrow_parameters() {
            return false;
        }

        matches!(
            self.peek_token_type(),
            TokenType::Colon | TokenType::ArrowWide
        )
    }

    /// Scan the generic arrow type parameter list.
    fn scan_generic_arrow_type_parameters(&mut self) -> Option<bool> {
        let mut angle_depth = 0usize;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut has_tree_disambiguator = false;

        loop {
            let token_type = self.peek_token_type();
            if token_type == TokenType::End {
                return None;
            }

            if self.current_token_is_statement_recovery_boundary(token_type) {
                return None;
            }

            // close the outer type parameter list
            match token_type {
                TokenType::LessThan => angle_depth += 1,
                TokenType::ShiftLeft => angle_depth += 2,
                TokenType::ShiftRight => {
                    if angle_depth == 2 {
                        self.bump();

                        return Some(has_tree_disambiguator);
                    }
                    angle_depth = angle_depth.saturating_sub(2);
                }
                TokenType::UnsignedShiftRight => {
                    if angle_depth == 3 {
                        self.bump();

                        return Some(has_tree_disambiguator);
                    }
                    angle_depth = angle_depth.saturating_sub(3);
                }
                TokenType::GreaterThan => {
                    angle_depth = angle_depth.saturating_sub(1);
                    if angle_depth == 0 {
                        self.bump();

                        return Some(has_tree_disambiguator);
                    }
                }
                TokenType::OpenParenthesis => paren_depth += 1,
                TokenType::CloseParenthesis => paren_depth = paren_depth.saturating_sub(1),
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenType::Comma | TokenType::Assign
                    if angle_depth == 1
                        && paren_depth == 0
                        && bracket_depth == 0
                        && brace_depth == 0 =>
                {
                    has_tree_disambiguator = true;
                }
                _ => {}
            }

            // note top level `extends`
            if angle_depth == 1
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && self.current_keyword() == Some(Keyword::Extends)
            {
                has_tree_disambiguator = true;
            }

            self.bump();
        }
    }

    /// Scan the generic arrow parameter list.
    fn scan_generic_arrow_parameters(&mut self) -> bool {
        if !self.peek_is(TokenType::OpenParenthesis) {
            return false;
        }

        let mut depth = 0usize;

        loop {
            let token_type = self.peek_token_type();

            match token_type {
                TokenType::End => return false,
                TokenType::Semicolon
                    if self.current_token_is_statement_recovery_boundary(token_type) =>
                {
                    return false;
                }
                TokenType::OpenParenthesis => depth += 1,
                TokenType::CloseParenthesis => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        self.bump();

                        return true;
                    }
                }
                _ => {}
            }

            self.bump();
        }
    }

    /// Peek a tree literal (including the `<` and `>` tokens).
    #[inline]
    pub fn peek_tree_literal(&mut self) -> ParserResult<()> {
        let mark = self.cursor_checkpoint();
        let result = (|| {
            if !self.peek_is(TokenType::LessThan) {
                return Err(ParserError::unexpected(self.peek()?.span));
            }
            let unexpected_span = self.peek()?.span;

            // probe the immediate tree head shape in tag mode
            self.bump_with_contextual_lex_mode(ContextualLexMode::TreeTag);

            let next_token_type = self.peek_token_type();
            if next_token_type == TokenType::Divide {
                return Err(ParserError::unexpected(unexpected_span));
            }
            let starts_tag_close = self.peek_starts_tree_tag_close();
            if !starts_tag_close && next_token_type != TokenType::Identifier {
                return Err(ParserError::unexpected(unexpected_span));
            }

            // exclude generic arrow function disambiguation: <T,>(...)
            if next_token_type == TokenType::Identifier {
                self.eat_tree_literal_identifier()?;

                if self.peek_is(TokenType::Comma) {
                    return Err(ParserError::unexpected(unexpected_span));
                }
            }

            Ok(())
        })();

        self.rewind(mark);

        result
    }

    /// Skip whitespace-only tree string tokens.
    /// Whitespace-only text between sibling tree elements is ignored.
    pub(crate) fn skip_tree_whitespace(&mut self) -> ParserResult<bool> {
        self.skip_tree_whitespace_in_child_mode(ContextualLexMode::Normal)
    }

    /// Skip whitespace-only tree content in the requested lexing mode.
    pub(crate) fn skip_tree_whitespace_in_child_mode(
        &mut self,
        follow_mode: ContextualLexMode,
    ) -> ParserResult<bool> {
        let mut skipped = false;
        loop {
            let token = *self.peek()?;

            // skip non-meaningful whitespace-only tree strings
            if token.token.ty == TokenType::Literal
                && token.token.literal == Some(TokenLiteral::TreeString)
            {
                let content = self.get_span_str(token.span);
                if Self::tree_text_is_ignored_whitespace(content) {
                    self.bump_with_contextual_lex_mode(follow_mode);
                    skipped = true;
                    continue;
                }
            }

            break;
        }

        Ok(skipped)
    }

    /// Return true when a tree text token is ignored whitespace.
    fn tree_text_is_ignored_whitespace(content: &str) -> bool {
        let mut has_newline = false;

        for byte in content.bytes() {
            match byte {
                b'\n' | b'\r' => has_newline = true,
                b' ' | b'\t' => {}
                _ => return false,
            }
        }

        has_newline
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
    pub fn eat_tree_literal(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_tree_literal_with_follow(ContextualLexMode::Normal)
    }

    /// Eat a tree literal and advance in the requested mode after it closes.
    pub(crate) fn eat_tree_literal_with_follow(
        &mut self,
        follow_mode: ContextualLexMode,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let first = self.eat_tree_literal_open(follow_mode)?;
        let mut stack = match first {
            TreeLiteralOpen::Complete(expression_id) => return Ok(expression_id),
            TreeLiteralOpen::Open(tree_literal) => vec![tree_literal],
        };

        loop {
            self.skip_tree_whitespace_in_child_mode(ContextualLexMode::TreeChild)?;

            let Some(current) = stack.last() else {
                return Err(ParserError::unexpected(self.anchor_span_here()));
            };
            let closes_current = self
                .try_eat_tree_literal_closing(current.path.as_ref(), current.close_follow_mode)?;
            if closes_current {
                let Some(tree_literal) = stack.pop() else {
                    return Err(ParserError::unexpected(self.anchor_span_here()));
                };
                let expression_id = self.insert_tree_literal_expression(tree_literal, true)?;

                if let Some(parent) = stack.last_mut() {
                    let element = self.insert_tree_literal_element(expression_id);
                    parent.elements.push(element);
                    continue;
                }

                return Ok(expression_id);
            }

            if self.peek_is(TokenType::LessThan) && self.peek_tree_literal().is_ok() {
                match self.eat_tree_literal_open(ContextualLexMode::TreeChild)? {
                    TreeLiteralOpen::Complete(expression_id) => {
                        let element = self.insert_tree_literal_element(expression_id);
                        let Some(parent) = stack.last_mut() else {
                            return Err(ParserError::unexpected(self.anchor_span_here()));
                        };
                        parent.elements.push(element);
                    }
                    TreeLiteralOpen::Open(tree_literal) => stack.push(tree_literal),
                }
                continue;
            }

            let element_ambient_context = self.flags.with_tree_literal(true);
            let element_expression_context =
                self.flags.not_in_position().with_statement_position(true);
            let flags = self
                .flags
                .with_ambient_context(element_ambient_context)
                .with_expression_context(element_expression_context);
            let old_flags = self.swap_flags(flags);
            let element = self.eat_tree_argument_with_follow(ContextualLexMode::TreeChild);
            self.restore_flags(old_flags);

            let element = element?;
            let Some(parent) = stack.last_mut() else {
                return Err(ParserError::unexpected(self.anchor_span_here()));
            };
            parent.elements.push(element);
        }
    }

    /// Eat one tree literal opening.
    fn eat_tree_literal_open(
        &mut self,
        follow_mode: ContextualLexMode,
    ) -> ParserResult<TreeLiteralOpen> {
        let start = self.span_start();
        self.eat_tree_opening_angle()?;

        // parse the tag path
        let mut path_segment_spans = None;
        let path: Option<Path> = if self.peek_is(TokenType::Identifier) {
            let (path, segment_spans, _last_span) =
                self.eat_tree_literal_path_with_endpoint_spans()?;

            // jsx namespace names cannot be followed by member access
            if self.tree_literal_path_has_namespace_member(&path) {
                return Err(ParserError::unexpected(self.peek()?.span));
            }

            path_segment_spans = Some(segment_spans);
            Some(path)
        } else {
            None
        };

        // generic arguments on the tag: typed and tree-tag components
        let generic_arguments = if path.is_some()
            && (self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft))
        {
            let static_ambient_context = self.flags.with_tree_literal(false);
            let flags = self.flags.with_ambient_context(static_ambient_context);
            let old_flags = self.swap_flags(flags);
            let generic_arguments = self.eat_generic_arguments();
            self.restore_flags(old_flags);

            generic_arguments?
        } else {
            Vec::new()
        };

        // parse tag arguments
        let arguments = self.eat_tree_literal_header_arguments()?;

        // close self-closing tags immediately
        if self.peek_is(TokenType::Divide) {
            self.bump();
            self.eat_tree_tag_close(follow_mode)?;
            let opening_span = self.get_span_from(&start);
            let tree_literal = OpenTreeLiteral {
                start,
                path,
                path_segment_spans,
                generic_arguments,
                arguments,
                elements: Vec::new(),
                opening_span,
                close_follow_mode: follow_mode,
            };
            let expression_id = self.insert_tree_literal_expression(tree_literal, false)?;

            return Ok(TreeLiteralOpen::Complete(expression_id));
        }

        // enter tree child lexing after an opening tag
        self.eat_tree_tag_close(ContextualLexMode::TreeChild)?;
        let opening_span = self.get_span_from(&start);
        self.skip_tree_whitespace_in_child_mode(ContextualLexMode::TreeChild)?;

        Ok(TreeLiteralOpen::Open(OpenTreeLiteral {
            start,
            path,
            path_segment_spans,
            generic_arguments,
            arguments,
            elements: Vec::new(),
            opening_span,
            close_follow_mode: follow_mode,
        }))
    }

    /// Eat tree literal header arguments.
    fn eat_tree_literal_header_arguments(
        &mut self,
    ) -> ParserResult<Option<Vec<LocalNodeId<Argument>>>> {
        self.skip_tree_whitespace()?;
        if self.peek_is(TokenType::Divide) || self.peek_starts_tree_tag_close() {
            return Ok(None);
        }

        let mut arguments = Vec::new();
        while self.has_more_tokens() {
            self.skip_tree_whitespace()?;
            if self.peek_is(TokenType::Divide) || self.peek_starts_tree_tag_close() {
                break;
            }

            let argument_ambient_context = self.flags.with_tree_literal(true);
            let argument_expression_context = self.flags.not_in_position();
            let flags = self
                .flags
                .with_ambient_context(argument_ambient_context)
                .with_expression_context(argument_expression_context);
            let old_flags = self.swap_flags(flags);
            let argument = self.eat_tree_literal_argument();
            self.restore_flags(old_flags);

            let argument = argument?;
            arguments.push(argument);
        }

        Ok(Some(arguments))
    }

    /// Try to eat a closing tag for the current open tree literal.
    fn try_eat_tree_literal_closing(
        &mut self,
        path: Option<&Path>,
        follow_mode: ContextualLexMode,
    ) -> ParserResult<bool> {
        if !self.peek_is(TokenType::LessThan) {
            return Ok(false);
        }

        let closing_start = self.cursor_checkpoint();
        self.bump_with_contextual_lex_mode(ContextualLexMode::TreeTag);

        if !self.peek_is(TokenType::Divide) {
            self.rewind(closing_start);
            return Ok(false);
        }
        self.bump();

        if path.is_none() && self.peek_starts_tree_tag_close() {
            self.eat_tree_tag_close(follow_mode)?;
            return Ok(true);
        }

        if path.is_some() && self.peek_starts_tree_tag_close() {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        let Some(path) = path else {
            return Err(ParserError::unexpected(self.peek()?.span));
        };

        let closing_path = self.eat_tree_literal_path()?;
        if self.tree_literal_path_has_namespace_member(&closing_path) {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        self.skip_tree_whitespace()?;
        self.eat_tree_tag_close(follow_mode)?;
        if closing_path != *path {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        Ok(true)
    }

    /// Insert one tree literal expression from an open tree literal.
    fn insert_tree_literal_expression(
        &mut self,
        tree_literal: OpenTreeLiteral,
        has_children: bool,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let OpenTreeLiteral {
            start,
            path,
            path_segment_spans,
            generic_arguments,
            arguments,
            elements,
            opening_span,
            close_follow_mode: _,
        } = tree_literal;

        let left = if let Some(path) = path {
            let Some(segment_spans) = path_segment_spans.as_deref() else {
                return Err(ParserError::unexpected(opening_span));
            };
            let Some(first_segment_span) = segment_spans.first().copied() else {
                return Err(ParserError::unexpected(opening_span));
            };
            let Some(last_segment_span) = segment_spans.last().copied() else {
                return Err(ParserError::unexpected(opening_span));
            };
            let path_span = Span::new(
                first_segment_span.file,
                first_segment_span.start,
                last_segment_span.end,
            );
            let expression_id = self.insert_node(
                Expression::QualifiedReference {
                    path,
                    generic_arguments: vec![],
                },
                path_span,
            );
            self.set_path_expression_spans(expression_id, segment_spans)?;
            Some(expression_id)
        } else {
            None
        };

        let elements = has_children.then_some(elements);
        let expression = Expression::TreeExpression {
            left,
            generic_arguments,
            arguments,
            elements,
        };
        let expression_id = self.insert_node(expression, self.get_span_from(&start));
        self.tree.set_side_span(
            expression_id,
            NodeSpanType::Region(NodeSpanRegion::Opening),
            opening_span,
        );

        Ok(expression_id)
    }

    /// Insert one positional tree child for a parsed tree literal expression.
    fn insert_tree_literal_element(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Argument> {
        self.insert_node(
            Argument::Positional {
                value: expression_id,
            },
            self.tree.get_span(expression_id),
        )
    }

    /// Return true when a tree literal path combines namespace and member syntax.
    pub(crate) fn tree_literal_path_has_namespace_member(&self, path: &Path) -> bool {
        if path.segments.len() <= 1 {
            return false;
        }

        path.segments
            .iter()
            .any(|segment| self.strings.get(*segment).contains(':'))
    }
}
