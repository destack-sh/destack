use dyst_token::TokenType;

use crate::{AstResult, Expression, Keyword, NodeId, Parser, Runtime};

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
    pub fn eat_try(&mut self, runtime: Option<Runtime>) -> AstResult<NodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Try)?;

        // try block
        if self.peek_block().is_ok() {
            // try block
            let block_id = self.eat_block()?;
            let block_id = self
                .tree
                .allocate(Expression::Block(block_id), self.get_span_from(start));

            // try block with catch
            if self.peek_keyword(Keyword::Catch).is_ok() {
                self.bump(); // eat keyword

                // catch expression
                let catch_scrutinee_id = self
                    .with_options(self.options.in_before_block(), |parser| {
                        parser.eat_expression()
                    })?;

                // catch match cases
                self.eat_token(TokenType::OpenBrace)?;
                let catch_match_cases_id = self.eat_match_cases()?;
                self.eat_token(TokenType::CloseBrace)?;

                // catch match
                let catch_match_id = self.tree.allocate(
                    Expression::Match {
                        runtime,
                        value: catch_scrutinee_id,
                        cases: catch_match_cases_id,
                    },
                    self.get_span_from(start),
                );
                let try_id = self.tree.allocate(
                    Expression::Try {
                        runtime,
                        try_block: block_id,
                        catch_block: Some(catch_match_id),
                    },
                    self.get_span_from(start),
                );
                Ok(try_id)
            }
            // try block without catch
            else {
                let try_id = self.tree.allocate(
                    Expression::Try {
                        runtime,
                        try_block: block_id,
                        catch_block: None,
                    },
                    self.get_span_from(start),
                );
                Ok(try_id)
            }
        }
        // try expression
        else {
            let expression_id = self.eat_expression()?;
            let try_id = self.tree.allocate(
                Expression::Try {
                    runtime,
                    try_block: expression_id,
                    catch_block: None,
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
        Block, Expression, MatchCase, Pattern, assert_expr_path, assert_node, assert_path,
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

        let try_id = parser.eat_try(None).unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { runtime: _, try_block, catch_block: _ } => {
            assert_node!(parser.tree, *try_block, Expression::Call { runtime: _, receiver, dynamic_arguments: _ } => {
                assert_expr_path!(parser.session, parser.tree.get(*receiver), "foo");
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

        let try_id = parser.eat_try(None).unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { runtime: _, try_block, catch_block: _ } => {
            assert_node!(parser.tree, *try_block, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    assert_node!(parser.tree, expressions[0], Expression::Call { runtime: _, receiver, dynamic_arguments: _ } => {
                        assert_expr_path!(parser.session, parser.tree.get(*receiver), "foo");
                    });
                });
            });
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

        let try_id = parser.eat_try(None).unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { runtime: _, try_block: _, catch_block } => {
            // catch match: value is path e, one case with wildcard and block body
            assert_node!(parser.tree, catch_block.unwrap(), Expression::Match { runtime: _, value, cases } => {
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
