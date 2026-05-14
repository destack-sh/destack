//! Parse loops, for, while, etc.

use destack_dir::{
    Asynchrony, BindingKeyword, BlockContext, Expression, ForEachBinding, ForEachOperator, Keyword,
    LocalNodeId, NodeType, Pattern, TokenType, WhileForm,
};

use crate::{ParseError, ParseResult, Parser};

impl Parser {
    /// Eat a loop (e.g., `loop { ... }`).
    ///
    /// Examples:
    /// ```
    /// loop {
    ///     y = getNext()
    ///     if (y < 0) {
    ///         break
    ///     }
    /// }
    /// ```
    pub fn eat_loop(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Loop)?;

        // body
        let body_id = self.eat_block(BlockContext::Statement)?;

        // loop
        let loop_id = self.insert_node(
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
    /// for (const x in zeds.iter()) a: {
    ///     if (y > 5) {
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
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::For)?;

        // asynchrony
        let asynchrony = if self.is_keyword(Keyword::Await) {
            self.bump(); // eat await keyword
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };

        self.eat_token(TokenType::OpenParenthesis)?;

        let close_span = self.find_matching_close_after_open_maybe(
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        );
        let has_top_level_semicolon = if let Some(close_span) = close_span {
            self.has_token_before_matching_close_after_open(
                close_span,
                TokenType::Semicolon,
                false,
            )?
        } else {
            false
        };

