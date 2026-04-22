use destack_source::Span;

use crate::{Constant, Intrinsic, LocalNodeId, EffectRegionSet, Type};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl Parser {
    /// Parse a constant.
    pub(super) fn parse_constant(&mut self) -> ParseResult<Constant> {
        // read the next token
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("constant", self.pos()))?;
        let token_ty = token.ty;
        let token_text = self.tree.source_text(token.span).to_string();
        let token_start = token.start;

        // parse the literal
        match token_ty {
            TokenType::Identifier if token_text == "null" => {
                self.bump();
                Ok(Constant::Null)
            }
            TokenType::BooleanLiteral => {
                let value = token_text == "true";
                self.bump();
                Ok(Constant::Boolean { value })
            }
            TokenType::IntLiteral => {
                self.bump();
                self.parse_int_constant(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("integer constant '{token_text}'"), token_start)
                })
            }
            TokenType::FloatLiteral => {
                self.bump();
                self.parse_float_constant(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("float constant '{token_text}'"), token_start)
                })
            }
            TokenType::CharacterLiteral => {
                self.bump();
                let value = self.parse_char_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("char literal '{token_text}'"), token_start)
                })?;
                Ok(Constant::Char { value })
            }
            _ => Err(ParseError::unexpected("constant", token_ty, token_start)),
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
        let token_ty = token.ty;
        let token_text = self.tree.source_text(token.span).to_string();
        let token_start = token.start;
        let expected = self.tree.get(expected_type).clone();

        // validate the literal against the expected type
        match token_ty {
            TokenType::Identifier if token_text == "null" => {
                let Type::Reference { is_nullable, .. } = expected else {
                    return Err(ParseError::invalid("null constant type", token_start));
                };
                if !is_nullable {
                    return Err(ParseError::invalid("null constant type", token_start));
                }
                self.bump();
                Ok(Constant::Null)
            }
            TokenType::BooleanLiteral => {
                if !matches!(expected, Type::Boolean) {
                    return Err(ParseError::invalid("boolean constant type", token_start));
                }
                let value = token_text == "true";
                self.bump();
                Ok(Constant::Boolean { value })
            }
            TokenType::CharacterLiteral => {
                if !matches!(
                    expected,
                    Type::Int {
                        width: 32,
                        is_signed: false
                    }
                ) {
                    return Err(ParseError::invalid("char constant type", token_start));
                }
                self.bump();
                let value = self.parse_char_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("char literal '{token_text}'"), token_start)
                })?;
                Ok(Constant::Char { value })
            }
            TokenType::IntLiteral => {
                self.bump();

                // expected integer shape
                let has_suffix = token_text.chars().any(|c| c.is_ascii_alphabetic());
                let (width, is_signed) = match self.tree.get(expected_type) {
                    Type::Int { width, is_signed } => (*width, *is_signed),
                    Type::Isize => (self.tree.pointer_bits(), true),
                    Type::Usize => (self.tree.pointer_bits(), false),
                    _ => {
                        return Err(ParseError::invalid("integer constant type", token_start));
                    }
                };
                let width = u8::try_from(width)
                    .map_err(|_| ParseError::invalid("integer width", token_start))?;

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
                    let value: i64 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("integer constant", token_start))?;
                    Ok(Constant::Int {
                        value,
                        width,
                        is_signed: true,
                    })
                } else {
                    // unsigned payload
                    let value: u64 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("integer constant", token_start))?;
                    Ok(Constant::UInt { value, width })
                }
            }
            TokenType::FloatLiteral => {
                self.bump();

                // expected float shape
                let has_suffix = token_text.chars().any(|c| c.is_ascii_alphabetic());
                let width = match expected {
                    Type::Float { width } => width,
                    _ => {
                        return Err(ParseError::invalid("float constant type", token_start));
                    }
                };
                let width = u8::try_from(width)
                    .map_err(|_| ParseError::invalid("float width", token_start))?;

                // typed literal
                if has_suffix {
                    let constant = self.parse_float_constant(&token_text).ok_or_else(|| {
                        ParseError::invalid(&format!("float constant '{token_text}'"), token_start)
                    })?;
                    match constant {
                        Constant::Float {
                            width: const_width, ..
                        } if const_width == width => Ok(constant),
                        _ => Err(ParseError::invalid("float constant type", token_start)),
                    }
                }
                // f32 payload
                else if width == 32 {
                    let value: f32 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("float constant", token_start))?;
                    Ok(Constant::Float {
                        bits: value.to_bits() as u64,
                        width,
                    })
                } else {
                    // f64 payload
                    let value: f64 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("float constant", token_start))?;
                    Ok(Constant::Float {
                        bits: value.to_bits(),
                        width,
                    })
                }
            }
            _ => Err(ParseError::unexpected("constant", token_ty, token_start)),
        }
    }

    /// Parse an integer literal (just the number, no type suffix).
    pub(super) fn parse_int_literal(&mut self) -> ParseResult<i64> {
        let token = self.eat_token(TokenType::IntLiteral)?;
        let text = self.tree.source_text(token.span).to_string();

        // strip type suffix and parse
        let digits: String = text
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();

        digits
            .parse()
            .map_err(|_| ParseError::invalid("integer", token.start))
    }

    /// Parse an integer literal and return its span.
    pub(super) fn parse_int_literal_part(&mut self) -> ParseResult<(i64, Span)> {
        let token = self.eat_token(TokenType::IntLiteral)?;
        let token_start = token.start;
        let token_text = self.tree.source_text(token.span).to_string();
        let token_length = token_text.len();
        let span = self.span_at(token_start, token_length);

        // strip type suffix and parse
        let digits: String = token_text
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        let value = digits
            .parse()
            .map_err(|_| ParseError::invalid("integer", token_start))?;

        Ok((value, span))
    }

    /// Parse an integer literal and append its span as one source segment.
    pub(super) fn parse_int_segment(&mut self, segment_spans: &mut Vec<Span>) -> ParseResult<i64> {
        let (value, span) = self.parse_int_literal_part()?;
        segment_spans.push(span);

        Ok(value)
    }

    /// Parse a primitive type from string.
    pub(super) fn parse_primitive_type(&self, text: &str) -> Option<Type> {
        Some(match text {
            "int8" => Type::Int {
                width: 8,
                is_signed: true,
            },
            "int16" => Type::Int {
                width: 16,
                is_signed: true,
            },
            "int32" => Type::Int {
                width: 32,
                is_signed: true,
            },
            "int64" => Type::Int {
                width: 64,
                is_signed: true,
            },
            "int128" => Type::Int {
                width: 128,
                is_signed: true,
            },
            "int256" => Type::Int {
                width: 256,
                is_signed: true,
            },
            "uint8" => Type::Int {
                width: 8,
                is_signed: false,
            },
            "uint16" => Type::Int {
                width: 16,
                is_signed: false,
            },
            "uint32" => Type::Int {
                width: 32,
                is_signed: false,
            },
            "uint64" => Type::Int {
                width: 64,
                is_signed: false,
            },
            "uint128" => Type::Int {
                width: 128,
                is_signed: false,
            },
            "uint256" => Type::Int {
                width: 256,
                is_signed: false,
            },
            "isize" => Type::Isize,
            "usize" => Type::Usize,
            "float32" => Type::Float { width: 32 },
            "float64" => Type::Float { width: 64 },
            "typeDescriptor" => Type::TypeDescriptor,
            "typeId" => Type::TypeId,
            _ => return None,
        })
    }

    /// Parse a memory region keyword into a region set.
    pub(super) fn parse_memory_location(
        &self,
        text: &str,
        start: usize,
    ) -> ParseResult<EffectRegionSet> {
        let location = match text {
            "none" => EffectRegionSet::NONE,
            "any" => EffectRegionSet::ANY,
            "heap" => EffectRegionSet::HEAP,
            "rawHeap" => EffectRegionSet::RAW_HEAP,
            "stack" => EffectRegionSet::STACK,
            "global" => EffectRegionSet::GLOBAL,
            "shared" => EffectRegionSet::SHARED,
            "local" => EffectRegionSet::LOCAL,
            "constant" => EffectRegionSet::CONSTANT,
            "io" => EffectRegionSet::IO,
            _ => {
                return Err(ParseError::invalid(
                    &format!("memory region '{text}'"),
                    start,
                ));
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
        } else if let Some(width) = suffix.strip_prefix('u') {
            (false, width)
        } else {
            return None;
        };
        let width: u8 = width_text.parse().ok()?;

        if is_signed {
            let value: i64 = digits.parse().ok()?;
            Some(Constant::Int {
                value,
                width,
                is_signed: true,
            })
        } else {
            let value: u64 = digits.parse().ok()?;
            Some(Constant::UInt { value, width })
        }
    }

    /// Parse a float constant with type suffix.
    pub(super) fn parse_float_constant(&self, text: &str) -> Option<Constant> {
        let suffix_start = text.rfind("float").or_else(|| text.rfind('f'))?;
        let (digits, suffix) = text.split_at(suffix_start);
        let width_text = suffix
            .strip_prefix("float")
            .or_else(|| suffix.strip_prefix('f'))?;
        let width: u8 = width_text.parse().ok()?;

        if width == 32 {
            let value: f32 = digits.parse().ok()?;
            Some(Constant::Float {
                bits: value.to_bits() as u64,
                width,
            })
        } else {
            let value: f64 = digits.parse().ok()?;
            Some(Constant::Float {
                bits: value.to_bits(),
                width,
            })
        }
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

    /// Parse an intrinsic name from an opcode like `intrinsic.sqrt`.
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
