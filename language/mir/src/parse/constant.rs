use crate::{Constant, Intrinsic, LocalNodeId, MemoryLocationSet, Type};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl<'a> Parser<'a> {
    /// Parse a constant.
    pub(super) fn parse_constant(&mut self) -> ParseResult<Constant> {
        // read the next token
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("constant", self.pos()))?;
        let token_ty = token.ty;
        let token_text = token.text.to_string();
        let token_start = token.start;

        // parse the literal
        match token_ty {
            TokenType::BoolLiteral => {
                let value = token_text == "true";
                self.bump();
                Ok(Constant::Boolean { value })
            }
            TokenType::IntLiteral => {
                self.bump();
                parse_int_constant(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("integer constant '{token_text}'"), token_start)
                })
            }
            TokenType::FloatLiteral => {
                self.bump();
                parse_float_constant(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("float constant '{token_text}'"), token_start)
                })
            }
            TokenType::CharLiteral => {
                self.bump();
                let value = parse_char_literal(&token_text).ok_or_else(|| {
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
        let token_text = token.text.to_string();
        let token_start = token.start;
        let expected = self.tree.get(expected_type).clone();

        // validate the literal against the expected type
        match token_ty {
            TokenType::BoolLiteral => {
                if !matches!(expected, Type::Boolean) {
                    return Err(ParseError::invalid("bool constant type", token_start));
                }
                let value = token_text == "true";
                self.bump();
                Ok(Constant::Boolean { value })
            }
            TokenType::CharLiteral => {
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
                let value = parse_char_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("char literal '{token_text}'"), token_start)
                })?;
                Ok(Constant::Char { value })
            }
            TokenType::IntLiteral => {
                self.bump();
                let has_suffix = token_text.chars().any(|c| c.is_ascii_alphabetic());
                let (width, is_signed) =
                    self.int_type_info_for_literal(expected_type, token_start)?;
                let width = u8::try_from(width)
                    .map_err(|_| ParseError::invalid("integer width", token_start))?;

                if has_suffix {
                    let constant = parse_int_constant(&token_text).ok_or_else(|| {
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
                } else if is_signed {
                    let value: i64 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("integer constant", token_start))?;
                    Ok(Constant::Int {
                        value,
                        width,
                        is_signed: true,
                    })
                } else {
                    let value: u64 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("integer constant", token_start))?;
                    Ok(Constant::UInt { value, width })
                }
            }
            TokenType::FloatLiteral => {
                self.bump();
                let has_suffix = token_text.chars().any(|c| c.is_ascii_alphabetic());
                let width = match expected {
                    Type::Float { width } => width,
                    _ => {
                        return Err(ParseError::invalid("float constant type", token_start));
                    }
                };
                let width = u8::try_from(width)
                    .map_err(|_| ParseError::invalid("float width", token_start))?;

                if has_suffix {
                    let constant = parse_float_constant(&token_text).ok_or_else(|| {
                        ParseError::invalid(&format!("float constant '{token_text}'"), token_start)
                    })?;
                    match constant {
                        Constant::Float {
                            width: const_width, ..
                        } if const_width == width => Ok(constant),
                        _ => Err(ParseError::invalid("float constant type", token_start)),
                    }
                } else if width == 32 {
                    let value: f32 = token_text
                        .parse()
                        .map_err(|_| ParseError::invalid("float constant", token_start))?;
                    Ok(Constant::Float {
                        bits: value.to_bits() as u64,
                        width,
                    })
                } else {
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

    /// Resolve integer width and signedness for a literal type.
    pub(super) fn int_type_info_for_literal(
        &self,
        expected_type: LocalNodeId<Type>,
        start: usize,
    ) -> ParseResult<(u16, bool)> {
        let ty = self.tree.get(expected_type);
        match ty {
            Type::Int { width, is_signed } => Ok((*width, *is_signed)),
            Type::Isize => Ok((u16::from(self.options.pointer_bytes) * 8, true)),
            Type::Usize => Ok((u16::from(self.options.pointer_bytes) * 8, false)),
            _ => Err(ParseError::invalid("integer constant type", start)),
        }
    }

    /// Parse an integer literal (just the number, no type suffix).
    pub(super) fn parse_int_literal(&mut self) -> ParseResult<i64> {
        let token = self.eat_token(TokenType::IntLiteral)?;
        // strip type suffix and parse
        let text = token.text;
        let digits: String = text
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        digits
            .parse()
            .map_err(|_| ParseError::invalid("integer", token.start))
    }
}

/// Parse a primitive type from string.
pub(super) fn parse_primitive_type(s: &str) -> Option<Type> {
    Some(match s {
        "i8" => Type::Int {
            width: 8,
            is_signed: true,
        },
        "i16" => Type::Int {
            width: 16,
            is_signed: true,
        },
        "i32" => Type::Int {
            width: 32,
            is_signed: true,
        },
        "i64" => Type::Int {
            width: 64,
            is_signed: true,
        },
        "i128" => Type::Int {
            width: 128,
            is_signed: true,
        },
        "i256" => Type::Int {
            width: 256,
            is_signed: true,
        },
        "u8" => Type::Int {
            width: 8,
            is_signed: false,
        },
        "u16" => Type::Int {
            width: 16,
            is_signed: false,
        },
        "u32" => Type::Int {
            width: 32,
            is_signed: false,
        },
        "u64" => Type::Int {
            width: 64,
            is_signed: false,
        },
        "u128" => Type::Int {
            width: 128,
            is_signed: false,
        },
        "u256" => Type::Int {
            width: 256,
            is_signed: false,
        },
        "isize" => Type::Isize,
        "usize" => Type::Usize,
        "f32" => Type::Float { width: 32 },
        "f64" => Type::Float { width: 64 },
        "type" => Type::Type,
        _ => return None,
    })
}

/// Parse a memory location keyword into a location set.
pub(super) fn parse_memory_location(text: &str, start: usize) -> ParseResult<MemoryLocationSet> {
    let location = match text {
        "none" => MemoryLocationSet::NONE,
        "any" => MemoryLocationSet::ANY,
        "arguments" => MemoryLocationSet::ARGUMENTS,
        "heap" => MemoryLocationSet::HEAP,
        "stack" => MemoryLocationSet::STACK,
        "global" => MemoryLocationSet::GLOBAL,
        "shared" => MemoryLocationSet::SHARED,
        "local" => MemoryLocationSet::LOCAL,
        "constant" => MemoryLocationSet::CONSTANT,
        "inaccessible" => MemoryLocationSet::INACCESSIBLE,
        "io" => MemoryLocationSet::IO,
        _ => {
            return Err(ParseError::invalid(
                &format!("memory location '{text}'"),
                start,
            ));
        }
    };
    Ok(location)
}

/// Parse an integer constant with type suffix.
pub(super) fn parse_int_constant(s: &str) -> Option<Constant> {
    // find where the type suffix starts
    let suffix_start = s.find(|c: char| c.is_ascii_alphabetic())?;
    let (digits, suffix) = s.split_at(suffix_start);

    let is_signed = suffix.starts_with('i');
    let width: u8 = suffix[1..].parse().ok()?;

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
pub(super) fn parse_float_constant(s: &str) -> Option<Constant> {
    let suffix_start = s.rfind('f')?;
    let (digits, suffix) = s.split_at(suffix_start);
    let width: u8 = suffix[1..].parse().ok()?;

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
pub(super) fn parse_string_literal(s: &str) -> Option<String> {
    let s = s.strip_prefix('"')?.strip_suffix('"')?;
    parse_escape_sequences(s)
}

/// Parse a char literal, handling escape sequences.
pub(super) fn parse_char_literal(s: &str) -> Option<char> {
    let s = s.strip_prefix('\'')?.strip_suffix('\'')?;
    let unescaped = parse_escape_sequences(s)?;
    let mut chars = unescaped.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some(c)
}

/// Parse escape sequences in a string.
pub(super) fn parse_escape_sequences(s: &str) -> Option<String> {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
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
                    // \xHH: two hex digits
                    let h1 = chars.next()?.to_digit(16)?;
                    let h2 = chars.next()?.to_digit(16)?;
                    char::from_u32(h1 * 16 + h2)?
                }
                'u' => {
                    // \u{HHHH}: Unicode escape
                    if chars.next()? != '{' {
                        return None;
                    }
                    let mut value = 0u32;
                    loop {
                        match chars.next()? {
                            '}' => break,
                            c => {
                                let digit = c.to_digit(16)?;
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
            result.push(c);
        }
    }

    Some(result)
}

/// Parse an intrinsic name from an opcode like `intrinsic.sqrt`.
pub(super) fn parse_intrinsic_name(opcode_text: &str, pos: usize) -> ParseResult<Intrinsic> {
    let name = opcode_text
        .strip_prefix("intrinsic.")
        .ok_or_else(|| ParseError::invalid("intrinsic opcode", pos))?;
    name.parse::<Intrinsic>()
        .map_err(|_| ParseError::invalid(&format!("intrinsic '{name}'"), pos))
}
