//! Parse functions and closures.

use crate::ScopedMutability;
use crate::parse::prelude::*;
use dyst_token::TokenType;

use crate::{
    AstResult, Definition, FunctionStyle, Keyword, Mutability, NodeId, Parser, Runtime,
    SelfParameter, Visibility,
};

impl<'a> Parser<'a> {
    /// Peek a self keyword (also accepts `this`).
    fn peek_self_keyword(&mut self) -> AstResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Self_ || keyword == Keyword::This {
            Ok(keyword)
        } else {
            Err(AstError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a self keyword (also accepts `this`).
    fn eat_self_keyword(&mut self) -> AstResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Self_ || keyword == Keyword::This {
            self.bump(); // eat self
            Ok(keyword)
        } else {
            Err(AstError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a Function or "lambda" definition or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    ///
    /// Examples:
    /// ```
    /// // function style
    ///
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
    ) -> AstResult<NodeId<Definition>> {
        let start = self.mark();

        // function
        self.eat_keyword(Keyword::Function)?;

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

        // dynamic parameters
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;
        // self parameter as first parameter
        // self, *self, *var self parameter
        // (also accept `this` and `&`)
        let self_parameter: Option<SelfParameter> = {
            // var_ self
            if self.peek_keyword(Keyword::Var).is_ok() {
                let mutability = self.eat_scoped_mutability()?;
                self.eat_self_keyword()?; // eat self
                Some(SelfParameter {
                    mutability,
                    is_pointer: false,
                })
            }
            // self
            else if self.peek_self_keyword().is_ok() {
                self.bump(); // eat self
                Some(SelfParameter {
                    mutability: ScopedMutability::Unscoped {
                        mutability: Mutability::Immutable,
                    },
                    is_pointer: false,
                })
            }
            // &var_ self
            else if (self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::ElementwiseAnd).is_ok())
                && self.peek_next_keyword(Keyword::Var).is_ok()
            {
                self.bump(); // eat &
                let mutability = self.eat_scoped_mutability()?;
                self.eat_self_keyword()?; // eat self
                Some(SelfParameter {
                    mutability,
                    is_pointer: true,
                })
            }
            // &self
            else if self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::ElementwiseAnd).is_ok()
            {
                self.bump(); // eat &
                self.eat_self_keyword()?; // eat self
                Some(SelfParameter {
                    mutability: ScopedMutability::Unscoped {
                        mutability: Mutability::Immutable,
                    },
                    is_pointer: true,
                })
            }
            // other
            else {
                None
            }
        };

        // optional separator after self (comma or newline) before other parameters
        if self_parameter.is_some() && self.peek_item_stop().is_ok() {
            self.eat_item_stop_with_newlines()?;
        }

        // other parameters
        let dynamic_parameters = if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            vec![]
        } else {
            self.eat_parameters_body()?
        };
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        // return type
        let return_type = if self.peek_arrow().is_ok() {
            self.bump(); // eat arrow
            let return_type = self
                .with_options(self.options.nested_in_before_block(), |parser| {
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

        // body
        let body = if self.peek_token(TokenType::OpenBrace).is_ok() {
            Some(self.eat_block()?)
        } else {
            None
        };

        let function_id = self.tree.allocate(
            Definition::Function {
                name,
                visibility,
                runtime,
                // NOTE #Incomplete: support lambda function style
                //  (same postfix problem as with struct literals?)
                style: FunctionStyle::Function,
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
        Definition, Expression, Mutability, ScopedMutability, TypeLiteral, WithClause, assert_node,
        assert_path, assert_string,
    };

    #[test]
    fn test_parse_function_with_clause() {
        let mut test = TestParser::new(
            r###"
function foo() => int32 with (
  Time,
  F: Numeric, // should be a where clause, linted later
) {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { name, with_clauses, where_clauses, return_type, .. } => {
            // function name
            assert_string!(parser.session, name.unwrap(), "foo");

            let with_clauses = with_clauses.as_ref().unwrap();
            assert_eq!(with_clauses.len(), 2);
            assert!(where_clauses.is_none());

            // Time
            assert_node!(parser.tree, with_clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser.session, *path, "Time");
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, with_clauses[1], WithClause { alias, right } => {
                assert_string!(parser.session, alias.unwrap(), "F");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Numeric");
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
            assert_string!(parser.session, name.unwrap(), "a");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(!self_param.is_pointer);
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
            assert_string!(parser.session, name.unwrap(), "b");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_pointer);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            assert_eq!(dynamic_parameters.len(), 1);
            let param = parser.tree.get(dynamic_parameters[0]);
            assert_string!(parser.session, param.name, "x");

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
            assert_string!(parser.session, name.unwrap(), "c");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_pointer);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });

            assert!(dynamic_parameters.is_empty());
            assert!(where_clauses.is_none());
        });
    }
}
