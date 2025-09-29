use dyst_token::TokenType;

use crate::parse::expression::ExpressionParserOptions;
use crate::{Keyword, Match, NodeId, ParseResult, Parser, Try};

impl<'a> Parser<'a> {
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
                self.bump(); // eat keyword

                // catch expression
                let catch_expression_id = self.eat_expression(ExpressionParserOptions {
                    is_before_block: true,
                    ..ExpressionParserOptions::default()
                })?;

                // catch match cases
                self.eat_token(TokenType::OpenBrace)?;
                let catch_match_cases_id = self.eat_match_cases()?;
                self.eat_token(TokenType::CloseBrace)?;

                // catch match
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
        Expression, Match, MatchCase, Pattern, Try, assert_expr_path, assert_node, assert_path,
    };

    #[test]
    fn test_try_expression() {
        let mut test = TestParser::new(
            r###"
try foo()
"###,
        );
        let mut parser = test.prepare();
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
        let mut parser = test.prepare();
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
        let mut parser = test.prepare();
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
