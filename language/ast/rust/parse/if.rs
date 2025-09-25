use crate::parse::expression::ExpressionParserOptions;
use crate::{If, Keyword, NodeId, ParseResult, Parser, Runtime};

impl<'a> Parser<'a> {
    /// Parse an if / else statement.
    ///
    /// Examples:
    /// ```
    /// // if
    /// @if x > 0 {
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
    pub fn eat_if(&mut self, runtime: Option<Runtime>) -> ParseResult<NodeId<If>> {
        let start = self.mark();
        self.eat_keyword(Keyword::If)?;
        let condition_id = self.eat_expression(ExpressionParserOptions {
            is_before_block: true,
            ..ExpressionParserOptions::default()
        })?;
        let then_block_id = self.eat_block()?;
        self.eat_newlines_maybe()?;
        let if_node = if self.peek_keyword(Keyword::Else).is_ok() {
            self.bump(); // eat else
            self.eat_newlines_maybe()?;
            if self.peek_keyword(Keyword::If).is_ok() {
                // if ... else if ...
                let else_if_id = self.eat_if(runtime)?;
                If::IfElseIf {
                    runtime,
                    condition: condition_id,
                    then_block: then_block_id,
                    else_if: else_if_id,
                }
            } else {
                // if ... else ...
                let else_block_id = self.eat_block()?;
                If::IfElse {
                    runtime,
                    condition: condition_id,
                    then_block: then_block_id,
                    else_block: else_block_id,
                }
            }
        } else {
            // if ...
            If::If {
                runtime,
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
    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Block, Expression, If, assert_bool, assert_expr_path, assert_node,
        assert_path,
    };

    #[test]
    fn test_parse_if_basic() {
        let mut test = TestParser::new("if true {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, If::If { condition, then_block, .. } => {
            // condition is boolean true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(lit_id) => {
                assert_bool!(parser.tree, *lit_id, true);
            });
            // empty then block
            assert_node!(parser.tree, *then_block, Block {  format: _, statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_if_else() {
        let mut test = TestParser::new("if false {} else {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, If::IfElse { condition, then_block, else_block, .. } => {
            // condition is boolean false
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(lit_id) => {
                assert_bool!(parser.tree, *lit_id, false);
            });
            // empty then block
            assert_node!(parser.tree, *then_block, Block { format: _, statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
            // empty else block
            assert_node!(parser.tree, *else_block, Block { format: _, statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_if_else_if() {
        let mut test = TestParser::new("if true {} else if false {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, If::IfElseIf { condition, then_block, else_if, .. } => {
            // condition is boolean true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(lit_id) => {
                assert_bool!(parser.tree, *lit_id, true);
            });
            // empty then block
            assert_node!(parser.tree, *then_block, Block { format: _, statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
            // nested else-if should be simple If
            assert_node!(parser.tree, *else_if, If::If { condition: inner_condition, then_block: inner_then, .. } => {
                assert_node!(parser.tree, *inner_condition, Expression::ScalarLiteral(lit_id) => {
                    assert_bool!(parser.tree, *lit_id, false);
                });
                assert_node!(parser.tree, *inner_then, Block { format: _, statements, label } => {
                    assert!(label.is_none());
                    assert!(statements.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if_ambiguous() {
        // ambiguous because y and z could be interpreted as struct literals
        //  (this is disambiguated in a condition / guard clause, see ExpressionParserOptions)
        let mut test = TestParser::new(
            r"
if x > y {
    y
} else if y == z { 
    x 
}",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, If::IfElseIf { condition, then_block, else_if, .. } => {
            // if x > y
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                // x
                assert_expr_path!(parser.session, parser.tree.get(*left), "x");
                // y
                assert_expr_path!(parser.session, parser.tree.get(*right), "y");
            });
            // { y }
            assert_node!(parser.tree, *then_block, Block { format: _, statements, label } => {
                assert!(label.is_none());
                assert_eq!(statements.len(), 1);
            });
            // else if y == z
            assert_node!(parser.tree, *else_if, If::If { condition: inner_condition, then_block: inner_then, .. } => {
                // y == z
                assert_node!(parser.tree, *inner_condition, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::Equal);
                    // y
                    assert_expr_path!(parser.session, parser.tree.get(*left), "y");
                    // z
                    assert_expr_path!(parser.session, parser.tree.get(*right), "z");
                });
                // { x }
                assert_node!(parser.tree, *inner_then, Block { format: _, statements, label } => {
                    assert!(label.is_none());
                    assert_eq!(statements.len(), 1);
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if_else() {
        let mut test = TestParser::new("if true {} else if false {} else {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, If::IfElseIf { condition, then_block, else_if, .. } => {
            // if true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(lit_id) => {
                assert_bool!(parser.tree, *lit_id, true);
            });
            // { }
            assert_node!(parser.tree, *then_block, Block { format: _, statements, label } => {
                assert!(label.is_none());
                assert!(statements.is_empty());
            });
            // else if false { } else { }
            assert_node!(parser.tree, *else_if, If::IfElse { condition: inner_condition, then_block: inner_then, else_block: inner_else, .. } => {
                // else if false
                assert_node!(parser.tree, *inner_condition, Expression::ScalarLiteral(lit_id) => {
                    assert_bool!(parser.tree, *lit_id, false);
                });
                // { }
                assert_node!(parser.tree, *inner_then, Block { format: _, statements, label } => {
                    assert!(label.is_none());
                    assert!(statements.is_empty());
                });
                // else { }
                assert_node!(parser.tree, *inner_else, Block { format: _, statements, label } => {
                    assert!(label.is_none());
                    assert!(statements.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if_else_multiline() {
        let mut test = TestParser::new(
            r"
if v < lo { lo } 
else if v > hi { hi }
else { v }
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, If::IfElseIf { condition, then_block, else_if, .. } => {
            // if v < lo
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                // v
                assert_expr_path!(parser.session, parser.tree.get(*left), "v");
                // lo
                assert_expr_path!(parser.session, parser.tree.get(*right), "lo");
            });
            // { lo }
            assert_node!(parser.tree, *then_block, Block { format: _, statements, label } => {
                assert!(label.is_none());
                assert_eq!(statements.len(), 1);
            });
            // else if v > hi { hi } else { v }
            assert_node!(parser.tree, *else_if, If::IfElse { condition: inner_condition, then_block: inner_then, else_block: inner_else, .. } => {
                // else if v > hi
                assert_node!(parser.tree, *inner_condition, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    // v
                    assert_expr_path!(parser.session, parser.tree.get(*left), "v");
                    // hi
                    assert_expr_path!(parser.session, parser.tree.get(*right), "hi");
                });
                // { hi }
                assert_node!(parser.tree, *inner_then, Block { format: _, statements, label } => {
                    assert!(label.is_none());
                    assert_eq!(statements.len(), 1);
                });
                // else { v }
                assert_node!(parser.tree, *inner_else, Block { format: _, statements, label } => {
                    assert!(label.is_none());
                    assert_eq!(statements.len(), 1);
                });
            });
        });
    }

    #[test]
    fn test_parse_if_with_expression_condition() {
        let mut test = TestParser::new(
            r###"
if x > 0 {
    print("positive")
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, If::If { condition, then_block, .. } => {
            // condition is binary expression x > 0
            assert_node!(parser.tree, *condition, Expression::Binary { left: _, operator: _, right: _ });
            // then block has one statement
            assert_node!(parser.tree, *then_block, Block { format: _, statements, label } => {
                assert!(label.is_none());
                assert_eq!(statements.len(), 1);
            });
        });
    }
}
