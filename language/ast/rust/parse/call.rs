//! Parse calls, static calls, dynamic calls, etc.

use dyst_language_token::TokenType;

use crate::{Call, Cast, Expression, Index, Keyword, NodeId, ParseResult, Parser, Runtime};

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
    /// [int32](1, 2, 3)
    /// [Validate: false](1, 2, 3)
    /// (Vector2 {x: 1, y: 2}, (true, 3))
    /// ```
    pub fn eat_call_postfix(
        &mut self,
        receiver_id: NodeId<Expression>,
        runtime: Runtime,
    ) -> ParseResult<NodeId<Call>> {
        let start = self.mark();
        // static arguments (may not exist or be empty)
        let static_arguments = if self.peek_token(TokenType::OpenBracket).is_ok() {
            self.eat_token(TokenType::OpenBracket)?;
            if self.peek_token(TokenType::CloseBracket).is_ok() {
                self.eat_token(TokenType::CloseBracket)?;
                None
            } else {
                let static_arguments = self.eat_arguments_body()?;
                self.eat_token(TokenType::CloseBracket)?;
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
                runtime,
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
    use crate::parse::tests::TestParse;
    use crate::{
        Argument, Cast, Expression, Index, IntType, PrimitiveType, Runtime, ScalarLiteral, Type,
        assert_node,
    };

    #[test]
    fn test_parse_index_postfix() {
        // [1]
        let test = TestParse::new("[1]");
        let mut parser = test.parser();
        let recv = parser
            .tree
            .allocate(Expression::Error, parser.peek().unwrap().span);

        let index_id = parser.eat_index_postfix(recv).unwrap();
        assert_node!(parser.tree, index_id, Index { receiver, index } => {
            assert_eq!(*receiver, recv);
            assert_node!(parser.tree, *index, Expression::ScalarLiteral(lit_id) => {
                assert_node!(parser.tree, *lit_id, ScalarLiteral::Integer(1, _));
            });
        });
    }

    #[test]
    fn test_parse_call_postfix() {
        // [Validate: false](1, x: 2)
        let test = TestParse::new("[Validate: false](1, x: 2)");
        let mut parser = test.parser();
        let recv = parser
            .tree
            .allocate(Expression::Error, parser.peek().unwrap().span);

        let call_id = parser.eat_call_postfix(recv, Runtime::Dynamic).unwrap();
        assert_node!(parser.tree, call_id, crate::Call { receiver, runtime, static_arguments, dynamic_arguments } => {
            assert_eq!(*receiver, recv);
            assert_eq!(*runtime, Runtime::Dynamic);

            // [Validate: false]
            let static_args = static_arguments.as_ref().expect("expected static args");
            assert_eq!(static_args.len(), 1);
            assert_node!(parser.tree, static_args[0], Argument::Named { name, value } => {
                assert_eq!(*name, parser.strings.intern("Validate"));
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(lit_id) => {
                    assert_node!(parser.tree, *lit_id, ScalarLiteral::Boolean(false));
                });
            });

            // (1, x: 2)
            assert_eq!(dynamic_arguments.len(), 2);

            // 1
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(lit_id) => {
                    assert_node!(parser.tree, *lit_id, ScalarLiteral::Integer(1, _));
                });
            });

            // x: 2
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Named { name, value } => {
                assert_eq!(*name, parser.strings.intern("x"));
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(lit_id) => {
                    assert_node!(parser.tree, *lit_id, ScalarLiteral::Integer(2, _));
                });
            });
        });
    }

    #[test]
    fn test_parse_call_postfix_empty() {
        // ()
        let test = TestParse::new("()");
        let mut parser = test.parser();
        let recv = parser
            .tree
            .allocate(Expression::Error, parser.peek().unwrap().span);

        let call_id = parser.eat_call_postfix(recv, Runtime::Dynamic).unwrap();
        assert_node!(parser.tree, call_id, crate::Call { receiver, runtime, static_arguments, dynamic_arguments } => {
            assert_eq!(*receiver, recv);
            assert_eq!(*runtime, Runtime::Dynamic);
            assert!(static_arguments.is_none());
            assert_eq!(dynamic_arguments.len(), 0);
        });
    }

    #[test]
    fn test_parse_as_postfix() {
        // as int32
        let test = TestParse::new("as int32");
        let mut parser = test.parser();
        let recv = parser
            .tree
            .allocate(Expression::Error, parser.peek().unwrap().span);

        let cast_id = parser.eat_as_postfix(recv).unwrap();
        assert_node!(parser.tree, cast_id, Cast { receiver, r#type } => {
            assert_eq!(*receiver, recv);
            assert_node!(parser.tree, *r#type, Type::Primitive(PrimitiveType::Int(IntType { width: 32, is_signed: true })));
        });
    }
}
