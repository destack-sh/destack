use crate::{Parser, ParserError, ParserResult};

use destack_dir::{
    FloatType, IntegerType, Keyword, ScalarAlias, TokenType, TypeLiteral, VarianceBound,
};

impl Parser {
    /// Return the always-available type literal at the current token.
    #[inline]
    fn peek_universal_type_literal(&self) -> Option<TypeLiteral> {
        match self.peek_token_str() {
            "undefined" => Some(TypeLiteral::Undefined),
            "unknown" => Some(TypeLiteral::Unknown),
            "object" => Some(TypeLiteral::Object),
            "null" => Some(TypeLiteral::Null),
            "any" => Some(TypeLiteral::Any),
            "never" => Some(TypeLiteral::Never),
            _ => None,
        }
    }

    /// Return the type-only literal at the current token sequence.
    #[inline]
    fn peek_contextual_type_literal(&self) -> Option<TypeLiteral> {
        match self.peek_token_str() {
            "boolean" => Some(TypeLiteral::Boolean),
            "void" => Some(TypeLiteral::Void),
            "char" => Some(TypeLiteral::Character),
            "string" => Some(TypeLiteral::String),
            "bigint" => Some(TypeLiteral::Bigint),
            "number" => Some(TypeLiteral::Number),
            "int" => Some(TypeLiteral::Alias(ScalarAlias::Int)),
            "isize" => Some(TypeLiteral::Integer(IntegerType::Pointer {
                is_signed: true,
            })),
            "uint" => Some(TypeLiteral::Alias(ScalarAlias::Uint)),
            "usize" => Some(TypeLiteral::Integer(IntegerType::Pointer {
                is_signed: false,
            })),
            "float" => Some(TypeLiteral::Alias(ScalarAlias::Float)),
            "symbol" => Some(TypeLiteral::Symbol),
            "unique" if self.peek_next_identifier_is("symbol") => Some(TypeLiteral::UniqueSymbol),
            _ => None,
        }
    }

    /// Parse a variance bound when present.
    ///
    /// Examples:
    /// ```ds
    /// extends T
    /// implements Shape
    /// super Base
    /// ```
    #[inline]
    pub fn parse_variance_bound_if_present(&mut self) -> Option<VarianceBound> {
        let bound = if self.peek_is_keyword(Keyword::Implements) {
            Some(VarianceBound::Implements)
        } else if self.peek_is_keyword(Keyword::Extends) {
            Some(VarianceBound::Extends)
        } else if self.peek_is_keyword(Keyword::Super) {
            Some(VarianceBound::Super)
        } else {
            None
        };

        if bound.is_some() {
            self.bump();
        }

        bound
    }

    /// Return the explicit width encoded in the current type literal.
    #[inline]
    fn peek_type_width(&self, prefix: &'static str) -> Option<u16> {
        self.peek_token_str()
            .strip_prefix(prefix)
            .and_then(|width| width.parse::<u16>().ok())
    }

    /// Return whether the next token is the given identifier text.
    #[inline]
    fn peek_next_identifier_is(&self, expected: &str) -> bool {
        let token = self.peek_next_token();

        token.is(TokenType::Identifier) && self.token_str(token) == expected
    }

    /// Return the explicitly sized type literal at the current token.
    fn peek_sized_type_literal(&self) -> Option<TypeLiteral> {
        let identifier = self.peek_token_str();

        match identifier {
            _ if let Some(width) = self.peek_type_width("int") => {
                Some(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: true,
                }))
            }
            _ if let Some(width) = self.peek_type_width("uint") => {
                Some(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: false,
                }))
            }
            _ if let Some(width) = self.peek_type_width("u") => {
                Some(TypeLiteral::Integer(IntegerType::Fixed {
                    width,
                    is_signed: false,
                }))
            }
            "float16" => Some(TypeLiteral::Float(FloatType::Float16)),
            "bfloat16" => Some(TypeLiteral::Float(FloatType::Bfloat16)),
            "float32" => Some(TypeLiteral::Float(FloatType::Float32)),
            "float64" => Some(TypeLiteral::Float(FloatType::Float64)),
            _ => None,
        }
    }

    /// Return an unambiguous intrinsic type literal in value space.
    pub(crate) fn peek_intrinsic_type_literal(&self) -> Option<TypeLiteral> {
        if !self.peek_is(TokenType::Identifier) {
            return None;
        }

        self.peek_sized_type_literal()
    }

    /// Return the type literal represented by the current token sequence.
    pub fn peek_type_literal(&self) -> Option<TypeLiteral> {
        // require identifier text
        if !self.peek_is(TokenType::Identifier) {
            return None;
        }

        // resolve literals available in all grammar spaces
        if let Some(literal) = self.peek_universal_type_literal() {
            return Some(literal);
        }

        // resolve one token contextual literals
        if let Some(literal) = self.peek_contextual_type_literal() {
            return Some(literal);
        }

        // resolve numeric literals with width suffixes
        self.peek_sized_type_literal()
    }

    /// Parse one type literal token sequence.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// int32
    /// unique symbol
    /// ```
    pub fn parse_type_literal(&mut self) -> ParserResult<TypeLiteral> {
        let literal = self
            .peek_type_literal()
            .ok_or_else(|| ParserError::unexpected(self.peek_token_span()))?;

        // consume the literal tokens
        self.bump();
        if let TypeLiteral::UniqueSymbol = literal {
            self.bump();
        }

        Ok(literal)
    }
}
