use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use destack_ast::{
    Block, BlockFormat, Expression, Keyword, LocalNodeId, MatchCase, MatchKind, NodeType, Pattern,
    TokenType,
};

impl Parser {
    /// Eat a match statement. Tolerates switch-kind syntax for #Compatibility.
    ///
    /// Examples:
    /// ```
    /// match <expr> {
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
        // (accept switch for #Compatibility)
        let keyword = self.eat_keyword_in(&[Keyword::Match, Keyword::Switch])?;
        let kind = if keyword == Keyword::Switch {
            MatchKind::Switch
        } else {
            MatchKind::Match
        };

        // body
        self.eat_match_body(kind)
    }

    /// Eat a match body (without the match keyword)
    pub fn eat_match_body(&mut self, kind: MatchKind) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // value
        let value_id = self.with_options(self.options.in_before_block(), |parser| {
            parser.eat_expression_parenthesized_maybe()
        })?;

        // cases
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::MatchCase)?;
        let cases_id = self.eat_match_cases(kind)?;
        self.eat_token(TokenType::CloseBrace)?;

        // match
        let match_id = self.tree.insert(
            Expression::Match {
                kind,
                value: value_id,
                cases: cases_id,
            },
            self.get_span_from(start),
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
        kind: MatchKind,
    ) -> ParseResult<Vec<LocalNodeId<MatchCase>>> {
        let mut cases: Vec<LocalNodeId<MatchCase>> = Vec::new();
        while self.peek().is_ok() {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // allow statement separators between cases (newline/semicolon)
            else if self.peek_statement_stop().is_ok() {
                self.eat_statement_stop_with_newlines()?;
            }
            // case
            else {
                let case = self
                    .eat_match_case(kind)
                    .for_node_type(NodeType::MatchCase)?;
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
    /// (x, y) if x > y => {
    ///     ...
    /// }
    /// ```
    fn eat_match_case(&mut self, kind: MatchKind) -> ParseResult<LocalNodeId<MatchCase>> {
        let start = self.mark();

        let (pattern_id, guard) = {
            match kind {
                MatchKind::Switch => {
                    // default case
                    if self.peek_keyword(Keyword::Default).is_ok() {
                        self.bump();
                        self.eat_colon()?;
                        self.eat_newlines_maybe()?;
                        let pattern_id = self
                            .tree
                            .insert(Pattern::Wildcard, self.get_span_from(start));

                        (pattern_id, None)
                    }
                    // regular case
                    else {
                        self.eat_keyword(Keyword::Case)?;
                        let pattern_id = self.eat_pattern()?;
                        self.eat_colon()?;
                        self.eat_newlines_maybe()?;
                        let guard = None;

                        (pattern_id, guard)
                    }
                }
                // match-kind
                MatchKind::Match => {
                    // pattern
                    let pattern_id = self.with_options(self.options.in_match_case(), |parser| {
                        parser.eat_pattern()
                    })?;

                    // guard
                    let guard = if self.peek_keyword(Keyword::If).is_ok() {
                        self.eat_keyword(Keyword::If)?;
                        let guard = self.with_options(
                            ParserOptions {
                                in_match_case: true,
                                in_before_block: true,
                                ..Default::default()
                            },
                            |parser| parser.eat_expression_parenthesized_maybe(),
                        )?;
                        Some(guard)
                    } else {
                        None
                    };

                    // "arrow"
                    self.eat_arrow()?;

                    (pattern_id, guard)
                }
            }
        };

        // body
        if self.peek_block().is_ok() {
            let block_id = self.eat_block()?;
            let match_case_id = self.tree.insert(
                MatchCase::Block {
                    pattern: pattern_id,
                    body: block_id,
                    guard,
                },
                self.get_span_from(start),
            );
            Ok(match_case_id)
        }
        // implicit case block/expression
        else if kind == MatchKind::Switch {
            // eat expressions until we hit a break (inclusive) or case / default (exclusive)
            self.eat_newlines_maybe()?;
            let mut expressions: Vec<LocalNodeId<Expression>> = Vec::new();
            while self.peek_keyword(Keyword::Case).is_err()
                && self.peek_keyword(Keyword::Default).is_err()
                && self.peek_token(TokenType::CloseBrace).is_err()
            {
                let expression_id = self.try_eat_expression(TokenType::Newline)?;
                expressions.push(expression_id);
                if self.peek_any_stop().is_ok() {
                    self.eat_any_stop_with_newlines()?;
                }
                if matches!(self.tree.get(expression_id), Expression::Break { .. }) {
                    break;
                }
            }
            // single expression case
            let match_case_id = if expressions.len() == 1 {
                self.tree.insert(
                    MatchCase::Expression {
                        pattern: pattern_id,
                        body: expressions[0],
                        guard,
                    },
                    self.get_span_from(start),
                )
            }
            // multiple expression block
            else {
                let block_id = self.tree.insert(
                    Block {
                        format: BlockFormat::Implicit,
                        label: None,
                        expressions,
                    },
                    self.get_span_from(start),
                );
                self.tree.insert(
                    MatchCase::Block {
                        pattern: pattern_id,
                        body: block_id,
                        guard,
                    },
                    self.get_span_from(start),
                )
            };
            Ok(match_case_id)
        }
        // single expression
        else {
            let expression_id = self.try_eat_expression(TokenType::Newline)?;
            let match_case_id = self.tree.insert(
                MatchCase::Expression {
                    pattern: pattern_id,
                    body: expression_id,
                    guard,
                },
                self.get_span_from(start),
            );
            Ok(match_case_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{Block, Expression, MatchCase, MatchKind, Pattern, ScalarLiteral};

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_match_simple_arms() {
        let mut test = TestParser::new(
            r###"
match x {
    1 => 10
    2 => 20
    x => x
    _ => 0
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match().unwrap();

        assert_node!(parser.tree, match_id, Expression::Match { kind: MatchKind::Match, value, cases } => {
            // value: path x
            assert_expression_path!(parser, parser.tree.get(*value), "x");

            assert_eq!(cases.len(), 4);

            // case 0: 1 => 10
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(10)));
            });

            // case 1: 2 => 20
            assert_node!(parser.tree, cases[1], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
            });

            // case 2: x => x
            assert_node!(parser.tree, cases[2], MatchCase::Expression { pattern, body: _, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Binding { mutability: None, name, pattern: _ } => {
                    assert_string!(parser, *name, "x");
                });
            });

            // case 3: _ => 0
            assert_node!(parser.tree, cases[3], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
        });
    }

    #[test]
    fn test_parse_match_with_guard() {
        let mut test = TestParser::new(
            r###"
match x {
    2 if true => 20
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, match_id, Expression::Match { kind: MatchKind::Match, value: _, cases } => {
            assert_eq!(cases.len(), 1);

            // case: 2 if true => 20
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
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
    fn test_parse_match_with_paths() {
        let mut test = TestParser::new(
            r"
match self {
    TetrisPieceShape.I => Color.Blue
    TetrisPieceShape.J => Color.Red
    _ => Color.Gray
}
    ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match().unwrap();

        // match self { ... }
        assert_node!(parser.tree, match_id, Expression::Match { kind: MatchKind::Match, value, cases } => {
            // self
            assert_expression_path!(parser, parser.tree.get(*value), "self");

            assert_eq!(cases.len(), 3);

            // TetrisPieceShape.I => Color.Blue
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                // TetrisPieceShape.I
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.I");
                });
                // Color.Blue
                assert_expression_path!(parser, parser.tree.get(*body), "Color.Blue");
            });

            // TetrisPieceShape.J => Color.Red
            assert_node!(parser.tree, cases[1], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                // TetrisPieceShape.J
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.J");
                });
                // Color.Red
                assert_expression_path!(parser, parser.tree.get(*body), "Color.Red");
            });

            // _ => Color.Gray
            assert_node!(parser.tree, cases[2], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                // Color.Gray
                assert_expression_path!(parser, parser.tree.get(*body), "Color.Gray");
            });
        });
    }

    /// Parse a switch-case statement for #Compatibility.
    #[test]
    fn test_parse_match_from_switch_case() {
        let mut test = TestParser::new(
            r###"
switch (left.type) {
    case 'static':
        left.field;
        break;
    case 'dynamic':
        left.value;
        something();
        // implicitly break
    case 'literal':
        something();
        // implicitly break, one statement (no block
    default:
        something();
        return left.name;
  }
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let switch_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, switch_id, Expression::Match { kind: MatchKind::Switch, value: _, cases } => {
            assert_eq!(cases.len(), 4);

            // case 'static' (block)
            assert_node!(parser.tree, cases[0], MatchCase::Block { pattern, body, guard } => {
                assert!(guard.is_none());
                // 'static'
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                        assert_string!(parser, *literal, "static");
                    });
                });
                // body
                assert_node!(parser.tree, *body, Block { format: _, label: _, expressions } => {
                    assert_eq!(expressions.len(), 2);
                });
            });

            // case 'dynamic' (block)
            assert_node!(parser.tree, cases[1], MatchCase::Block { pattern, body, guard } => {
                assert!(guard.is_none());
                // 'dynamic'
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                        assert_string!(parser, *literal, "dynamic");
                    });
                });
                // body
                assert_node!(parser.tree, *body, Block { format: _, label: _, expressions } => {
                    assert_eq!(expressions.len(), 2);
                });
            });

            // case 'literal' (expression)
            assert_node!(parser.tree, cases[2], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                // 'literal'
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(literal)) => {
                        assert_string!(parser, *literal, "literal");
                    });
                });
                // body (one statement, no block)
                assert_node!(parser.tree, *body, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "something");
                });
            });

            // case default (block)
            assert_node!(parser.tree, cases[3], MatchCase::Block { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                // body
                assert_node!(parser.tree, *body, Block { format: _, label: _, expressions } => {
                    assert_eq!(expressions.len(), 2);
                });
            });
        });
    }
}
