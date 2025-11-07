use dyst_ast::{DefinitionMeta, DefinitionType, FloatType, Keyword, Mutability, VarianceBound};

use crate::{
    Expression, IntType, NodeId, Parser, ParserError, ParserResult, TokenType, TypeLiteral,
    TypeUnaryOperator, UnaryOperator,
};

impl<'a> Parser<'a> {
    /// Eat a variance modifier maybe.
    #[inline]
    pub fn eat_variance_modifier_maybe(&mut self) -> ParserResult<Option<VarianceBound>> {
        if self.peek_keyword(Keyword::Extends).is_ok() {
            self.bump(); // eat extends
            Ok(Some(VarianceBound::Extends))
        } else if self.peek_keyword(Keyword::Super).is_ok() {
            self.bump(); // eat super
            Ok(Some(VarianceBound::Super))
        } else {
            Ok(None)
        }
    }

    /// Whether the token type can start an expression.
    #[inline]
    fn is_start_of_expression(&self, token_str: &str, token_type: TokenType) -> bool {
        token_type == TokenType::OpenParenthesis
            || token_type == TokenType::Identifier
            || token_type == TokenType::Literal
            // (if we're before a block then { is a terminator, not the start of a block)
            || (token_type == TokenType::OpenBrace && !self.options.in_before_block)
            || UnaryOperator::from_prefix_token( token_type).is_some()
            || TypeUnaryOperator::from_prefix_token(token_str, token_type).is_some()
    }

