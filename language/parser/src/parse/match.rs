use crate::parse::parser::ParserFlags;
use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use destack_dir::{
    Block, BlockContext, BlockForm, Expression, Keyword, LocalNodeId, MatchCase, MatchForm,
    MatchSelector, NodeType, Pattern, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

impl Parser {
    /// Return parser contexts for a match value expression.
    #[inline]
    fn match_value_contexts(&self) -> (ParserFlags, ParserFlags) {
        let ambient_context = self.flags.with_before_block(true);
        let expression_context = self.flags;
        (ambient_context, expression_context)
    }

    /// Return parser contexts for a match-case pattern.
    #[inline]
    fn match_pattern_contexts(&self) -> (ParserFlags, ParserFlags) {
        let ambient_context = self.flags.with_match_case(true);
        let expression_context = self.flags;
        (ambient_context, expression_context)
    }

    /// Return parser contexts for a match-case guard.
    #[inline]
    fn match_guard_contexts(&self) -> (ParserFlags, ParserFlags) {
        let ambient_context = self.flags.with_match_case(true).with_before_block(true);
        let expression_context = ParserFlags::default();
        (ambient_context, expression_context)
    }

    /// Eat a match statement.
    ///
    /// Examples:
    /// ```
    /// match (<expr>) {
    ///     (x, y, ..) => {
    ///         ...
    ///     }
    ///     (x, y, z) => {
    ///         ...
    ///     }
    ///     _ = ohNoes()
    /// }
    /// ```
    pub fn eat_match(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // keyword
        let keyword = self.eat_keyword_in(&[Keyword::Match, Keyword::Switch])?;
        let form = if keyword == Keyword::Switch {
            MatchForm::Switch
        } else {
            MatchForm::Match
        };

        // body
        self.eat_match_body(form)
    }

    /// Eat a match body (without the match keyword)
    pub fn eat_match_body(&mut self, form: MatchForm) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // value
        let (value_ambient_context, value_expression_context) = self.match_value_contexts();
        let value_id = self.with_flags(
            self.flags
                .with_ambient_context(value_ambient_context)
                .with_expression_context(value_expression_context),
            |parser| parser.eat_parenthesized_expression(),
        )?;

        // cases
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::MatchCase)?;
        let cases_id = self.eat_match_cases(form)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::MatchCase)?;

        // match
        let match_id = self.insert_node(
            Expression::Match {
                form,
                value: value_id,
                cases: cases_id,
            },
            self.get_span_from(&start),
        );
        Ok(match_id)
    }

    /// Eat multiple match cases separated as statements (without the `{` and `}`).
    ///
    /// Examples:
    /// ```
    /// 2 => parse_int(2)
    /// (x, y) => {
    ///     ...
    /// }
    /// ```
    pub(crate) fn eat_match_cases(
        &mut self,
        form: MatchForm,
    ) -> ParseResult<Vec<LocalNodeId<MatchCase>>> {
        let mut cases: Vec<LocalNodeId<MatchCase>> = Vec::new();
        let mut has_default_case = false;
        while self.has_more_tokens() {
            // read the current token once per iteration
            let token_type = self.peek_token_type();

            // stop on closing brace
            if token_type == TokenType::CloseBrace {
                break;
            }
            // allow statement separators between cases (newline/semicolon)
            else if Self::is_statement_stop_token(token_type) {
                self.eat_statement_stop()?;
            }
            // case
            else {
                // reject duplicate default selectors in switch blocks
                if form == MatchForm::Switch
                    && has_default_case
                    && self.is_keyword(Keyword::Default)
                {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                let case = self
                    .eat_match_case(form)
                    .for_node_type(NodeType::MatchCase)?;

                // track default selectors for duplicate checks
                if form == MatchForm::Switch {
                    let selector = match self.tree.get(case) {
                        MatchCase::Expression { selector, .. }
                        | MatchCase::Block { selector, .. } => selector,
                    };
                    if matches!(selector, MatchSelector::Default) {
                        has_default_case = true;
                    }
                }

                cases.push(case);
            }
        }
        Ok(cases)
    }

    /// Eat a match case.
    ///
    /// Examples:
    /// ```
    /// 2 => parse_int(2)
    ///
    /// (x, y) if (x > y) => {
    ///     ...
    /// }
    /// ```
    fn eat_match_case(&mut self, form: MatchForm) -> ParseResult<LocalNodeId<MatchCase>> {
        // decorators before match arms
        let mut pending_case_decorators = if self.peek_is(TokenType::At) {
            self.eat_decorators_maybe()?
        } else {
            smallvec::SmallVec::new()
        };

        let start = self.span_start();
        let mut guard_clause_span = None;

        let selector = match form {
            MatchForm::Switch => {
                // default case
                if self.is_keyword(Keyword::Default) {
                    self.bump();
                    self.eat_colon()?;
                    MatchSelector::Default
                }
                // regular case
                else {
                    self.eat_keyword(Keyword::Case)?;
                    let pattern_start = self.span_start();
                    // allow a wildcard here so switch cases do not bind `_`
                    let pattern = if self.peek_identifier_str_is("_") {
                        self.bump();
                        self.tree
                            .insert(Pattern::Wildcard, self.get_span_from(&pattern_start))
                    } else {
                        let (guard_ambient_context, guard_expression_context) =
                            self.match_guard_contexts();
                        let value = self.eat_expression(
                            self.flags
                                .with_ambient_context(guard_ambient_context)
                                .with_expression_context(guard_expression_context),
                        )?;
                        self.insert_node(
                            Pattern::Expression { value },
                            self.get_span_from(&pattern_start),
                        )
                    };
                    // guard
                    let guard = if self.is_keyword(Keyword::If) {
                        let guard_start = self.span_start();
                        self.eat_keyword(Keyword::If)?;
                        let (guard_ambient_context, guard_expression_context) =
                            self.match_guard_contexts();
                        let guard = self.with_flags(
                            self.flags
                                .with_ambient_context(guard_ambient_context)
                                .with_expression_context(guard_expression_context),
                            |parser| parser.eat_parenthesized_expression(),
                        )?;
                        guard_clause_span = Some(self.get_span_from(&guard_start));
                        Some(guard)
                    } else {
                        None
                    };

                    self.eat_colon()?;
                    MatchSelector::Pattern { pattern, guard }
                }
            }
            // match form
            MatchForm::Match => {
                // pattern
                let (pattern_ambient_context, pattern_expression_context) =
                    self.match_pattern_contexts();
                let pattern = self.with_flags(
                    self.flags
                        .with_ambient_context(pattern_ambient_context)
                        .with_expression_context(pattern_expression_context),
                    |parser| parser.eat_pattern(),
                )?;

                // guard
                let guard = if self.is_keyword(Keyword::If) {
                    let guard_start = self.span_start();
                    self.eat_keyword(Keyword::If)?;
                    let (guard_ambient_context, guard_expression_context) =
                        self.match_guard_contexts();
                    let guard = self.with_flags(
                        self.flags
                            .with_ambient_context(guard_ambient_context)
                            .with_expression_context(guard_expression_context),
                        |parser| parser.eat_parenthesized_expression(),
                    )?;
                    guard_clause_span = Some(self.get_span_from(&guard_start));
                    Some(guard)
                } else {
                    None
                };

                // "arrow"
                self.eat_arrow()?;

                MatchSelector::Pattern { pattern, guard }
            }
        };

        // switch case body: consume statements until break or next case boundary
        if form == MatchForm::Switch {
            // eat expressions until we hit a break (inclusive) or case / default (exclusive)

            // empty case body before the next case, default, or closing brace
            let is_empty_case = self.is_keyword(Keyword::Case)
                || self.is_keyword(Keyword::Default)
                || self.peek_is(TokenType::CloseBrace);
            if is_empty_case {
                let block_id = self.insert_node(
                    Block {
                        context: BlockContext::Statement,
                        form: BlockForm::Implicit,
                        leading_expressions: Vec::new(),
                        tail_expression: None,
                    },
                    self.get_span_from(&start),
                );
                let match_case_id = self.insert_node(
                    MatchCase::Block {
                        selector,
                        body: block_id,
                    },
                    self.get_span_from(&start),
                );
                self.record_match_case_guard_clause(match_case_id, guard_clause_span);
                if !pending_case_decorators.is_empty() {
                    self.attach_decorators(
                        match_case_id.id,
                        std::mem::take(&mut pending_case_decorators),
                    );
                }
                return Ok(match_case_id);
            }
            let mut expressions: Vec<LocalNodeId<Expression>> = Vec::new();
            while self.has_more_tokens() {
                // consume empty statements between switch body statements
                if self.peek_is(TokenType::Semicolon) {
                    self.eat_statement_stop()?;
                    continue;
                }

                // stop at the next case boundary
                if self.is_keyword(Keyword::Case)
                    || self.is_keyword(Keyword::Default)
                    || self.peek_is(TokenType::CloseBrace)
                {
                    break;
                }
                let statement_start = self.span_start();
                let expression_id = self
                    .with_statement_recovery(
                        &statement_start,
                        |parser| parser.try_eat_statement_expression().map(Some),
                        None,
                    )
                    .unwrap_or_else(|| {
                        self.tree
                            .insert(Expression::Error, self.get_span_from(&statement_start))
                    });
                expressions.push(expression_id);

                // consume real separators without treating eof as progress
                if matches!(
                    self.peek_token_type(),
                    TokenType::Comma | TokenType::Semicolon
                ) {
                    self.eat_any_stop()?;
                }
            }
            // single expression case
            let match_case_id = if expressions.len() == 1 {
                self.insert_node(
                    MatchCase::Expression {
                        selector,
                        body: expressions[0],
                    },
                    self.get_span_from(&start),
                )
            }
            // multiple expression block
            else {
                let block_id = self.insert_node(
                    Block {
                        context: BlockContext::Statement,
                        form: BlockForm::Implicit,
                        leading_expressions: expressions,
                        tail_expression: None,
                    },
                    self.get_span_from(&start),
                );
                self.insert_node(
                    MatchCase::Block {
                        selector,
                        body: block_id,
                    },
                    self.get_span_from(&start),
                )
            };
            self.record_match_case_guard_clause(match_case_id, guard_clause_span);
            if !pending_case_decorators.is_empty() {
                self.attach_decorators(
                    match_case_id.id,
                    std::mem::take(&mut pending_case_decorators),
                );
            }
            Ok(match_case_id)
        }
        // block body
        else if self.is_block_start() {
            let block_id = self.eat_block(BlockContext::Expression)?;
            let match_case_id = self.insert_node(
                MatchCase::Block {
                    selector,
                    body: block_id,
                },
                self.get_span_from(&start),
            );
            self.record_match_case_guard_clause(match_case_id, guard_clause_span);
            if !pending_case_decorators.is_empty() {
                self.attach_decorators(
                    match_case_id.id,
                    std::mem::take(&mut pending_case_decorators),
                );
            }
            Ok(match_case_id)
        }
        // single expression
        else {
            let expression_id = self.with_flags(self.flags.in_match_case_body(), |parser| {
                parser.try_eat_expression_until_statement_boundary()
            })?;
            let match_case_id = self.insert_node(
                MatchCase::Expression {
                    selector,
                    body: expression_id,
                },
                self.get_span_from(&start),
            );
            self.record_match_case_guard_clause(match_case_id, guard_clause_span);
            if !pending_case_decorators.is_empty() {
                self.attach_decorators(
                    match_case_id.id,
                    std::mem::take(&mut pending_case_decorators),
                );
            }
            Ok(match_case_id)
        }
    }

    /// Record the guard clause span for one match case.
    fn record_match_case_guard_clause(
        &mut self,
        match_case_id: LocalNodeId<MatchCase>,
        guard_clause_span: Option<Span>,
    ) {
        if let Some(guard_clause_span) = guard_clause_span {
            self.tree.set_side_span(
                match_case_id,
                NodeSpanType::Region(NodeSpanRegion::Clause),
                guard_clause_span,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{
        BinaryOperator, Block, CommentKind, Declarator, Expression, LetKind, MatchCase, MatchForm,
        MatchSelector, Mutability, NodeType, Pattern, PatternField, ScalarLiteral,
    };
    use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

    use crate::{
        TestParser, assert_comment, assert_expression_path, assert_node, assert_string,
        block_expression_ids,
    };

    #[test]
    fn test_parse_match_simple_arms() {
        let mut test = TestParser::new(
            r###"
match (x) {
    1 => 10
    2 => 20
    x => x
    _ => 0
}
"###,
        );
        let mut parser = test.prepare();

        let match_id = parser.eat_match().unwrap();

        assert_node!(parser.tree, match_id, Expression::Match { form: MatchForm::Match, value, cases } => {
            // value: path x
            assert_expression_path!(parser, parser.tree.get(*value), "x");

            assert_eq!(cases.len(), 4);

            // case 0: 1 => 10
            assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(10)));
            });

            // case 1: 2 => 20
            assert_node!(parser.tree, cases[1], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
            });

            // case 2: x => x
            assert_node!(parser.tree, cases[2], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body: _ } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: _ } => {
                    assert_string!(parser, *name, "x");
                });
            });

            // case 3: _ => 0
            assert_node!(parser.tree, cases[3], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
        });
    }

    #[test]
    fn test_parse_match_object_pattern_arms_with_expression_bodies() {
        let mut test = TestParser::new(
            r#"
match (shape) {
    { kind: "circle", radius } => radius
    { kind: "square", size } => size
}
"#,
        );
        let mut parser = test.prepare();
        let match_id = parser.eat_match().unwrap();

        test.assert_no_errors(&parser);

        assert_node!(parser.tree, match_id, Expression::Match { cases, .. } => {
            assert_eq!(cases.len(), 2);

            // { kind: "circle", radius } => radius
            assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                    assert_eq!(fields.len(), 2);
                });
                assert_expression_path!(parser, parser.tree.get(*body), "radius");
            });

            // { kind: "square", size } => size
            assert_node!(parser.tree, cases[1], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                    assert_eq!(fields.len(), 2);
                });
                assert_expression_path!(parser, parser.tree.get(*body), "size");
            });
        });
    }

    #[test]
    fn test_parse_match_with_guard() {
        let mut test = TestParser::new(
            r###"
match (x) {
    2 if (true) => 20
}
"###,
        );
        let mut parser = test.prepare();

        let match_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, match_id, Expression::Match { form: MatchForm::Match, value: _, cases } => {
            assert_eq!(cases.len(), 1);

            // case: 2 if (true) => 20
            assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                let guard_clause_span = parser
                    .tree
                    .get_side_span(cases[0], NodeSpanType::Region(NodeSpanRegion::Clause))
                    .expect("expected guard clause span");
                assert_eq!(parser.get_span_str(guard_clause_span), "if (true)");

                // guard: true
                let guard_id = guard.expect("expected guard");
                assert_node!(parser.tree, guard_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));

                // pattern: 2
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });

                // body: 20
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
            });
        });
    }

    #[test]
    fn test_parse_match_tuple_guard_with_comparison() {
        let mut test = TestParser::new(
            r###"
match (pair) {
    (_, count) if (count > 0) => count
    _ => 0
}
"###,
        );
        let mut parser = test.prepare();

        let match_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, match_id, Expression::Match { form: MatchForm::Match, value: _, cases } => {
            assert_eq!(cases.len(), 2);

            assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert_node!(parser.tree, *pattern, Pattern::Tuple { fields } => {
                    assert_eq!(fields.len(), 2);

                    assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                        assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                    });
                    assert_node!(parser.tree, fields[1], PatternField::Named { name: _, is_shorthand, pattern } => {
                        assert!(*is_shorthand);
                        assert!(pattern.is_none());
                    });
                });

                let guard_id = guard.expect("expected guard");
                assert_node!(parser.tree, guard_id, Expression::Binary { left, operator: BinaryOperator::GreaterThan, right } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "count");
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                });
                assert_expression_path!(parser, parser.tree.get(*body), "count");
            });

            assert_node!(parser.tree, cases[1], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                assert!(guard.is_none());
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
        });
    }

    #[test]
    fn test_parse_match_with_paths() {
        let mut test = TestParser::new(
            r"
match (self) {
    TetrisPieceShape.I => Color.Blue
    TetrisPieceShape.J => Color.Red
    _ => Color.Gray
}
    ",
        );
        let mut parser = test.prepare();

        let match_id = parser.eat_match().unwrap();

        // match (self) { ... }
        assert_node!(parser.tree, match_id, Expression::Match { form: MatchForm::Match, value, cases } => {
            // self
            assert_expression_path!(parser, parser.tree.get(*value), "self");

            assert_eq!(cases.len(), 3);

            // TetrisPieceShape.I => Color.Blue
            assert_node!(parser.tree, cases[0], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                // TetrisPieceShape.I
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.I");
                });
                // Color.Blue
                assert_expression_path!(parser, parser.tree.get(*body), "Color.Blue");
            });

            // TetrisPieceShape.J => Color.Red
            assert_node!(parser.tree, cases[1], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                // TetrisPieceShape.J
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.J");
                });
                // Color.Red
                assert_expression_path!(parser, parser.tree.get(*body), "Color.Red");
            });

            // _ => Color.Gray
            assert_node!(parser.tree, cases[2], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                // Color.Gray
                assert_expression_path!(parser, parser.tree.get(*body), "Color.Gray");
            });
        });
    }

    #[test]
    fn test_parse_match_parenthesized_value_keeps_inner_span() {
        let mut test = TestParser::new(
            r"
match (value) {
    _ => result
}
",
        );
        let mut parser = test.prepare();

        let match_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, match_id, Expression::Match { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");

            let value_span = parser.tree.get_span(*value);
            let value_text = &parser.file.text()[value_span.start as usize..value_span.end as usize];
            assert_eq!(value_text, "value");
        });
    }

    #[test]
    fn test_parse_match_arm_keeps_if_else_branch_semicolons_as_statements() {
        let mut test = TestParser::new(
            r#"
match (result) {
    Ok(value) => {
        if (value.valid) { use(value); } else { reset(); }
    };
    Err(error) => report(error)
}
"#,
        );
        let mut parser = test.prepare();

        let match_id = parser.eat_match().unwrap();

        assert_node!(parser.tree, match_id, Expression::Match { cases, .. } => {
            assert_eq!(cases.len(), 2);

            assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
                let body_block = parser.tree.get(*body);
                assert!(body_block.leading_expressions.is_empty());
                let if_expression_id = body_block
                    .tail_expression
                    .expect("expected match arm tail expression");

                assert_node!(parser.tree, if_expression_id, Expression::If { then_expression, else_expression, .. } => {
                    assert_node!(parser.tree, *then_expression, Expression::Block(then_block_id) => {
                        let then_block = parser.tree.get(*then_block_id);
                        assert_eq!(then_block.leading_expressions.len(), 1);
                        assert!(then_block.tail_expression.is_none());
                    });

                    let else_expression = else_expression.expect("expected else expression");
                    assert_node!(parser.tree, else_expression, Expression::Block(else_block_id) => {
                        let else_block = parser.tree.get(*else_block_id);
                        assert_eq!(else_block.leading_expressions.len(), 1);
                        assert!(else_block.tail_expression.is_none());
                    });
                });
            });
        });
    }

    /// Parse a switch-case statement.
    #[test]
    fn test_parse_match_from_switch_case() {
        let mut test = TestParser::new(
            r###"
switch (left.type) {
    case "static":
        left.field;
        break;
    case "dynamic":
        left.value;
        something();
        // implicitly break
    case "literal":
        something();
        // implicitly break, one statement (no block
    default:
        something();
        return left.name;
  }
"###,
        );
        let mut parser = test.prepare();

        let switch_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, value: _, cases } => {
            assert_eq!(cases.len(), 4);

            // case "static" block
            assert_node!(parser.tree, cases[0], MatchCase::Block { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                // selector
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                        assert_string!(parser, *literal, "static");
                    });
                });
                // body
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 2);
                });
            });

            // case "dynamic" block
            assert_node!(parser.tree, cases[1], MatchCase::Block { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                // selector
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                        assert_string!(parser, *literal, "dynamic");
                    });
                });
                // body
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 2);
                });
            });

            // case "literal" expression
            assert_node!(parser.tree, cases[2], MatchCase::Expression { selector: MatchSelector::Pattern { pattern, guard }, body } => {
                assert!(guard.is_none());
                // selector
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                        assert_string!(parser, *literal, "literal");
                    });
                });
                // body
                assert_node!(parser.tree, *body, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "something");
                });
            });

            // case default (block)
            assert_node!(parser.tree, cases[3], MatchCase::Block { selector: MatchSelector::Default, body } => {
                // body
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 2);
                });
            });
        });
    }

    /// Parse a switch case with a block statement followed by a regex expression statement.
    #[test]
    fn test_parse_switch_case_block_then_regex_expression_statement() {
        let mut test = TestParser::new(
            r###"
switch(a) { case 1: {}
/foo/ }
"###,
        );
        let mut parser = test.prepare();

        // switch(a) { case 1: {} /foo/ }
        let switch_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
            assert_eq!(cases.len(), 1);
            assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 2);
                    assert_node!(parser.tree, expressions[0], Expression::Block(_));
                    assert_node!(parser.tree, expressions[1], Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
                });
            });
        });
    }

    /// Parse switch case statements that begin with a semicolon before a parenthesized call.
    #[test]
    fn test_parse_switch_case_leading_semicolon_parenthesized_call() {
        let mut test = TestParser::new(
            r###"
switch (tag.injectTo) {
  case "body":
    ;(bodyTags ??= []).push(tag)
    break
}
"###,
        );
        let mut parser = test.prepare();

        let switch_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
            assert_eq!(cases.len(), 1);
            assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 2);

                    // first statement: (bodyTags ??= []).push(tag)
                    assert_node!(parser.tree, expressions[0], Expression::Call { left, arguments, .. } => {
                        assert_eq!(arguments.len(), 1);
                                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                                    assert_string!(parser, *name, "push");
                                    assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                                        assert_node!(parser.tree, *expression, Expression::Assign { left, right, .. } => {
                                    assert_expression_path!(parser, parser.tree.get(*left), "bodyTags");
                                            assert_node!(parser.tree, *right, Expression::ArrayExpression { elements } => {
                                                assert!(elements.is_empty());
                                            });
                                        });
                                    });
                        });
                    });

                    // second statement: break
                    assert_node!(parser.tree, expressions[1], Expression::Break { .. });
                });
            });
        });
    }

    /// Parse a switch case with an if block, following assignments, then fallthrough case.
    #[test]
    fn test_parse_switch_case_if_block_then_assignments_then_fallthrough_case() {
        let mut test = TestParser::new_with_language(
            r###"
switch (tag) {
  case dataViewTag:
    if ((object.byteLength != other.byteLength) ||
        (object.byteOffset != other.byteOffset)) {
      return false;
    }
    object = object.buffer;
    other = other.buffer;

  case arrayBufferTag:
    return true;
}
"###,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let switch_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
            assert_eq!(cases.len(), 2);

            // first case body
            assert_node!(parser.tree, cases[0], MatchCase::Block { selector, body } => {
                match selector {
                    MatchSelector::Pattern { pattern, guard } => {
                        assert!(guard.is_none());
                        assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "dataViewTag");
                        });
                    }
                    _ => panic!("expected pattern selector"),
                };
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 3);
                    assert_node!(parser.tree, expressions[0], Expression::If { .. });
                    let first_assign_id = parser.unwrap_label_expression(expressions[1]);
                    let second_assign_id = parser.unwrap_label_expression(expressions[2]);
                    assert_node!(parser.tree, first_assign_id, Expression::Assign { .. });
                    assert_node!(parser.tree, second_assign_id, Expression::Assign { .. });
                });
            });

            // second case body
            assert_node!(parser.tree, cases[1], MatchCase::Expression { selector, body } => {
                match selector {
                    MatchSelector::Pattern { pattern, guard } => {
                        assert!(guard.is_none());
                        assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                            assert_expression_path!(parser, parser.tree.get(*value), "arrayBufferTag");
                        });
                    }
                    _ => panic!("expected pattern selector"),
                };
                let return_id = parser.unwrap_label_expression(*body);
                assert_node!(parser.tree, return_id, Expression::Return { .. });
            });
        });
    }

    /// Parse switch case statements that continue after one break statement.
    #[test]
    fn test_parse_switch_case_with_multiple_break_statements() {
        let mut test = TestParser::new(
            r###"
switch (value) {
  case 1:
    break;
    break;
}
"###,
        );
        let mut parser = test.prepare();

        let switch_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
            assert_eq!(cases.len(), 1);
            assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 2);
                    assert_node!(parser.tree, expressions[0], Expression::Break { .. });
                    assert_node!(parser.tree, expressions[1], Expression::Break { .. });
                });
            });
        });
    }

    /// Recover a switch case body that reaches EOF before the switch closes.
    #[test]
    fn test_parse_switch_case_body_recovers_at_eof() {
        let mut test = TestParser::new_with_language(
            "switch (cond) { case 10: let a = 20;",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let roots = parser.parse();

        assert_eq!(roots.len(), 1);
        assert_node!(parser.tree, roots[0], Expression::Match { form, value, cases } => {
            assert_eq!(*form, MatchForm::Switch);
            assert_expression_path!(parser, parser.tree.get(*value), "cond");
            assert_eq!(cases.len(), 1);
            assert_node!(parser.tree, cases[0], MatchCase::Expression { selector, body } => {
                assert_node!(selector, MatchSelector::Pattern { pattern, guard } => {
                    assert!(guard.is_none());
                    assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(10)));
                    });
                });
                assert_node!(parser.tree, *body, Expression::Let { kind, export, mutability, declarators, is_ambient, is_shared } => {
                    assert_eq!(*kind, LetKind::Let);
                    assert_eq!(*export, None);
                    assert_eq!(*mutability, Mutability::Mutable);
                    assert!(!*is_ambient);
                    assert!(!*is_shared);
                    assert_eq!(declarators.len(), 1);
                    assert_node!(parser.tree, declarators[0], Declarator { pattern, ty, value } => {
                        assert!(ty.is_none());
                        assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern } => {
                            assert_string!(parser, *name, "a");
                            assert!(pattern.is_none());
                        });
                        assert_node!(parser.tree, value.expect("expected initializer"), Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
                    });
                });
            });
        });

        test.assert_error_leaves(&parser, &[(Some(NodeType::MatchCase), None, "")]);
    }

    /// Parse minified switch cases where `continue` is followed by `}` and another `if`.
    #[test]
    fn test_parse_switch_case_minified_if_continue_then_if() {
        let mut test = TestParser::new_with_language(
            "switch(op[0]){default:if(!(t=_.trys,t=t.length>0&&t[t.length-1])&&(op[0]===6||op[0]===2)){_=0;continue}if(op[0]===3&&(!t||op[1]>t[0]&&op[1]<t[3])){_.label=op[1];break}}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let switch_id = parser.eat_match().unwrap();

        // switch(op[0]) { default: if (...) { _ = 0; continue } if (...) { _.label = op[1]; break } }
        assert_node!(parser.tree, switch_id, Expression::Match { form: MatchForm::Switch, cases, .. } => {
            assert_eq!(cases.len(), 1);

            // default case body keeps both if statements
            assert_node!(parser.tree, cases[0], MatchCase::Block { selector, body } => {
                assert!(matches!(selector, MatchSelector::Default));
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 2);
                    assert_node!(parser.tree, expressions[0], Expression::If { .. });
                    assert_node!(parser.tree, expressions[1], Expression::If { .. });
                });
            });
        });
    }

    #[test]
    fn test_parse_switch_case_boundary_comment_ownership() {
        let mut test = TestParser::new_with_language(
            r#"switch (state) {
  // before-ready
  case "ready":
    start() // ready-tail
    break
  default:
    stop() // default-tail
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Match { form, cases, .. } => {
            assert_eq!(*form, MatchForm::Switch);
            assert_eq!(cases.len(), 2);

            let first_case_annotations = parser.tree.get_decorators(cases[0].id);
            assert!(first_case_annotations.is_empty());

            assert_node!(parser.tree, cases[0], MatchCase::Block { body, .. } => {
                assert_node!(parser.tree, *body, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*body));
                    assert_eq!(expressions.len(), 2);
                    let start_annotations = parser.tree.get_decorators(expressions[0].id);
                    assert!(start_annotations.is_empty());
                });
            });

            assert_node!(parser.tree, cases[1], MatchCase::Expression { body, .. } => {
                let default_annotations = parser.tree.get_decorators(body.id);
                assert!(default_annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comments().len(), 3);
        assert_comment!(parser, 0, CommentKind::Line, "before-ready");

        assert_comment!(parser, 1, CommentKind::Line, "ready-tail");

        assert_comment!(parser, 2, CommentKind::Line, "default-tail");
    }
}
