//! Parse calls, static calls, dynamic calls, etc.

use dyst_ast::{Argument, Expression, Keyword, LocalNodeId, PostfixPosition, TokenType};

use crate::{ParseResult, Parser};

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
    pub fn eat_index(
        &mut self,
        receiver_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // open bracket
        self.eat_token(TokenType::OpenBracket)?;

        // bare index
        if self.peek_token(TokenType::CloseBracket).is_ok() {
            self.bump(); // eat close bracket
            let index_id = self.tree.insert(
                Expression::Index {
                    position,
                    left: receiver_id,
                    index: None,
                },
                self.get_span_from(start),
            );
            return Ok(index_id);
        }

        // expression
        let index = self.with_options(self.options.nested(), |parser| parser.eat_expression())?;

        // close bracket
        self.eat_token(TokenType::CloseBracket)?;

        // index
        let index_id = self.tree.insert(
            Expression::Index {
                position,
                left: receiver_id,
                index: Some(index),
            },
            self.get_span_from(start),
        );
        Ok(index_id)
    }

    /// Eat a new constructor call (including the receiver).
    ///
    /// Examples:
    /// ```
    /// new
    /// ```
    pub fn eat_new(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::New)?;

        // receiver
        let left = self.with_options(self.options.in_new_receiver(), |parser| {
            parser.eat_expression()
        })?;

        // static arguments (may be empty)
        let static_arguments = self.eat_static_arguments_maybe()?;

        // dynamic arguments (may be empty)
        let dynamic_arguments = self.eat_dynamic_arguments()?;

        // call
        let call_id = self.tree.insert(
            Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            },
            self.get_span_from(start),
        );
        Ok(call_id)
    }

    /// Eat a delete expression.
    ///
    /// Examples:
    /// ```
    /// delete
    /// delete foo
    /// delete foo.bar
    /// delete foo['result']
    /// ```
    pub fn eat_delete(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Delete)?;

        // value
        let value = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_expression()
        })?;

        // delete
        let delete_id = self
            .tree
            .insert(Expression::Delete { value }, self.get_span_from(start));
        Ok(delete_id)
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
    pub fn eat_call(
        &mut self,
        receiver_id: LocalNodeId<Expression>,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        position: PostfixPosition,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // static arguments (may be empty)
        let static_arguments = match static_arguments {
            Some(static_arguments) => Some(static_arguments),
            None => self.eat_static_arguments_maybe()?,
        };

        // dynamic arguments (may be empty)
        let dynamic_arguments = self.eat_dynamic_arguments()?;

        // call
        let call_id = self.tree.insert(
            Expression::Call {
                position,
                left: receiver_id,
                static_arguments,
                dynamic_arguments,
            },
            self.get_span_from(start),
        );
        Ok(call_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{Argument, Expression, LocalNodeId, Name, Path, PostfixPosition, ScalarLiteral};
    use smallvec::smallvec;

    use crate::{
        Parser, TestParser, assert_expression_path, assert_node, assert_path, assert_string,
    };

    fn make_receiver(parser: &mut Parser<'_>) -> LocalNodeId<Expression> {
        let receiver_str = parser.strings.intern("receiver");
        let receiver_path = Path {
            segments: smallvec![receiver_str],
        };
        parser.tree.insert(
            Expression::Path {
                path: receiver_path,
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
        let recv = make_receiver(&mut parser);

        let index_id = parser.eat_index(recv, PostfixPosition::Direct).unwrap();
        assert_node!(parser.tree, index_id, Expression::Index { position, left, index } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_eq!(*left, recv);
            assert_node!(parser.tree, index.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    #[test]
    fn test_parse_call_postfix() {
        // (1, x: 2)
        let mut test = TestParser::new("(1, x: 2)");
        let mut parser = test.prepare();
        let recv = make_receiver(&mut parser);
        let call_id = parser
            .eat_call(recv, None, PostfixPosition::Direct)
            .unwrap();

        assert_node!(parser.tree, call_id, Expression::Call { position, left, static_arguments: None, dynamic_arguments } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_eq!(*left, recv);

            // (1, x: 2)
            assert_eq!(dynamic_arguments.len(), 2);

            // 1
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // x: 2
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    }

    #[test]
    fn test_parse_call_expression_with_static_arguments() {
        // foo<int32>(1, x: 2)
        let mut test = TestParser::new("foo<T>(1, x: 2)");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // foo<int32>(1, x: 2)
        assert_node!(parser.tree, expression_id, Expression::Call { position, left, static_arguments: Some(static_arguments), dynamic_arguments } => {
            assert_eq!(*position, PostfixPosition::Direct);
            // foo
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            // <T>
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "T");
            });
            // (1, x: 2)
            assert_eq!(dynamic_arguments.len(), 2);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    }
}
