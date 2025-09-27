//! Parse functions and closures.

use crate::ScopedMutability;
use crate::parse::prelude::*;
use dyst_token::TokenType;

use crate::{
    Function, FunctionStyle, Keyword, Mutability, NodeId, NodeType, ParseResult, Parser, Runtime,
    SelfParameter, TypeParserOptions, Visibility,
};

impl<'a> Parser<'a> {
    /// Peek a self keyword (also accepts `this`).
    fn peek_self_keyword(&mut self) -> ParseResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Self_ || keyword == Keyword::This {
            Ok(keyword)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a self keyword (also accepts `this`).
    fn eat_self_keyword(&mut self) -> ParseResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Self_ || keyword == Keyword::This {
            self.bump(); // eat self
            Ok(keyword)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
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
            Some(self.eat_identifier().for_node_type(NodeType::Function)?)
        } else {
            None
        };

        // static parameters
        let static_parameters = if self.peek_token(TokenType::LessThan).is_ok() {
            self.bump(); // eat less than
            let static_parameters = self
                .eat_parameters_body()
                .for_node_type(NodeType::Function)?;
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
                let mutability = self
                    .eat_scoped_mutability()
                    .for_node_type(NodeType::Function)?;
                self.eat_self_keyword().for_node_type(NodeType::Function)?; // eat self
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
                || self.peek_token(TokenType::BitwiseAnd).is_ok())
                && self.peek_next_keyword(Keyword::Var).is_ok()
            {
                self.bump(); // eat &
                let mutability = self
                    .eat_scoped_mutability()
                    .for_node_type(NodeType::Function)?;
                self.eat_self_keyword().for_node_type(NodeType::Function)?; // eat self
                Some(SelfParameter {
                    mutability,
                    is_pointer: true,
                })
            }
            // &self
            else if self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::BitwiseAnd).is_ok()
            {
                self.bump(); // eat &
                self.eat_self_keyword().for_node_type(NodeType::Function)?; // eat self
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
            self.eat_parameters_body()
                .for_node_type(NodeType::Function)?
        };
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        // return type
        let return_type = if self.peek_arrow().is_ok() {
            self.bump(); // eat arrow
            let return_type = self
                .eat_type(TypeParserOptions::default())
                .for_node_type(NodeType::Function)?;
            Some(return_type)
        } else {
            None
        };

        // with (postfix)
        let with = if self.peek_keyword(Keyword::With).is_ok() {
            self.bump(); // eat with
            let with = self.eat_with_body().for_node_type(NodeType::Function)?;
            Some(with)
        } else {
            None
        };

        // body
        let body = if self.peek_token(TokenType::OpenBrace).is_ok() {
            Some(self.eat_block().for_node_type(NodeType::Function)?)
        } else {
            None
        };

        let function_id = self.tree.allocate(
            Function {
                name,
                visibility,
                runtime,
                // NOTE @Incomplete: support lambda function style
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
    use crate::{
        Function, IntType, Mutability, PrimitiveType, ScopedMutability, Type, WithClause,
        assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_function_with_clause() {
        let mut test = TestParser::new(
            r###"
function foo() => int32 with (
  !Bar,
  Time,
  F: Numeric,
) {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Function { name, with, return_type, .. } => {
            // function name
            assert_string!(parser.session, name.unwrap(), "foo");

            // with clause present
            let with_id = with.expect("expected with declaration");
            let with = parser.tree.get(with_id);
            assert_eq!(with.clauses.len(), 3);

            // !Bar
            assert_node!(parser.tree, with.clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Type::Not(inner_id) => {
                    assert_node!(parser.tree, *inner_id, Type::Path { path, .. } => {
                        assert_path!(parser.session, *path, "Bar");
                    });
                });
            });

            // Time
            assert_node!(parser.tree, with.clauses[1], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Type::Path { path, static_arguments } => {
                    assert_path!(parser.session, *path, "Time");
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, with.clauses[2], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "F");
                });
                assert_node!(parser.tree, *assertion, Type::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Numeric");
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
        let mut test = TestParser::new("function a(self) {}");
        let mut parser = test.prepare();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Function { name, self_parameter, dynamic_parameters, .. } => {
            assert_string!(parser.session, name.unwrap(), "a");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(!self_param.is_pointer);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            assert!(dynamic_parameters.is_empty());
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
        assert_node!(parser.tree, function_id, Function { name, self_parameter, dynamic_parameters, .. } => {
            assert_string!(parser.session, name.unwrap(), "b");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_pointer);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            assert_eq!(dynamic_parameters.len(), 1);
            let param = parser.tree.get(dynamic_parameters[0]);
            assert_string!(parser.session, param.name, "x");

            let param_type = param.r#type.expect("expected type for parameter x");
            assert_node!(parser.tree, param_type, Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                assert_eq!(*width, 32);
                assert!(*is_signed);
            });
        });
    }

    #[test]
    fn test_parse_function_self_parameter_mutable_pointer() {
        let mut test = TestParser::new("function c(&var self) {}");
        let mut parser = test.prepare();

        let function_id = parser.eat_function(None).unwrap();
        assert_node!(parser.tree, function_id, Function { name, self_parameter, dynamic_parameters, .. } => {
            assert_string!(parser.session, name.unwrap(), "c");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert!(self_param.is_pointer);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });

            assert!(dynamic_parameters.is_empty());
        });
    }
}
