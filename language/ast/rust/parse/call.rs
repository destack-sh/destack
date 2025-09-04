//! Parse calls, static calls, dynamic calls, etc.

use destack_language_token::TokenType;

use crate::{Call, Cast, Expression, FunctionRuntime, Index, Keyword, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an index (postfix, excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// [1]
    /// [1..3]
    /// ["bar"]
    /// [variable+1]
    /// ```
    pub fn eat_index_postfix(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Index>> {
        let start = self.mark();
        self.eat_token(TokenType::OpenBracket)?;
        let index = self.eat_expression(None)?;
        self.eat_token(TokenType::CloseBracket)?;
        let index_id = self.tree.allocate(
            Index {
                receiver: receiver_id,
                index,
            },
            self.get_span_from(start),
        );
        Ok(index_id)
    }

    /// Eat a call (postfix, excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// ()
    /// (1, 2, 3)
    /// <int32>(1, 2, 3)
    /// <Validate: false>(1, 2, 3)
    /// (Vector2 {x: 1, y: 2}, (true, 3))
    /// ```
    pub fn eat_call_postfix(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Call>> {
        let start = self.mark();
        // static arguments (may not exist or be empty)
        let static_arguments = if self.peek_token(TokenType::LessThan).is_ok() {
            self.eat_token(TokenType::LessThan)?;
            if self.peek_token(TokenType::GreaterThan).is_ok() {
                self.eat_token(TokenType::GreaterThan)?;
                None
            } else {
                let static_arguments = self.eat_arguments_body()?;
                self.eat_token(TokenType::GreaterThan)?;
                Some(static_arguments)
            }
        } else {
            None
        };
        // dynamic arguments (may be empty)
        self.eat_token(TokenType::OpenParenthesis)?;
        let dynamic_arguments = if self.peek_token(TokenType::CloseParenthesis).is_ok() {
            vec![]
        } else {
            self.eat_arguments_body()?
        };
        self.eat_token(TokenType::CloseParenthesis)?;
        // call
        let call_id = self.tree.allocate(
            Call {
                // todo!: determine / pass function runtime? (lookbehind?)
                runtime: FunctionRuntime::Dynamic,
                receiver: receiver_id,
                static_arguments,
                dynamic_arguments,
            },
            self.get_span_from(start),
        );
        Ok(call_id)
    }

    /// Eat an as cast (postfix, excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// as int32
    /// as Vector2
    /// as some_module.MyType
    /// ```
    pub fn eat_as_postfix(&mut self, receiver_id: NodeId<Expression>) -> ParseResult<NodeId<Cast>> {
        let start = self.mark();
        self.eat_keyword(Keyword::As)?;
        let r#type = self.eat_type()?;
        let cast_id = self.tree.allocate(
            Cast {
                receiver: receiver_id,
                r#type,
            },
            self.get_span_from(start),
        );
        Ok(cast_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, Span, tokenize_semantic};

    use crate::{
        Argument, Expression, FunctionRuntime, IntType, Parser, PrimitiveType, ScalarLiteral, Type,
    };

    fn make_dummy_receiver<'a>(parser: &mut Parser<'a>) -> crate::NodeId<Expression> {
        parser
            .tree
            .allocate(Expression::Error, Span { start: 0, end: 0 })
    }

    #[test]
    fn test_parse_index_postfix() {
        let input = "[1]";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let recv = make_dummy_receiver(&mut parser);

        let index_id = parser.eat_index_postfix(recv).unwrap();
        let index = parser.tree.get(index_id);

        // receiver
        assert_eq!(index.receiver, recv);
        // [1]
        let idx_expr = parser.tree.get(index.index);
        match idx_expr {
            &Expression::ScalarLiteral(lit_id) => match parser.tree.get(lit_id) {
                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                _ => panic!("expected integer literal"),
            },
            _ => panic!("expected scalar literal expression"),
        }
    }

    #[test]
    fn test_parse_call_postfix() {
        let input = "<Validate: false>(1, x: 2)";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let recv = make_dummy_receiver(&mut parser);

        let call_id = parser.eat_call_postfix(recv).unwrap();
        let call = parser.tree.get(call_id);

        // receiver and runtime
        assert_eq!(call.receiver, recv);
        assert_eq!(call.runtime, FunctionRuntime::Dynamic);

        // <Validate: false>
        let static_args = call
            .static_arguments
            .as_ref()
            .expect("expected static args");
        assert_eq!(static_args.len(), 1);
        match parser.tree.get(static_args[0]) {
            Argument::Named { name, value } => {
                assert_eq!(*name, parser.strings.intern("Validate"));
                match parser.tree.get(*value) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Boolean(b) => assert!(!b),
                        _ => panic!("expected boolean literal"),
                    },
                    _ => panic!("expected scalar literal expression"),
                }
            }
            _ => panic!("expected named argument"),
        }

        // (1, x: 2)
        assert_eq!(call.dynamic_arguments.len(), 2);
        // 1
        match parser.tree.get(call.dynamic_arguments[0]) {
            Argument::Positional { value } => match parser.tree.get(*value) {
                Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                    ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                    _ => panic!("expected integer literal"),
                },
                _ => panic!("expected scalar literal expression"),
            },
            _ => panic!("expected positional argument"),
        }
        // x: 2
        match parser.tree.get(call.dynamic_arguments[1]) {
            Argument::Named { name, value } => {
                assert_eq!(*name, parser.strings.intern("x"));
                match parser.tree.get(*value) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 2),
                        _ => panic!("expected integer literal"),
                    },
                    _ => panic!("expected scalar literal expression"),
                }
            }
            _ => panic!("expected named argument"),
        }
    }

    #[test]
    fn test_parse_call_postfix_empty() {
        let input = "()";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let recv = make_dummy_receiver(&mut parser);

        let call_id = parser.eat_call_postfix(recv).unwrap();
        let call = parser.tree.get(call_id);

        assert_eq!(call.receiver, recv);
        assert_eq!(call.runtime, FunctionRuntime::Dynamic);
        assert_eq!(call.dynamic_arguments.len(), 0);
    }

    #[test]
    fn test_parse_call_postfix_multiline_dynamic() {
        let input = "(\n1\nx: 2\n)";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let recv = make_dummy_receiver(&mut parser);

        let call_id = parser.eat_call_postfix(recv).unwrap();
        let call = parser.tree.get(call_id);

        assert!(call.static_arguments.is_none());
        assert!(call.dynamic_arguments.len() >= 2);

        // find positional integer 1 among dynamic args
        let mut found_positional_one = false;
        let mut found_named_x_two = false;
        for arg_id in &call.dynamic_arguments {
            match parser.tree.get(*arg_id) {
                Argument::Positional { value } => {
                    if let Expression::ScalarLiteral(lit_id) = parser.tree.get(*value) {
                        match parser.tree.get(*lit_id) {
                            ScalarLiteral::Integer(n, _) if *n == 1 => found_positional_one = true,
                            _ => {}
                        }
                    }
                }
                Argument::Named { name, value } => {
                    if *name == parser.strings.intern("x")
                        && let Expression::ScalarLiteral(lit_id) = parser.tree.get(*value)
                    {
                        match parser.tree.get(*lit_id) {
                            ScalarLiteral::Integer(n, _) if *n == 2 => found_named_x_two = true,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        assert!(found_positional_one, "missing positional integer 1");
        assert!(found_named_x_two, "missing named x: 2");
    }

    #[test]
    fn test_parse_as_postfix() {
        let input = "as int32";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let recv = make_dummy_receiver(&mut parser);

        let cast_id = parser.eat_as_postfix(recv).unwrap();
        let cast = parser.tree.get(cast_id);

        // receiver
        assert_eq!(cast.receiver, recv);
        // int32
        match parser.tree.get(cast.r#type) {
            &Type::Primitive(PrimitiveType::Int(IntType { width, is_signed })) => {
                assert_eq!(width, 32);
                assert!(is_signed);
            }
            _ => panic!("expected int32 type"),
        }
    }
}
