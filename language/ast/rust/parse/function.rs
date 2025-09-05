//! Parse functions and closures.

use destack_language_token::TokenType;

use crate::{Function, FunctionRuntime, FunctionStyle, Keyword, NodeId, ParseResult, Parser};

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
    /// function () => 0
    /// function x(x) => x + 1
    /// function y(x: int32) => x + 1
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
        let dynamic_parameters = if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            vec![]
        } else {
            self.eat_parameters_body()?
        };
        self.eat_token(TokenType::CloseParenthesis)?;

        // with
        let with = if self.peek_keyword(Keyword::With).is_ok() {
            self.eat_keyword(Keyword::With)?;
            let with = self.eat_with_header()?;
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
                // TODO: support lambda function style
                style: FunctionStyle::Function,
                with,
                static_parameters,
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

    use crate::{Parser, PrimitiveType, Type, WithClause};

    #[test]
    fn test_parse_function_with_parenthesized_multiline_with() {
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
}
