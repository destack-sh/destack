use crate::{If, Keyword, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Parse an if / else statement.
    ///
    /// Examples:
    /// ```
    /// // if
    /// if x > 0 {
    ///     print("positive")
    /// }
    ///
    /// // if else
    /// if x > 0 {
    ///     print("positive")
    /// } else {
    ///     print("not positive")
    /// }
    ///
    /// // if else if
    /// if x > 0 {
    ///     print("positive")
    /// } else if x == 0 {
    ///     print("zero")
    /// } else {
    ///     print("negative")
    /// }
    /// ```
    pub fn eat_if(&mut self) -> ParseResult<NodeId<If>> {
        let start = self.mark();
        self.eat_keyword(Keyword::If)?;
        let condition_id = self.eat_expression(None)?;
        let then_block_id = self.eat_block()?;
        let if_node = if self.peek_keyword(Keyword::Else).is_ok() {
            self.eat_keyword(Keyword::Else)?;
            if self.peek_keyword(Keyword::If).is_ok() {
                // if ... else if ...
                let else_if_id = self.eat_if()?;
                If::IfElseIf {
                    condition: condition_id,
                    then_block: then_block_id,
                    else_if: else_if_id,
                }
            } else {
                // if ... else ...
                let else_block_id = self.eat_block()?;
                If::IfElse {
                    condition: condition_id,
                    then_block: then_block_id,
                    else_block: else_block_id,
                }
            }
        } else {
            // if ...
            If::If {
                condition: condition_id,
                then_block: then_block_id,
            }
        };

        let if_id = self.tree.allocate(if_node, self.get_span_from(start));
        Ok(if_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParse;
    use crate::{Block, Expression, If, assert_bool, assert_node};

    #[test]
    fn test_parse_if_basic() {
        let test = TestParse::new("if true {}");
        let mut parser = test.parser();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, If::If { condition, then_block } => {
            // condition is boolean true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(lit_id) => {
                assert_bool!(parser.tree, *lit_id, true);
            });
            // empty then block
            assert_node!(parser.tree, *then_block, Block { statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_if_else() {
        let test = TestParse::new("if false {} else {}");
        let mut parser = test.parser();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, If::IfElse { condition, then_block, else_block } => {
            // condition is boolean false
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(lit_id) => {
                assert_bool!(parser.tree, *lit_id, false);
            });
            // empty then block
            assert_node!(parser.tree, *then_block, Block { statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
            // empty else block
            assert_node!(parser.tree, *else_block, Block { statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_if_else_if() {
        let test = TestParse::new("if true {} else if false {}");
        let mut parser = test.parser();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, If::IfElseIf { condition, then_block, else_if } => {
            // condition is boolean true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(lit_id) => {
                assert_bool!(parser.tree, *lit_id, true);
            });
            // empty then block
            assert_node!(parser.tree, *then_block, Block { statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
            // nested else-if should be simple If
            assert_node!(parser.tree, *else_if, If::If { condition: inner_condition, then_block: inner_then } => {
                assert_node!(parser.tree, *inner_condition, Expression::ScalarLiteral(lit_id) => {
                    assert_bool!(parser.tree, *lit_id, false);
                });
                assert_node!(parser.tree, *inner_then, Block { statements, label } => {
                    assert!(label.is_none());
                    assert!(statements.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if_else() {
        let test = TestParse::new("if true {} else if false {} else {}");
        let mut parser = test.parser();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, If::IfElseIf { condition, then_block, else_if } => {
            // condition is boolean true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(lit_id) => {
                assert_bool!(parser.tree, *lit_id, true);
            });
            // empty then block
            assert_node!(parser.tree, *then_block, Block { statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
            // nested else-if should become IfElse with final else
            assert_node!(parser.tree, *else_if, If::IfElse { condition: inner_condition, then_block: inner_then, else_block: inner_else } => {
                assert_node!(parser.tree, *inner_condition, Expression::ScalarLiteral(lit_id) => {
                    assert_bool!(parser.tree, *lit_id, false);
                });
                assert_node!(parser.tree, *inner_then, Block { statements, label } => {
                    assert!(label.is_none());
                    assert!(statements.is_empty());
                });
                assert_node!(parser.tree, *inner_else, Block { statements, label } => {
                    assert!(label.is_none());
                    assert!(statements.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_with_expression_condition() {
        let test = TestParse::new(
            r###"
if x > 0 {
    print("positive")
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, If::If { condition, then_block } => {
            // condition is binary expression x > 0
            assert_node!(parser.tree, *condition, Expression::Binary { lhs: _, operator: _, rhs: _ });
            // then block has one statement
            assert_node!(parser.tree, *then_block, Block { statements, label } => {
                assert!(label.is_none());
                assert_eq!(statements.len(), 1);
            });
        });
    }
}
