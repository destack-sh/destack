//! Parse loops, for, while, etc.

use crate::parse::prelude::*;
use crate::{For, Keyword, Loop, NodeId, NodeType, ParseResult, Parser, Runtime, While};

impl<'a> Parser<'a> {
    /// Eat a loop (e.g., `loop { ... }`).
    ///
    /// Examples:
    /// ```
    /// loop {
    ///     y = getNext()
    ///     if y < 0 {
    ///         break
    ///     }
    /// }
    /// ```
    pub fn eat_loop(&mut self, runtime: Option<Runtime>) -> ParseResult<NodeId<Loop>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Loop)?;

        // body
        let block_id = self.eat_block().for_node_type(NodeType::Loop)?;

        // loop
        let loop_id = self.tree.allocate(
            Loop {
                runtime,
                body: block_id,
            },
            self.get_span_from(start),
        );
        Ok(loop_id)
    }

    /// Eat a for loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// for item in items {
    ///     item
    /// }
    ///
    /// @for x in 1..10 {
    ///     y = 2
    /// }
    ///
    /// for x in zeds.iter() a: {
    ///     if y > 5 {
    ///         continue :a
    ///     }
    ///     y = 2
    /// }
    /// ```
    pub fn eat_for(&mut self, runtime: Option<Runtime>) -> ParseResult<NodeId<For>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::For)?;

        // pattern
        let pattern_id = self
            .with_options(self.options.in_before_block(), |parser| {
                parser.eat_pattern()
            })
            .for_node_type(NodeType::For)?;

        // in
        self.eat_keyword(Keyword::In)?;

        // iterator
        let iterator_id = self
            .with_options(self.options.nested_in_before_block(), |parser| {
                parser.eat_expression()
            })
            .for_node_type(NodeType::For)?;

        // body
        let block_id = self.eat_block().for_node_type(NodeType::For)?;

        // for
        let for_id = self.tree.allocate(
            For {
                runtime,
                pattern: pattern_id,
                iterator: iterator_id,
                body: block_id,
            },
            self.get_span_from(start),
        );
        Ok(for_id)
    }

    /// Eat a while loop (including keyword and header).
    ///
    /// Examples:
    /// ```
    /// @while x > 1 {
    ///     y = 2
    /// }
    ///
    /// while y < 10 l: {
    ///     y = 2
    ///     break :l
    /// }
    /// ```
    pub fn eat_while(&mut self, runtime: Option<Runtime>) -> ParseResult<NodeId<While>> {
        let start = self.mark();

        // header
        self.eat_keyword(Keyword::While)?;

        // condition
        let condition_id = self
            .with_options(self.options.in_before_block(), |parser| {
                parser.eat_expression()
            })
            .for_node_type(NodeType::While)?;

        // body
        let block_id = self.eat_block().for_node_type(NodeType::While)?;

        // while
        let while_id = self.tree.allocate(
            While {
                runtime,
                condition: condition_id,
                body: block_id,
            },
            self.get_span_from(start),
        );
        Ok(while_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Block, Expression, For, Loop, Pattern, While, assert_expr_path,
        assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_loop() {
        let mut test = TestParser::new(
            r###"
loop {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let loop_id = parser.eat_loop(None).unwrap();
        assert_node!(parser.tree, loop_id, Loop { body, .. } => {
            let _block = parser.tree.get(*body);
        });
    }

    #[test]
    fn test_parse_for_loop() {
        let mut test = TestParser::new(
            r###"
for item in items {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for(None).unwrap();
        assert_node!(parser.tree, for_id, For { pattern, iterator, body: _, .. } => {
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { name } => {
                assert_string!(parser.session, *name, "item");
            });
            // in items
            assert_expr_path!(parser.session, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_for_loop_with_label() {
        let mut test = TestParser::new(
            r###"
for item in items outer: {
    x
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let for_id = parser.eat_for(None).unwrap();
        assert_node!(parser.tree, for_id, For { pattern, iterator, body: _, .. } => {
            // item
            assert_node!(parser.tree, *pattern, Pattern::Binding { name } => {
                assert_string!(parser.session, *name, "item");
            });
            // in items
            assert_expr_path!(parser.session, parser.tree.get(*iterator), "items");
        });
    }

    #[test]
    fn test_parse_while_loop() {
        let mut test = TestParser::new(
            r###"
while x {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let while_id = parser.eat_while(None).unwrap();
        assert_node!(parser.tree, while_id, While { condition, body: _, .. } => {
            assert_expr_path!(parser.session, parser.tree.get(*condition), "x");
        });
    }

    #[test]
    fn test_parse_while_loop_nested() {
        let mut test = TestParser::new(
            r###"
while x > y {
    while a < b {
        inner_work()
    }
    outer_work()
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let while_id = parser.eat_while(None).unwrap();

        // while x > y
        assert_node!(parser.tree, while_id, While { condition, body, .. } => {
            // x > y
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                // x
                assert_expr_path!(parser.session, parser.tree.get(*left), "x");
                // >
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                // y
                assert_expr_path!(parser.session, parser.tree.get(*right), "y");
            });

            assert_node!(parser.tree, *body, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 2);

                // while a < b
                assert_node!(parser.tree, expressions[0], Expression::While(nested_while_id) => {
                    assert_node!(parser.tree, *nested_while_id, While { condition: nested_condition, body: _, .. } => {
                        // a < b
                        assert_node!(parser.tree, *nested_condition, Expression::Binary { left, operator, right } => {
                            // a
                            assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                            // <
                            assert_eq!(*operator, BinaryOperator::LessThan);
                            // b
                            assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                        });
                    });
                });
            });
        });
    }
}
