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

            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //
            // module
            if keyword == Some(Keyword::Module) {
                let module_id = self
                    .eat_module(visibility)
                    .for_node_type(NodeType::Module)?;
                Statement::Module(module_id)
            }
            // struct
            else if keyword == Some(Keyword::Struct) {
                let struct_id = self
                    .eat_struct(visibility)
                    .for_node_type(NodeType::Struct)?;
                Statement::Struct(struct_id)
            }
            // enum
            else if keyword == Some(Keyword::Enum) {
                let enum_id = self.eat_enum(visibility).for_node_type(NodeType::Enum)?;
                Statement::Enum(enum_id)
            }
            // union
            else if keyword == Some(Keyword::Union) {
                let union_id = self.eat_union(visibility).for_node_type(NodeType::Union)?;
                Statement::Union(union_id)
            }
            // trait
            else if keyword == Some(Keyword::Trait) {
                let trait_id = self.eat_trait(visibility).for_node_type(NodeType::Trait)?;
                Statement::Trait(trait_id)
            }
            // implement
            else if keyword == Some(Keyword::Implement) {
                let implement_id = self.eat_implement().for_node_type(NodeType::Implement)?;
                Statement::Implement(implement_id)
            }
            // function
            else if keyword == Some(Keyword::Function) {
                let function_id = self
                    .eat_function(visibility)
                    .for_node_type(NodeType::Function)?;
                Statement::Function(function_id)
            }
            // block
            else if self.peek_block().is_ok() {
                let block_id = self.eat_block().for_node_type(NodeType::Block)?;
                Statement::Block(block_id)
            }
            //
            // ------------------------------------------------------------
            // Context
            // ------------------------------------------------------------
            //
            // with
            else if keyword == Some(Keyword::With) {
                let with_id = self.eat_with().for_node_type(NodeType::With)?;
                Statement::With(with_id)
            }
            // use
            else if keyword == Some(Keyword::Use) {
                let use_id = self.eat_use(visibility).for_node_type(NodeType::Use)?;
                Statement::Use(use_id)
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
