//! Parse calls, static calls, dynamic calls, etc.

use destack_ast::{
    Argument, Expression, Keyword, LocalNodeId, NodeType, PostfixPosition, TokenType,
};
use destack_source::Span;

use crate::{ParseResult, Parser};

impl Parser {
    /// Eat an explicit index (postfix, excluding the receiver, with `[` and `]`).
    ///
    /// Examples:
    /// ```
    /// []
    /// [1]
    /// ["bar"]
    /// [variable+1]
    /// ```
    pub fn eat_index(
        &mut self,
        receiver_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();
        let receiver_span = self.tree.get_span(receiver_id);

        // open bracket
        self.eat_token(TokenType::OpenBracket)?;
        self.eat_newlines_maybe()?;

        // bare index
        if self.peek_is(TokenType::CloseBracket) {
            self.bump(); // eat close bracket
            let index_span = self.get_span_from(&start);
            let span = Span::new(index_span.file, receiver_span.start, index_span.end);
            let index_id = self.insert_node(
                Expression::Index {
                    position,
                    left: receiver_id,
                    index: None,
                },
                span,
            );
            return Ok(index_id);
        }

        // missing index
        let is_missing_index = Self::is_expression_slot_boundary_token(self.peek_token_type());
        let index = if is_missing_index {
            self.recover_missing_expression_here(NodeType::Expression)
        } else {
            let index_options = if self.options.is_in_type() {
                self.options.nested().in_type()
            } else {
                self.options.nested()
            };
            self.eat_expression(index_options)?
        };

        self.eat_newlines_maybe()?;

        // close bracket
        if !is_missing_index {
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;
        }

        // index
        let index_expression = if self.options.is_in_type() {
            Expression::TypeIndex {
                left: receiver_id,
                index,
            }
        } else {
            Expression::Index {
                position,
                left: receiver_id,
                index: Some(index),
            }
        };
        let index_span = self.get_span_from(&start);
        let span = Span::new(index_span.file, receiver_span.start, index_span.end);
        let index_id = self.insert_node(index_expression, span);
        Ok(index_id)
    }

