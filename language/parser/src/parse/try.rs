use crate::{ParseResult, Parser};
use dyst_ast::{Expression, Keyword, LocalNodeId};

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
    ///
    /// try {
    ///     ...
    /// } catch e {
    ///     ... // regular catch
    /// }
    ///
    ///
    /// try { // explicitly unwraps all Results inside
    ///     ...
    /// } catch e { // match all errors
    ///     NumericError(x) => Error(@format("bad number: {x}"))
    ///     FormatError => Error(@format("bad format {e}"))
    ///     // it's exhaustive! otherwise `_ =>` like in match (it is a match)
    /// } finally {
    ///     ...
    /// }
    /// ```
    pub fn eat_try(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Try)?;

        // try block
        if self.peek_block().is_ok() {
            // try block
            let try_expression = self.eat_block()?;
            let try_expression = self
                .tree
                .insert(Expression::Block(try_expression), self.get_span_from(start));

            // catch
            let (catch_expression, catch_pattern) = if self.peek_keyword(Keyword::Catch).is_ok() {
                self.bump(); // eat keyword
                // no pattern or catch match
                if self.peek_block().is_ok() || self.peek_keyword(Keyword::Match).is_ok() {
                    let catch_expression = self.with_options(
                        self.options.not_in_position().in_statement_position(),
                        |parser| parser.eat_expression(),
                    )?;
                    (Some(catch_expression), None)
                }
                // catch pattern with expression content
                else {
                    let catch_pattern = self.with_options(
                        self.options.not_in_position().in_before_block(),
                        |parser| parser.eat_pattern(),
                    )?;
                    let catch_expression = self.with_options(
                        self.options.not_in_position().in_statement_position(),
                        |parser| parser.eat_expression(),
                    )?;
                    (Some(catch_expression), Some(catch_pattern))
                }
            } else {
                (None, None)
            };

            // finally
            let finally_expression = if self.peek_keyword(Keyword::Finally).is_ok() {
                self.bump(); // eat keyword
                let finally_expression = self.with_options(
                    self.options.not_in_position().in_statement_position(),
                    |parser| parser.eat_expression(),
                )?;
                Some(finally_expression)
            } else {
                None
            };

            // try
            let try_id = self.tree.insert(
                Expression::Try {
                    try_expression,
                    catch_pattern,
                    catch_expression,
                    finally_expression,
                },
                self.get_span_from(start),
            );
            Ok(try_id)
        }
        // try expression
        else {
            let expression_id = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let try_id = self.tree.insert(
                Expression::Try {
                    try_expression: expression_id,
                    catch_pattern: None,
                    catch_expression: None,
                    finally_expression: None,
                },
                self.get_span_from(start),
            );
            Ok(try_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{Block, Expression, Pattern};

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_try_expression() {
        let mut test = TestParser::new(
            r###"
try foo()
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { try_expression, catch_pattern: None, catch_expression: None, finally_expression: None } => {
            assert_node!(parser.tree, *try_expression, Expression::Call { position: _, left, static_arguments: _, dynamic_arguments: _ } => {
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
            });
        });
    }

    #[test]
    fn test_try_expression_without_catch_or_finally() {
        let mut test = TestParser::new(
            r###"
try {
    foo()
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { try_expression, catch_pattern: None, catch_expression: None, finally_expression: None } => {
            assert_node!(parser.tree, *try_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    assert_node!(parser.tree, expressions[0], Expression::Call { position: _, left, static_arguments: _, dynamic_arguments: _ } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    });
                });
            });
        });
    }

    #[test]
    fn test_try_expression_with_catch() {
        let mut test = TestParser::new(
            r###"
try {
    foo()
} catch e {
    bar()
} finally {
    baz()
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { try_expression, catch_pattern: Some(catch_pattern), catch_expression: Some(catch_expression), finally_expression: Some(finally_expression) } => {
            // try
            assert_node!(parser.tree, *try_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    assert_node!(parser.tree, expressions[0], Expression::Call { position: _, left, static_arguments: _, dynamic_arguments: _ } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    });
                });
            });
            // catch e
            assert_node!(parser.tree, *catch_pattern, Pattern::Binding { mutability: _, name, pattern: _ } => {
                assert_string!(parser, *name, "e");
            });
            // catch expression
            assert_node!(parser.tree, *catch_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    assert_node!(parser.tree, expressions[0], Expression::Call { position: _, left, static_arguments: _, dynamic_arguments: _ } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "bar");
                    });
                });
            });
            // finally expression
            assert_node!(parser.tree, *finally_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    assert_node!(parser.tree, expressions[0], Expression::Call { position: _, left, static_arguments: _, dynamic_arguments: _ } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "baz");
                    });
                });
            });
        });
    }
}
