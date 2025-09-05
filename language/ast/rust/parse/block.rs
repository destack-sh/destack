use crate::{
    Block, Break, Continue, Defer, Keyword, NodeId, ParseError, ParseResult, Parser, Return,
    Statement,
};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Peek a block (test with and without label).
    pub fn peek_block(&self) -> ParseResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_token(TokenType::Identifier).is_ok()
                && self.peek_next_token(TokenType::Colon).is_ok()
                && self.peek_next_next_token(TokenType::OpenBrace).is_ok()
        {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(self.peek()?.span))
        }
    }

    /// Eat a block (including the label, `{`, and `}`).
    pub fn eat_block(&mut self) -> ParseResult<NodeId<Block>> {
        let start = self.mark();
        // label
        let label = if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let label = self.eat_identifier()?;
            self.eat_colon()?;
            Some(label)
        } else {
            None
        };
        // body
        self.eat_token(TokenType::OpenBrace)?;
        let block_id = self.eat_block_body()?;
        let block = self.tree.get_mut(block_id);
        block.label = label;
        self.eat_token(TokenType::CloseBrace)?;
        self.tree.set_span(block_id, self.get_span_from(start));
        Ok(block_id)
    }

    /// Eat a block of statements (without the label, `{`, and `}`)
    pub fn eat_block_body(&mut self) -> ParseResult<NodeId<Block>> {
        let start = self.mark();
        let mut statements: Vec<NodeId<Statement>> = Vec::new();

        loop {
            // break if we're at the end of the block
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // consume any statement stops (semicolon or newline)
            else if self.peek_statement_stop().is_ok() {
                self.eat_statement_stop()?;
            }
            // keep eating statements
            else {
                let statement_id = self.eat_statement()?;
                statements.push(statement_id);
            }
        }

        let block = Block {
            label: None,
            statements,
        };
        let block_id = self.tree.allocate(block, self.get_span_from(start));
        Ok(block_id)
    }

    /// Eat a break statement.
    ///
    /// Examples:
    /// ```
    /// break
    /// break :label
    /// break :label 17
    /// break 15
    /// ```
    pub fn eat_break(&mut self) -> ParseResult<NodeId<Break>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Break)?;
        // label
        let label = if self.peek_token(TokenType::Colon).is_ok() {
            self.eat_colon()?;
            Some(self.eat_identifier()?)
        } else {
            None
        };
        // value (if not at a statement stop)
        let value_id = if self.peek_statement_stop().is_err() {
            let value_id = self.eat_expression(None)?;
            Some(value_id)
        } else {
            None
        };
        // break
        let break_id = self.tree.allocate(
            Break {
                label,
                value: value_id,
            },
            self.get_span_from(start),
        );
        Ok(break_id)
    }

    /// Eat a continue statement.
    ///
    /// Examples:
    /// ```
    /// continue
    /// continue :label
    /// ```
    pub fn eat_continue(&mut self) -> ParseResult<NodeId<Continue>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Continue)?;
        // label
        let label = if self.peek_token(TokenType::Colon).is_ok() {
            self.eat_colon()?;
            Some(self.eat_identifier()?)
        } else {
            None
        };
        // continue
        let continue_id = self
            .tree
            .allocate(Continue { label }, self.get_span_from(start));
        Ok(continue_id)
    }

    /// Eat a return statement.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 17
    /// ```
    pub fn eat_return(&mut self) -> ParseResult<NodeId<Return>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Return)?;
        // value
        let value_id = if self.peek_statement_stop().is_err() {
            let value_id = self.eat_expression(None)?;
            Some(value_id)
        } else {
            None
        };
        // return
        let return_id = self
            .tree
            .allocate(Return { value: value_id }, self.get_span_from(start));
        Ok(return_id)
    }

    /// Eat a defer statement.
    ///
    /// Examples:
    /// ```
    /// defer someFunction()
    ///
    /// defer {
    ///     someFunction()
    ///     someOtherFunction()
    /// }
    ///
    /// defer :label {
    ///     someOtherFunction()
    /// }
    /// ```
    pub fn eat_defer(&mut self) -> ParseResult<NodeId<Defer>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Defer)?;
        // block
        if self.peek_block().is_ok() {
            let block_id = self.eat_block()?;
            let defer_id = self
                .tree
                .allocate(Defer::Block(block_id), self.get_span_from(start));
            Ok(defer_id)
        }
        // statement
        else {
            let expression_id = self.eat_expression(None)?;
            let defer_id = self
                .tree
                .allocate(Defer::Expression(expression_id), self.get_span_from(start));
            Ok(defer_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Expression, Parser, ScalarLiteral};

    #[test]
    fn test_eat_empty_block() {
        let input = r###"{
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        let block_id = parser.eat_block().unwrap();
        let block = parser.tree.get(block_id);
        assert_eq!(block.label, None);
        assert!(block.statements.is_empty());
    }

    #[test]
    fn test_eat_labeled_empty_block() {
        let input = r###"lbl: {
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        let block_id = parser.eat_block().unwrap();
        let block = parser.tree.get(block_id);
        assert_eq!(block.label, Some(parser.strings.intern("lbl")));
        assert!(block.statements.is_empty());
    }

    #[test]
    fn test_eat_block_body_with_statement_separators() {
        let input = r###"{
    // statements tested elsewhere; ensure separators consumed
    
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        let block_id = parser.eat_block().unwrap();
        let block = parser.tree.get(block_id);
        assert!(block.statements.is_empty());
    }

    #[test]
    fn test_break_variants() {
        let input = r###"
break
break :l
break :l 17
break 15
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        // break
        parser.eat_newline().unwrap();
        let b0 = parser.eat_break().unwrap();
        let b0 = parser.tree.get(b0);
        assert!(b0.label.is_none());
        assert!(b0.value.is_none());

        // break :l
        parser.eat_newline().unwrap();
        let b1 = parser.eat_break().unwrap();
        let b1 = parser.tree.get(b1);
        assert_eq!(b1.label, Some(parser.strings.intern("l")));
        assert!(b1.value.is_none());

        // break :l 17
        parser.eat_newline().unwrap();
        let b2 = parser.eat_break().unwrap();
        let b2 = parser.tree.get(b2);
        assert_eq!(b2.label, Some(parser.strings.intern("l")));
        match b2.value {
            Some(expr_id) => match parser.tree.get(expr_id) {
                Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                    ScalarLiteral::Integer(n, _) => assert_eq!(*n, 17),
                    _ => panic!("expected integer literal"),
                },
                _ => panic!("expected scalar literal expression"),
            },
            None => panic!("expected value for break"),
        }

        // break 15
        parser.eat_newline().unwrap();
        let b3 = parser.eat_break().unwrap();
        let b3 = parser.tree.get(b3);
        assert!(b3.label.is_none());
        match b3.value {
            Some(expr_id) => match parser.tree.get(expr_id) {
                Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                    ScalarLiteral::Integer(n, _) => assert_eq!(*n, 15),
                    _ => panic!("expected integer literal"),
                },
                _ => panic!("expected scalar literal expression"),
            },
            None => panic!("expected value for break"),
        }
    }

    #[test]
    fn test_continue_variants() {
        let input = r###"
continue
continue :lbl
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        // continue
        parser.eat_newline().unwrap();
        let c0 = parser.eat_continue().unwrap();
        let c0 = parser.tree.get(c0);
        assert!(c0.label.is_none());

        // continue :lbl
        parser.eat_newline().unwrap();
        let c1 = parser.eat_continue().unwrap();
        let c1 = parser.tree.get(c1);
        assert_eq!(c1.label, Some(parser.strings.intern("lbl")));
    }

    #[test]
    fn test_return_variants() {
        let input = r###"
return
return 42
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        // return
        parser.eat_newline().unwrap();
        let r0 = parser.eat_return().unwrap();
        let r0 = parser.tree.get(r0);
        assert!(r0.value.is_none());

        // return 42
        parser.eat_newline().unwrap();
        let r1 = parser.eat_return().unwrap();
        let r1 = parser.tree.get(r1);
        match r1.value {
            Some(expr_id) => match parser.tree.get(expr_id) {
                Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                    ScalarLiteral::Integer(n, _) => assert_eq!(*n, 42),
                    _ => panic!("expected integer literal"),
                },
                _ => panic!("expected scalar literal"),
            },
            None => panic!("expected return value"),
        }
    }

    #[test]
    fn test_defer_expression_and_block() {
        let input = r###"
defer someFunction()
defer {}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        // defer someFunction()
        parser.eat_newline().unwrap();
        let d0 = parser.eat_defer().unwrap();
        match parser.tree.get(d0) {
            crate::Defer::Expression(expr_id) => match parser.tree.get(*expr_id) {
                Expression::Call(_) => {}
                _ => panic!("expected call expression for defer"),
            },
            _ => panic!("expected defer expression"),
        }

        // defer {}
        parser.eat_newline().unwrap();
        let d1 = parser.eat_defer().unwrap();
        match parser.tree.get(d1) {
            crate::Defer::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert!(block.statements.is_empty());
            }
            _ => panic!("expected defer block"),
        }
    }
}