    /// Eat a new constructor call (including the receiver).
    ///
    /// Examples:
    /// ```
    /// new Foo
    /// new Foo()
    /// new Foo(1, 2)
    /// new Foo<T>()
    /// ```
    pub fn eat_new(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();

        // keyword
        self.eat_keyword(Keyword::New)?;

        // receiver
        let receiver_options = self.options.not_in_position().in_new_receiver();
        let left = self.with_options(receiver_options, |parser| {
            parser.eat_expression(parser.options)
        })?;

        // hoist static arguments parsed on the receiver
        let mut static_arguments = None;
        if let Expression::Path {
            static_arguments: path_arguments,
            ..
        } = self.tree.get_mut(left)
        {
            static_arguments = path_arguments.take();
        }

        // static arguments (may be empty)
        if static_arguments.is_none() {
            static_arguments = self.eat_static_arguments_with_follow_maybe(false, true, true);
        }

        // dynamic arguments (optional in JS: `new Foo` is valid without parentheses)
        let dynamic_arguments = self.eat_dynamic_arguments_maybe()?.unwrap_or_default();

        // call
        let call_id = self.insert_node(
            Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            },
            self.get_span_from(&start),
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
        let start = self.mark_span();

        // keyword
        self.eat_keyword(Keyword::Delete)?;

        // value
        let value_options = self.options.not_in_position();
        let value = self.with_options(value_options, |parser| {
            parser.eat_expression(parser.options)
        })?;

        // delete
        let delete_id = self
            .tree
            .insert(Expression::Delete { value }, self.get_span_from(&start));
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
        let start = self.mark_span();
        let receiver_span = self.tree.get_span(receiver_id);

        // static arguments (may be empty)
        let static_arguments = match static_arguments {
            Some(static_arguments) => Some(static_arguments),
            None => self.eat_static_arguments_maybe()?,
        };

        // dynamic arguments (may be empty)
        let dynamic_arguments = self.eat_dynamic_arguments()?;

        // call
        let call_id = self.insert_node(
            Expression::Call {
                position,
                left: receiver_id,
                static_arguments,
                dynamic_arguments,
            },
            {
                let call_span = self.get_span_from(&start);
                Span::new(call_span.file, receiver_span.start, call_span.end)
            },
        );
        Ok(call_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, BinaryOperator, Declaration, Expression, LocalNodeId, NodeType, Path,
        PostfixPosition, ScalarLiteral, TypeBinaryOperator,
    };
    use destack_source::LanguageType;
    use smallvec::smallvec;

    use crate::{Parser, TestParser, assert_expression_path, assert_node, assert_path};

    fn make_receiver(parser: &mut Parser) -> LocalNodeId<Expression> {
        let receiver_str = parser.strings.intern("receiver");
        let receiver_path = Path {
            segments: smallvec![receiver_str],
        };
        let span = parser.peek().unwrap().span;
        parser.insert_node(
            Expression::Path {
                path: receiver_path,
                static_arguments: None,
            },
            span,
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
    fn test_parse_index_postfix_multiline() {
        // [
        //   1
        // ]
        let mut test = TestParser::new("[\n  1\n]");
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
        // (1, 2)
        let mut test = TestParser::new("(1, 2)");
        let mut parser = test.prepare();
        let recv = make_receiver(&mut parser);
        let call_id = parser
            .eat_call(recv, None, PostfixPosition::Direct)
            .unwrap();

        assert_node!(parser.tree, call_id, Expression::Call { position, left, static_arguments: None, dynamic_arguments } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_eq!(*left, recv);

            // (1, 2)
            assert_eq!(dynamic_arguments.len(), 2);

            // 1
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // 2
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    }

    #[test]
    fn test_parse_member_postfix_missing_name() {
        // foo.
        let mut test = TestParser::new("foo.");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo.
        assert_node!(parser.tree, expression_id, Expression::Member { left, name: None, static_arguments: None } => {
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
    }

    #[test]
    fn test_parse_private_member_postfix_missing_name() {
        // foo.#
        let mut test = TestParser::new("foo.#");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo.#
        assert_node!(parser.tree, expression_id, Expression::PrivateMember { left, name: None, static_arguments: None } => {
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
    }

    #[test]
    fn test_parse_optional_member_postfix_missing_name() {
        // foo?.
        let mut test = TestParser::new("foo?.");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo?.
        assert_node!(parser.tree, expression_id, Expression::Member { left, name: None, static_arguments: None } => {
            assert_node!(parser.tree, *left, Expression::Maybe { left, position } => {
                assert_eq!(*position, PostfixPosition::Direct);
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
            });
        });
    }

    #[test]
    fn test_parse_parenthesized_member_postfix_missing_name_preserves_outer_close() {
        // (foo.)
        let mut test = TestParser::new("(foo.)");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ")")]);

        // (foo.)
        assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Member { left, name: None, static_arguments: None } => {
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
            });
        });
    }

    #[test]
    fn test_parse_call_postfix_missing_close_parenthesis() {
        // foo(
        let mut test = TestParser::new("foo(");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo(
        assert_node!(parser.tree, expression_id, Expression::Call { position, left, static_arguments: None, dynamic_arguments } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert!(dynamic_arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_call_postfix_missing_close_parenthesis_after_argument() {
        // foo(1
        let mut test = TestParser::new("foo(1");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo(1
        assert_node!(parser.tree, expression_id, Expression::Call { position, left, static_arguments: None, dynamic_arguments } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_indirect_call_postfix_missing_close_parenthesis_after_argument() {
        // foo.(1
        let mut test = TestParser::new("foo.(1");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo.(1
        assert_node!(parser.tree, expression_id, Expression::Call { position, left, static_arguments: None, dynamic_arguments } => {
            assert_eq!(*position, PostfixPosition::Indirect);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_optional_call_postfix_missing_close_parenthesis_after_argument() {
        // foo?.(1
        let mut test = TestParser::new("foo?.(1");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo?.(1
        assert_node!(parser.tree, expression_id, Expression::Call { position, left, static_arguments: None, dynamic_arguments } => {
            assert_eq!(*position, PostfixPosition::Indirect);
            assert_node!(parser.tree, *left, Expression::Maybe { left, position } => {
                assert_eq!(*position, PostfixPosition::Direct);
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
            });
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_index_postfix_missing_expression() {
        // foo[
        let mut test = TestParser::new("foo[");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo[
        assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index) } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_node!(parser.tree, *index, Expression::Missing);
        });
    }

    #[test]
    fn test_parse_index_postfix_missing_close_bracket() {
        // foo[1
        let mut test = TestParser::new("foo[1");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo[1
        assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index) } => {
            assert_eq!(*position, PostfixPosition::Direct);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_node!(parser.tree, *index, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    #[test]
    fn test_parse_indirect_index_postfix_missing_expression() {
        // foo.[
        let mut test = TestParser::new("foo.[");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo.[
        assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index) } => {
            assert_eq!(*position, PostfixPosition::Indirect);
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            assert_node!(parser.tree, *index, Expression::Missing);
        });
    }

    #[test]
    fn test_parse_optional_index_postfix_missing_expression() {
        // foo?.[
        let mut test = TestParser::new("foo?.[");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

        // foo?.[
        assert_node!(parser.tree, expression_id, Expression::Index { position, left, index: Some(index) } => {
            assert_eq!(*position, PostfixPosition::Indirect);
            assert_node!(parser.tree, *left, Expression::Maybe { left, position } => {
                assert_eq!(*position, PostfixPosition::Direct);
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
            });
            assert_node!(parser.tree, *index, Expression::Missing);
        });
    }

    #[test]
    fn test_parse_parenthesized_index_postfix_missing_expression_preserves_outer_close() {
        // (foo[)
        let mut test = TestParser::new("(foo[)");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ")")]);

        // (foo[)
        assert_node!(parser.tree, expression_id, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::Index { position, left, index: Some(index) } => {
                assert_eq!(*position, PostfixPosition::Direct);
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
                assert_node!(parser.tree, *index, Expression::Missing);
            });
        });
    }

    #[test]
    fn test_parse_call_expression_with_static_arguments() {
        // foo<T>(1, 2)
        let mut test = TestParser::new("foo<T>(1, 2)");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // foo<T>(1, 2)
        assert_node!(parser.tree, expression_id, Expression::Call { position, left, static_arguments: Some(static_arguments), dynamic_arguments } => {
            assert_eq!(*position, PostfixPosition::Direct);
            // foo
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
            // <T>
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "T");
            });
            // (1, 2)
            assert_eq!(dynamic_arguments.len(), 2);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    }

    #[test]
    fn test_parse_new_without_parentheses() {
        // new Foo (without parentheses, valid JS)
        let mut test = TestParser::new("new Foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        assert_node!(parser.tree, expression_id, Expression::New { left, static_arguments, dynamic_arguments } => {
            // Foo
            assert_expression_path!(parser, parser.tree.get(*left), "Foo");
            // no static arguments
            assert!(static_arguments.is_none());
            // empty dynamic arguments (no parentheses)
            assert!(dynamic_arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_new_with_empty_parentheses() {
        // new Foo() (with empty parentheses)
        let mut test = TestParser::new("new Foo()");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        assert_node!(parser.tree, expression_id, Expression::New { left, static_arguments, dynamic_arguments } => {
            // Foo
            assert_expression_path!(parser, parser.tree.get(*left), "Foo");
            // no static arguments
            assert!(static_arguments.is_none());
            // empty dynamic arguments
            assert!(dynamic_arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_new_with_arguments() {
        // new Foo(1, 2)
        let mut test = TestParser::new("new Foo(1, 2)");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        assert_node!(parser.tree, expression_id, Expression::New { left, static_arguments, dynamic_arguments } => {
            // Foo
            assert_expression_path!(parser, parser.tree.get(*left), "Foo");
            // no static arguments
            assert!(static_arguments.is_none());
            // (1, 2)
            assert_eq!(dynamic_arguments.len(), 2);
        });
    }

    #[test]
    fn test_parse_new_type_arguments_before_if_keyword() {
        // new A<T> if (0);
        let mut test = TestParser::new_with_options("new A<T> if (0);", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // new A<T>
        assert_node!(parser.tree, expression_id, Expression::New { left, static_arguments: Some(static_arguments), dynamic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers, value } => {
                assert!(modifiers.is_none());
                assert_expression_path!(parser, parser.tree.get(*value), "T");
            });
            assert!(dynamic_arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_new_type_arguments_without_parenthesized_call() {
        // new A<T>
        let mut test = TestParser::new_with_options("new A<T>", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // new A<T>
        assert_node!(parser.tree, expression_id, Expression::New { left, static_arguments: Some(static_arguments), dynamic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers, value } => {
                assert!(modifiers.is_none());
                assert_expression_path!(parser, parser.tree.get(*value), "T");
            });
            assert!(dynamic_arguments.is_empty());
        });
    }

    #[test]
    fn test_parse_new_type_arguments_without_parentheses_as_comparison() {
        // new A < T
        let mut test = TestParser::new_with_options("new A < T", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // new A < T
        assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::LessThan);
            assert_node!(parser.tree, *left, Expression::New { left, static_arguments, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "A");
                assert!(static_arguments.is_none());
                assert!(dynamic_arguments.is_empty());
            });
            assert_expression_path!(parser, parser.tree.get(*right), "T");
        });
    }

    #[test]
    fn test_parse_new_multiple_comparisons_without_parenthesized_call() {
        // new A < B > C
        let mut test = TestParser::new_with_options("new A < B > C", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // new A < B > C
        assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::GreaterThan);
            assert_expression_path!(parser, parser.tree.get(*right), "C");
            assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                assert_node!(parser.tree, *left, Expression::New { left, static_arguments, dynamic_arguments } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "A");
                    assert!(static_arguments.is_none());
                    assert!(dynamic_arguments.is_empty());
                });
                assert_expression_path!(parser, parser.tree.get(*right), "B");
            });
        });
    }

    #[test]
    fn test_parse_shift_left_comparison_not_type_arguments_like_babel() {
        // f<< T > (()=>T) > T
        let mut test =
            TestParser::new_with_options("f<< T > (()=>T) > T", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // diagnostics
        test.assert_no_errors(&parser);

        // f<< T > (()=>T) > T
        assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::GreaterThan);
            assert_expression_path!(parser, parser.tree.get(*right), "T");

            // f<< T > (()=>T)
            assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);

                // f<< T
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::ShiftLeft);
                    assert_expression_path!(parser, parser.tree.get(*left), "f");
                    assert_expression_path!(parser, parser.tree.get(*right), "T");
                });

                // (()=>T)
                assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                            assert!(signature.dynamic_parameters.is_empty());
                            assert!(signature.return_type.is_none());
                            assert_expression_path!(parser, parser.tree.get(body.unwrap()), "T");
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_new_with_type_identifier_receiver_and_spread_argument() {
        // new type(...instances)
        let mut test =
            TestParser::new_with_options("new type(...instances)", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // new type(...instances)
        assert_node!(parser.tree, expression_id, Expression::New { left, static_arguments, dynamic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "type");
            assert!(static_arguments.is_none());
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Spread { modifiers, label, value } => {
                assert!(modifiers.is_none());
                assert!(label.is_none());
                assert_expression_path!(parser, parser.tree.get(*value), "instances");
            });
        });
    }

    #[test]
    fn test_parse_new_parenthesized_cast_receiver_with_static_arguments() {
        let mut test = TestParser::new_with_options(
            "new (Promise as PromiseConstructor)<Foo>((resolve, reject) => {})",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        assert_node!(parser.tree, expression_id, Expression::New { left, static_arguments: Some(static_arguments), dynamic_arguments } => {
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::TypeBinary { operator, .. } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);
                });
            });
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "Foo");
            });
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Declaration(_) );
            });
        });
    }
}
