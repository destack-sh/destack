use crate::{ParseError, ParseResult, Parser};

use destack_dir::{FloatType, IntegerType, Keyword, TokenType, TypeLiteral, VarianceBound};

impl Parser {
    /// Map identifier text to always-available type literals.
    #[inline]
    fn type_literal_always_available_str(&self, identifier: &str) -> Option<TypeLiteral> {
        match identifier {
            "undefined" => Some(TypeLiteral::Undefined),
            "unknown" => Some(TypeLiteral::Unknown),
            "object" => Some(TypeLiteral::Object),
            "null" => Some(TypeLiteral::Null),
            "any" => Some(TypeLiteral::Any),
            "never" => Some(TypeLiteral::Never),
            _ => None,
        }
    }

    /// Map identifier text to type-only literals.
    #[inline]
    fn type_literal_type_context_str(
        &self,
        identifier: &str,
        next_identifier: Option<&str>,
    ) -> Option<TypeLiteral> {
        match identifier {
            "boolean" => Some(TypeLiteral::Boolean),
            "void" => Some(TypeLiteral::Void),
            "char" => Some(TypeLiteral::Character),
            "string" => Some(TypeLiteral::String),
            "bigint" => Some(TypeLiteral::Bigint),
            "number" => Some(TypeLiteral::Number),
            "int" => Some(TypeLiteral::Integer(IntegerType::Integer {
                is_signed: true,
            })),
            "isize" => Some(TypeLiteral::Integer(IntegerType::Pointer {
                is_signed: true,
            })),
            "uint" => Some(TypeLiteral::Integer(IntegerType::Integer {
                is_signed: false,
            })),
            "usize" => Some(TypeLiteral::Integer(IntegerType::Pointer {
                is_signed: false,
            })),
            "float" => Some(TypeLiteral::Float(FloatType::Float)),
            "symbol" => Some(TypeLiteral::Symbol),
            "unique" if next_identifier == Some("symbol") => Some(TypeLiteral::UniqueSymbol),
            _ => None,
        }
    }

    /// Eat a variance bound when present.
    ///
    /// Examples:
    /// ```ds
    /// extends T
    /// implements Shape
    /// super Base
    /// ```
    #[inline]
    pub fn eat_variance_bound_if_present(&mut self) -> ParseResult<Option<VarianceBound>> {
        let bound = if self.is_keyword(Keyword::Implements) {
            Some(VarianceBound::Implements)
        } else if self.is_keyword(Keyword::Extends) {
            Some(VarianceBound::Extends)
        } else if self.is_keyword(Keyword::Super) {
            Some(VarianceBound::Super)
        } else {
            None
        };

        if bound.is_some() {
            self.bump();
        }

        Ok(bound)
    }

    /// Return the explicit width encoded in one type literal name.
    #[inline]
    fn type_width_if_present(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Peek one non composite type literal.
    pub fn peek_type_literal(&mut self) -> ParseResult<TypeLiteral> {
        // leading token
        let next = *self.peek()?;
        let next_type = next.token.ty;
        let identifier_span = if next_type == TokenType::Identifier {
            Some(next.span)
        } else {
            None
        };

        // always available type literals
        if let Some(identifier_span) = identifier_span
            && let Some(literal) =
                self.type_literal_always_available_str(self.get_span_str(identifier_span))
        {
            return Ok(literal);
        }

        // bail if not inside static or type context
        if !self.flags.is_in_type() && !self.flags.is_in_static() {
            return Err(ParseError::unexpected(next.span));
        }

        // contextual identifiers like `number` or `unique symbol`
        let next_next = self.next_token();
        let next_next_type = next_next.token.ty;
        if let Some(identifier_span) = identifier_span {
            let next_identifier_span = if next_next_type == TokenType::Identifier {
                Some(next_next.span)
            } else {
                None
            };
            let identifier = self.get_span_str(identifier_span);
            let next_identifier = next_identifier_span.map(|span| self.get_span_str(span));
            if let Some(literal) = self.type_literal_type_context_str(identifier, next_identifier) {
                return Ok(literal);
            }
        }

        let next_str = self.get_span_str(next.span);
        let next_next_str =
            (next_next_type != TokenType::End).then(|| self.get_span_str(next_next.span));

        // regular single-token type literals like `boolean` or `uint32`
        match next_str {
            "boolean" => Ok(TypeLiteral::Boolean),
            "void" => Ok(TypeLiteral::Void),
            "char" => Ok(TypeLiteral::Character),
            "string" => Ok(TypeLiteral::String),
            "bigint" => Ok(TypeLiteral::Bigint),
            "number" => Ok(TypeLiteral::Number),
            "int" => Ok(TypeLiteral::Integer(IntegerType::Integer {
                is_signed: true,
            })),
            "isize" => Ok(TypeLiteral::Integer(IntegerType::Pointer {
                is_signed: true,
            })),
            int_str if let Some(width) = self.type_width_if_present("int", int_str) => {
                Ok(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: true,
                }))
            }
            "uint" => Ok(TypeLiteral::Integer(IntegerType::Integer {
                is_signed: false,
            })),
            "usize" => Ok(TypeLiteral::Integer(IntegerType::Pointer {
                is_signed: false,
            })),
            uint_str if let Some(width) = self.type_width_if_present("uint", uint_str) => {
                Ok(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: false,
                }))
            }
            uint_str if let Some(width) = self.type_width_if_present("u", uint_str) => {
                Ok(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: false,
                }))
            }
            "float" => Ok(TypeLiteral::Float(FloatType::Float)),
            "float32" => Ok(TypeLiteral::Float(FloatType::Float32)),
            "float64" => Ok(TypeLiteral::Float(FloatType::Float64)),
            "symbol" => Ok(TypeLiteral::Symbol),
            "unique" if next_next_str == Some("symbol") => Ok(TypeLiteral::UniqueSymbol),
            _ => Err(ParseError::unexpected(next.span)),
        }
    }

    /// Eat one non composite type literal.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// uint32
    /// unique symbol
    /// ```
    pub fn eat_type_literal(&mut self, literal: Option<TypeLiteral>) -> ParseResult<TypeLiteral> {
        // resolve the literal kind first
        let literal = match literal {
            Some(literal) => literal,
            None => self.peek_type_literal()?,
        };

        // consume the literal tokens
        self.bump();
        if let TypeLiteral::UniqueSymbol = literal {
            self.bump();
        }
        Ok(literal)
    }
}
