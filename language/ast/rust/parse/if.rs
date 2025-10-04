use crate::{Expression, Keyword, NodeId, AstError, AstResult, Parser, Runtime};

impl<'a> Parser<'a> {
    /// Parse an if / else expression.
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
    pub fn eat_if(&mut self, runtime: Option<Runtime>) -> AstResult<NodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::If)?;

        // condition
        let condition_id = self.with_options(self.options.nested_in_before_block(), |parser| {
            parser.eat_expression()
        })?;

        // then block
        let then_block_id = self.eat_block()?;
        self.eat_newlines_maybe()?;

        // if / else if / else node
        let if_node = if self.peek_keyword(Keyword::Else).is_ok() {
            self.bump(); // eat else
            self.eat_newlines_maybe()?;
            let else_expr_id = self.eat_expression()?;
            
            // else expression must be an if or block expression
            let else_expr_node = self.tree.get(else_expr_id);
            match else_expr_node {
                Expression::If { .. } | Expression::Block(_) => {}
                _ => {
                    let else_span = self.tree.get_span_by_id(else_expr_id.id);
                    return Err(AstError::unexpected(else_span));
                }
            }

            Expression::If {
                runtime,
                condition: condition_id,
                then_block: then_block_id,
                else_block: Some(else_expr_id),
            }
        } else {
            // if ...
            Expression::If {
                runtime,
                condition: condition_id,
                then_block: then_block_id,
                else_block: None,
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
        BinaryOperator, Block, Expression, ScalarLiteral, assert_expr_path, assert_node,
        assert_path,
    };

    #[test]
    fn test_parse_if_basic() {
        let mut test = TestParser::new("if true {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_block, .. } => {
            // condition is boolean true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // empty then block
            assert_node!(parser.tree, *then_block, Block {  format: _, expressions, label } => {
                assert!(label.is_none());
                assert!(expressions.is_empty());
            });
        });
    }

    #[test]
    fn test_parse_if_else() {
        let mut test = TestParser::new("if false {} else {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_block, else_block, .. } => {
            // condition is boolean false
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
            // empty then block
            assert_node!(parser.tree, *then_block, Block { format: _, expressions, label } => {
                assert!(label.is_none());
                assert!(expressions.is_empty());
            });
            // empty else block
            assert_node!(parser.tree, else_block.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert!(expressions.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if() {
        let mut test = TestParser::new("if true {} else if false {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_block, else_block, .. } => {
            // true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // then block
            assert_node!(parser.tree, *then_block, Block { format: _, expressions, label } => {
                assert!(label.is_none());
                assert!(expressions.is_empty());
            });
            // nested else-if should be simple If
            assert_node!(parser.tree, else_block.unwrap(), Expression::If { condition: inner_condition, then_block: inner_then, .. } => {
                assert_node!(parser.tree, *inner_condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                assert_node!(parser.tree, *inner_then, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert!(expressions.is_empty());
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
        assert_node!(parser.tree, if_id, Expression::If { condition, then_block, else_block, .. } => {
            // if x > y
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                // x
                assert_expr_path!(parser.session, parser.tree.get(*left), "x");
                // y
                assert_expr_path!(parser.session, parser.tree.get(*right), "y");
            });
            // { y }
            assert_node!(parser.tree, *then_block, Block { format: _, expressions, label } => {
                assert!(label.is_none());
                assert_eq!(expressions.len(), 1);
            });
            // else if y == z
            assert_node!(parser.tree, else_block.unwrap(), Expression::If { condition: inner_condition, then_block: inner_then, .. } => {
                // y == z
                assert_node!(parser.tree, *inner_condition, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::Equal);
                    // y
                    assert_expr_path!(parser.session, parser.tree.get(*left), "y");
                    // z
                    assert_expr_path!(parser.session, parser.tree.get(*right), "z");
                });
                // { x }
                assert_node!(parser.tree, *inner_then, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if_else() {
        let mut test = TestParser::new("if true {} else if false {} else {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if(None).unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_block, else_block, .. } => {
            // if true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // { }
            assert_node!(parser.tree, *then_block, Block { format: _, expressions, label } => {
                assert!(label.is_none());
                assert!(expressions.is_empty());
            });
            // else if false { } else { }
            assert_node!(parser.tree, else_block.unwrap(), Expression::If { condition: inner_condition, then_block: inner_then, else_block: inner_else, .. } => {
                // else if false
                assert_node!(parser.tree, *inner_condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                // { }
                assert_node!(parser.tree, *inner_then, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert!(expressions.is_empty());
                });
                // else { }
                assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                        assert!(label.is_none());
                        assert!(expressions.is_empty());
                    });
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
        assert_node!(parser.tree, if_id, Expression::If { condition, then_block, else_block, .. } => {
            // if v < lo
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                // v
                assert_expr_path!(parser.session, parser.tree.get(*left), "v");
                // lo
                assert_expr_path!(parser.session, parser.tree.get(*right), "lo");
            });
            // { lo }
            assert_node!(parser.tree, *then_block, Block { format: _, expressions, label } => {
                assert!(label.is_none());
                assert_eq!(expressions.len(), 1);
            });
            // else if v > hi { hi } else { v }
            assert_node!(parser.tree, else_block.unwrap(), Expression::If { condition: inner_condition, then_block: inner_then, else_block: inner_else, .. } => {
                // else if v > hi
                assert_node!(parser.tree, *inner_condition, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    // v
                    assert_expr_path!(parser.session, parser.tree.get(*left), "v");
                    // hi
                    assert_expr_path!(parser.session, parser.tree.get(*right), "hi");
                });
                // { hi }
                assert_node!(parser.tree, *inner_then, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                });
                // else { v }
                assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                        assert!(label.is_none());
                        assert_eq!(expressions.len(), 1);
                    });
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
        assert_node!(parser.tree, if_id, Expression::If { condition, then_block, .. } => {
            // condition is binary expression x > 0
            assert_node!(parser.tree, *condition, Expression::Binary { left: _, operator: _, right: _ });
            // then block has one expression
            assert_node!(parser.tree, *then_block, Block { format: _, expressions, label } => {
                assert!(label.is_none());
                assert_eq!(expressions.len(), 1);
            });
        });
    }
}
