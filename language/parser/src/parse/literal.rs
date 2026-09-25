use std::borrow::Cow;

use crate::lex::{InvalidEscape, cook};

use crate::lex::decode_html_entity;
use crate::{Parser, ParserError, ParserResult};

use tspp_dir::{Literal, NodeType, NumberBase, TokenLiteral, TokenSpan, TokenType};

/// One integer token body and its lexer classification.
#[derive(Clone, Copy)]
struct IntegerLiteral<'a> {
    /// The complete token source.
    source: &'a str,
    /// The classified numeric base.
    base: NumberBase,
    /// Whether the token has a bigint suffix.
    is_bigint: bool,
}

impl<'a> IntegerLiteral<'a> {
    /// Create one classified integer literal.
    const fn new(source: &'a str, base: NumberBase, is_bigint: bool) -> Self {
        Self {
            source,
            base,
            is_bigint,
        }
    }

    /// Return whether this is a legacy leading-zero decimal.
    fn uses_legacy_leading_zero(self) -> bool {
        if self.base != NumberBase::Decimal {
            return false;
        }

        let Some(body) = self.body() else {
            return false;
        };
        let bytes = body.as_bytes();
        if bytes.len() < 2 || bytes[0] != b'0' {
            return false;
        }

        bytes[1].is_ascii_digit() || bytes[1] == b'_'
    }

    /// Return whether this is a legacy octal without an explicit prefix.
    fn is_legacy_octal(self) -> bool {
        if self.base != NumberBase::Octal {
            return false;
        }

        let Some(body) = self.body() else {
            return false;
        };

        body.starts_with('0') && !body.starts_with("0o") && !body.starts_with("0O")
    }

    /// Return whether the token contains digits invalid for its base.
    fn has_invalid_digits(self) -> bool {
        let Some(digits) = self.digit_source() else {
            return true;
        };
        if digits.is_empty() {
            return true;
        }

        let radix = self.radix();
        digits
            .chars()
            .any(|character| character != '_' && character.to_digit(radix).is_none())
    }

    /// Convert this integer with saturation at the DIR scalar limit.
    fn value(self) -> Option<i64> {
        let digits = self.digit_source()?;

        let radix = self.radix();
        let mut value = 0u128;
        let maximum = i64::MAX as u128;

        for character in digits.chars() {
            if character == '_' {
                continue;
            }

            let digit = character.to_digit(radix)? as u128;
            let next = value
                .checked_mul(radix as u128)
                .and_then(|value| value.checked_add(digit));
            let Some(next) = next else {
                return Some(i64::MAX);
            };
            if next > maximum {
                return Some(i64::MAX);
            }
            value = next;
        }

        Some(value as i64)
    }

    /// Return the token body without its bigint suffix.
    fn body(self) -> Option<&'a str> {
        if self.is_bigint {
            self.source.strip_suffix('n')
        } else {
            Some(self.source)
        }
    }

    /// Return the digits after the suffix and radix prefix.
    fn digit_source(self) -> Option<&'a str> {
        let body = self.body()?;

        match self.base {
            NumberBase::Decimal => Some(body),
            NumberBase::Binary => body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")),
            NumberBase::Octal => body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")),
            NumberBase::Hexadecimal => body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")),
        }
    }

    /// Return the integer radix.
    const fn radix(self) -> u32 {
        match self.base {
            NumberBase::Decimal => 10,
            NumberBase::Binary => 2,
            NumberBase::Octal => 8,
            NumberBase::Hexadecimal => 16,
        }
    }
}

impl Parser {
    /// Parse one numeric scalar literal with an explicit sign.
    pub(crate) fn parse_signed_numeric_literal(&mut self) -> ParserResult<Literal> {
        let start = self.mark_parse_start();
        let is_negative = match self.peek_token_type() {
            TokenType::Add => false,
            TokenType::Subtract => true,
            _ => return Err(ParserError::unexpected(self.peek_token_span())),
        };

        // apply the sign only to numeric scalar families
        self.bump();
        let value = self.parse_scalar_literal()?;
        let value = match (is_negative, value) {
            (true, Literal::Integer(number)) => Literal::Integer(-number),
            (true, Literal::Bigint(number)) => Literal::Bigint(-number),
            (true, Literal::Float(number)) => Literal::Float(-number),
            (false, value @ Literal::Integer(_))
            | (false, value @ Literal::Bigint(_))
            | (false, value @ Literal::Float(_)) => value,
            (_, _) => return Err(ParserError::unexpected(self.range_since(&start))),
        };

        Ok(value)
    }

    /// Decode one single-quoted character literal.
    fn decode_character_literal(literal: &str) -> Option<char> {
        let content = literal.strip_prefix('\'')?.strip_suffix('\'')?;
        let decoded = cook(content).ok()?;
        let mut characters = decoded.chars();
        let character = characters.next()?;

        characters.next().is_none().then_some(character)
    }

    /// Return true when the current token starts a template literal.
    #[inline]
    pub fn peek_template_literal_start(&self) -> bool {
        self.peek_is(TokenType::TemplateString) || self.peek_is(TokenType::TemplateStringStart)
    }

    /// Return true when the current token starts a scalar literal.
    #[inline]
    pub fn peek_scalar_literal_start(&self) -> bool {
        self.peek_is(TokenType::Literal)
    }

    /// Peek a scalar literal token.
    #[inline]
    pub fn peek_scalar_literal(&self) -> ParserResult<TokenSpan> {
        if self.peek_is(TokenType::Literal) {
            Ok(self.peek_token_span())
        } else {
            Err(ParserError::unexpected(self.peek_token_span()))
        }
    }

