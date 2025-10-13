//! Parse functions and closures.

use crate::parse::prelude::*;
use crate::{ScopedMutability, TokenType};

use crate::{
    Definition, FunctionStyle, Keyword, Mutability, NodeId, Parser, ParserResult, Runtime,
    SelfParameter, Visibility,
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
        if self.peek_keyword(Keyword::Var).is_ok() {
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
            && self.peek_next_keyword(Keyword::Var).is_ok()
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
    /// If no function keyword is provided, it is a lambda function.
    /// (Lambda functions cannot have a name, runtime, or static parameters.)
    ///
    /// Examples:
    /// ```
    /// // lambda style (type context)
    /// (a: int32) => int32
    /// (int32) => (boolean, int32)
    ///
    /// // lambda style (value context)
    /// (a) => a > 2
    /// (a: int32) => {
    ///    print("Hello, world!")
    /// }
    ///
    /// // function style
    /// function () // anonymous function with empty signature
    ///
    /// function foo() // just declaration, no body, no opening `{`
    ///
    /// function foo<T, U>(x: T) => (int32, boolean) where (
    ///    T: Copy
    ///    U: Numeric
    /// ) {
    ///    print("Hello, world!")
    /// }
    ///
    /// function baz(a: int32, b: boolean) => (
    ///    MyStruct,
    ///    boolean
    /// ) with Disk, Time { // with can be on next line
    ///    ...
    /// }
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
    ) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // function / style
        let style = if self.peek_keyword(Keyword::Function).is_ok() {
            self.bump(); // eat function
            FunctionStyle::Function
        } else {
            FunctionStyle::Lambda
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

            // static parameters
            let static_parameters = if self.peek_token(TokenType::LessThan).is_ok() {
                self.bump(); // eat less than
                let static_parameters = self.eat_parameters_body()?;
                self.eat_token(TokenType::GreaterThan)?;
                Some(static_parameters)
            } else {
                None
            };

            (runtime, name, static_parameters)
        } else {
            (Runtime::Dynamic, None, None)
        };

        // dynamic parameters
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

        // return type info (including with/where)
        // only for functions or lambda types
        let (return_type, with_clauses, where_clauses) = {
            if style == FunctionStyle::Function || self.options.in_type {
                // return type
                let return_type = if self.peek_arrow().is_ok() {
                    self.bump(); // eat arrow
                    let return_type = self
                        .with_options(self.options.nested_type_in_before_block(), |parser| {
                            parser.eat_expression()
                        })?;
                    Some(return_type)
                } else {
                    None
                };

                // with
                let with_clauses = self.eat_with_header_maybe()?;

                // where
                let where_clauses = self.eat_where_maybe()?;

                (return_type, with_clauses, where_clauses)
            } else {
                (None, None, None)
            }
        };

        // body
        // only for functions or lambda values
        let body = {
            // function with block body
            if style == FunctionStyle::Function && self.peek_token(TokenType::OpenBrace).is_ok() {
                Some(self.eat_expression()?)
            }
            // lambda with expression body
            else if style == FunctionStyle::Lambda && !self.options.in_type {
                self.eat_arrow()?;
                Some(self.eat_expression()?)
            }
            // no body
            else {
                None
            }
        };

        let function_id = self.tree.insert(
            Definition::Function {
                name,
                visibility,
                runtime,
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
    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Definition, Expression, Mutability, ScopedMutability, TypeLiteral,
        WhereClause, WithClause, assert_expr_path, assert_node, assert_path, assert_string,
    };

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

        let function_id = parser.eat_function(None).unwrap();
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

        let function_id = parser.eat_function(None).unwrap();
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

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, self_parameter, dynamic_parameters, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "b");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_reference);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            assert_eq!(dynamic_parameters.len(), 1);
            let param = parser.tree.get(dynamic_parameters[0]);
            assert_string!(parser, param.name, "x");

            let param_type = param.ty.expect("expected type for parameter x");
            assert_node!(parser.tree, param_type, Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_function_self_parameter_mutable_pointer() {
        let mut test = TestParser::new("function c(&var self) {}");
        let mut parser = test.prepare();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, self_parameter, dynamic_parameters, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "c");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_reference);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });

            assert!(dynamic_parameters.is_empty());
            assert!(where_clauses.is_none());
        });
    }
}
