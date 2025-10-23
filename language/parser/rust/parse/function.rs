//! Parse functions and closures.

use dyst_ast::{FunctionAccessor, FunctionCardinality, NodeType, Parameter};

use crate::parse::prelude::*;
use crate::{ScopedMutability, TokenType};

use crate::{
    Definition, ExportMode, FunctionStyle, Keyword, Mutability, NodeId, Parser, ParserResult,
    Runtime, SelfParameter, Visibility,
};

impl<'a> Parser<'a> {
    /// Peek a self keyword (also accepts `this`).
    fn peek_self_keyword(&mut self) -> ParserResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Self_ || keyword == Keyword::This {
            Ok(keyword)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a self keyword (also accepts `this`).
    fn eat_self_keyword(&mut self) -> ParserResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Self_ || keyword == Keyword::This {
            self.bump(); // eat self
            Ok(keyword)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat the self parameter maybe.
    fn eat_self_parameter_maybe(&mut self) -> ParserResult<Option<SelfParameter>> {
        // var_ self
        if self.peek_keyword(Keyword::Var).is_ok() || self.peek_keyword(Keyword::Mut).is_ok() {
            let mutability = self.eat_scoped_mutability()?;
            self.eat_self_keyword()?; // eat self
            Ok(Some(SelfParameter {
                mutability,
                is_reference: false,
            }))
        }
        // self
        else if self.peek_self_keyword().is_ok() {
            self.bump(); // eat self
            Ok(Some(SelfParameter {
                mutability: ScopedMutability::Unscoped {
                    mutability: Mutability::Immutable,
                },
                is_reference: false,
            }))
        }
        // &var_ self
        else if (self.peek_token(TokenType::Multiply).is_ok()
            || self.peek_token(TokenType::ElementwiseAnd).is_ok())
            && (self.peek_next_keyword(Keyword::Var).is_ok()
                || self.peek_next_keyword(Keyword::Mut).is_ok())
        {
            self.bump(); // eat &
            let mutability = self.eat_scoped_mutability()?;
            self.eat_self_keyword()?; // eat self
            Ok(Some(SelfParameter {
                mutability,
                is_reference: true,
            }))
        }
        // &self
        else if self.peek_token(TokenType::Multiply).is_ok()
            || self.peek_token(TokenType::ElementwiseAnd).is_ok()
        {
            self.bump(); // eat &
            self.eat_self_keyword()?; // eat self
            Ok(Some(SelfParameter {
                mutability: ScopedMutability::Unscoped {
                    mutability: Mutability::Immutable,
                },
                is_reference: true,
            }))
        }
        // other
        else {
            Ok(None)
        }
    }

    /// Eat a function or "lambda" definition or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    /// If no function keyword is provided, it is a shorthand regular function or a lambda.
    /// (Lambda functions cannot have a name, runtime, or static parameters.)
    ///
    /// Examples:
    /// ```
    /// // lambda style (type context)
    /// (a: int32) => int32
    /// (int32) => (boolean, int32)
    /// (x): int32 => x
    ///
    /// // lambda style (value context)
    /// (a) => a > 2
    /// (a): int32 => a > 2
    /// (a: int32) => {
    ///    print("Hello, world!")
    /// }
    ///
    /// // function style
    /// function () // anonymous function with empty signature
    ///
    /// function foo() // just declaration, no body, no opening `{`
    /// foo()
    ///
    /// function foo<T, U>(x: T) => (int32, boolean) where (
    ///    T: Copy
    ///    U: Numeric
    /// ) {
    ///    print("Hello, world!")
    /// }
    /// foo<T, U>(x: T) => (int32, boolean) ... // shorthand
    ///
    /// function baz(a: int32, b: boolean) => (
    ///    MyStruct,
    ///    boolean
    /// ) with Disk, Time { // with can be on next line
    ///    ...
    /// }
    ///
    /// // getter/setter style
    /// get foo() => int32
    /// set foo(value: int32)
    ///
    /// function @comptime() {
    ///    ...
    /// }
    ///
    /// // optional , if newline-delimited
    /// function longBar<Validate: boolean>(
    ///   /// doc comment for `a`
    ///   a: int32
    ///   /// doc comment for `b`
    ///   b: boolean
    ///   // regular comment
    ///   c: Vector2
    /// ) => (
    ///    int32,
    ///    isGood: boolean
    /// ) with (
    ///   Time
    /// ) {
    ///    ...
    /// }
    /// ```
    pub fn eat_function(
        &mut self,
        visibility: Option<Visibility>,
        export: Option<ExportMode>,
        expect_maybe: bool,
        expect_body: bool,
    ) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // async
        let is_async = if self.peek_keyword(Keyword::Async).is_ok() {
            self.bump(); // eat async keyword
            true
        } else {
            false
        };

        // accessor
        let accessor = if self.peek_keyword(Keyword::Get).is_ok() {
            self.bump(); // eat get keyword
            Some(FunctionAccessor::Getter)
        } else if self.peek_keyword(Keyword::Set).is_ok() {
            self.bump(); // eat set keyword
            Some(FunctionAccessor::Setter)
        } else {
            None
        };

        // function style
        // regular `function` style
        let style = if self.peek_keyword(Keyword::Function).is_ok() {
            self.bump(); // eat function keyword
            FunctionStyle::Function
        }
        // shorthand `name()` style
        else if self.peek_token(TokenType::Identifier).is_ok()
            && (!expect_maybe
                && self
                    .peek_next_token_in(&[TokenType::LessThan, TokenType::OpenParenthesis])
                    .is_ok()
                || expect_maybe
                    && self
                        .peek_next_next_token_in(&[TokenType::LessThan, TokenType::OpenParenthesis])
                        .is_ok())
        {
            FunctionStyle::Function
        }
        // lambda style
        else {
            FunctionStyle::Lambda
        };

        // cardinality
        let is_generator = if self.peek_token(TokenType::Multiply).is_ok() {
            self.bump(); // eat *
            true
        } else {
            false
        };

        // function style: runtime, name, static parameters
        let (runtime, name, static_parameters) = if style == FunctionStyle::Function {
            // runtime
            let runtime = if self.peek_token(TokenType::At).is_ok() {
                self.eat_token(TokenType::At)?;
                Runtime::Static
            } else {
                Runtime::Dynamic
            };

            // name
            let name = if self.peek_identifier().is_ok() {
                Some(self.eat_identifier()?)
            } else {
                None
            };

            // maybe keyword after name (maybe)
            if expect_maybe {
                self.eat_token(TokenType::Maybe)?;
            }

            // static parameters
            let static_parameters = self
                .eat_static_parameters_maybe()
                .for_node_type(NodeType::Definition)?;

            (runtime, name, static_parameters)
        } else {
            // static parameters
            let static_parameters = self
                .eat_static_parameters_maybe()
                .for_node_type(NodeType::Definition)?;

            (Runtime::Dynamic, None, static_parameters)
        };

        // dynamic parameters
        let (self_parameter, dynamic_parameters) = {
            // regular `(...) => ...` function/lambda
            if style == FunctionStyle::Function
                || self.options.in_type
                || self.peek_token(TokenType::OpenParenthesis).is_ok()
            {
                self.eat_token(TokenType::OpenParenthesis)?;
                self.eat_newlines_maybe()?;

                // self parameter maybe
                let self_parameter = self.eat_self_parameter_maybe()?;
                // optional separator after self (comma or newline) before other parameters
                if self_parameter.is_some() && self.peek_item_stop().is_ok() {
                    self.eat_item_stop_with_newlines()?;
                }

                // other dynamic parameters
                let dynamic_parameters = if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                    vec![]
                } else {
                    self.eat_parameters_body()?
                };
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseParenthesis)?;

                (self_parameter, dynamic_parameters)
            }
            // simple no-parentheses `x => y` lambda value
            else {
                let parameter_name = self.eat_identifier()?;
                let parameter_id = self.tree.insert(
                    Parameter::Named {
                        name: parameter_name,
                        ty: None,
                        default: None,
                    },
                    self.get_span_from(start),
                );

                (None, vec![parameter_id])
            }
        };

