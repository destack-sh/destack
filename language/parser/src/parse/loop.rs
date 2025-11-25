//! Parse loops, for, while, etc.

use dyst_ast::{Asynchrony, Expression, ForEachKind, Keyword, LocalNodeId, TokenType, WhileKind};

use crate::{ParseResult, Parser};

impl Parser {
    /// Eat a loop (e.g., `loop { ... }`).
    ///
    /// Examples:
    /// ```
    /// loop {
    ///     y = getNext()
    ///     if y < 0 {
    ///         break
    ///     }
    /// }
    /// ```
    pub fn eat_loop(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Loop)?;

        // body
        let body_id = self.eat_block()?;

        // loop
        let loop_id = self.tree.insert(
            Expression::Loop { body: body_id },
            self.get_span_from(start),
        );
        Ok(loop_id)
    }

    /// Eat a for each or for condition loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// for item in items {
    ///     item
    /// }
    ///
    /// @for x in 1..10 {
    ///     y = 2
    /// }
    ///
    /// for x in zeds.iter() a: {
    ///     if y > 5 {
    ///         continue :a
    ///     }
    ///     y = 2
    /// }
    ///
    /// for (var x = 0; x < 10; x++) {
    ///     y = 2
    /// }
    /// ```
    pub fn eat_for(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::For)?;

        // asynchrony
        let asynchrony = if self.peek_keyword(Keyword::Await).is_ok() {
            self.bump(); // eat await keyword
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };

        let in_parenthesis = self.peek_token(TokenType::OpenParenthesis).is_ok();

        // for condition loop
        if asynchrony == Asynchrony::Sync
            && in_parenthesis
            && self
                .find_before_matching_close(
                    TokenType::OpenParenthesis,
                    TokenType::CloseParenthesis,
                    TokenType::Semicolon,
                )
                .is_ok()
        {
            // open parenthesis
            self.bump();

            // initialization
            let initialization_id = if self.peek_token(TokenType::Semicolon).is_ok() {
                None
            } else {
                Some(self.with_options(self.options.nested(), |parser| parser.eat_expression())?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // condition
            let condition_id = if self.peek_token(TokenType::Semicolon).is_ok() {
                None
            } else {
                Some(self.with_options(self.options.nested(), |parser| parser.eat_expression())?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // increment
            let increment_id = if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                None
            } else {
                Some(self.with_options(self.options.nested(), |parser| parser.eat_expression())?)
            };

            // close parenthesis
            self.eat_token(TokenType::CloseParenthesis)?;

            // body
            let body_id = self.eat_block()?;

            // for
            let for_id = self.tree.insert(
                Expression::For {
                    initialization: initialization_id,
                    condition: condition_id,
                    increment: increment_id,
                    body: body_id,
                },
                self.get_span_from(start),
            );
            Ok(for_id)
        }
        // explicit pattern for loop
        else {
            if in_parenthesis {
                // open parenthesis
                self.bump();
            }

            // pattern
            let pattern_id = self.with_options(
                self.options
                    .not_in_position()
                    .in_for_each()
                    .in_before_block(),
                |parser| parser.eat_pattern(),
            )?;

            // in
            let kind = match self.eat_keyword_in(&[Keyword::In, Keyword::Of])? {
                Keyword::In => ForEachKind::In,
                Keyword::Of => ForEachKind::Of,
                _ => unreachable!(),
            };

            // iterator
            let iterator_id = self
                .with_options(self.options.nested().in_before_block(), |parser| {
                    parser.eat_expression()
                })?;

            if in_parenthesis {
                // close parenthesis
                self.eat_token(TokenType::CloseParenthesis)?;
            }

            // body
            let body_id = self.eat_block()?;

            // for
            let for_id = self.tree.insert(
                Expression::ForEach {
                    asynchrony,
                    kind,
                    pattern: pattern_id,
                    iterator: iterator_id,
                    body: body_id,
                },
                self.get_span_from(start),
            );
            Ok(for_id)
        }
    }

    /// Eat a while or do-while loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// @while x > 1 {
    ///     y = 2
    /// }
    ///
    /// while y < 10 l: {
    ///     y = 2
    ///     break :l
    /// }
    ///
    /// do {
    ///     y = 2
    /// } while x > 1
    /// ```
    pub fn eat_while(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // do-while loop
        if self.peek_keyword(Keyword::Do).is_ok() {
            // do keyword
            self.bump(); // eat do keyword

            // body
            let body_id = self.eat_block()?;

            // while keyword
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition_id = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;

            // while
            let while_id = self.tree.insert(
                Expression::While {
                    kind: WhileKind::DoWhile,
                    condition: condition_id,
                    body: body_id,
                },
                self.get_span_from(start),
            );
            Ok(while_id)
        }
        // while loop
        else {
            // while keyword
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition_id = self
                .with_options(self.options.not_in_position().in_before_block(), |parser| {
                    parser.eat_expression()
                })?;

            // body
            let body_id = self.eat_block()?;

            // while
            let while_id = self.tree.insert(
                Expression::While {
                    kind: WhileKind::While,
                    condition: condition_id,
                    body: body_id,
                },
                self.get_span_from(start),
            );
            Ok(while_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Asynchrony, BinaryOperator, Block, Expression, ForEachKind, Mutability, Pattern,
        ScalarLiteral, UnaryOperator, WhileKind,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_loop() {
        let mut test = TestParser::new(
            r###"
loop {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let loop_id = parser.eat_loop().unwrap();
        assert_node!(parser.tree, loop_id, Expression::Loop { body, .. } => {
            let _block = parser.tree.get(*body);
        });
    }

    #[test]
    fn test_parse_for_loop() {
        let mut test = TestParser::new(
            r###"
for item in items {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { pattern, iterator, body: _, .. } => {
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: None, name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
            // items
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_each_loop_in_parentheses() {
        let mut test = TestParser::new(
            r###"
for (item in items) {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, pattern, iterator, body: _, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Sync);
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: None, name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
            // items
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_with_async_in_parentheses() {
        let mut test = TestParser::new(
            r###"
for await (item of items) {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, kind, pattern, iterator, body: _, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert_eq!(*kind, ForEachKind::Of);
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: None, name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
            // items
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_with_label() {
        let mut test = TestParser::new(
            r###"
for const item in items outer: {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { pattern, iterator, body: _, .. } => {
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: Some(mutability), name, pattern: None } => {
                assert_eq!(*mutability, Mutability::Immutable);
                assert_string!(parser, *name, "item");
            });
            // in items
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_condition_empty() {
        let mut test = TestParser::new(
            r###"
for (;;) {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body: _, .. } => {
            assert!(initialization.is_none());
            assert!(condition.is_none());
            assert!(increment.is_none());
        });
    }

    #[test]
    fn test_parse_for_loop_condition_with_initialization() {
        let mut test = TestParser::new(
            r###"
for (var x = 0; x < 10; x++) {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body: _, .. } => {
            // var x = 0
            assert_node!(parser.tree, initialization.unwrap(), Expression::Let { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: None, name, pattern: None } => {
                    assert_string!(parser, *name, "x");
                });
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
            // x < 10
            assert_node!(parser.tree, condition.unwrap(), Expression::Binary { left, operator, right } => {
                // x
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                // <
                assert_eq!(*operator, BinaryOperator::LessThan);
                // 10
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(10)));
            });
            // x++
            assert_node!(parser.tree, increment.unwrap(), Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::PostIncrement);
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, path, "x");
                });
            });
        });
    }

    #[test]
    fn test_parse_while_loop() {
        let mut test = TestParser::new(
            r###"
while x {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, while_id, Expression::While { condition, body: _, .. } => {
            assert_expression_path!(parser, parser.tree.get(*condition), "x");
        });
    }

    #[test]
    fn test_parse_while_loop_nested() {
        let mut test = TestParser::new(
            r###"
while x > y {
    while a < b {
        inner_work()
    }
    outer_work()
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let while_id = parser.eat_while().unwrap();

        // while x > y
        assert_node!(parser.tree, while_id, Expression::While { condition, body, .. } => {
            // x > y
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                // x
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                // >
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                // y
                assert_expression_path!(parser, parser.tree.get(*right), "y");
            });

            assert_node!(parser.tree, *body, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 2);

                // while a < b
                assert_node!(parser.tree, expressions[0], Expression::While { condition: nested_condition, body: _, .. } => {
                    // a < b
                    assert_node!(parser.tree, *nested_condition, Expression::Binary { left, operator, right } => {
                        // a
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        // <
                        assert_eq!(*operator, BinaryOperator::LessThan);
                        // b
                        assert_expression_path!(parser, parser.tree.get(*right), "b");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_do_while_loop() {
        let mut test = TestParser::new(
            r###"
do { x } while true
"###,
        );

        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // do { x } while true
        let do_while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, do_while_id, Expression::While { kind, condition, body: _, .. } => {
            assert_eq!(*kind, WhileKind::DoWhile);
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        });
    }
}
