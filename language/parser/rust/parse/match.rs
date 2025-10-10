use crate::TokenType;

use crate::parse::prelude::*;
use crate::{Expression, Keyword, MatchCase, NodeId, NodeType, Parser, ParserResult, Runtime};

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
    pub fn eat_match(&mut self, runtime: Option<Runtime>) -> ParserResult<NodeId<Expression>> {
        // keyword
        self.eat_keyword(Keyword::Match)?;

        // body
        self.eat_match_body(runtime)
    }

    /// Eat a match body (without the match keyword)
    pub fn eat_match_body(&mut self, runtime: Option<Runtime>) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // value
        let value_id = self.with_options(self.options.in_before_block(), |parser| {
            parser.try_eat_expression(TokenType::OpenBrace)
        })?;

        // cases
        self.eat_token(TokenType::OpenBrace)?;
        let cases_id = self.eat_match_cases()?;
        self.eat_token(TokenType::CloseBrace)?;

        // match
        let match_id = self.tree.insert(
            Expression::Match {
                runtime,
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
    pub(crate) fn eat_match_cases(&mut self) -> ParserResult<Vec<NodeId<MatchCase>>> {
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
                let case = self.eat_match_case().for_node_type(NodeType::MatchCase)?;
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
    fn eat_match_case(&mut self) -> ParserResult<NodeId<MatchCase>> {
        let start = self.mark();

        // pattern
        let pattern_id = self.eat_pattern()?;

        // guard
        let guard = if self.peek_keyword(Keyword::If).is_ok() {
            self.eat_keyword(Keyword::If)?;
            let guard = self.with_options(self.options.nested_in_before_block(), |parser| {
                parser.try_eat_expression(TokenType::FatArrow)
            })?;
            Some(guard)
        } else {
            None
        };

        // arrow
        self.eat_arrow()?;

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
        // expression
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
    use crate::parse::tests::TestParser;
    use crate::{
        Expression, MatchCase, Pattern, ScalarLiteral, assert_expr_path, assert_node, assert_path,
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
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match(None).unwrap();

        assert_node!(parser.tree, match_id, Expression::Match { runtime: _, value, cases } => {
            // value: path x
            assert_expr_path!(parser, parser.tree.get(*value), "x");

            assert_eq!(cases.len(), 3);

            // case 0: 1 => 10
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::ScalarLiteral(ScalarLiteral::Integer(1)));
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(10)));
            });

            // case 1: 2 => 20
            assert_node!(parser.tree, cases[1], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::ScalarLiteral(ScalarLiteral::Integer(2)));
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
            });

            // case 2: _ => 0
            assert_node!(parser.tree, cases[2], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
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
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match(None).unwrap();
        assert_node!(parser.tree, match_id, Expression::Match { runtime: _, value: _, cases } => {
            assert_eq!(cases.len(), 1);

            // case: 2 if true => 20
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
                // guard: true
                let guard_id = guard.expect("expected guard");
                assert_node!(parser.tree, guard_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));

                // pattern: 2
                assert_node!(parser.tree, *pattern, Pattern::ScalarLiteral(ScalarLiteral::Integer(2)));

                // body: 20
                assert_node!(parser.tree, *body, Expression::ScalarLiteral(ScalarLiteral::Integer(20)));
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
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match(None).unwrap();

        // match self { ... }
        assert_node!(parser.tree, match_id, Expression::Match { runtime: _, value, cases } => {
            // self
            assert_expr_path!(parser, parser.tree.get(*value), "self");

            assert_eq!(cases.len(), 3);

            // TetrisPieceShape.I => Color.Blue
            assert_node!(parser.tree, cases[0], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                // TetrisPieceShape.I
                assert_node!(parser.tree, *pattern, Pattern::Path { path } => {
                    assert_path!(parser, *path, "TetrisPieceShape.I");
                });
                // Color.Blue
                assert_expr_path!(parser, parser.tree.get(*body), "Color.Blue");
            });

            // TetrisPieceShape.J => Color.Red
            assert_node!(parser.tree, cases[1], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                // TetrisPieceShape.J
                assert_node!(parser.tree, *pattern, Pattern::Path { path } => {
                    assert_path!(parser, *path, "TetrisPieceShape.J");
                });
                // Color.Red
                assert_expr_path!(parser, parser.tree.get(*body), "Color.Red");
            });

            // _ => Color.Gray
            assert_node!(parser.tree, cases[2], MatchCase::Expression { pattern, body, guard } => {
                assert!(guard.is_none());
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                // Color.Gray
                assert_expr_path!(parser, parser.tree.get(*body), "Color.Gray");
            });
        });
    }
}