        // return type info (including with/where)
        // only for functions or lambda types
        let (return_type, with_clauses, where_clauses) = {
            // lambda with explicit return type
            if style == FunctionStyle::Lambda && self.peek_colon().is_ok() {
                self.bump(); // eat colon
                self.eat_newlines_maybe()?;
                // return type
                let return_type = self
                    .with_options(self.options.nested_type_in_before_block(), |parser| {
                        parser.eat_expression()
                    })?;
                (Some(return_type), None, None)
            }
            // regular function with return type or lambda type
            else if style == FunctionStyle::Function || self.options.in_type {
                // return type
                let return_type = if self.peek_arrow().is_ok() || self.peek_colon().is_ok() {
                    self.bump(); // eat arrow or colon
                    self.eat_newlines_maybe()?;
                    let return_type = self
                        .with_options(self.options.nested_type_in_before_block(), |parser| {
                            parser.eat_expression()
                        })?;
                    self.eat_newlines_maybe()?;
                    Some(return_type)
                } else {
                    None
                };

                // with
                let with_clauses = self.eat_with_header_maybe()?;

                // where
                let where_clauses = self.eat_where_maybe()?;

                (return_type, with_clauses, where_clauses)
            }
            // nothing
            else {
                (None, None, None)
            }
        };

        // body
        // only for functions or lambda values
        let body = {
            // expect body but no opening brace
            if expect_body && self.peek_token(TokenType::OpenBrace).is_err() {
                return Err(ParserError::expected(
                    self.peek()?.span,
                    TokenType::OpenBrace,
                ));
            }
            // function with body
            if style == FunctionStyle::Function && self.peek_token(TokenType::OpenBrace).is_ok() {
                Some(self.eat_expression()?)
            }
            // lambda with body
            else if style == FunctionStyle::Lambda && !self.options.in_type {
                self.eat_arrow()?;
                self.eat_newlines_maybe()?;
                Some(self.eat_expression()?)
            }
            // no body
            else {
                None
            }
        };