        // for condition loop
        if asynchrony == Asynchrony::Sync && has_top_level_semicolon {
            // c style for clauses always allow comma operator expressions
            let mut clause_flags = self.flags.nested();
            clause_flags.set_allow_sequence_expression(true);

            // initialization
            let initialization_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.eat_expression(clause_flags)?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // condition
            let condition_id = if self.peek_is(TokenType::Semicolon) {
                None
            } else {
                Some(self.eat_expression(clause_flags)?)
            };
            self.eat_token(TokenType::Semicolon)?;

            // increment
            let increment_id = if self.peek_is(TokenType::CloseParenthesis) {
                None
            } else {
                Some(self.eat_expression(clause_flags)?)
            };

            // close parenthesis
            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseParenthesis,
                NodeType::Expression,
                |parser, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type) || parser.is_block_start()
                },
            )?;

            // body
            let body_id = self.eat_block_or_statement()?;

            // for
            let for_id = self.insert_node(
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
            // binding
            let binding = self.eat_for_each_binding()?;

            // in
            let operator = match self.eat_keyword_in(&[Keyword::In, Keyword::Of])? {
                Keyword::In => ForEachOperator::In,
                Keyword::Of => ForEachOperator::Of,
                _ => unreachable!(),
            };

            // reject using bindings in semicolon statement `for ... in`
            // block-value mode allows them
            if !self.language.is_destack()
                && operator == ForEachOperator::In
                && matches!(binding, ForEachBinding::Using { .. })
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // iterator
            let iterator_flags = self.flags.nested().in_before_block();
            let iterator_id =
                self.with_flags(iterator_flags, |parser| parser.eat_expression(parser.flags))?;

            // close parenthesis
            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseParenthesis,
                NodeType::Expression,
                |parser, token_type| {
                    Self::is_close_delimiter_boundary_token(token_type) || parser.is_block_start()
                },
            )?;

            // body
            let body_id = self.eat_block_or_statement()?;

            // for
            let for_id = self.insert_node(
                Expression::ForEach {
                    asynchrony,
                    operator,
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
        let using_asynchrony = self.for_each_using_binding_asynchrony();

        if let Some(using_asynchrony) = using_asynchrony {
            if using_asynchrony == Asynchrony::Async {
                self.bump(); // eat await
            }

            self.bump(); // eat using
            let pattern_flags = self.flags.not_in_position().in_for_each();
            let pattern = self.with_flags(pattern_flags, |parser| parser.eat_pattern())?;
            Ok(ForEachBinding::Using {
                asynchrony: using_asynchrony,
                pattern,
            })
        } else {
            let keyword = self.peek_for_each_keyword();

            if keyword.is_none() {
                // for each without declarations keeps expression heads as expression patterns
                let start = self.span_start();
                let expression = self
                    .eat_expression(self.flags.not_in_position().in_for_each().in_before_block())?;

                let pattern = self.insert_node(
                    Pattern::Expression { value: expression },
                    self.get_span_from(&start),
                );
                Ok(ForEachBinding::Pattern {
                    pattern,
                    keyword: None,
                })
            } else {
                self.bump();

                // declaration forms keep binding-pattern parsing
                let pattern_flags = self.flags.not_in_position().in_for_each();
                let pattern = self.with_flags(pattern_flags, |parser| parser.eat_pattern())?;
                Ok(ForEachBinding::Pattern { pattern, keyword })
            }
        }
    }

    /// Return the using asynchrony when a for each header starts a using binding.
    fn for_each_using_binding_asynchrony(&mut self) -> Option<Asynchrony> {
        // resolve `using` with optional `await` prefix
        let asynchrony = if self.using_keyword_is(Asynchrony::Async) {
            Asynchrony::Async
        } else if self.using_keyword_is(Asynchrony::Sync) {
            Asynchrony::Sync
        } else {
            return None;
        };

        // keep the binding head on the same line
        let declarator_token_type = self.using_binding_head_token(asynchrony)?;

        // using bindings start with a binding pattern shape
        if !self.token_can_start_using_binding_pattern(declarator_token_type) {
            return None;
        }

        // disambiguate identifier starts that should remain expression headers
        if declarator_token_type == TokenType::Identifier {
            let declarator_keyword = self.lookahead(|parser| {
                if asynchrony == Asynchrony::Async {
                    parser.bump();
                }
                parser.bump();
                parser.current_keyword()
            });

            // `for (using in ...)` should parse as identifier `using`
            if declarator_keyword == Some(Keyword::In) {
                return None;
            }

            // `for (using of of)` should parse as identifier `using` in semicolon statement forms
            if declarator_keyword == Some(Keyword::Of) && asynchrony == Asynchrony::Sync {
                return None;
            }
        }

        Some(asynchrony)
    }
    /// Return the declaration keyword for a for each pattern binding.
    fn peek_for_each_keyword(&mut self) -> Option<BindingKeyword> {
        let keyword = self.peek_any_keyword().ok()?;
        match keyword {
            Keyword::Let => Some(BindingKeyword::Let),
            Keyword::Const | Keyword::Readonly => Some(BindingKeyword::Const),
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
        let start = self.span_start();

        // do-while loop
        if self.is_keyword(Keyword::Do) {
            // do keyword
            self.bump(); // eat do keyword

            // body
            let body_id = self.eat_block_or_statement()?;

            // while keyword
            self.eat_keyword(Keyword::While)?;

            // condition
            let condition_flags = self.flags.not_in_position();
            let condition_id = self.with_flags(condition_flags, |parser| {
                parser.eat_parenthesized_expression()
            })?;

            // while
            let while_id = self.insert_node(
                Expression::While {
                    form: WhileForm::DoWhile,
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
            let condition_flags = self.flags.not_in_position().in_before_block();
            let condition_id = self.with_flags(condition_flags, |parser| {
                parser.eat_parenthesized_expression()
            })?;

            // body
            let body_id = self.eat_block_or_statement()?;

            // while
            let while_id = self.insert_node(
                Expression::While {
                    form: WhileForm::While,
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
    use destack_dir::{
        Argument, Asynchrony, BinaryOperator, BindingKeyword, Block, Declarator, Expression,
        ForEachBinding, ForEachOperator, GenericArgument, Key, Keyword, Name, Pattern,
        PatternField, ScalarLiteral, TokenType, TypeExpression, TypeLiteral, TypeMember,
        UnaryOperator, WhileForm,
    };
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
        block_expression_ids,
    };

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

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body: _, .. } => {
            assert_eq!(*keyword, Some(BindingKeyword::Const));
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
            // items
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_with_missing_close_parenthesis() {
        let mut test = TestParser::new("for (item in items { body }");
        let mut parser = test.prepare();
        let for_id = parser.eat_for().unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body, .. } => {
            assert_eq!(*keyword, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "item");
            });
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
            let _body = parser.tree.get(*body);
        });
    }

    #[test]
    fn test_parse_for_loop_with_line_comment_after_keyword() {
        let mut test =
            TestParser::new_with_language("for // comment\n(;;);", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body } => {
            assert!(initialization.is_none());
            assert!(condition.is_none());
            assert!(increment.is_none());
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert!(expressions.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_for_loop_with_block_comment_after_keyword() {
        let mut test =
            TestParser::new_with_language("for /* comment */(;;);", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body } => {
            assert!(initialization.is_none());
            assert!(condition.is_none());
            assert!(increment.is_none());
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert!(expressions.is_empty());
            });
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

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body: _, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Sync);
            assert_eq!(*keyword, Some(BindingKeyword::Const));
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
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

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, operator, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body: _, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert_eq!(*operator, ForEachOperator::Of);
            assert_eq!(*keyword, Some(BindingKeyword::Const));
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
            // items
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    /// Parse for of headers with multiline destructured bindings.
    #[test]
    fn test_parse_for_of_multiline_destructured_binding() {
        let mut test = TestParser::new_with_language(
            r###"
for (
  const {
    relation,
  }
  of selectedRelations
) {}
"###,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        // for (const { ... } of selectedRelations) {}
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, operator, binding, iterator, body } => {
            assert_eq!(*asynchrony, Asynchrony::Sync);
            assert_eq!(*operator, ForEachOperator::Of);

            // const { relation }
            assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
                assert_eq!(*keyword, Some(BindingKeyword::Const));
                assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                    assert_eq!(fields.len(), 1);
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None, .. } => {
                        assert_name!(parser, *name, "relation");
                    });
                });
            });

            // selectedRelations
            assert_expression_path!(parser, parser.tree.get(*iterator), "selectedRelations");
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert!(expressions.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_for_of_await_generic_call_with_object_type_argument() {
        let mut test = TestParser::new_with_language(
            r###"
for (const { item } of await fetchList<{ item: string }>(values)) {}
"###,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, operator, binding, iterator, body } => {
            assert_eq!(*asynchrony, Asynchrony::Sync);
            assert_eq!(*operator, ForEachOperator::Of);

            // const { item }
            assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
                assert_eq!(*keyword, Some(BindingKeyword::Const));
                assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                    assert_eq!(fields.len(), 1);
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None, .. } => {
                        assert_name!(parser, *name, "item");
                    });
                });
            });

            // await fetchList<{ item: string }>(values)
            assert_node!(parser.tree, *iterator, Expression::Await { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, generic_arguments, arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "fetchList");
                    assert_eq!(arguments.len(), 1);
                    assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "values");
                    });

                    assert_eq!(generic_arguments.len(), 1);

                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                                assert_eq!(properties.len(), 1);
                                assert_node!(parser.tree, properties[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type: value, .. } => {
                                    assert_string!(parser, *name, "item");
                                    assert_node!(parser.tree, value.expect("expected declared type"), TypeExpression::Literal { value } => {
                                        assert_eq!(*value, TypeLiteral::String);
                                    });
                                });
                            });
                    });
                });
            });

            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert!(expressions.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_for_each_binding_const_object_stops_before_of() {
        let mut test = TestParser::new_with_language(
            r###"
for (const { item } of fetchList<{ item: string }>(values)) {}
"###,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        parser.eat_keyword(Keyword::For).unwrap();
        parser.eat_token(TokenType::OpenParenthesis).unwrap();

        let binding = parser.eat_for_each_binding().unwrap();
        assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
            assert_eq!(keyword, Some(BindingKeyword::Const));
            assert_node!(parser.tree, pattern, Pattern::Object { fields } => {
                assert_eq!(fields.len(), 1);
            });
        });

        assert!(parser.peek_keyword(Keyword::Of).is_ok());
    }

    #[test]
    fn test_parse_for_of_tagged_object_binding() {
        let mut test = TestParser::new(
            r###"
for (const Shape.Line { start: Point { x, y }, end } of lines) {}
"###,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();

        assert!(parser.errors.is_empty());

        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding, iterator, body, .. } => {
            assert_eq!(*operator, ForEachOperator::Of);

            assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
                assert_eq!(*keyword, Some(BindingKeyword::Const));
                assert_node!(parser.tree, *pattern, Pattern::TaggedObject { ty, fields } => {
                    assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                        assert_path!(parser, *path, "Shape.Line");
                    });
                    assert_eq!(fields.len(), 2);

                    assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), .. } => {
                        assert_name!(parser, *name, "start");
                        assert_node!(parser.tree, *pattern, Pattern::TaggedObject { ty, fields } => {
                            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                                assert_path!(parser, *path, "Point");
                            });
                            assert_eq!(fields.len(), 2);
                        });
                    });

                    assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                        assert_name!(parser, *name, "end");
                    });
                });
            });

            assert_expression_path!(parser, parser.tree.get(*iterator), "lines");
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert!(expressions.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_for_loop_with_inline_if_body() {
        let mut test = TestParser::new(
            r###"
for (let r in t)
    if (r !== "default" && !Object.prototype.hasOwnProperty.call(e, r)) i(e, t, r)
"###,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body, .. } => {
            assert_eq!(*keyword, Some(BindingKeyword::Let));
            // r
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "r");
            });
            // t
            assert_expression_path!(parser, parser.tree.get(*iterator), "t");
            // body
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
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

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body: _, .. } => {
            assert_eq!(*keyword, Some(BindingKeyword::Const));
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
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

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Using { asynchrony, pattern }, iterator, body: _, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Sync);
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_with_using_identifier_member_binding_in() {
        let mut test =
            TestParser::new_with_language("for (using().foo in items);", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding, iterator, .. } => {
            assert_eq!(*operator, ForEachOperator::In);

            assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
                assert!(keyword.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                        assert_string!(parser, *name, "foo");
                        assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
                            assert!(arguments.is_empty());
                            assert_expression_path!(parser, parser.tree.get(*left), "using");
                        });
                    });
                });
            });

            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_with_using_identifier_member_binding_of() {
        let mut test =
            TestParser::new_with_language("for (using().foo of items);", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding, iterator, .. } => {
            assert_eq!(*operator, ForEachOperator::Of);

            assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
                assert!(keyword.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                        assert_string!(parser, *name, "foo");
                        assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
                            assert!(arguments.is_empty());
                            assert_expression_path!(parser, parser.tree.get(*left), "using");
                        });
                    });
                });
            });

            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_with_await_using_of_binding() {
        let mut test = TestParser::new_with_language(
            "for await (await using of of items);",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, operator, binding, iterator, .. } => {
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert_eq!(*operator, ForEachOperator::Of);

            assert_node!(binding, ForEachBinding::Using { asynchrony, pattern } => {
                assert_eq!(*asynchrony, Asynchrony::Async);
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "of");
                });
            });

            assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_in_with_member_expression_binding() {
        // for (a[b in c] in d);
        let mut test =
            TestParser::new_with_language("for (a[b in c] in d);", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, .. } => {
            assert_eq!(*operator, ForEachOperator::In);
            assert_eq!(*keyword, None);
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
            TestParser::new_with_language("for (a(b in c)[1] in d);", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, .. } => {
            assert_eq!(*operator, ForEachOperator::In);
            assert_eq!(*keyword, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Index { .. });
            });
            assert_expression_path!(parser, parser.tree.get(*iterator), "d");
        });
    }

    #[test]
    fn test_parse_for_in_with_array_expression_binding() {
        // for ([a, b[a], {c, d = e, [f]: [g, h().a, (1).i, ...j[2]]}] in 3);
        let mut test = TestParser::new_with_language(
            "for ([a, b[a], {c, d = e, [f]: [g, h().a, (1).i, ...j[2]]}] in 3);",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, .. } => {
            assert_eq!(*operator, ForEachOperator::In);
            assert_eq!(*keyword, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ArrayExpression { .. });
            });
            assert_node!(parser.tree, *iterator, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        });
    }

    #[test]
    fn test_parse_for_in_with_unary_binding_expression() {
        // source: for (+i in {});
        let mut test = TestParser::new_with_language("for (+i in {});", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, .. } => {
            assert_eq!(*operator, ForEachOperator::In);
            assert_eq!(*keyword, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Unary { .. });
            });
            assert_node!(parser.tree, *iterator, Expression::ObjectExpression { .. });
        });
    }

    #[test]
    fn test_parse_for_in_with_binary_binding_expression() {
        // source: for (i + 1 in {});
        let mut test =
            TestParser::new_with_language("for (i + 1 in {});", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, .. } => {
            assert_eq!(*operator, ForEachOperator::In);
            assert_eq!(*keyword, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Binary { .. });
            });
            assert_node!(parser.tree, *iterator, Expression::ObjectExpression { .. });
        });
    }

    #[test]
    fn test_parse_for_in_with_parenthesized_binary_binding_expression() {
        // source: for((1 + 1) in list) process(x);
        let mut test = TestParser::new_with_language(
            "for((1 + 1) in list) process(x);",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { operator, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body, .. } => {
            assert_eq!(*operator, ForEachOperator::In);
            assert_eq!(*keyword, None);
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Binary { .. });
                });
            });
            assert_expression_path!(parser, parser.tree.get(*iterator), "list");
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
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
for (let x = 0; x < 10; x++) {
    x
}
"###,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body: _, .. } => {
            // let x = 0
            assert_node!(parser.tree, initialization.unwrap(), Expression::Let { declarators, .. } => {
                assert_eq!(declarators.len(), 1);
                assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
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
                assert_node!(parser.tree, *right, Expression::Identifier { name } => {
                    assert_string!(parser, *name, "x");
                });
            });
        });
    }

    /// Parse multiline C style for loop headers in JavaScript.
    #[test]
    fn test_parse_for_loop_condition_multiline_header() {
        let mut test = TestParser::new_with_language(
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

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body } => {
            assert!(initialization.is_some());
            assert!(condition.is_some());
            assert!(increment.is_some());
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    /// Parse C style for headers with comma operator in init and increment.
    #[test]
    fn test_parse_for_loop_condition_with_sequence_clauses() {
        let mut test = TestParser::new_with_language(
            r###"
for (start = 0, end = 10; start < end; start++, end--) {}
"###,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

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

    /// Parse JavaScript for of loops with `type` as an identifier binding.
    #[test]
    fn test_parse_for_of_with_type_identifier_binding() {
        let mut test = TestParser::new_with_language(
            r###"
for (type of values) {}
"###,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let for_id = parser.eat_for().unwrap();
        assert_node!(parser.tree, for_id, Expression::ForEach { binding, operator, iterator, .. } => {
            // for of
            assert_eq!(*operator, ForEachOperator::Of);

            // binding
            assert!(matches!(binding, ForEachBinding::Pattern { .. }));
            let ForEachBinding::Pattern { pattern, keyword } = binding else {
                panic!("expected for each pattern binding");
            };
            assert!(keyword.is_none());
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "type");
            });

            // iterator
            assert_expression_path!(parser, parser.tree.get(*iterator), "values");
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
    while (a < b) {
        inner_work()
    }
    outer_work()
}
"###,
        );
        let mut parser = test.prepare();

        let while_id = parser.eat_while().unwrap();

        // while (x > y)
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

            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
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
    fn test_parse_while_parenthesized_condition_keeps_inner_span() {
        let mut test = TestParser::new("while (x > y) {}");
        let mut parser = test.prepare();

        let while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, while_id, Expression::While { condition, .. } => {
            assert_node!(parser.tree, *condition, Expression::Binary { .. });

            let condition_span = parser.tree.get_span(*condition);
            let condition_text = &parser.file.text()
                [condition_span.start as usize..condition_span.end as usize];
            assert_eq!(condition_text, "x > y");
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

        // do { x } while true
        let do_while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, do_while_id, Expression::While { form, condition, body: _, .. } => {
            assert_eq!(*form, WhileForm::DoWhile);
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        });
    }

    #[test]
    fn test_parse_do_while_parenthesized_condition_keeps_inner_span() {
        let mut test = TestParser::new("do x; while (value)");
        let mut parser = test.prepare();

        let while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, while_id, Expression::While { form, condition, .. } => {
            assert_eq!(*form, WhileForm::DoWhile);
            assert_expression_path!(parser, parser.tree.get(*condition), "value");

            let condition_span = parser.tree.get_span(*condition);
            let condition_text = &parser.file.text()
                [condition_span.start as usize..condition_span.end as usize];
            assert_eq!(condition_text, "value");
        });
    }

    /// JavaScript allows single statement body without braces.
    #[test]
    fn test_parse_do_while_single_statement() {
        let mut test = TestParser::new("do x; while (true)");

        let mut parser = test.prepare();

        let do_while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, do_while_id, Expression::While { form, condition, body } => {
            assert_eq!(*form, WhileForm::DoWhile);
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // body should be a block with single expression
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    #[test]
    fn test_parse_do_while_continue_statement() {
        let mut test = TestParser::new("do continue; while (true)");

        let mut parser = test.prepare();

        let do_while_id = parser.eat_while().unwrap();
        assert_node!(parser.tree, do_while_id, Expression::While { form, condition, body } => {
            assert_eq!(*form, WhileForm::DoWhile);
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // body should be a block with continue statement
            assert_node!(parser.tree, *body, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*body));
                assert_eq!(expressions.len(), 1);
                assert_node!(parser.tree, expressions[0], Expression::Continue { label: None });
            });
        });
    }
}
