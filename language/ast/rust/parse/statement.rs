//! Parse statements.

use crate::{Keyword, NodeId, ParseResult, Parser, Statement};

impl<'a> Parser<'a> {
    /// Parse a statement (with the `;` or `\n`).
    pub fn eat_statement(&mut self) -> ParseResult<NodeId<Statement>> {
        let statement_id = self.eat_statement_body()?;
        self.eat_statement_stop()?;
        Ok(statement_id)
    }

    /// Parse a statement body (without the `;` or `\n`).
    pub fn eat_statement_body(&mut self) -> ParseResult<NodeId<Statement>> {
        let start = self.mark();
        let statement = {
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //
            // module
            if self.peek_keyword(Keyword::Module).is_ok() {
                let module_id = self.eat_module()?;
                Statement::Module(module_id)
            }
            // struct
            else if self.peek_keyword(Keyword::Struct).is_ok() {
                let struct_id = self.eat_struct()?;
                Statement::Struct(struct_id)
            }
            // enum
            else if self.peek_keyword(Keyword::Enum).is_ok() {
                let enum_id = self.eat_enum()?;
                Statement::Enum(enum_id)
            }
            // union
            else if self.peek_keyword(Keyword::Union).is_ok() {
                let union_id = self.eat_union()?;
                Statement::Union(union_id)
            }
            // trait
            else if self.peek_keyword(Keyword::Trait).is_ok() {
                let trait_id = self.eat_trait()?;
                Statement::Trait(trait_id)
            }
            // implement
            else if self.peek_keyword(Keyword::Implement).is_ok() {
                let implement_id = self.eat_implement()?;
                Statement::Implement(implement_id)
            }
            // function
            else if self.peek_keyword(Keyword::Function).is_ok() {
                let function_id = self.eat_function()?;
                Statement::Function(function_id)
            }
            //
            // ------------------------------------------------------------
            // Usings
            // ------------------------------------------------------------
            //
            // using
            else if self.peek_keyword(Keyword::Using).is_ok() {
                let using_id = self.eat_using()?;
                Statement::Using(using_id)
            }
            //
            // ------------------------------------------------------------
            // Documentation
            // ------------------------------------------------------------
            //
            // doc
            else if self.peek_doc().is_ok() {
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
                let expression_id = self.eat_expression(None)?;
                Statement::Expression(expression_id)
            }
        };

        let statement_id = self.tree.allocate(statement, self.get_span_from(start));
        Ok(statement_id)
    }
}
