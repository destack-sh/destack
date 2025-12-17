use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Declaration, DeclarationDescriptor, DeclarationType, Expression, FloatType, IntType, Keyword,
    LocalNodeId, Mutability, TokenType, TypeKind, TypeLiteral, TypeUnaryOperator, UnaryOperator,
    VarianceBound,
};

impl Parser {
    /// Eat a variance bound maybe.
    #[inline]
    pub fn eat_variance_bound_maybe(&mut self) -> ParseResult<Option<VarianceBound>> {
        // implements
        if self.peek_keyword(Keyword::Implements).is_ok() {
            self.bump(); // eat implements
            Ok(Some(VarianceBound::Implements))
        }
        // extends
        else if self.peek_keyword(Keyword::Extends).is_ok() {
            self.bump(); // eat extends
            Ok(Some(VarianceBound::Extends))
        }
        // super
        else if self.peek_keyword(Keyword::Super).is_ok() {
            self.bump(); // eat super
            Ok(Some(VarianceBound::Super))
        }
        // none
        else {
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

    /// Eat a composite / declaration type literal.
    pub fn eat_composite_type_literal(&mut self) -> ParseResult<TypeLiteral> {
        let next = self.eat_keyword_any()?;
        match next {
            Keyword::Type => Ok(TypeLiteral::Composite(DeclarationType::Type)),
            Keyword::Namespace => Ok(TypeLiteral::Composite(DeclarationType::Namespace)),
            Keyword::Struct => Ok(TypeLiteral::Composite(DeclarationType::Struct)),
            Keyword::Class => Ok(TypeLiteral::Composite(DeclarationType::Class)),
            Keyword::Enum => Ok(TypeLiteral::Composite(DeclarationType::Enum)),
            Keyword::Union => Ok(TypeLiteral::Composite(DeclarationType::Union)),
            Keyword::Interface => Ok(TypeLiteral::Composite(DeclarationType::Interface)),
            Keyword::Extension => Ok(TypeLiteral::Composite(DeclarationType::Extension)),
            Keyword::Function => Ok(TypeLiteral::Composite(DeclarationType::Function)),
            _ => Err(ParseError::unexpected(self.peek()?.span)),
        }
    }

    /// Peek a type literal (except composite types).
    /// Certain type literals are only parsed at the AST-level in static or type contexts.
    /// (This prevents shadowing in case we have a variable or parameter named `int` or `number`.)
    pub fn peek_type_literal(&self) -> ParseResult<TypeLiteral> {
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
            return Err(ParseError::unexpected(next.span));
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
            "boolean" => Ok(TypeLiteral::Boolean),
            // character
            "character" => Ok(TypeLiteral::Character),
            // string
            "string" => Ok(TypeLiteral::String),
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
            // symbol
            "symbol" => Ok(TypeLiteral::Symbol),
            // unique symbol
            "unique" if next_next_str == Some("symbol") => Ok(TypeLiteral::UniqueSymbol),
            // composite type
            _ => Err(ParseError::unexpected(next.span)),
        }
    }

    /// Eat a type literal (except composite types).
    pub fn eat_type_literal(&mut self, literal: Option<TypeLiteral>) -> ParseResult<TypeLiteral> {
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
    /// type 1 | 2 | 3
    /// readonly T
    /// newtype T = int32
    /// newtype Foo<T> = Baz<T> | null
    /// newtype T = { a: int32, b: boolean } | true
    /// ```
    pub fn eat_type(
        &mut self,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        let keyword: Keyword =
            self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;

        // kind
        let kind = match keyword {
            Keyword::Type => TypeKind::Structural,
            Keyword::Readonly => TypeKind::Structural,
            Keyword::Newtype => TypeKind::Nominal,
            _ => unreachable!(),
        };

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
            let (name, name_span) = if let Some((n, s)) = self.eat_name_maybe_with_span()? {
                (Some(n), Some(s))
            } else {
                (None, None)
            };
            descriptor.name = name;

            // static parameters
            let static_parameters = self.eat_static_parameters_maybe()?;

            // if followed by =, then it's a type alias
            if self.peek_token(TokenType::Assign).is_ok() {
                // =
                self.eat_token(TokenType::Assign)?;
                self.eat_newlines_maybe()?;
                // value
                let value_id = self
                    .with_options(self.options.not_in_position().in_type(), |parser| {
                        parser.eat_expression()
                    })?;
                // type declaration wrapped in expression
                let declaration = Declaration::Type {
                    kind,
                    mutability,
                    descriptor,
                    static_parameters,
                    value: value_id,
                };
                let declaration_id = self.tree.insert(declaration, self.get_span_from(start));

                // set main span to the name identifier
                if let Some(span) = name_span {
                    self.tree.set_main_span(declaration_id, span);
                }

                let expression = Expression::Declaration(declaration_id);
                Ok(self.tree.insert(expression, self.get_span_from(start)))
            }
            // otherwise it's a type expression with static arguments
            else {
                // re-parse from before the static parameters to get them as a arguments
                self.restore(speculative_start.0, speculative_start.1);
                let right = self
                    .with_options(self.options.not_in_position().in_type(), |parser| {
                        parser.eat_expression()
                    })?;
                let operator = if mutability == Some(Mutability::Immutable) {
                    TypeUnaryOperator::Readonly
                } else if kind == TypeKind::Nominal {
                    TypeUnaryOperator::Newtype
                } else {
                    TypeUnaryOperator::Type
                };
                let expression = Expression::TypeUnary { operator, right };
                Ok(self.tree.insert(expression, self.get_span_from(start)))
            }
        }
        // type expression
        else {
            let right = self.with_options(self.options.not_in_position().in_type(), |parser| {
                parser.eat_expression()
            })?;
            let operator = if mutability == Some(Mutability::Immutable) {
                TypeUnaryOperator::Readonly
            } else if kind == TypeKind::Nominal {
                TypeUnaryOperator::Newtype
            } else {
                TypeUnaryOperator::Type
            };
            let expression = Expression::TypeUnary { operator, right };
            Ok(self.tree.insert(expression, self.get_span_from(start)))
        }
    }

    /// Eat extends types maybe.
    pub fn eat_extends_types_maybe(&mut self) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        // check for extends keyword before calling underlying implementation
        if self.peek_keyword(Keyword::Extends).is_ok() {
            self.bump(); // eat extends
            self.eat_super_types_maybe(&[Keyword::Implements, Keyword::With, Keyword::Where])
        } else {
            Ok(None)
        }
    }

