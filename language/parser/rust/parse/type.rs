use dyst_ast::{FloatType, Keyword, Visibility};

use crate::{
    Expression, IntType, NodeId, Parser, ParserError, ParserResult, TokenType, TypeLiteral,
    UnaryOperator,
};

impl<'a> Parser<'a> {
    /// Whether the token type can start an expression.
    fn is_start_of_expression(&self, token_type: TokenType) -> bool {
        token_type == TokenType::OpenParenthesis
            || token_type == TokenType::Identifier
            || token_type == TokenType::Literal
            // (if we're before a block then { is a terminator, not the start of a block)
            || (token_type == TokenType::OpenBrace && !self.options.in_before_block)
            || UnaryOperator::from_token_type(token_type).is_some()
    }

    /// Whether the token string encodes a type literal with an explicit width.
    fn is_type_with_width(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Peek a type literal.
    /// Certain type literals are only parsed at the AST-level in static or type contexts.
    /// (This prevents shadowing in case we have a variable or parameter named `int` or `number`.)
    pub fn peek_type_literal(&self) -> ParserResult<TypeLiteral> {
        let next = self.peek()?;
        let next_str = self.get_span_str(next.span);

        // always available type literals
        let literal = match next_str {
            // undefined
            "undefined" => Some(TypeLiteral::Undefined),
            // void
            "void" => Some(TypeLiteral::Void),
            // null
            "null" => Some(TypeLiteral::Null),
            _ => None,
        };
        if let Some(literal) = literal {
            return Ok(literal);
        }

        // bail if not inside static or type context
        if !self.options.in_type && !self.options.in_static {
            return Err(ParserError::unexpected(next.span));
        }

        let next_type = next.token.ty;
        let next_next = self.peek_next();
        let next_next_type = next_next.as_ref().map(|next| next.token.ty).ok();

        // !, $, _
        if (next_type == TokenType::Not
            || next_type == TokenType::Virtual
            || next_type == TokenType::Wildcard)
            // if next token doesn't start a related expression
            && (next_next_type.is_none() || !self.is_start_of_expression(next_next_type.unwrap()))
        {
            return match next_type {
                TokenType::Not => Ok(TypeLiteral::Never),
                TokenType::Virtual => Ok(TypeLiteral::Any),
                TokenType::Wildcard => Ok(TypeLiteral::Infer),
                _ => unreachable!(),
            };
        }

        // regular single-token type literals (also only inside static/type context)
        match next_str {
            // boolean
            "boolean" | "bool" => Ok(TypeLiteral::Boolean),
            // character
            "character" | "char" => Ok(TypeLiteral::Character),
            // string
            "string" | "str" => Ok(TypeLiteral::String),
            // number
            "number" => Ok(TypeLiteral::Number),
            // Self
            "Self"
                // if next token doesn't start a related expression
                if next_next_type.is_none()
                    || next_next_type.unwrap() != TokenType::OpenBrace
                    || self.options.in_before_block =>
            {
                Ok(TypeLiteral::Self_)
            }
            // int (followed by number or nothing)
            "int" => Ok(TypeLiteral::Int(IntType {
                width: None,
                is_signed: true,
            })),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Ok(TypeLiteral::Int(IntType {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            int_str if let Some(width) = self.is_type_with_width("i", int_str) => {
                Ok(TypeLiteral::Int(IntType {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            // uint (followed by number or nothing)
            "uint" => Ok(TypeLiteral::Int(IntType {
                width: None,
                is_signed: false,
            })),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Ok(TypeLiteral::Int(IntType {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Ok(TypeLiteral::Int(IntType {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            // float (followed by number or nothing)
            "float" => Ok(TypeLiteral::Float(FloatType { width: None })),
            float_str if let Some(width) = self.is_type_with_width("float", float_str) => {
                Ok(TypeLiteral::Float(FloatType { width: Some(width) }))
            }
            float_str if let Some(width) = self.is_type_with_width("f", float_str) => {
                Ok(TypeLiteral::Float(FloatType { width: Some(width) }))
            }
            // composite type
            _ => Err(ParserError::unexpected(next.span)),
        }
    }

    /// Eat a type literal and return its value.
    pub fn eat_type_literal(&mut self) -> ParserResult<TypeLiteral> {
        let literal = self.peek_type_literal()?;
        self.bump();
        Ok(literal)
    }

    /// Eat a type alias or expression.
    ///
    /// Examples:
    /// ```
    /// type T = int32
    /// type T = foo()
    /// type T = { a: int32, b: boolean } | true
    /// type 1 | 2 |3
    /// ```
    pub fn eat_type_alias_or_expression(
        &mut self,
        visibility: Option<Visibility>,
    ) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Type)?;

        // alias
        if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Assign).is_ok() {
            let alias = self.eat_identifier()?;
            self.eat_token(TokenType::Assign)?;
            let value =
                self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
            let expression = Expression::Type {
                name: Some(alias),
                value,
                visibility,
            };
            Ok(self.tree.insert(expression, self.get_span_from(start)))
        }
        // expression
        else {
            let value =
                self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
            let expression = Expression::Type {
                name: None,
                value,
                visibility,
            };
            Ok(self.tree.insert(expression, self.get_span_from(start)))
        }
    }

    /// Eat super types maybe. May be parenthesized.
    ///
    /// Examples:
    /// ```
    /// : Foo
    /// : Foo, Bar
    /// : (Foo, Bar)
    /// ```
    pub fn eat_super_types_maybe(&mut self) -> ParserResult<Option<Vec<NodeId<Expression>>>> {
        if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            let is_parenthesized = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                self.eat_newlines_maybe()?;
                true
            } else {
                false
            };
            let super_types = self.eat_super_types()?;
            if is_parenthesized {
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseParenthesis)?;
            }
            return Ok(Some(super_types));
        }
        Ok(None)
    }

    /// Eat super types.
    ///
    /// Examples:
    /// ```
    /// Foo
    /// Foo, Bar<X>
    /// ```
    pub fn eat_super_types(&mut self) -> ParserResult<Vec<NodeId<Expression>>> {
        let mut super_types: Vec<NodeId<Expression>> = Vec::new();
        loop {
            // eat until open brace or close parenthesis
            if self.peek_token(TokenType::OpenBrace).is_ok()
                || self.peek_token(TokenType::CloseParenthesis).is_ok()
            {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // keep eating super types
            else {
                let super_type = self.with_options(self.options.in_before_block(), |parser| {
                    parser.eat_expression()
                })?;
                super_types.push(super_type);
            }
        }
        Ok(super_types)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Expression, IntType, ScalarLiteral, TypeLiteral, assert_node, assert_string,
    };

    #[test]
    fn test_parse_type_alias() {
        let mut test = TestParser::new("type T = int32");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T = int32
        assert_node!(parser.tree, expr_id, Expression::Type { name, value, visibility } => {
            assert_string!(parser, name.unwrap(), "T");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
            assert_eq!(*visibility, None);
        });
    }

    #[test]
    fn test_parse_type_expression() {
        let mut test = TestParser::new("type 1 | 2 | 3");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type 1 | 2 |3
        assert_node!(parser.tree, expr_id, Expression::Type { name, value, visibility: None } => {
            assert_eq!(*name, None);
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
        });
    }
}
