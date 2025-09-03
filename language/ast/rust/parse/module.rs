use crate::{Keyword, Module, NodeId, ParseResult, Parser};

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
        let module = Module { name, body: block };
        let module_id = self.tree.allocate(module, self.span_from(start));
        Ok(module_id)
    }
}
