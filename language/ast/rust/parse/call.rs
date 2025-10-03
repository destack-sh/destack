//! Parse calls, static calls, dynamic calls, etc.

use dyst_token::TokenType;

use crate::{Expression, NodeId, ParseResult, Parser, Runtime};

impl<'a> Parser<'a> {
    /// Eat an explicit index (postfix, excluding the receiver, with `[` and `]`).
    ///
    /// Examples:
    /// ```
    /// []
    /// [1]
    /// [1..3]
    /// ["bar"]
    /// [variable+1]
    /// ```
    pub fn eat_index_postfix_explicit(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // open bracket
        self.eat_token(TokenType::OpenBracket)?;

        // bare index
        if self.peek_token(TokenType::CloseBracket).is_ok() {
            self.bump(); // eat close bracket
            let index_id = self.tree.allocate(
                Expression::Index {
                    receiver: receiver_id,
                    index: None,
                },
                self.get_span_from(start),
            );
            return Ok(index_id);
        }

        // expression
        let index =
            self.with_options(self.options.in_nested(), |parser| parser.eat_expression())?;

        // close bracket
        self.eat_token(TokenType::CloseBracket)?;

        // index
        let index_id = self.tree.allocate(
            Expression::Index {
                receiver: receiver_id,
                index: Some(index),
            },
            self.get_span_from(start),
        );
        Ok(index_id)
    }

    /// Eat an implicit tuple index (postfix, excluding the receiver, with `.`).
    ///
    /// Examples:
    /// ```
    /// .1
    /// ```
    pub fn eat_index_postfix_implicit(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // dot
        self.eat_token(TokenType::Dot)?;

        // literal
        let literal_id = self.eat_scalar_literal()?;
        let literal_id = self.tree.allocate(
            Expression::ScalarLiteral(literal_id),
            self.get_span_from(start),
        );

        // index
        let index_id = self.tree.allocate(
            Expression::Index {
                receiver: receiver_id,
                index: Some(literal_id),
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
        runtime: Option<Runtime>,
    ) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // dynamic arguments (may be empty)
        let dynamic_arguments = self.eat_dynamic_arguments()?;

        // call
        let call_id = self.tree.allocate(
            Expression::Call {
                runtime,
                receiver: receiver_id,
                dynamic_arguments,
            },
            self.get_span_from(start),
        );
        Ok(call_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Expression, NodeId, Parser, Runtime, ScalarLiteral, assert_node, assert_string,
    };

    fn make_self_expression(parser: &mut Parser<'_>) -> NodeId<Expression> {
        let self_str = parser.intern_string("self");
        let self_path = parser.intern_path(vec![self_str]);
        parser.tree.allocate(
            Expression::Path {
                path: self_path,
                static_arguments: None,
            },
            parser.peek().unwrap().span,
        )
    }

    #[test]
    fn test_parse_index_postfix_explicit() {
        // [1]
        let mut test = TestParser::new("[1]");
        let mut parser = test.prepare();
        let recv = make_self_expression(&mut parser);

        let index_id = parser.eat_index_postfix_explicit(recv).unwrap();
        assert_node!(parser.tree, index_id, Expression::Index { receiver, index } => {
            assert_eq!(*receiver, recv);
            assert_node!(parser.tree, index.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    #[test]
    fn test_parse_index_postfix_implicit() {
        // .1
        let mut test = TestParser::new(".1");
        let mut parser = test.prepare();
        let recv = make_self_expression(&mut parser);
        let index_id = parser.eat_index_postfix_implicit(recv).unwrap();
        assert_node!(parser.tree, index_id, Expression::Index { receiver, index } => {
            assert_eq!(*receiver, recv);
            assert_node!(parser.tree, index.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    #[test]
    fn test_parse_call_postfix() {
        // (1, x: 2)
        let mut test = TestParser::new("(1, x: 2)");
        let mut parser = test.prepare();
        let recv = make_self_expression(&mut parser);
        let call_id = parser
            .eat_call_postfix(recv, Some(Runtime::Dynamic))
            .unwrap();

        assert_node!(parser.tree, call_id, Expression::Call { receiver, runtime, dynamic_arguments } => {
            assert_eq!(*receiver, recv);
            assert_eq!(*runtime, Some(Runtime::Dynamic));

            // (1, x: 2)
            assert_eq!(dynamic_arguments.len(), 2);

            // 1
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // x: 2
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Named { name, value } => {
                assert_string!(parser.session, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    }
}
