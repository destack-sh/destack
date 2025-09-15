use dyst_language_token::TokenType;

use crate::parse::expression::ExpressionParserOptions;
use crate::{Keyword, Match, MatchCase, NodeId, ParseResult, Parser, Try};

impl<'a> Parser<'a> {
    /// Eat a match statement.
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
    pub fn eat_match(&mut self) -> ParseResult<NodeId<Match>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Match)?;
        let value_id = self.try_eat_expression(
            ExpressionParserOptions {
                is_before_block: true,
                ..ExpressionParserOptions::default()
            },
            TokenType::OpenBrace,
        )?;
        self.eat_token(TokenType::OpenBrace)?;
        let cases_id = self.eat_match_body()?;
        self.eat_token(TokenType::CloseBrace)?;
        let match_id = self.tree.allocate(
            Match {
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
    pub fn eat_match_body(&mut self) -> ParseResult<Vec<NodeId<MatchCase>>> {
        let mut cases: Vec<NodeId<MatchCase>> = Vec::new();
        loop {
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
                let case = self.eat_match_case()?;
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
    pub fn eat_match_case(&mut self) -> ParseResult<NodeId<MatchCase>> {
        let start = self.mark();
        // pattern
        let pattern_id = self.eat_pattern()?;
        // guard
        let guard = if self.peek_keyword(Keyword::If).is_ok() {
            self.eat_keyword(Keyword::If)?;
            let guard =
                self.try_eat_expression(ExpressionParserOptions::default(), TokenType::Arrow)?;
            Some(guard)
        } else {
            None
        };
        // arrow
        self.eat_token(TokenType::Arrow)?;
        // body
        if self.peek_block().is_ok() {
            let block_id = self.eat_block()?;
            let match_case_id = self.tree.allocate(
                MatchCase::Block {
                    pattern: pattern_id,
                    body: block_id,
                    guard,
                },
                self.get_span_from(start),
            );
            Ok(match_case_id)
        }
        // expression
        else {
            let expression_id =
                self.try_eat_expression(ExpressionParserOptions::default(), TokenType::Newline)?;
            let match_case_id = self.tree.allocate(
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

    /// Eat a try statement.
    ///
    /// Examples:
    /// ```
    /// try fileOperation() // implicitly unwraps the Result, returns Error case
    ///
    /// try { // implicitly unwraps all Results inside
    ///     let a = riskyOperationA() // a is Result.Ok(_) from riskyOperationA
    ///     riskyOperationB(a)
    /// } // no catch needed if containing function has compatible Result type (Into suffices)
    ///
    /// try { // explicitly unwraps all Results inside
    ///     ...
    /// } catch e { // match all errors
    ///     NumericError(x) => Error(@format("bad number: {x}"))
    ///     FormatError => Error(@format("bad format {e}"))
    ///     // it's exhaustive! otherwise `_ =>` like in match (it is a match)
    /// }
    /// ```
    pub fn eat_try_catch(&mut self) -> ParseResult<NodeId<Try>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Try)?;

        // try block
        if self.peek_block().is_ok() {
            // try block
            let block_id = self.eat_block()?;

            // try block with catch
            if self.peek_keyword(Keyword::Catch).is_ok() {
                // catch match
                self.bump(); // eat catch 
                let catch_expression_id = self.eat_expression(ExpressionParserOptions {
                    is_before_block: true,
                    ..ExpressionParserOptions::default()
                })?;
                self.eat_token(TokenType::OpenBrace)?;
                let catch_match_cases_id = self.eat_match_body()?;
                self.eat_token(TokenType::CloseBrace)?;
                let catch_match_id = self.tree.allocate(
                    Match {
                        value: catch_expression_id,
                        cases: catch_match_cases_id,
                    },
                    self.get_span_from(start),
                );
                let try_id = self.tree.allocate(
                    Try::BlockWithCatch {
                        try_block: block_id,
                        catch_match: catch_match_id,
                    },
                    self.get_span_from(start),
                );
                Ok(try_id)
            }
            // try block without catch
            else {
                let try_id = self.tree.allocate(
                    Try::Block {
                        try_block: block_id,
                    },
                    self.get_span_from(start),
                );
                Ok(try_id)
            }
        }
        // try expression
        else {
            let expression_id = self.eat_expression(ExpressionParserOptions::default())?;
            let try_id = self.tree.allocate(
                Try::Expression {
                    try_expression: expression_id,
                },
                self.get_span_from(start),
            );
            Ok(try_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Expression, Match, MatchCase, Pattern, Try, assert_bool, assert_int, assert_node,
        assert_path, assert_expr_path,
    };

    #[test]
    fn test_match_simple_literal_arms() {
        let mut test = TestParser::new(
            r###"
match x {
    1 => 10
    2 => 20
    _ => 0
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match().unwrap();

        assert_node!(parser.tree, match_id, Match { value, cases } => {
            // value: path x
            assert_expr_path!(parser.session, parser.tree.get(*value), "x");

            assert_eq!(cases.len(), 3);

            // case 0: 1 => 10
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Literal(lit_id) => {
                    assert_int!(parser.tree, *lit_id, 1);
                });
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(lit_id) => {
                    assert_int!(parser.tree, *lit_id, 10);
                });
            });

            // case 1: 2 => 20
            assert_node!(parser.tree, cases[1], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Literal(lit_id) => {
                    assert_int!(parser.tree, *lit_id, 2);
                });
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(lit_id) => {
                    assert_int!(parser.tree, *lit_id, 20);
                });
            });

            // case 2: _ => 0
            assert_node!(parser.tree, cases[2], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(lit_id) => {
                    assert_int!(parser.tree, *lit_id, 0);
                });
            });
        });
    }

    #[test]
    fn test_match_with_guard() {
        let mut test = TestParser::new(
            r###"
match x {
    2 if true => 20
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match().unwrap();
        assert_node!(parser.tree, match_id, Match { value: _, cases } => {
            assert_eq!(cases.len(), 1);

            // case: 2 if true => 20
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
                // guard: true
                let guard_id = guard.expect("expected guard");
                assert_node!(parser.tree, guard_id, Expression::ScalarLiteral(lit_id) => {
                    assert_bool!(parser.tree, *lit_id, true);
                });

                // pattern: 2
                assert_node!(parser.tree, *pattern, Pattern::Literal(lit_id) => {
                    assert_int!(parser.tree, *lit_id, 2);
                });

                // body: 20
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(lit_id) => {
                    assert_int!(parser.tree, *lit_id, 20);
                });
            });
        });
    }

    #[test]
    fn test_match_with_paths() {
        let mut test = TestParser::new(
            r"
match self {
    TetrisPieceShape.I => Color.Blue
    TetrisPieceShape.J => Color.Red
    _ => Color.Gray
}
    ",
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match().unwrap();

        // match self { ... }
        assert_node!(parser.tree, match_id, Match { value, cases } => {
            // self
            assert_expr_path!(parser.session, parser.tree.get(*value), "self");

            assert_eq!(cases.len(), 3);

            // TetrisPieceShape.I => Color.Blue
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                // TetrisPieceShape.I
                assert_node!(parser.tree, *pattern, Pattern::Path(path_id) => {
                    assert_path!(parser.session, *path_id, "TetrisPieceShape.I");
                });
                // Color.Blue
                assert_expr_path!(parser.session, parser.tree.get(*body), "Color.Blue");
            });

            // TetrisPieceShape.J => Color.Red
            assert_node!(parser.tree, cases[1], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                // TetrisPieceShape.J
                assert_node!(parser.tree, *pattern, Pattern::Path(path_id) => {
                    assert_path!(parser.session, *path_id, "TetrisPieceShape.J");
                });
                // Color.Red
                assert_expr_path!(parser.session, parser.tree.get(*body), "Color.Red");
            });

            // _ => Color.Gray
            assert_node!(parser.tree, cases[2], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                // Color.Gray
                assert_expr_path!(parser.session, parser.tree.get(*body), "Color.Gray");
            });
        });
    }

    #[test]
    fn test_try_expression() {
        let mut test = TestParser::new(
            r###"
try foo()
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let try_id = parser.eat_try_catch().unwrap();
        assert_node!(parser.tree, try_id, Try::Expression { try_expression } => {
            assert_node!(parser.tree, *try_expression, Expression::Call(call_id) => {
                let call = parser.tree.get(*call_id);
                // receiver is path foo
                assert_expr_path!(parser.session, parser.tree.get(call.receiver), "foo");
            });
        });
    }

    #[test]
    fn test_try_block_without_catch() {
        let mut test = TestParser::new(
            r###"
try {
    foo()
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let try_id = parser.eat_try_catch().unwrap();
        assert_node!(parser.tree, try_id, Try::Block { try_block } => {
            let _block = parser.tree.get(*try_block);
        });
    }

    #[test]
    fn test_try_block_with_catch() {
        let mut test = TestParser::new(
            r###"
try {
    foo()
} catch e {
    _ => {
        bar()
    }
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let try_id = parser.eat_try_catch().unwrap();
        assert_node!(parser.tree, try_id, Try::BlockWithCatch { try_block, catch_match } => {
            // try block exists
            let _block = parser.tree.get(*try_block);

            // catch match: value is path e, one case with wildcard and block body
            assert_node!(parser.tree, *catch_match, Match { value, cases } => {
                assert_expr_path!(parser.session, parser.tree.get(*value), "e");

                assert_eq!(cases.len(), 1);
                assert_node!(parser.tree, cases[0], MatchCase::Block { pattern, body: _, guard } => {
                    assert!(guard.is_none());
                    assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                });
            });
        });
    }
}