    /// Parse a scalar literal and return its value.
    ///
    /// Examples:
    /// ```tspp
    /// true
    /// false
    /// 1
    /// 1n
    /// 1.0
    /// 0x1234
    /// "Hello, world!"
    /// 'a'
    /// b'a'
    /// b"abc"
    /// /abc/
    /// /abc/g
    /// ```
    pub fn parse_scalar_literal(&mut self) -> ParserResult<Literal> {
        let literal_span = self.eat();
        let Some(body) = literal_span.token.literal() else {
            return Err(ParserError::unexpected(literal_span.span.range()));
        };
        let has_adjacent_identifier_suffix =
            self.peek_numeric_literal_adjacent_identifier_suffix(literal_span);
        let literal_str = self.file.span_str(literal_span.span);

        match body {
            // boolean literal
            TokenLiteral::Boolean { value } => Ok(Literal::Boolean(value)),

            // int literal
            TokenLiteral::Int {
                base,
                is_empty,
                is_bigint,
            } => {
                let integer = IntegerLiteral::new(literal_str, base, is_bigint);

                if is_empty {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // reject legacy leading-zero decimal forms
                if integer.uses_legacy_leading_zero() {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // reject legacy octal literals without an explicit 0o/0O prefix
                if integer.is_legacy_octal() {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // reject invalid digits for prefixed literals
                if integer.has_invalid_digits() {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // require a separator before an identifier starts
                if has_adjacent_identifier_suffix {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // convert the validated token into the DIR scalar range
                let Some(value) = integer.value() else {
                    return Err(ParserError::unexpected(literal_span.span.range()));
                };

                // int
                if is_bigint {
                    Ok(Literal::Bigint(value))
                } else {
                    Ok(Literal::Integer(value))
                }
            }

            // float literal
            TokenLiteral::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // reject legacy leading-zero decimal forms
                if Self::uses_legacy_float_leading_zero(literal_str) {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // require a separator after numeric literals before an identifier starts
                if has_adjacent_identifier_suffix {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                let content: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                content.parse::<f64>().map(Literal::Float).map_err(|_| {
                    ParserError::expected(literal_span.span.range(), TokenType::Literal)
                        .in_node(NodeType::Expression)
                })
            }

            // html entity character literal
            TokenLiteral::Character {
                is_terminated,
                is_html_entity,
            } => {
                if !is_terminated || !is_html_entity {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                decode_html_entity(literal_str)
                    .map(Literal::Character)
                    .ok_or_else(|| {
                        ParserError::expected(literal_span.span.range(), TokenType::Literal)
                            .in_node(NodeType::Expression)
                    })
            }

            // string or TS++ character literal
            TokenLiteral::String {
                is_terminated,
                has_invalid_escape,
            } => {
                if !is_terminated || has_invalid_escape {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                if literal_str.starts_with('\'') {
                    return Self::decode_character_literal(literal_str)
                        .map(Literal::Character)
                        .ok_or_else(|| {
                            ParserError::expected(literal_span.span.range(), TokenType::Literal)
                                .in_node(NodeType::Expression)
                        });
                }

                // remove the matching string delimiters
                let has_delimiters = literal_str.len() >= 2
                    && ((literal_str.starts_with('"') && literal_str.ends_with('"'))
                        || (literal_str.starts_with('\'') && literal_str.ends_with('\'')));
                if !has_delimiters {
                    return Err(ParserError::unexpected(literal_span.span.range()));
                }
                let content = &literal_str[1..literal_str.len() - 1];
                let content = cook(content).map_err(|InvalidEscape| {
                    ParserError::expected(literal_span.span.range(), TokenType::Literal)
                        .in_node(NodeType::Expression)
                })?;

                let string_id = self.strings.intern(&content);
                Ok(Literal::String(string_id))
            }

            // regular expression literal
            TokenLiteral::RegexString { has_flags } => {
                // require opening and closing delimiters
                let closing = literal_str.rfind('/');
                let Some(closing) = closing.filter(|closing| *closing > 0) else {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                };
                if !literal_str.starts_with('/')
                    || (!has_flags && closing + 1 != literal_str.len())
                    || (has_flags && closing + 1 == literal_str.len())
                {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // reject raw line terminators
                let content = &literal_str[1..closing];
                if content
                    .chars()
                    .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
                {
                    return Err(ParserError::expected(
                        literal_span.span.range(),
                        TokenType::Literal,
                    )
                    .in_node(NodeType::Expression));
                }

                // retain the authored pattern and flags
                let content = self.strings.intern(content);
                let flags = has_flags.then(|| self.strings.intern(&literal_str[closing + 1..]));

                Ok(Literal::RegexString { content, flags })
            }

            // tree text content, raw text inside tree literals
            TokenLiteral::TreeString => {
                let string_id = self.strings.intern(literal_str);
                Ok(Literal::String(string_id))
            }
        }
    }

    /// Re-lex and eat the current regex literal.
    pub(crate) fn parse_regex_literal(&mut self) -> ParserResult<Literal> {
        if !self.re_lex_regex() {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        self.parse_scalar_literal()
    }

    /// Return true when a float literal uses legacy leading-zero syntax.
    fn uses_legacy_float_leading_zero(literal: &str) -> bool {
        let bytes = literal.as_bytes();
        if bytes.len() < 2 || bytes[0] != b'0' {
            return false;
        }

        let second = bytes[1] as char;
        second.is_ascii_digit() || second == '_'
    }

    /// Return true when a numeric literal is immediately followed by an identifier.
    fn peek_numeric_literal_adjacent_identifier_suffix(&self, literal: TokenSpan) -> bool {
        let next = self.peek_token_span();
        if next.token.ty() != TokenType::Identifier {
            return false;
        }

        next.span.start == literal.span.end
    }
}
