//! Parse functions and closures.

use destack_language_token::TokenType;

use crate::{
    Function, FunctionRuntime, FunctionStyle, Keyword, Mutability, NodeId, ParseResult, Parser,
    SelfParameter,
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
    /// function foo[T, U](x: T) => (int32, boolean) with (
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
    /// function longBar[Validate: boolean](
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
    pub fn eat_function(&mut self) -> ParseResult<NodeId<Function>> {
        let start = self.mark();

        // function
        self.eat_keyword(Keyword::Function)?;

        // runtime
        let runtime = if self.peek_token(TokenType::At).is_ok() {
            self.eat_token(TokenType::At)?;
            FunctionRuntime::Static
        } else {
            FunctionRuntime::Dynamic
        };

        // name
        let name = if self.peek_identifier().is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };

        // static parameters
        let static_parameters = if self.peek_token(TokenType::OpenBracket).is_ok() {
            self.eat_token(TokenType::OpenBracket)?;
            let static_parameters = self.eat_parameters_body()?;
            self.eat_token(TokenType::CloseBracket)?;
            Some(static_parameters)
        } else {
            None
        };

        // dynamic parameters
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;
        // self, *self, *var self parameter
        let self_parameter: Option<SelfParameter> = {
            // self
            if self.peek_keyword(Keyword::Self_).is_ok() {
                self.bump(); // eat self
                Some(SelfParameter {
                    mutability: Mutability::Immutable,
                    is_pointer: false,
                })
            }
            // *self
            else if self.peek_token(TokenType::Multiply).is_ok()
                && self.peek_next_keyword(Keyword::Self_).is_ok()
            {
                self.bump(); // eat *
                self.bump(); // eat self
                Some(SelfParameter {
                    mutability: Mutability::Immutable,
                    is_pointer: true,
                })
            }
            // *var self
            else if self.peek_token(TokenType::Multiply).is_ok()
                && self.peek_next_keyword(Keyword::Var).is_ok()
                && self.peek_next_next_keyword(Keyword::Self_).is_ok()
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
            self.eat_item_stop()?;
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
            self.eat_keyword(Keyword::With)?;
            let with = self.eat_with_body()?;
            Some(with)
        } else {
            None
        };

        // return type
        let return_type = if self.peek_token(TokenType::Arrow).is_ok() {
            self.eat_token(TokenType::Arrow)?;
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
                runtime,
                // todo!: support lambda function style
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
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{IntType, Mutability, Parser, PrimitiveType, Type, WithClause};

    #[test]
    fn test_parse_function_with() {
        let input = r##"
function foo() with (
  !Bar,
  Time[float32],
  F: Numeric,
) => int32 {
}
"##;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let function_id = parser.eat_function().unwrap();
        let function = parser.tree.get(function_id);

        // with present
        let with_id = function.with.expect("expected with declaration");
        let with = parser.tree.get(with_id);
        assert_eq!(with.clauses.len(), 3);

        // !Bar
        match parser.tree.get(with.clauses[0]) {
            WithClause::Declaration { target, .. } => match parser.tree.get(*target) {
                Type::Not(inner_id) => match parser.tree.get(*inner_id) {
                    Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.paths.intern(vec![parser.strings.intern("Bar")])
                        );
                    }
                    _ => panic!("expected path type inside !"),
                },
                _ => panic!("expected Not type"),
            },
            _ => panic!("expected declaration clause"),
        }

        // Time[float32]
        match parser.tree.get(with.clauses[1]) {
            WithClause::Declaration { target, .. } => match parser.tree.get(*target) {
                Type::Path {
                    path,
                    static_arguments,
                } => {
                    assert_eq!(
                        *path,
                        parser.paths.intern(vec![parser.strings.intern("Time")])
                    );
                    let args = static_arguments.as_ref().expect("expected static args");
                    assert_eq!(args.len(), 1);
                }
                _ => panic!("expected path type with static args"),
            },
            _ => panic!("expected declaration clause"),
        }

        // F: Numeric
        match parser.tree.get(with.clauses[2]) {
            WithClause::Assertion { target, assertion } => {
                match parser.tree.get(*target) {
                    Type::Path { path, .. } => {
                        assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("F")]));
                    }
                    _ => panic!("expected path type for target"),
                }
                match parser.tree.get(*assertion) {
                    Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.paths.intern(vec![parser.strings.intern("Numeric")])
                        );
                    }
                    _ => panic!("expected path type for assertion"),
                }
            }
            _ => panic!("expected assertion clause"),
        }

        // return type
        let ret = function.return_type.expect("expected return type");
        match parser.tree.get(ret) {
            Type::Primitive(PrimitiveType::Int(int_ty)) => {
                assert_eq!(int_ty.width, 32);
                assert!(int_ty.is_signed);
            }
            _ => panic!("expected int32 return type"),
        }
    }

    #[test]
    fn test_parse_function_self_parameter() {
        let input = r##"
function a(self) {}
function b(
  *self
  x: int32
) {}
function c(*var self) {}
"##;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        // function a(self) {}
        let a_id = parser.eat_function().unwrap();
        let a = parser.tree.get(a_id);
        let a_self = a.self_parameter.as_ref().expect("expected self param");
        assert!(!a_self.is_pointer);
        assert_eq!(a_self.mutability, Mutability::Immutable);
        assert!(a.dynamic_parameters.is_empty());
        parser.eat_newline().unwrap();

        // function b(*self \n x: int32) {}
        let b_id = parser.eat_function().unwrap();
        let b = parser.tree.get(b_id);
        let b_self = b.self_parameter.as_ref().expect("expected self param");
        assert!(b_self.is_pointer);
        assert_eq!(b_self.mutability, Mutability::Immutable);
        assert_eq!(b.dynamic_parameters.len(), 1);
        let b_param = parser.tree.get(b.dynamic_parameters[0]);
        assert_eq!(b_param.name, parser.strings.intern("x"));
        match b_param.r#type {
            Some(ty_id) => match parser.tree.get(ty_id) {
                Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                    assert_eq!(*width, 32);
                    assert!(*is_signed);
                }
                _ => panic!("expected int32 type for parameter x"),
            },
            None => panic!("expected type for parameter x"),
        }
        parser.eat_newline().unwrap();

        // function c(*var self) {}
        let c_id = parser.eat_function().unwrap();
        let c = parser.tree.get(c_id);
        let c_self = c.self_parameter.as_ref().expect("expected self param");
        assert!(c_self.is_pointer);
        assert_eq!(c_self.mutability, Mutability::Mutable);
        assert!(c.dynamic_parameters.is_empty());
        parser.eat_newline().unwrap();
    }
}
