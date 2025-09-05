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
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Expression, If, Parser, ScalarLiteral};

    #[test]
    fn test_parse_if_basic() {
        let input = "if true {}";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        let if_id = parser.eat_if().unwrap();
        match parser.tree.get(if_id) {
            If::If {
                condition,
                then_block,
            } => {
                // condition is boolean true
                match parser.tree.get(*condition) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Boolean(b) => assert!(*b),
                        other => panic!("expected boolean true, got {other:?}"),
                    },
                    other => panic!("expected scalar literal, got {other:?}"),
                }
                let block = parser.tree.get(*then_block);
                assert!(block.statements.is_empty());
            }
            _ => panic!("expected simple if variant"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let input = "if false {} else {}";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        let if_id = parser.eat_if().unwrap();
        match parser.tree.get(if_id) {
            If::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                // condition is boolean false
                match parser.tree.get(*condition) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Boolean(b) => assert!(!*b),
                        other => panic!("expected boolean false, got {other:?}"),
                    },
                    other => panic!("expected scalar literal, got {other:?}"),
                }
                let then_b = parser.tree.get(*then_block);
                assert!(then_b.statements.is_empty());
                let else_b = parser.tree.get(*else_block);
                assert!(else_b.statements.is_empty());
            }
            _ => panic!("expected if-else variant"),
        }
    }

    #[test]
    fn test_parse_if_else_if_else() {
        let input = "if true {} else if false {} else {}";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        let top_if_id = parser.eat_if().unwrap();
        // top: IfElseIf
        match parser.tree.get(top_if_id) {
            If::IfElseIf {
                condition,
                then_block,
                else_if,
            } => {
                // top condition is boolean true
                match parser.tree.get(*condition) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Boolean(b) => assert!(*b),
                        other => panic!("expected boolean true, got {other:?}"),
                    },
                    other => panic!("expected scalar literal, got {other:?}"),
                }
                let then_b = parser.tree.get(*then_block);
                assert!(then_b.statements.is_empty());

                // nested else-if should become IfElse with final else
                match parser.tree.get(*else_if) {
                    If::IfElse {
                        condition: inner_condition,
                        then_block: inner_then,
                        else_block: inner_else,
                    } => {
                        // inner condition is boolean false
                        match parser.tree.get(*inner_condition) {
                            Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                                ScalarLiteral::Boolean(b) => assert!(!*b),
                                other => panic!("expected boolean false, got {other:?}"),
                            },
                            other => panic!("expected scalar literal, got {other:?}"),
                        }
                        let inner_then_b = parser.tree.get(*inner_then);
                        assert!(inner_then_b.statements.is_empty());
                        let inner_else_b = parser.tree.get(*inner_else);
                        assert!(inner_else_b.statements.is_empty());
                    }
                    other => panic!("expected nested IfElse, got {other:?}"),
                }
            }
            _ => panic!("expected top-level IfElseIf variant"),
        }
    }
}
