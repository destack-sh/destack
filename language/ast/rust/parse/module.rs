use crate::{BlockFormat, Keyword, Module, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a module declaration (incl. `module` keyword).
    pub fn eat_module(&mut self) -> ParseResult<NodeId<Module>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Module)?;
        let name = if self.peek_identifier().is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };
        let block = self.eat_block()?;
        let module = Module {
            format: BlockFormat::Explicit,
            name,
            body: block,
        };
        let module_id = self.tree.allocate(module, self.get_span_from(start));
        Ok(module_id)
    }

    // Eat a module body (aka a module file, without `module` keyword or braces).
    pub fn eat_module_body(&mut self, format: BlockFormat) -> ParseResult<NodeId<Module>> {
        let start = self.mark();
        let block = self.eat_block_body(BlockFormat::Implicit)?;
        let module = Module {
            format,
            name: None,
            body: block,
        };
        let module_id = self.tree.allocate(module, self.get_span_from(start));
        Ok(module_id)
    }
}
