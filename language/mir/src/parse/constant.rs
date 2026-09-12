use crate::source::TokenType;
use destack_source::Span;

use crate::{Constant, FloatType, Intrinsic, LayoutMeasure, LocalNodeId, StorageSet, Type, TypeId};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

impl Parser {
    /// Parse one witness constant: the receiver type, the applied interface, and the member.
    fn parse_witness_constant(&mut self) -> ParseResult<Constant> {
        let (receiver, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;
        let (interface, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("witness member", self.pos()))?;
        if self.token_type(token) != TokenType::Identifier {
            return Err(ParseError::invalid("witness member", token.start()));
        }
        let member = self.tree.source_text(token.span).to_string();
        self.bump();

        Ok(Constant::Witness {
            receiver: TypeId::from(receiver),
            interface: TypeId::from(interface),
            member: self.strings.intern(&member),
        })
    }

    /// Parse a constant.
    pub(super) fn parse_constant(&mut self) -> ParseResult<Constant> {
        // read the next token
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("constant", self.pos()))?;
        let kind = self.token_type(token);
        let token_text = self.tree.source_text(token.span).to_string();
        let token_start = token.start();

        // parse the literal
        match kind {
            TokenType::Identifier if token_text == "null" => {
                self.bump();
                Ok(Constant::Null)
            }
            TokenType::Identifier if let Some((index, _)) = self.generic_parameter(&token_text) => {
                self.bump();
                Ok(Constant::Parameter(index))
            }
            TokenType::Identifier
                if let Some(measure) = LayoutMeasure::from_keyword(&token_text) =>
            {
                self.bump();
                let (ty, _) = self.parse_type_use_part()?;
                Ok(Constant::Layout {
                    ty: TypeId::from(ty),
                    measure,
                })
            }
            TokenType::Identifier if token_text == "witness" => {
                self.bump();
                self.parse_witness_constant()
            }
            TokenType::BooleanLiteral => {
                let value = token_text == "true";
                self.bump();
                Ok(Constant::Boolean { value })
            }
            TokenType::Integer => {
                self.bump();
                self.parse_int_constant(&token_text)
                    .or_else(|| self.parse_float_constant(&token_text))
                    .ok_or_else(|| {
                        ParseError::invalid(
                            &format!("numeric constant '{token_text}'"),
                            token_start,
                        )
                    })
            }
            TokenType::Float => {
                self.bump();
                self.parse_float_constant(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("float constant '{token_text}'"), token_start)
                })
            }
            TokenType::Identifier if self.parse_float_constant(&token_text).is_some() => {
                let value = self.parse_float_constant(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("float constant '{token_text}'"), token_start)
                })?;
                self.bump();
                Ok(value)
            }
            TokenType::Character => {
                self.bump();
                let value = self.parse_char_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("char literal '{token_text}'"), token_start)
                })?;
                Ok(Constant::Char { value })
            }
            _ => Err(ParseError::unexpected("constant", kind, token_start)),
        }
    }

    /// Parse a constant and validate it against the expected type.
    pub(super) fn parse_constant_for_type(
        &mut self,
        expected_type: LocalNodeId<Type>,
    ) -> ParseResult<Constant> {
        // read the next token
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("constant", self.pos()))?;
        let kind = self.token_type(token);
        let token_text = self.tree.source_text(token.span).to_string();
        let token_start = token.start();
        let expected_type = self.tree.storage_type(expected_type);
        let expected = self.tree.type_definition(expected_type).clone();

        // validate the literal against the expected type
        match kind {
            TokenType::Identifier if token_text == "null" => {
                if !matches!(expected, Type::Pointer { .. }) {
                    return Err(ParseError::invalid("null constant type", token_start));
                }
                self.bump();
                Ok(Constant::Null)
            }
            TokenType::Identifier if let Some((index, _)) = self.generic_parameter(&token_text) => {
                self.bump();
                Ok(Constant::Parameter(index))
            }
            TokenType::Identifier
                if let Some(measure) = LayoutMeasure::from_keyword(&token_text) =>
            {
                if !matches!(expected, Type::Int { .. } | Type::Usize | Type::Isize) {
                    return Err(ParseError::invalid("layout constant type", token_start));
                }
                self.bump();
                let (ty, _) = self.parse_type_use_part()?;
                Ok(Constant::Layout {
                    ty: TypeId::from(ty),
                    measure,
                })
            }
            TokenType::Identifier if token_text == "witness" => {
                self.bump();
                self.parse_witness_constant()
            }
            TokenType::BooleanLiteral => {
                if !matches!(expected, Type::Boolean) {
                    return Err(ParseError::invalid("boolean constant type", token_start));
                }
                let value = token_text == "true";
                self.bump();
                Ok(Constant::Boolean { value })
            }
            TokenType::Character => {
                if !matches!(expected, Type::Character) {
                    return Err(ParseError::invalid("char constant type", token_start));
                }
                self.bump();
                let value = self.parse_char_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("char literal '{token_text}'"), token_start)
                })?;
                Ok(Constant::Char { value })
            }
            TokenType::Integer | TokenType::Float | TokenType::Identifier
                if matches!(expected, Type::Float(_)) =>
            {
                self.bump();

                let has_suffix = token_text.contains("float") || token_text.contains("bfloat");
                let Type::Float(float_type) = expected else {
                    unreachable!("float type was checked by the match guard")
                };
                // typed literal
                if has_suffix {
                    let constant = self.parse_float_constant(&token_text).ok_or_else(|| {
                        ParseError::invalid(&format!("float constant '{token_text}'"), token_start)
                    })?;
                    match constant {
                        Constant::Float {
                            format: const_format,
                            ..
                        } if const_format == float_type => Ok(constant),
                        _ => Err(ParseError::invalid("float constant type", token_start)),
                    }
                } else {
                    // infer payload from expected type
                    let value: f64 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("float constant", token_start))?;
                    Ok(Constant::Float {
                        bits: destack_core::float_to_bits(float_type.format(), value),
                        format: float_type,
                    })
                }
            }
            TokenType::Integer => {
                self.bump();

                // expected integer shape
                let has_suffix = token_text.chars().any(|c| c.is_ascii_alphabetic());
                let (width, is_signed) = match self.tree.type_definition(expected_type) {
                    Type::Int { width, is_signed } => (*width, *is_signed),
                    Type::Isize => (self.target_layout.pointer_bits(), true),
                    Type::Usize => (self.target_layout.pointer_bits(), false),
                    _ => {
                        return Err(ParseError::invalid("integer constant type", token_start));
                    }
                };
                // typed literal
                if has_suffix {
                    let constant = self.parse_int_constant(&token_text).ok_or_else(|| {
                        ParseError::invalid(
                            &format!("integer constant '{token_text}'"),
                            token_start,
                        )
                    })?;
                    match constant {
                        Constant::Int {
                            width: const_width,
                            is_signed: true,
                            ..
                        } if is_signed && const_width == width => Ok(constant),
                        Constant::UInt {
                            width: const_width, ..
                        } if !is_signed && const_width == width => Ok(constant),
                        Constant::Int { .. } | Constant::UInt { .. } => {
                            Err(ParseError::invalid("integer constant type", token_start))
                        }
                        _ => Err(ParseError::invalid("integer constant type", token_start)),
                    }
                }
                // signed payload
                else if is_signed {
                    let value: i128 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("integer constant", token_start))?;
                    Ok(Constant::Int {
                        value,
                        width,
                        is_signed: true,
                    })
                } else {
                    // unsigned payload
                    let value: u128 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("integer constant", token_start))?;
                    Ok(Constant::UInt { value, width })
                }
            }
            _ => Err(ParseError::unexpected("constant", kind, token_start)),
        }
    }

    /// Parse one unsuffixed integer literal.
    pub(super) fn parse_int_literal(&mut self) -> ParseResult<i128> {
        let token = self.eat_token(TokenType::Integer)?;
        let text = self.tree.source_text(token.span).to_string();

        self.parse_int_literal_payload(&text, token.start())
    }

    /// Parse an integer literal and return its span.
    pub(super) fn parse_int_literal_part(&mut self) -> ParseResult<(i128, Span)> {
        let token = self.eat_token(TokenType::Integer)?;
        let token_start = token.start();
        let token_text = self.tree.source_text(token.span).to_string();
        let token_length = token_text.len();
        let span = self.span_at(token_start, token_length);

        let value = self.parse_int_literal_payload(&token_text, token_start)?;

        Ok((value, span))
    }

    /// Parse the unsuffixed integer payload from one MIR integer token.
    fn parse_int_literal_payload(&self, text: &str, start: usize) -> ParseResult<i128> {
        let digits: String = text
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();

        digits
            .parse()
            .map_err(|_| ParseError::invalid("integer", start))
    }

    /// Parse an integer literal and append its span as one source segment.
    pub(super) fn parse_int_segment(&mut self, segment_spans: &mut Vec<Span>) -> ParseResult<i128> {
        let (value, span) = self.parse_int_literal_part()?;
        segment_spans.push(span);

        Ok(value)
    }

    /// Parse one storage keyword.
    pub(super) fn parse_storage(&self, text: &str, start: usize) -> ParseResult<StorageSet> {
        let location = match text {
            "none" => StorageSet::NONE,
            "any" => StorageSet::ANY,
            "local" => StorageSet::LOCAL,
            "shared" => StorageSet::SHARED,
            "frame" => StorageSet::FRAME,
            "global" => StorageSet::GLOBAL,
            _ => {
                return Err(ParseError::invalid(&format!("storage '{text}'"), start));
            }
        };

        Ok(location)
    }

    /// Parse an integer constant with type suffix.
    pub(super) fn parse_int_constant(&self, text: &str) -> Option<Constant> {
        // find where the type suffix starts
        let suffix_start = text.find(|c: char| c.is_ascii_alphabetic())?;
        let (digits, suffix) = text.split_at(suffix_start);

        // parse the typed integer payload
        let (is_signed, width_text) = if let Some(width) = suffix.strip_prefix("int") {
            (true, width)
        } else if let Some(width) = suffix.strip_prefix("uint") {
            (false, width)
        } else if let Some(width) = suffix.strip_prefix('i') {
            (true, width)
        } else {
            let width = suffix.strip_prefix('u')?;
            (false, width)
        };
        let width: u16 = width_text.parse().ok()?;

        if is_signed {
            let value: i128 = digits.parse().ok()?;
            Some(Constant::Int {
                value,
                width,
                is_signed: true,
            })
        } else {
            let value: u128 = digits.parse().ok()?;
            Some(Constant::UInt { value, width })
        }
    }

    /// Parse a float constant with type suffix.
    pub(super) fn parse_float_constant(&self, text: &str) -> Option<Constant> {
        let suffix_start = text.rfind("float")?;
        let (digits, suffix) = text.split_at(suffix_start);
        let format = match suffix {
            "float32" => FloatType::Float32,
            "float64" => FloatType::Float64,
            _ => return None,
        };
        let value: f64 = digits.parse().ok()?;

        Some(Constant::Float {
            bits: destack_core::float_to_bits(format.format(), value),
            format,
        })
    }

    /// Parse a string literal, handling escape sequences.
    pub(super) fn parse_string_literal(&self, text: &str) -> Option<String> {
        let text = text.strip_prefix('"')?.strip_suffix('"')?;

        self.parse_escape_sequences(text)
    }

    /// Parse a char literal, handling escape sequences.
    pub(super) fn parse_char_literal(&self, text: &str) -> Option<char> {
        let text = text.strip_prefix('\'')?.strip_suffix('\'')?;
        let unescaped = self.parse_escape_sequences(text)?;
        let mut chars = unescaped.chars();
        let value = chars.next()?;

        // ensure the literal contains exactly one scalar value
        if chars.next().is_some() {
            return None;
        }

        Some(value)
    }

    /// Parse escape sequences in a string.
    pub(super) fn parse_escape_sequences(&self, text: &str) -> Option<String> {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(character) = chars.next() {
            if character == '\\' {
                let escaped = chars.next()?;
                let replacement = match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    '\'' => '\'',
                    '0' => '\0',
                    'x' => {
                        // hex escape
                        let first_digit = chars.next()?.to_digit(16)?;
                        let second_digit = chars.next()?.to_digit(16)?;
                        char::from_u32(first_digit * 16 + second_digit)?
                    }
                    'u' => {
                        // unicode escape
                        if chars.next()? != '{' {
                            return None;
                        }
                        let mut value = 0u32;
                        loop {
                            match chars.next()? {
                                '}' => break,
                                character => {
                                    let digit = character.to_digit(16)?;
                                    value = value * 16 + digit;
                                }
                            }
                        }
                        char::from_u32(value)?
                    }
                    _ => return None,
                };
                result.push(replacement);
            } else {
                result.push(character);
            }
        }

        Some(result)
    }

    /// Parse an intrinsic name from an opcode like `intrinsic.math.float.sqrt`.
    pub(super) fn parse_intrinsic_name(
        &self,
        opcode_text: &str,
        position: usize,
    ) -> ParseResult<Intrinsic> {
        let name = opcode_text
            .strip_prefix("intrinsic.")
            .ok_or_else(|| ParseError::invalid("intrinsic opcode", position))?;

        name.parse::<Intrinsic>()
            .map_err(|_| ParseError::invalid(&format!("intrinsic '{name}'"), position))
    }
}
