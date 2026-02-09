//! Parse loops, for, while, etc.

use destack_ast::{
    Asynchrony, Expression, ForEachBinding, ForEachDeclarationKind, ForEachKind, Keyword,
    LocalNodeId, Pattern, TokenType, WhileKind,
};

use crate::{ParseError, ParseResult, Parser};

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
            self.get_span_from(&start),
        );
        Ok(loop_id)
    }

    /// Eat a for each or for condition loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// for (const item in items) {
    ///     item
    /// }
    ///
    /// for (const x in 1..10) {
    ///     y = 2
    /// }
    ///
    /// for (const x in zeds.iter()) a: {
    ///     if y > 5 {
    ///         continue :a
    ///     }
    ///     y = 2
    /// }
    ///
    /// for (let x = 0; x < 10; x++) {
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

        let in_parenthesis = self.peek_is(TokenType::OpenParenthesis);

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
            // C style for clauses always allow comma operator expressions
            let mut clause_options = self.options.nested();
            clause_options.allow_sequence_expression = true;

            // open parenthesis
            self.bump();
            self.eat_newlines_maybe()?;

            // initialization
            let initialization_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.with_options(clause_options, |parser| parser.eat_expression())?)
            };
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::Semicolon)?;
            self.eat_newlines_maybe()?;

            // condition
            let condition_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.with_options(clause_options, |parser| parser.eat_expression())?)
            };
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::Semicolon)?;
            self.eat_newlines_maybe()?;

            // increment
            let increment_id = if self.peek_is(TokenType::CloseParenthesis) {
                None
            } else {
                Some(self.with_options(clause_options, |parser| parser.eat_expression())?)
            };

            // close parenthesis
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;

            // body
            let body_id = self.eat_block_or_statement()?;

            // for
            let for_id = self.tree.insert(
                Expression::For {
                    initialization: initialization_id,
                    condition: condition_id,
                    increment: increment_id,
                    body: body_id,
                },
                self.get_span_from(&start),
            );
            Ok(for_id)
        }
        // explicit pattern for loop
        else {
            if in_parenthesis {
                // open parenthesis
                self.bump();
            }

            // binding
            let binding = self.eat_for_each_binding()?;

            // in
            let kind = match self.eat_keyword_in(&[Keyword::In, Keyword::Of])? {
                Keyword::In => ForEachKind::In,
                Keyword::Of => ForEachKind::Of,
                _ => unreachable!(),
            };

            // reject using bindings in for-in loops for js and ts
            // destack allows this, see SPECIFICATION.md
            if !self.language.is_destack()
                && kind == ForEachKind::In
                && matches!(binding, ForEachBinding::Using { .. })
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

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
            let body_id = self.eat_block_or_statement()?;

            // for
            let for_id = self.tree.insert(
                Expression::ForEach {
                    asynchrony,
                    kind,
                    binding,
                    iterator: iterator_id,
                    body: body_id,
                },
                self.get_span_from(&start),
            );
            Ok(for_id)
        }
    }

    /// Eat a for each binding (pattern or using).
    fn eat_for_each_binding(&mut self) -> ParseResult<ForEachBinding> {
        let using_asynchrony = if self.peek_keyword(Keyword::Await).is_ok()
            && self.peek_next_keyword(Keyword::Using).is_ok()
        {
            self.bump(); // eat await
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };

        if self.peek_keyword(Keyword::Using).is_ok() {
            self.bump(); // eat using
            let pattern = self.with_options(
                self.options
                    .not_in_position()
                    .in_for_each()
                    .in_before_block(),
                |parser| parser.eat_pattern(),
            )?;
            Ok(ForEachBinding::Using {
                asynchrony: using_asynchrony,
                pattern,
            })
        } else {
            let declaration_kind = self.peek_for_each_declaration_kind();

            if declaration_kind.is_none() {
                // for each without declarations keeps expression heads as expression patterns
                let start = self.mark();
                let expression = self.with_options(
                    self.options
                        .not_in_position()
                        .in_for_each()
                        .in_before_block(),
                    |parser| parser.eat_expression(),
                )?;

                let pattern = self.tree.insert(
                    Pattern::Expression { value: expression },
                    self.get_span_from(&start),
                );
                Ok(ForEachBinding::Pattern {
                    pattern,
                    declaration_kind: None,
                })
            } else {
                // declaration forms keep binding-pattern parsing
                let pattern = self.with_options(
                    self.options
                        .not_in_position()
                        .in_for_each()
                        .in_before_block(),
                    |parser| parser.eat_pattern(),
                )?;
                Ok(ForEachBinding::Pattern {
                    pattern,
                    declaration_kind,
                })
            }
        }
    }

    /// Return the declaration keyword for a for each pattern binding.
    fn peek_for_each_declaration_kind(&mut self) -> Option<ForEachDeclarationKind> {
        let keyword = self.peek_any_keyword().ok()?;
        match keyword {
            Keyword::Var => Some(ForEachDeclarationKind::Var),
            Keyword::Let => Some(ForEachDeclarationKind::Let),
            Keyword::Const | Keyword::Readonly => Some(ForEachDeclarationKind::Const),
            _ => None,
        }
    }

    /// Eat a while or do-while loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// while (x > 1) {
    ///     y = 2
    /// }
    ///
    /// while (y < 10) l: {
    ///     y = 2
    ///     break :l
    /// }
    ///
    /// do {
    ///     y = 2
    /// } while (x > 1)
    ///
    /// do console.log("test"); while (true)
    /// ```
    pub fn eat_while(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // do-while loop
        if self.peek_keyword(Keyword::Do).is_ok() {
            // do keyword
            self.bump(); // eat do keyword

            // body
            self.eat_newlines_maybe()?;
            let body_id = self.eat_block_or_statement()?;

            // while keyword
            self.eat_newlines_maybe()?;
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition_id = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression_parenthesized_maybe()
            })?;

            // while
            let while_id = self.tree.insert(
                Expression::While {
                    kind: WhileKind::DoWhile,
                    condition: condition_id,
                    body: body_id,
                },
                self.get_span_from(&start),
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
                    parser.eat_expression_parenthesized_maybe()
                })?;

            // body
            let body_id = self.eat_block_or_statement()?;

            // while
            let while_id = self.tree.insert(
                Expression::While {
                    kind: WhileKind::While,
                    condition: condition_id,
                    body: body_id,
                },
                self.get_span_from(&start),
            );
            Ok(while_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Asynchrony, BinaryOperator, Block, Declarator, Expression, ForEachBinding,
        ForEachDeclarationKind, ForEachKind, Mutability, Pattern, ScalarLiteral, UnaryOperator,
        WhileKind,
    };
    use destack_source::LanguageType;

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
for (const item in items) {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, body: _, .. } => {
            assert_eq!(*declaration_kind, Some(ForEachDeclarationKind::Const));
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: Some(mutability), name, pattern: None } => {
                assert_eq!(*mutability, Mutability::Immutable);
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
for (const item in items) {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, body: _, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Sync);
            assert_eq!(*declaration_kind, Some(ForEachDeclarationKind::Const));
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: Some(mutability), name, pattern: None } => {
                assert_eq!(*mutability, Mutability::Immutable);
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
for await (const item of items) {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, kind, binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, body: _, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert_eq!(*kind, ForEachKind::Of);
            assert_eq!(*declaration_kind, Some(ForEachDeclarationKind::Const));
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: Some(mutability), name, pattern: None } => {
                assert_eq!(*mutability, Mutability::Immutable);
                assert_string!(parser, *name, "item");
            });
            // items
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_with_inline_if_body() {
        let mut test = TestParser::new(
            r###"
for (var r in t)
    if (r !== "default" && !Object.prototype.hasOwnProperty.call(e, r)) i(e, t, r)
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, body, .. } => {
            assert_eq!(*declaration_kind, Some(ForEachDeclarationKind::Var));
            // r
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: Some(mutability), name, pattern: None } => {
                assert_eq!(*mutability, Mutability::Mutable);
                assert_string!(parser, *name, "r");
            });
            // t
            assert_expression_path!(parser, parser.tree.get(*iterator), "t");
            // body
            assert_node!(parser.tree, *body, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                assert_node!(parser.tree, expressions[0], Expression::If { .. } => {});
            });
        });
    }

    #[test]
    fn test_parse_for_loop_with_label() {
        let mut test = TestParser::new(
            r###"
for (const item in items) outer: {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, body: _, .. } => {
            assert_eq!(*declaration_kind, Some(ForEachDeclarationKind::Const));
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
    fn test_parse_for_loop_with_using_binding() {
        let mut test = TestParser::new(
            r###"
for (using item of items) {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Using { asynchrony, pattern }, iterator, body: _, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Sync);
            assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: None, name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_in_with_member_expression_binding() {
        // for (a[b in c] in d);
        let mut test =
            TestParser::new_with_options("for (a[b in c] in d);", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { kind, binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, .. } => {
            assert_eq!(*kind, ForEachKind::In);
            assert_eq!(*declaration_kind, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Index { .. });
            });
            assert_expression_path!(parser, parser.tree.get(*iterator), "d");
        });
    }

    #[test]
    fn test_parse_for_in_with_call_expression_binding() {
        // for (a(b in c)[1] in d);
        let mut test =
            TestParser::new_with_options("for (a(b in c)[1] in d);", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { kind, binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, .. } => {
            assert_eq!(*kind, ForEachKind::In);
            assert_eq!(*declaration_kind, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Index { .. });
            });
            assert_expression_path!(parser, parser.tree.get(*iterator), "d");
        });
    }

    #[test]
    fn test_parse_for_in_with_array_expression_binding() {
        // for ([a, b[a], {c, d = e, [f]: [g, h().a, (1).i, ...j[2]]}] in 3);
        let mut test = TestParser::new_with_options(
            "for ([a, b[a], {c, d = e, [f]: [g, h().a, (1).i, ...j[2]]}] in 3);",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { kind, binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, .. } => {
            assert_eq!(*kind, ForEachKind::In);
            assert_eq!(*declaration_kind, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { .. });
            });
            assert_node!(parser.tree, *iterator, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        });
    }

    #[test]
    fn test_parse_for_in_with_unary_binding_expression() {
        // source: for (+i in {});
        let mut test = TestParser::new_with_options("for (+i in {});", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { kind, binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, .. } => {
            assert_eq!(*kind, ForEachKind::In);
            assert_eq!(*declaration_kind, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Unary { .. });
            });
            assert_node!(parser.tree, *iterator, Expression::ObjectExpression { .. });
        });
    }

    #[test]
    fn test_parse_for_in_with_binary_binding_expression() {
        // source: for (i + 1 in {});
        let mut test = TestParser::new_with_options("for (i + 1 in {});", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { kind, binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, .. } => {
            assert_eq!(*kind, ForEachKind::In);
            assert_eq!(*declaration_kind, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Binary { .. });
            });
            assert_node!(parser.tree, *iterator, Expression::ObjectExpression { .. });
        });
    }

    #[test]
    fn test_parse_for_in_with_parenthesized_binary_binding_expression() {
        // source: for((1 + 1) in list) process(x);
        let mut test = TestParser::new_with_options(
            "for((1 + 1) in list) process(x);",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { kind, binding: ForEachBinding::Pattern { pattern, declaration_kind }, iterator, body, .. } => {
            assert_eq!(*kind, ForEachKind::In);
            assert_eq!(*declaration_kind, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Binary { .. });
                });
            });
            assert_expression_path!(parser, parser.tree.get(*iterator), "list");
            assert_node!(parser.tree, *body, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                assert_node!(parser.tree, expressions[0], Expression::Call { .. });
            });
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
            assert_node!(parser.tree, initialization.unwrap(), Expression::Let { declarators, .. } => {
                assert_eq!(declarators.len(), 1);
                assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: None, name, pattern: None } => {
                        assert_string!(parser, *name, "x");
                    });
                    assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                });
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

    /// Parse multiline C style for loop headers in JavaScript.
    #[test]
    fn test_parse_for_loop_condition_multiline_header() {
        let mut test = TestParser::new_with_options(
            r###"
for (
  start = 0, end = Math.min(len, newLen);
  start < end && items[start] === newItems[start];
  start++
) {
  work();
}
"###,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body } => {
            assert!(initialization.is_some());
            assert!(condition.is_some());
            assert!(increment.is_some());
            assert_node!(parser.tree, *body, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    /// Parse C style for headers with comma operator in init and increment.
    #[test]
    fn test_parse_for_loop_condition_with_sequence_clauses() {
        let mut test = TestParser::new_with_options(
            r###"
for (start = 0, end = 10; start < end; start++, end--) {}
"###,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, .. } => {
            assert_node!(parser.tree, initialization.expect("expected initialization"), Expression::SequenceExpression { expressions } => {
                assert_eq!(expressions.len(), 2);
            });
            assert_node!(parser.tree, condition.expect("expected condition"), Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
            });
            assert_node!(parser.tree, increment.expect("expected increment"), Expression::SequenceExpression { expressions } => {
                assert_eq!(expressions.len(), 2);
            });
        });
    }

    #[test]
    fn test_parse_while_loop() {
        let mut test = TestParser::new(
            r###"
while (x) {}
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
while (x > y) {
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
do { x } while (true)
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

    /// JavaScript allows single statement body without braces.
    #[test]
    fn test_parse_do_while_single_statement() {
        let mut test = TestParser::new("do x; while (true)");

        let mut parser = test.prepare();

        let do_while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, do_while_id, Expression::While { kind, condition, body } => {
            assert_eq!(*kind, WhileKind::DoWhile);
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // body should be a block with single expression
            assert_node!(parser.tree, *body, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    #[test]
    fn test_parse_do_while_continue_statement() {
        let mut test = TestParser::new("do continue; while (true)");

        let mut parser = test.prepare();

        let do_while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, do_while_id, Expression::While { kind, condition, body } => {
            assert_eq!(*kind, WhileKind::DoWhile);
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // body should be a block with continue statement
            assert_node!(parser.tree, *body, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                assert_node!(parser.tree, expressions[0], Expression::Continue { label: None });
            });
        });
    }
}