        // cardinality
        let cardinality = if is_async {
            // asynchronous
            if is_generator {
                FunctionCardinality::AsyncGenerator
            } else {
                FunctionCardinality::AsyncScalar
            }
        } else {
            // synchronous
            if is_generator {
                FunctionCardinality::Generator
            } else {
                FunctionCardinality::Scalar
            }
        };

        // function
        let function_id = self.tree.insert(
            Definition::Function {
                name,
                visibility,
                export,
                runtime,
                cardinality,
                accessor,
                style,
                static_parameters,
                self_parameter,
                dynamic_parameters,
                return_type,
                with_clauses,
                where_clauses,
                body,
            },
            self.get_span_from(start),
        );
        Ok(function_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{FunctionCardinality, FunctionStyle, Parameter};

    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Definition, Expression, Mutability, ScopedMutability, TypeLiteral,
        WhereClause, WithClause, assert_expr_path, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_function_lambda_with_newlines() {
        let mut test = TestParser::new("(x: number):\n\tnumber =>\n\tx");
        let mut parser = test.prepare();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        // (x: number): number => x
        assert_node!(parser.tree, function_id, Definition::Function { name: None, style, dynamic_parameters, return_type, body: Some(body), .. } => {
            assert_eq!(*style, FunctionStyle::Lambda);
            // x: number
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            });
            // number
            assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            // x
            assert_node!(parser.tree, *body, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    }

    #[test]
    fn test_parse_function_lambda_with_explicit_return_type() {
        let mut test = TestParser::new("(x): int32 => x");
        let mut parser = test.prepare();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        // (x): int32 => x
        assert_node!(parser.tree, function_id, Definition::Function { name: None, style, dynamic_parameters, return_type, body: Some(body), .. } => {
            assert_eq!(*style, FunctionStyle::Lambda);
            // x
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                assert_string!(parser, *name, "x");
            });
            // int32
            assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
            // x
            assert_node!(parser.tree, *body, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    }

    #[test]
    fn test_parse_function_shorthand_style() {
        let mut test = TestParser::new("foo() => int32 { body }");
        let mut parser = test.prepare();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, style, dynamic_parameters, return_type,  .. } => {
            assert_string!(parser, name.unwrap(), "foo");
            assert_eq!(*style, FunctionStyle::Function);
            // ()
            assert_eq!(dynamic_parameters.len(), 0);
            // int32
            assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
        });
    }

    #[test]
    fn test_parse_function_with_clause() {
        let mut test = TestParser::new(
            r###"
function foo() => int32 with (
  Time,
  F: Numeric,
) where Guard > Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, with_clauses, where_clauses, return_type, .. } => {
            // function name
            assert_string!(parser, name.unwrap(), "foo");

            let with_clauses = with_clauses.as_ref().unwrap();
            assert_eq!(with_clauses.len(), 2);

            // Time
            assert_node!(parser.tree, with_clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Time");
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, with_clauses[1], WithClause { alias, right } => {
                assert_string!(parser, alias.unwrap(), "F");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Numeric");
                });
            });

            // where Guard > Limit
            let where_clauses = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expr_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expr_path!(parser, parser.tree.get(*right), "Limit");
                });
            });

            // return type
            let ret = return_type.expect("expected return type");
            assert_node!(parser.tree, ret, Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
        });
    }

    #[test]
    fn test_parse_function_self_parameter_simple() {
        let mut test = TestParser::new("function a(self) {}");
        let mut parser = test.prepare();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, self_parameter, dynamic_parameters, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "a");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(!self_param.is_reference);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            assert!(dynamic_parameters.is_empty());
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_function_self_parameter_pointer() {
        let mut test = TestParser::new(
            r###"
function b(
  &self
  x: int32
) {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, self_parameter, dynamic_parameters, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "b");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_reference);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                let param_type = ty.expect("expected type for parameter x");
                assert_node!(parser.tree, param_type, Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                    assert_eq!(int_ty.width, Some(32));
                    assert!(int_ty.is_signed);
                });
            });
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_function_self_parameter_mutable_pointer() {
        let mut test = TestParser::new("function c(&var self) {}");
        let mut parser = test.prepare();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, self_parameter, dynamic_parameters, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "c");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_reference);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });

            assert!(dynamic_parameters.is_empty());
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_function_with_static_and_dynamic_parameters() {
        let mut test = TestParser::new(
            r"
function compute<Validate: bool, Precision: uint8>(data: uint8[]) {
    body
}
        ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, static_parameters, dynamic_parameters, .. } => {
            // compute
            assert_string!(parser, name.unwrap(), "compute");

            // <Validate: bool, Precision: uint8>
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 2);

            // Validate: bool
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "Validate");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
            });

            // Precision: uint8
            assert_node!(parser.tree, static_parameters[1], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "Precision");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                    assert_eq!(int_ty.width, Some(8));
                    assert!(!int_ty.is_signed);
                });
            });

            // data: uint8[]
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "data");
            });
        });
    }

    #[test]
    fn test_parse_function_with_function_return_type() {
        let mut test = TestParser::new("function foo() => (str: string) => boolean {}");
        let mut parser = test.prepare();

        // function foo() => (str: string) => boolean
        let function_id = parser.eat_function(None, None, false, false).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, return_type, .. } => {
            // foo
            assert_string!(parser, name.unwrap(), "foo");

            // (str: string) => boolean
            assert_node!(parser.tree, return_type.unwrap(), Expression::Definition(definition_id) => {
                assert_node!(parser.tree, *definition_id, Definition::Function { dynamic_parameters, return_type, .. } => {
                    assert_eq!(dynamic_parameters.len(), 1);
                    // str: string
                    assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                        assert_string!(parser, *name, "str");
                        assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                    });

                    // boolean
                    assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
                });
            });
        });
    }

    #[test]
    fn test_parse_function_with_async_generator() {
        let mut test = TestParser::new(
            r#"
async function* foo() => int32 { 
    yield 1
    yield 2
    yield 3
}"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function(None, None, false, false).unwrap();
        // async function* foo() => int32 { body }
        assert_node!(parser.tree, function_id, Definition::Function { name, cardinality, .. } => {
            // foo
            assert_string!(parser, name.unwrap(), "foo");
            assert_eq!(*cardinality, FunctionCardinality::AsyncGenerator);
        });
    }
}