    /// Eat implements types maybe.
    #[inline]
    pub fn eat_implements_types_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        // check for implements keyword before calling underlying implementation
        if self.peek_keyword(Keyword::Implements).is_ok() {
            self.bump(); // eat implements
            self.eat_super_types_maybe(&[Keyword::With, Keyword::Where])
        } else {
            Ok(None)
        }
    }

    /// Eat a super type clause maybe.
    #[inline]
    fn eat_super_types_maybe(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        let is_parenthesized = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump();
            self.eat_newlines_maybe()?;
            true
        } else {
            false
        };
        let types = self.with_options(self.options.in_super_type(), |parser| {
            parser.eat_super_types(terminators)
        })?;
        if is_parenthesized {
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
        }
        Ok(Some(types))
    }

    /// Eat super types (without the leading keyword).
    fn eat_super_types(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        let mut types: Vec<LocalNodeId<Expression>> = Vec::new();
        while self.peek().is_ok() {
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
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
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
    use destack_ast::{
        Argument, BinaryOperator, Declaration, Expression, IntType, ScalarLiteral, TypeLiteral,
        TypeUnaryOperator,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_type_alias() {
        let mut test = TestParser::new("type T = int32");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T = int32
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "T");
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
        });
    }

    #[test]
    fn test_parse_type_alias_with_static_parameters() {
        let mut test = TestParser::new("type T<A, B> = intp");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T<A, B> = int32
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, static_parameters, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "T");
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Pointer { is_signed: true })));
                assert!(static_parameters.is_some());
                assert_eq!(static_parameters.as_ref().unwrap().len(), 2);
            });
        });
    }

    #[test]
    fn test_parse_type_expression_with_static_parameters() {
        let mut test = TestParser::new("type T<A, B>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // type T<A, B>
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            assert_eq!(*operator, TypeUnaryOperator::Type);
            assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
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
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            assert_eq!(*operator, TypeUnaryOperator::Type);
            assert_node!(parser.tree, *right, Expression::Binary { left, operator, right, .. } => {
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
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            assert_eq!(*operator, TypeUnaryOperator::Readonly);
            assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "T");
            });
        });
    }

    #[test]
    fn test_parse_newtype_type_expression() {
        let mut test = TestParser::new("newtype T = int32");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // newtype T = int32
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor, value, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "T");
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
        });
    }

    #[test]
    fn test_parse_conditional_type_with_inline_object() {
        let mut test = TestParser::new("type T = A extends B ? {} : { a: string | undefined }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = A extends B ? {} : { a: string | undefined }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeBinary { right, .. } => {
                    assert_node!(parser.tree, *right, Expression::If { then_expression, else_expression, .. } => {
                        assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
                            assert!(properties.is_empty());
                        });
                        assert_node!(parser.tree, else_expression.unwrap(), Expression::ObjectExpression { properties, .. } => {
                            assert_eq!(properties.len(), 1);
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_conditional_type_with_semicolon_terminated_properties() {
        let mut test =
            TestParser::new("type T = X extends Y ? {} : { a: string | undefined; b: number; }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = X extends Y ? {} : { a: string | undefined; b: number; }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeBinary { right, .. } => {
                    assert_node!(parser.tree, *right, Expression::If { else_expression, .. } => {
                        assert_node!(parser.tree, else_expression.unwrap(), Expression::ObjectExpression { properties, .. } => {
                            assert_eq!(properties.len(), 2, "Expected 2 properties but got {}", properties.len());
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_type_intersection_with_inline_object() {
        let mut test = TestParser::new("type T = Z & { a: string | undefined }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = Z & { a: string | undefined }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Binary { right, .. } => {
                    assert_node!(parser.tree, *right, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_array_tuple_type() {
        let mut test = TestParser::new("type T = [string, number]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [string, number]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    // first element: string (positional)
                    assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::String));
                    });
                    // second element: number (positional)
                    assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_tuple_type() {
        let mut test = TestParser::new("type T = (string, number)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = (string, number)
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TupleExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    // string
                    assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "string");
                    });
                    // number
                    assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "number");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_labeled_tuple_type() {
        let mut test = TestParser::new("type T = [start: number, end: number]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [start: number, end: number]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    // start: number
                    assert_node!(parser.tree, elements[0], Argument::Labeled { label, value } => {
                        assert_string!(parser, *label, "start");
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                    // end: number
                    assert_node!(parser.tree, elements[1], Argument::Labeled { label, value } => {
                        assert_string!(parser, *label, "end");
                        assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_labeled_tuple_type_complex() {
        let mut test =
            TestParser::new("type T = [importCode: string, nameMap: Record<string, string>]");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // type T = [importCode: string, nameMap: Record<string, string>]
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { elements } => {
                    assert_eq!(elements.len(), 2);
                    // importCode: string
                    assert_node!(parser.tree, elements[0], Argument::Labeled { label, .. } => {
                        assert_string!(parser, *label, "importCode");
                    });
                    // nameMap: Record<string, string>
                    assert_node!(parser.tree, elements[1], Argument::Labeled { label, .. } => {
                        assert_string!(parser, *label, "nameMap");
                    });
                });
            });
        });
    }
}
