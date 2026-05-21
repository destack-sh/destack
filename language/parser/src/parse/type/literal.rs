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

    /// Return whether the next token is the given identifier text.
    #[inline]
    fn next_identifier_str_is(&mut self, expected: &str) -> bool {
        let token = self.next_token();

        token.token.ty == TokenType::Identifier && self.get_span_str(token.span) == expected
    }

    /// Peek one non composite type literal.
    pub fn peek_type_literal(&mut self) -> ParseResult<TypeLiteral> {
        // require identifier text
        let next = *self.peek()?;
        if next.token.ty != TokenType::Identifier {
            return Err(ParseError::unexpected(next.span));
        }

        // resolve literals available in all grammar spaces
        let identifier = self.get_span_str(next.span);
        if let Some(literal) = self.type_literal_always_available_str(identifier) {
            return Ok(literal);
        }

        // require type space for contextual literals
        if !self.flags.is_in_type() && !self.flags.is_in_static() {
            return Err(ParseError::unexpected(next.span));
        }

        // resolve one token contextual literals
        if let Some(literal) = self.type_literal_type_context_str(identifier, None) {
            return Ok(literal);
        }

        // resolve the only composite literal
        if identifier == "unique" {
            if self.next_identifier_str_is("symbol") {
                return Ok(TypeLiteral::UniqueSymbol);
            }

            return Err(ParseError::unexpected(next.span));
        }

        // resolve numeric literals with width suffixes
        match identifier {
            int if let Some(width) = self.type_width_if_present("int", int) => {
                Ok(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: true,
                }))
            }
            uint if let Some(width) = self.type_width_if_present("uint", uint) => {
                Ok(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: false,
                }))
            }
            uint if let Some(width) = self.type_width_if_present("u", uint) => {
                Ok(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: false,
                }))
            }
            "float32" => Ok(TypeLiteral::Float(FloatType::Float32)),
            "float64" => Ok(TypeLiteral::Float(FloatType::Float64)),
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
