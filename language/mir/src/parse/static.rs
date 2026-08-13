use destack_core::StringId;

use crate::source::TokenType;
use crate::{Static, StaticField, StaticId, StaticKey, TypeId};

use super::{ParseError, ParseResult, Parser};

impl Parser {
    /// Parse one closed compile-time value.
    pub(super) fn parse_static(&mut self) -> ParseResult<StaticId> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("static value", self.pos()))?;
        let ty = self.token_type(token);
        let text = self.tree.source_text(token.span).to_string();
        let start = token.start();

        match ty {
            TokenType::Identifier if text == "null" => {
                self.bump();
                Ok(self.tree.intern_static(Static::Null))
            }
            TokenType::Identifier if text == "undefined" => {
                self.bump();
                Ok(self.tree.intern_static(Static::Undefined))
            }
            TokenType::Identifier if text == "NaN" => {
                self.bump();
                Ok(self.tree.intern_static(Static::Float(f64::NAN.to_bits())))
            }
            TokenType::Identifier if text == "Infinity" => {
                self.bump();
                Ok(self
                    .tree
                    .intern_static(Static::Float(f64::INFINITY.to_bits())))
            }
            TokenType::Identifier if text == "-Infinity" => {
                self.bump();
                Ok(self
                    .tree
                    .intern_static(Static::Float(f64::NEG_INFINITY.to_bits())))
            }
            TokenType::BooleanLiteral => {
                self.bump();
                Ok(self.tree.intern_static(Static::Boolean(text == "true")))
            }
            TokenType::Integer => self.parse_static_integer(&text, start),
            TokenType::Float => self.parse_static_float(&text, start),
            TokenType::Character => self.parse_static_character(&text, start),
            TokenType::String => self.parse_static_string(&text, start),
            TokenType::Regex => self.parse_static_regex(&text, start),
            TokenType::OpenParenthesis => self.parse_static_tuple(),
            TokenType::OpenBracket => self.parse_static_array(),
            TokenType::OpenBrace => self.parse_static_object(),
            TokenType::Type => self.parse_static_type(),
            _ => self.parse_static_type_or_nominal(),
        }
    }

    /// Parse one static integer or bigint literal.
    fn parse_static_integer(&mut self, text: &str, start: usize) -> ParseResult<StaticId> {
        self.bump();
        let text = text.replace('_', "");

        if let Some(value) = text.strip_suffix('n') {
            let value = Self::parse_integer(value)
                .ok_or_else(|| ParseError::invalid("static bigint", start))?;

            Ok(self.tree.intern_static(Static::Bigint(value)))
        } else {
            let value = Self::parse_integer(&text)
                .ok_or_else(|| ParseError::invalid("static integer", start))?;

            Ok(self.tree.intern_static(Static::Integer(value)))
        }
    }

    /// Parse one static floating-point literal.
    fn parse_static_float(&mut self, text: &str, start: usize) -> ParseResult<StaticId> {
        self.bump();
        let text = text.replace('_', "");
        let value = text
            .parse::<f64>()
            .map_err(|_| ParseError::invalid("static float", start))?;

        Ok(self.tree.intern_static(Static::Float(value.to_bits())))
    }

    /// Parse one static character literal.
    fn parse_static_character(&mut self, text: &str, start: usize) -> ParseResult<StaticId> {
        self.bump();
        let value = self
            .parse_char_literal(text)
            .ok_or_else(|| ParseError::invalid("static character", start))?;

        Ok(self.tree.intern_static(Static::Character(value)))
    }

    /// Parse one static string literal.
    fn parse_static_string(&mut self, text: &str, start: usize) -> ParseResult<StaticId> {
        self.bump();
        let value = self
            .parse_string_literal(text)
            .ok_or_else(|| ParseError::invalid("static string", start))?;
        let value = self.strings.intern(&value);

        Ok(self.tree.intern_static(Static::String(value)))
    }

    /// Parse one static regular expression literal.
    fn parse_static_regex(&mut self, text: &str, start: usize) -> ParseResult<StaticId> {
        self.bump();
        let Some(close) = text.rfind('/') else {
            return Err(ParseError::invalid("static regular expression", start));
        };
        if close == 0 {
            return Err(ParseError::invalid("static regular expression", start));
        }

        let content = self.strings.intern(&text[1..close]);
        let flags = (close + 1 < text.len()).then(|| self.strings.intern(&text[close + 1..]));

        Ok(self.tree.intern_static(Static::Regex { content, flags }))
    }

    /// Parse one static array or repeated fixed array.
    fn parse_static_array(&mut self) -> ParseResult<StaticId> {
        self.eat_token(TokenType::OpenBracket)?;
        if self.eat_token_if(TokenType::CloseBracket) {
            return Ok(self.tree.intern_static(Static::Array(Vec::new())));
        }

        let first = self.parse_static()?;
        if self.eat_token_if(TokenType::Semicolon) {
            let length = self.parse_static_u64("fixed array length")?;
            self.eat_token(TokenType::CloseBracket)?;

            return Ok(self.tree.intern_static(Static::FixedArray {
                value: first,
                length,
            }));
        }

        let mut values = vec![first];
        while self.eat_token_if(TokenType::Comma) && !self.peek_is(TokenType::CloseBracket) {
            values.push(self.parse_static()?);
        }
        self.eat_token(TokenType::CloseBracket)?;

        Ok(self.tree.intern_static(Static::Array(values)))
    }

    /// Parse one static tuple or parenthesized value.
    fn parse_static_tuple(&mut self) -> ParseResult<StaticId> {
        self.eat_token(TokenType::OpenParenthesis)?;
        if self.eat_token_if(TokenType::CloseParenthesis) {
            return Ok(self.tree.intern_static(Static::Tuple(Vec::new())));
        }

        let first = self.parse_static()?;
        if !self.eat_token_if(TokenType::Comma) {
            self.eat_token(TokenType::CloseParenthesis)?;

            return Ok(first);
        }

        let mut values = vec![first];
        while !self.peek_is(TokenType::CloseParenthesis) {
            values.push(self.parse_static()?);
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(self.tree.intern_static(Static::Tuple(values)))
    }

    /// Parse one explicitly type-space static value.
    fn parse_static_type(&mut self) -> ParseResult<StaticId> {
        self.bump();
        let (ty, _) = self.parse_type_part()?;

        Ok(self.tree.intern_static(Static::Type(ty)))
    }

    /// Parse one bare type, nominal newtype value, or nominal struct value.
    fn parse_static_type_or_nominal(&mut self) -> ParseResult<StaticId> {
        let (ty, _) = self.parse_type_part()?;

        if self.peek_is(TokenType::OpenParenthesis) {
            self.parse_static_newtype(ty)
        } else if self.peek_is(TokenType::OpenBrace) {
            let fields = self.parse_static_fields()?;

            Ok(self.tree.intern_static(Static::Struct { ty, fields }))
        } else {
            Ok(self.tree.intern_static(Static::Type(ty)))
        }
    }

    /// Parse one static nominal newtype value.
    fn parse_static_newtype(&mut self, ty: TypeId) -> ParseResult<StaticId> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut values = Vec::new();

        while !self.peek_is(TokenType::CloseParenthesis) {
            values.push(self.parse_static()?);
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        let value = if values.len() == 1 {
            values[0]
        } else {
            self.tree.intern_static(Static::Tuple(values))
        };

        Ok(self.tree.intern_static(Static::Newtype { ty, value }))
    }

    /// Parse one static structural object.
    fn parse_static_object(&mut self) -> ParseResult<StaticId> {
        let fields = self.parse_static_fields()?;

        Ok(self.tree.intern_static(Static::Object(fields)))
    }

    /// Parse one ordered static field set.
    fn parse_static_fields(&mut self) -> ParseResult<Vec<StaticField>> {
        self.eat_token(TokenType::OpenBrace)?;
        let mut fields = Vec::new();

        while !self.peek_is(TokenType::CloseBrace) {
            let key = self.parse_static_key()?;
            self.eat_token(TokenType::Colon)?;
            let value = self.parse_static()?;
            fields.push(StaticField { key, value });

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::CloseBrace)?;

        Ok(fields)
    }

    /// Parse one static object key.
    fn parse_static_key(&mut self) -> ParseResult<StaticKey> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("static key", self.pos()))?;
        let ty = self.token_type(token);
        let text = self.tree.source_text(token.span).to_string();
        let start = token.start();

        match ty {
            TokenType::String => {
                let name = self.parse_static_string_id("static key")?;
                Ok(StaticKey::Name(name))
            }
            TokenType::Identifier => {
                self.bump();
                Ok(StaticKey::Name(self.strings.intern(&text)))
            }
            TokenType::Integer => {
                let index = self.parse_static_u64("static index")?;

                Ok(StaticKey::Index(index))
            }
            _ => Err(ParseError::unexpected("static key", ty, start)),
        }
    }

    /// Parse one static string and intern it.
    fn parse_static_string_id(&mut self, expected: &str) -> ParseResult<StringId> {
        let token = self.eat_token(TokenType::String)?;
        let text = self.tree.source_text(token.span).to_string();
        let value = self
            .parse_string_literal(&text)
            .ok_or_else(|| ParseError::invalid(expected, token.start()))?;

        Ok(self.strings.intern(&value))
    }

    /// Parse one decimal or hexadecimal static unsigned integer.
    fn parse_static_u64(&mut self, expected: &str) -> ParseResult<u64> {
        let token = self.eat_token(TokenType::Integer)?;
        let text = self.tree.source_text(token.span).replace('_', "");
        let value = if let Some(hex) = text.strip_prefix("0x") {
            u64::from_str_radix(hex, 16)
        } else {
            text.parse::<u64>()
        };

        value.map_err(|_| ParseError::invalid(expected, token.start()))
    }

    /// Parse one signed integer literal.
    fn parse_integer(text: &str) -> Option<i64> {
        let (sign, text) = text
            .strip_prefix('-')
            .map_or((1_i128, text), |text| (-1_i128, text));
        let (radix, digits) = if let Some(digits) = text.strip_prefix("0x") {
            (16, digits)
        } else if let Some(digits) = text.strip_prefix("0o") {
            (8, digits)
        } else if let Some(digits) = text.strip_prefix("0b") {
            (2, digits)
        } else {
            (10, text)
        };
        let magnitude = i128::from_str_radix(digits, radix).ok()?;
        let value = sign * magnitude;

        i64::try_from(value).ok()
    }
}
