//! Parse statements.

use dyst_token::TokenType;

use crate::parse::prelude::*;
use crate::{Keyword, NodeId, NodeType, ParseResult, Parser, ParserMark, Statement};

impl<'a> Parser<'a> {
    /// Eat a statement with recovery (return None if error and recovery is possible).
    #[inline]
    pub fn try_eat_statement(&mut self) -> ParseResult<Option<NodeId<Statement>>> {
        match self.eat_statement() {
            Ok(statement_id) => Ok(Some(statement_id)),
            Err(err) => {
                let err = err.for_node_type(NodeType::Statement);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize);
                self.try_recover(start, TokenType::Newline, Some(err))?;
                Ok(None)
            }
        }
    }

    /// Eat a statement body (without the `;` or `\n`).
    #[inline]
    pub fn eat_statement(&mut self) -> ParseResult<NodeId<Statement>> {
        let start = self.mark();

        // visibility
        let visibility = self.peek_visibility()?;
        if visibility.is_some() {
            self.bump(); // eat visibility
        }

        let statement = {
            let keyword = self.peek_any_keyword().ok();

            // with
            if keyword == Some(Keyword::With) {
                let with_id = self.eat_with().for_node_type(NodeType::With)?;
                Statement::With(with_id)
            }
            // use
            else if keyword == Some(Keyword::Use) {
                let use_id = self.eat_use(visibility).for_node_type(NodeType::Use)?;
                Statement::Use(use_id)
            }
            // break
            else if keyword == Some(Keyword::Break) {
                let break_id = self.eat_break().for_node_type(NodeType::Break)?;
                Statement::Break(break_id)
            }
            // continue
            else if keyword == Some(Keyword::Continue) {
                let continue_id = self.eat_continue().for_node_type(NodeType::Continue)?;
                Statement::Continue(continue_id)
            }
            // defer
            else if keyword == Some(Keyword::Defer) {
                let defer_id = self.eat_defer().for_node_type(NodeType::Defer)?;
                Statement::Defer(defer_id)
            }
            // return
            else if keyword == Some(Keyword::Return) {
                let return_id = self.eat_return().for_node_type(NodeType::Return)?;
                Statement::Return(return_id)
            }
            //
            // ------------------------------------------------------------
            // Expressions
            // ------------------------------------------------------------
            //
            // anything else is an expression
            else {
                let expression_id = self
                    .eat_expression(ExpressionParserOptions {
                        visibility,
                        ..ExpressionParserOptions::default()
                    })
                    .for_node_type(NodeType::Expression)?;
                Statement::Expression(expression_id)
            }
        };

        let statement_id = self.tree.allocate(statement, self.get_span_from(start));
        Ok(statement_id)
    }
}
