use crate::{ParseResult, Parser};
use dyst_ast::{Block, BlockFormat, Expression, IfKind, Keyword, LocalNodeId};

impl Parser {
    /// Eat something as a block (if it's not a block expression OR an if, wrap in a block expression).
    fn eat_expression_as_block(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        let expression_id = self.eat_expression()?;
        if !matches!(self.tree.get(expression_id), Expression::Block { .. })
            && !matches!(self.tree.get(expression_id), Expression::If { .. })
        {
            let block_id = self.tree.insert(
                Block {
                    format: BlockFormat::Explicit,
                    label: None,
                    expressions: vec![expression_id],
                },
                self.get_span_from(start),
            );
            let expression_id = self
                .tree
                .insert(Expression::Block(block_id), self.get_span_from(start));
            Ok(expression_id)
        } else {
            Ok(expression_id)
        }
    }

    /// Parse an if / else expression.
    ///
    /// Examples:
    /// ```
    /// // ternary
    /// cond ? a : b
    ///
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
    pub fn eat_if(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // NOTE: ternary if is parsed in expression parser, not in eat_if

        // keyword
        self.eat_keyword(Keyword::If)?;

        // condition
        let condition_id = self
            .with_options(self.options.nested().in_before_block(), |parser| {
                parser.eat_expression()
            })?;
        self.eat_newlines_maybe()?;

        // then block
        let then_expression_id = self
            .with_options(self.options.in_statement_position(), |parser| {
                parser.eat_expression_as_block()
            })?;

        // if / else if / else node
        let if_node = if self.peek_keyword_after_newlines(Keyword::Else).is_ok() {
            self.eat_newlines_maybe()?;
            self.eat_keyword(Keyword::Else)?;
            self.eat_newlines_maybe()?;
            let else_expr_id = self
                .with_options(self.options.in_statement_position(), |parser| {
                    parser.eat_expression_as_block()
                })?;
            Expression::If {
                kind: IfKind::If,
                condition: condition_id,
                then_expression: then_expression_id,
                else_expression: Some(else_expr_id),
            }
        } else {
            // if ...
            Expression::If {
                kind: IfKind::If,
                condition: condition_id,
                then_expression: then_expression_id,
                else_expression: None,
            }
        };

        let if_id = self.tree.insert(if_node, self.get_span_from(start));
        Ok(if_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{BinaryOperator, Block, Expression, ScalarLiteral};

    use crate::{TestParser, assert_expression_path, assert_node, assert_path};

    #[test]
    fn test_parse_if_basic() {
        let mut test = TestParser::new("if true {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
            // condition is boolean true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // empty then block
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert!(expressions.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_with_empty_blocks() {
        let mut test = TestParser::new("if false {} else {}");
        let mut parser = test.prepare();

        // if false { } else { }
        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // false
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
            // { }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert!(expressions.is_empty());
                });
            });
            // { }
            assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert!(expressions.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_parenthesized_with_trivial_blocks() {
        let mut test = TestParser::new("if (cond) { a } else { b }");
        let mut parser = test.prepare();

        // if (cond) { a } else { b }
        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // (cond)
            assert_node!(parser.tree, *condition, Expression::Parenthesized { expression } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "cond");
            });
            // { a }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                    assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
                });
            });
            // { b }
            assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                    assert_expression_path!(parser, parser.tree.get(expressions[0]), "b");
                });
            });
        });
    }

    #[test]
    fn test_parse_if_with_nested_parenthesized_condition() {
        let mut test = TestParser::new(
            r"
if cond {
    if (cond) {
        a
    } else {
        b
    }
}
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // if cond { if (cond) { a } else { b } }
        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // cond
            assert_expression_path!(parser, parser.tree.get(*condition), "cond");
            // { if (cond) { a } else { b } }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                    // if (cond) { a } else { b }
                    assert_node!(parser.tree, expressions[0], Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
                        // (cond)
                        assert_node!(parser.tree, *inner_condition, Expression::Parenthesized { expression } => {
                            assert_expression_path!(parser, parser.tree.get(*expression), "cond");
                        });
                        // { a }
                        assert_node!(parser.tree, *inner_then, Expression::Block(inner_block_id) => {
                            assert_node!(parser.tree, *inner_block_id, Block { format: _, expressions, label } => {
                                assert!(label.is_none());
                                assert_eq!(expressions.len(), 1);
                                assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
                            });
                        });
                        // { b }
                        assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(inner_block_id) => {
                            assert_node!(parser.tree, *inner_block_id, Block { format: _, expressions, label } => {
                                assert!(label.is_none());
                                assert_eq!(expressions.len(), 1);
                                assert_expression_path!(parser, parser.tree.get(expressions[0]), "b");
                            });
                        });
                    });
                });
            });
            assert!(else_expression.is_none());
        });
    }

    #[test]
    fn test_parse_if_else_if_with_empty_blocks() {
        let mut test = TestParser::new("if true {} else if false {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // then block
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert!(expressions.is_empty());
                });
            });
            // nested else-if should be simple If
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, .. } => {
                assert_node!(parser.tree, *inner_condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                        assert!(label.is_none());
                        assert!(expressions.is_empty());
                    });
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

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // if x > y
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                // x
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                // y
                assert_expression_path!(parser, parser.tree.get(*right), "y");
            });
            // { y }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                });
            });
            // else if y == z
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, .. } => {
                // y == z
                assert_node!(parser.tree, *inner_condition, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::Equal);
                    // y
                    assert_expression_path!(parser, parser.tree.get(*left), "y");
                    // z
                    assert_expression_path!(parser, parser.tree.get(*right), "z");
                });
                // { x }
                assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                        assert!(label.is_none());
                        assert_eq!(expressions.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if_else_with_empty_blocks() {
        let mut test = TestParser::new("if true {} else if false {} else {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // if true
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // { }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert!(expressions.is_empty());
                });
            });
            // else if false { } else { }
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
                // else if false
                assert_node!(parser.tree, *inner_condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                // { }
                assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                        assert!(label.is_none());
                        assert!(expressions.is_empty());
                    });
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

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // if v < lo
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                // v
                assert_expression_path!(parser, parser.tree.get(*left), "v");
                // lo
                assert_expression_path!(parser, parser.tree.get(*right), "lo");
            });
            // { lo }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                });
            });
            // else if v > hi { hi } else { v }
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
                // else if v > hi
                assert_node!(parser.tree, *inner_condition, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    // v
                    assert_expression_path!(parser, parser.tree.get(*left), "v");
                    // hi
                    assert_expression_path!(parser, parser.tree.get(*right), "hi");
                });
                // { hi }
                assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                        assert!(label.is_none());
                        assert_eq!(expressions.len(), 1);
                    });
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
if x > y {
    print("positive")
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
            // condition is binary expression x > y
            assert_node!(parser.tree, *condition, Expression::Binary { left: _, operator: _, right: _ });
            // then block has one expression
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                });
            });
        });
    }

    /// If the then expression is not a block, it should be coerced to a block.
    #[test]
    fn test_parse_if_else_if_coerce_to_block() {
        let mut test = TestParser::new(
            r###"
if x
    x
else if y
    y
else
    z
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // if x
            assert_expression_path!(parser, parser.tree.get(*condition), "x");
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                });
            });
            // else if y
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition, then_expression, else_expression, .. } => {
                assert_expression_path!(parser, parser.tree.get(*condition), "y");
                assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                        assert!(label.is_none());
                        assert_eq!(expressions.len(), 1);
                    });
                });
                // else z
                assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                        assert!(label.is_none());
                        assert_eq!(expressions.len(), 1);
                    });
                });
            });
        });
    }
}