    /// Whether the token string encodes a type literal with an explicit width.
    #[inline]
    fn is_type_with_width(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Eat a composite / definition type literal.
    pub fn eat_composite_type_literal(&mut self) -> ParserResult<TypeLiteral> {
        let next = self.eat_keyword_any()?;
        match next {
            Keyword::Type => Ok(TypeLiteral::Composite(DefinitionType::Type)),
            Keyword::Namespace => Ok(TypeLiteral::Composite(DefinitionType::Namespace)),
            Keyword::Struct => Ok(TypeLiteral::Composite(DefinitionType::Struct)),
            Keyword::Class => Ok(TypeLiteral::Composite(DefinitionType::Class)),
            Keyword::Enum => Ok(TypeLiteral::Composite(DefinitionType::Enum)),
            Keyword::Union => Ok(TypeLiteral::Composite(DefinitionType::Union)),
            Keyword::Interface => Ok(TypeLiteral::Composite(DefinitionType::Interface)),
            Keyword::Extension => Ok(TypeLiteral::Composite(DefinitionType::Extension)),
            Keyword::Function => Ok(TypeLiteral::Composite(DefinitionType::Function)),
            _ => Err(ParserError::unexpected(self.peek()?.span)),
        }
    }

    /// Peek a type literal (except composite types).
    /// Certain type literals are only parsed at the AST-level in static or type contexts.
    /// (This prevents shadowing in case we have a variable or parameter named `int` or `number`.)
    pub fn peek_type_literal(&self) -> ParserResult<TypeLiteral> {
        let next = self.peek()?;
        let next_str = self.get_span_str(next.span);

        // always available type literals
        let literal = match next_str {
            // undefined
            "undefined" => Some(TypeLiteral::Undefined),
            // unknown
            "unknown" => Some(TypeLiteral::Unknown),
            // void
            "void" => Some(TypeLiteral::Void),
            // null
            "null" => Some(TypeLiteral::Null),
            // any
            "any" => Some(TypeLiteral::Any),
            // never
            "never" => Some(TypeLiteral::Never),
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
        let next_next_str = next_next
            .as_ref()
            .map(|next| self.get_span_str(next.span))
            .ok();

        // !, _
        if (next_type == TokenType::Not
            || next_type == TokenType::Wildcard)
            // if next token doesn't start a related expression
            && (next_next_type.is_none() || !self.is_start_of_expression(self.get_span_str(next_next.unwrap().span), next_next_type.unwrap()))
        {
            return match next_type {
                TokenType::Not => Ok(TypeLiteral::Never),
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
            // bigint
            "bigint" => Ok(TypeLiteral::Bigint),
            // number
            "number" => Ok(TypeLiteral::Number),            
            // int (followed by number or nothing)
            "int" => Ok(TypeLiteral::Int(IntType::Arbitrary {
                width: None,
                is_signed: true,
            })),
            "intp" => Ok(TypeLiteral::Int(IntType::Pointer { is_signed: true })),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Ok(TypeLiteral::Int(IntType::Arbitrary {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            int_str if let Some(width) = self.is_type_with_width("i", int_str) => {
                Ok(TypeLiteral::Int(IntType::Arbitrary {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            // uint (followed by number or nothing)
            "uint" => Ok(TypeLiteral::Int(IntType::Arbitrary {
                width: None,
                is_signed: false,
            })),
            "uintp" => Ok(TypeLiteral::Int(IntType::Pointer { is_signed: false })),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Ok(TypeLiteral::Int(IntType::Arbitrary {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Ok(TypeLiteral::Int(IntType::Arbitrary {
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
            // symbol
            "symbol" => Ok(TypeLiteral::Symbol),
            // unique symbol
            "unique" if next_next_str == Some("symbol") => Ok(TypeLiteral::UniqueSymbol),
            // Self
            "Self"
                // if next token doesn't start a related expression
                if next_next_type.is_none()
                    || next_next_type.unwrap() != TokenType::OpenBrace
                    || self.options.in_before_block =>
            {
                Ok(TypeLiteral::Self_)
            }
            // composite type
            _ => Err(ParserError::unexpected(next.span)),
        }
    }

    /// Eat a type literal (except composite types).
    pub fn eat_type_literal(&mut self, literal: Option<TypeLiteral>) -> ParserResult<TypeLiteral> {
        let literal = match literal {
            Some(literal) => literal,
            None => self.peek_type_literal()?,
        };
        self.bump();
        if let TypeLiteral::UniqueSymbol = literal {
            self.bump(); // eat second token
        }
        Ok(literal)
    }

    /// Eat a type alias or expression.
    ///
    /// Examples:
    /// ```
    /// type T = int32
    /// type T = foo()
    /// type T = { a: int32, b: boolean } | true
    ///
    /// type 1 | 2 | 3
    /// readonly T
    /// ```
    pub fn eat_type(&mut self, mut meta: DefinitionMeta) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();
        let keyword = self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly])?;

        // mutability
        let mutability = if keyword == Keyword::Readonly {
            Some(Mutability::Immutable)
        } else {
            None
        };

        // alias (or expression with static parameters)
        if self.peek_identifier().is_ok()
            && (self.peek_next_token(TokenType::Assign).is_ok()
                || self.peek_next_token(TokenType::LessThan).is_ok())
        {
            // identifier
            // (speculative because we don't know yet if we'll have a `=` afterwards)
            let speculative_start = (self.mark(), self.tree.next_id());
            meta.name = self.eat_name_maybe()?;

            // static parameters
            let static_parameters = self.eat_static_parameters_maybe()?;

            // if followed by =, then it's a type alias
            if self.peek_token(TokenType::Assign).is_ok() {
                // =
                self.eat_token(TokenType::Assign)?;
                self.eat_newlines_maybe()?;
                // value
                let value_id =
                    self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
                // type
                let expression = Expression::LetType {
                    mutability,
                    meta,
                    static_parameters,
                    value: value_id,
                };
                Ok(self.tree.insert(expression, self.get_span_from(start)))
            }
            // otherwise it's a type expression with static arguments
            else {
                // re-parse from before the static parameters to get them as a arguments
                self.restore(speculative_start.0, speculative_start.1);
                let right =
                    self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
                let operator = if mutability == Some(Mutability::Immutable) {
                    TypeUnaryOperator::Readonly
                } else {
                    TypeUnaryOperator::Type
                };
                let expression = Expression::TypeUnary {
                    operator,
                    expression: right,
                };
                Ok(self.tree.insert(expression, self.get_span_from(start)))
            }
        }
        // type expression
        else {
            let right =
                self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
            let operator = if mutability == Some(Mutability::Immutable) {
                TypeUnaryOperator::Readonly
            } else {
                TypeUnaryOperator::Type
            };
            let expression = Expression::TypeUnary {
                operator,
                expression: right,
            };
            Ok(self.tree.insert(expression, self.get_span_from(start)))
        }
    }

    /// Eat extends types maybe.
    pub fn eat_extends_types_maybe(&mut self) -> ParserResult<Option<Vec<NodeId<Expression>>>> {
        // check for extends keyword before calling underlying implementation
        if self.peek_keyword(Keyword::Extends).is_ok() {
            self.bump(); // eat extends
            self.eat_super_type_body_maybe(&[Keyword::Implements, Keyword::With, Keyword::Where])
        } else {
            Ok(None)
        }
    }

    /// Eat implements types maybe.
    #[inline]
    pub fn eat_implements_types_maybe(&mut self) -> ParserResult<Option<Vec<NodeId<Expression>>>> {
        // check for implements keyword before calling underlying implementation
        if self.peek_keyword(Keyword::Implements).is_ok() {
            self.bump(); // eat implements
            self.eat_super_type_body_maybe(&[Keyword::With, Keyword::Where])
        } else {
            Ok(None)
        }
    }

    /// Eat a super type clause maybe.
    #[inline]
    fn eat_super_type_body_maybe(
        &mut self,
        terminators: &[Keyword],
    ) -> ParserResult<Option<Vec<NodeId<Expression>>>> {
        let is_parenthesized = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump();
            self.eat_newlines_maybe()?;
            true
        } else {
            false
        };
        let types = self.with_options(self.options.in_super_type(), |parser| {
            parser.eat_super_type_body(terminators)
        })?;
        if is_parenthesized {
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
        }
        Ok(Some(types))
    }

    /// Eat a type clause body (without the leading keyword).
    fn eat_super_type_body(
        &mut self,
        terminators: &[Keyword],
    ) -> ParserResult<Vec<NodeId<Expression>>> {
        let mut types: Vec<NodeId<Expression>> = Vec::new();
        loop {
            // eat until open brace or close parenthesis
            if self.peek_token(TokenType::OpenBrace).is_ok()
                || self.peek_token(TokenType::CloseParenthesis).is_ok()
                || terminators
                    .iter()
                    .any(|terminator| self.peek_keyword(*terminator).is_ok())
            {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // keep eating super types
            else {
                let ty = self.with_options(self.options.in_before_block(), |parser| {
                    parser.eat_expression()
                })?;
                types.push(ty);
            }
        }
        Ok(types)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::TypeUnaryOperator;

    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Expression, IntType, ScalarLiteral, TypeLiteral, assert_node, assert_path,
        assert_string,
    };

    #[test]
    fn test_parse_type_alias() {
        let mut test = TestParser::new("type T = int32");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T = int32
        assert_node!(parser.tree, expr_id, Expression::LetType { meta, value, .. } => {
            assert_string!(parser, meta.name.unwrap().string(), "T");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
        });
    }

    #[test]
    fn test_parse_type_alias_with_static_parameters() {
        let mut test = TestParser::new("type T<A, B> = intp");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T<A, B> = int32
        assert_node!(parser.tree, expr_id, Expression::LetType { meta, value, static_parameters, .. } => {
            assert_string!(parser, meta.name.unwrap().string(), "T");
            assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Pointer { is_signed: true })));
            assert!(static_parameters.is_some());
            assert_eq!(static_parameters.as_ref().unwrap().len(), 2);
        });
    }

    #[test]
    fn test_parse_type_expression_with_static_parameters() {
        let mut test = TestParser::new("type T<A, B>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T<A, B>
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, expression } => {
            assert_eq!(*operator, TypeUnaryOperator::Type);
            assert_node!(parser.tree, *expression, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "T");
                assert_eq!(static_arguments.as_ref().unwrap().len(), 2);
            })
        });
    }

    #[test]
    fn test_parse_type_expression() {
        let mut test = TestParser::new("type 1 | 2 | 3");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type 1 | 2 |3
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, expression } => {
            assert_eq!(*operator, TypeUnaryOperator::Type);
            assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, .. } => {
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

    #[test]
    fn test_parse_readonly_type_expression() {
        let mut test = TestParser::new("readonly T");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // readonly T
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, expression } => {
            assert_eq!(*operator, TypeUnaryOperator::Readonly);
            assert_node!(parser.tree, *expression, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "T");
            });
        });
    }
}
