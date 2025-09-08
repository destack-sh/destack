//! Parse statements.

use dyst_language_token::TokenType;

use crate::parse::expression::ExpressionParserOptions;
use crate::{Keyword, NodeId, ParseResult, Parser, Statement};

impl<'a> Parser<'a> {
    /// Eat a statement (with the `;` or `\n`).
    #[inline]
    pub fn eat_statement_with_stop(&mut self) -> ParseResult<NodeId<Statement>> {
        let statement_id = self.eat_statement()?;
        self.eat_statement_stop()?;
        Ok(statement_id)
    }

    /// Eat a statement body (without the `;` or `\n`).
    #[inline]
    pub fn eat_statement(&mut self) -> ParseResult<NodeId<Statement>> {
        let start = self.mark();
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
                let module_id = self.eat_module()?;
                Statement::Module(module_id)
            }
            // struct
            else if keyword == Some(Keyword::Struct) {
                let struct_id = self.eat_struct()?;
                Statement::Struct(struct_id)
            }
            // enum
            else if keyword == Some(Keyword::Enum) {
                let enum_id = self.eat_enum()?;
                Statement::Enum(enum_id)
            }
            // union
            else if keyword == Some(Keyword::Union) {
                let union_id = self.eat_union()?;
                Statement::Union(union_id)
            }
            // trait
            else if keyword == Some(Keyword::Trait) {
                let trait_id = self.eat_trait()?;
                Statement::Trait(trait_id)
            }
            // implement
            else if keyword == Some(Keyword::Implement) {
                let implement_id = self.eat_implement()?;
                Statement::Implement(implement_id)
            }
            // function
            else if keyword == Some(Keyword::Function) {
                let function_id = self.eat_function()?;
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
                let use_id = self.eat_use()?;
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
                let expression_id = self.eat_expression(ExpressionParserOptions::default())?;
                Statement::Expression(expression_id)
            }
        };

        let statement_id = self.tree.allocate(statement, self.get_span_from(start));
        Ok(statement_id)
    }
}
