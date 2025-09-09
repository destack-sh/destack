//! Parse statements.

use dyst_language_token::TokenType;

use crate::parse::expression::ExpressionParserOptions;
use crate::{Keyword, NodeId, ParseResult, Parser, ParserMark, Statement};

impl<'a> Parser<'a> {
    /// Eat a statement with recovery (return None if error and recovery is possible).
    #[inline]
    pub fn try_eat_statement(&mut self) -> ParseResult<Option<NodeId<Statement>>> {
        match self.eat_statement() {
            Ok(statement_id) => Ok(Some(statement_id)),
            Err(err) => {
                self.try_recover(ParserMark::new(err.span.start as usize), TokenType::Newline)?;
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
            let token = self.peek()?;
            let keyword = self.peek_any_keyword().ok();

            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //
            // module
            if keyword == Some(Keyword::Module) {
                let module_id = self.eat_module(visibility)?;
                Statement::Module(module_id)
            }
            // struct
            else if keyword == Some(Keyword::Struct) {
                let struct_id = self.eat_struct(visibility)?;
                Statement::Struct(struct_id)
            }
            // enum
            else if keyword == Some(Keyword::Enum) {
                let enum_id = self.eat_enum(visibility)?;
                Statement::Enum(enum_id)
            }
            // union
            else if keyword == Some(Keyword::Union) {
                let union_id = self.eat_union(visibility)?;
                Statement::Union(union_id)
            }
            // trait
            else if keyword == Some(Keyword::Trait) {
                let trait_id = self.eat_trait(visibility)?;
                Statement::Trait(trait_id)
            }
            // implement
            else if keyword == Some(Keyword::Implement) {
                let implement_id = self.eat_implement()?;
                Statement::Implement(implement_id)
            }
            // function
            else if keyword == Some(Keyword::Function) {
                let function_id = self.eat_function(visibility)?;
                Statement::Function(function_id)
            }
            //
            // ------------------------------------------------------------
            // Context
            // ------------------------------------------------------------
            //
            // with
            else if keyword == Some(Keyword::With) {
                let with_id = self.eat_with()?;
                Statement::With(with_id)
            }
            // use
            else if keyword == Some(Keyword::Use) {
                let use_id = self.eat_use(visibility)?;
                Statement::Use(use_id)
            }
            //
            // ------------------------------------------------------------
            // Documentation
            // ------------------------------------------------------------
            //
            // doc
            else if token.token.r#type == TokenType::DocLineComment
                || token.token.r#type == TokenType::DocBlockComment
            {
                let doc_id = self.eat_doc()?;
                Statement::Doc(doc_id)
            }
            //
            // ------------------------------------------------------------
            // Expressions
            // ------------------------------------------------------------
            //
            // expression (fallback)
            else {
                let expression_id = self.eat_expression(ExpressionParserOptions {
                    visibility,
                    ..ExpressionParserOptions::default()
                })?;
                Statement::Expression(expression_id)
            }
        };

        let statement_id = self.tree.allocate(statement, self.get_span_from(start));
        Ok(statement_id)
    }
}
