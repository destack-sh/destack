//! Parse functions and closures.

use dyst_language_token::TokenType;

use crate::{
    Function, FunctionStyle, Keyword, Mutability, NodeId, ParseResult, Parser, Runtime,
    SelfParameter, Visibility,
};

impl<'a> Parser<'a> {
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
    /// function foo<T, U>(x: T) => (int32, boolean) with (
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
    ///
    /// // lambda style
    ///
    /// () => 0
    /// (x) => x + 1
    /// (x: int32) => x + 1
    /// ```
    pub fn eat_function(
        &mut self,
        visibility: Option<Visibility>,
    ) -> ParseResult<NodeId<Function>> {
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
        // self, *self, *var self parameter
        // (also accept `this` and `&`)
        let self_parameter: Option<SelfParameter> = {
            // self
            if self.peek_keyword(Keyword::Self_).is_ok() || self.peek_keyword(Keyword::This).is_ok()
            {
                self.bump(); // eat self
                Some(SelfParameter {
                    mutability: Mutability::Immutable,
                    is_pointer: false,
                })
            }
            // var self
            else if self.peek_keyword(Keyword::Var).is_ok()
                && (self.peek_next_keyword(Keyword::Self_).is_ok()
                    || self.peek_next_keyword(Keyword::This).is_ok())
            {
                self.bump(); // eat var
                self.bump(); // eat self
                Some(SelfParameter {
                    mutability: Mutability::Mutable,
                    is_pointer: false,
                })
            }
            // *self
            else if (self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::BitwiseAnd).is_ok())
                && (self.peek_next_keyword(Keyword::Self_).is_ok()
                    || self.peek_next_keyword(Keyword::This).is_ok())
            {
                self.bump(); // eat *
                self.bump(); // eat self
                Some(SelfParameter {
                    mutability: Mutability::Immutable,
                    is_pointer: true,
                })
            }
            // *var self
            else if (self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::BitwiseAnd).is_ok())
                && self.peek_next_keyword(Keyword::Var).is_ok()
                && (self.peek_next_next_keyword(Keyword::Self_).is_ok()
                    || self.peek_next_next_keyword(Keyword::This).is_ok())
            {
                self.bump(); // eat *
                self.bump(); // eat var
                self.bump(); // eat self
                Some(SelfParameter {
                    mutability: Mutability::Mutable,
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

        // with
        let with = if self.peek_keyword(Keyword::With).is_ok() {
            self.bump(); // eat with
            let with = self.eat_with_body()?;
            Some(with)
        } else {
            None
        };

        // return type
        let return_type = if self.peek_token(TokenType::Arrow).is_ok() {
            self.bump(); // eat arrow
            Some(self.eat_type()?)
        } else {
            None
        };

        // body
        let body = if self.peek_token(TokenType::OpenBrace).is_ok() {
            Some(self.eat_block()?)
        } else {
            None
        };

        let function_id = self.tree.allocate(
            Function {
                name,
                visibility,
                runtime,
                // NOTE :Incomplete: support lambda function style
                //  (same postfix problem as with struct literals?)
                style: FunctionStyle::Function,
                with,
                static_parameters,
                self_parameter,
                dynamic_parameters,
                return_type,
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
    use crate::{Function, IntType, Mutability, PrimitiveType, Type, WithClause, assert_node};

    #[test]
    fn test_parse_function_with_clause() {
        let test = TestParser::new(
            r###"
function foo() with (
  !Bar,
  Time,
  F: Numeric,
) => int32 {
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Function { name, with, return_type, .. } => {
            // function name
            assert_eq!(*name, Some(parser.intern_string("foo")));

            // with clause present
            let with_id = with.expect("expected with declaration");
            let with = parser.tree.get(with_id);
            assert_eq!(with.clauses.len(), 3);

            // !Bar
            assert_node!(parser.tree, with.clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Type::Not(inner_id) => {
                    assert_node!(parser.tree, *inner_id, Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.intern_path(vec![parser.intern_string("Bar")])
                        );
                    });
                });
            });

            // Time
            assert_node!(parser.tree, with.clauses[1], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Type::Path { path, static_arguments } => {
                    assert_eq!(
                        *path,
                        parser.intern_path(vec![parser.intern_string("Time")])
                    );
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, with.clauses[2], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_eq!(*path, parser.intern_path(vec![parser.intern_string("F")]));
                });
                assert_node!(parser.tree, *assertion, Type::Path { path, .. } => {
                    assert_eq!(
                        *path,
                        parser.intern_path(vec![parser.intern_string("Numeric")])
                    );
                });
            });

            // return type
            let ret = return_type.expect("expected return type");
            assert_node!(parser.tree, ret, Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 32);
                assert!(int_ty.is_signed);
            });
        });
    }

    #[test]
    fn test_parse_function_self_parameter_simple() {
        let test = TestParser::new("function a(self) {}");
        let mut parser = test.parser();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Function { name, self_parameter, dynamic_parameters, .. } => {
            assert_eq!(*name, Some(parser.intern_string("a")));

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(!self_param.is_pointer);
            assert_eq!(self_param.mutability, Mutability::Immutable);

            assert!(dynamic_parameters.is_empty());
        });
    }

    #[test]
    fn test_parse_function_self_parameter_pointer() {
        let test = TestParser::new(
            r###"
function b(
  *self
  x: int32
) {}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Function { name, self_parameter, dynamic_parameters, .. } => {
            assert_eq!(*name, Some(parser.intern_string("b")));

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_pointer);
            assert_eq!(self_param.mutability, Mutability::Immutable);

            assert_eq!(dynamic_parameters.len(), 1);
            let param = parser.tree.get(dynamic_parameters[0]);
            assert_eq!(param.name, parser.intern_string("x"));

            let param_type = param.r#type.expect("expected type for parameter x");
            assert_node!(parser.tree, param_type, Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                assert_eq!(*width, 32);
                assert!(*is_signed);
            });
        });
    }

    #[test]
    fn test_parse_function_self_parameter_mutable_pointer() {
        let test = TestParser::new("function c(*var self) {}");
        let mut parser = test.parser();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Function { name, self_parameter, dynamic_parameters, .. } => {
            assert_eq!(*name, Some(parser.intern_string("c")));

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_pointer);
            assert_eq!(self_param.mutability, Mutability::Mutable);

            assert!(dynamic_parameters.is_empty());
        });
    }
}
