use destack_dir as dir;
use destack_js as js;

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Emit one statement body as a JavaScript block.
    pub(crate) fn emit_body(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Block>, EmitError> {
        // preserve an existing block
        if let dir::Expression::Block(block) = self.tree.get(source) {
            return self.emit_block(*block);
        }

        // require an executable statement
        let Some(statement) = self.emit_statement(source)? else {
            return Err(self.internal_error(
                "erased expression reached JavaScript statement body".to_string(),
            ));
        };
        let block = js::Block {
            statements: vec![statement],
        };

        Ok(self.insert_from_source(block, source))
    }

    /// Emit one DIR block as JavaScript.
    pub(crate) fn emit_block(
        &mut self,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> Result<js::LocalNodeId<js::Block>, EmitError> {
        let block = self.tree.get(block_id);
        let mut statements = Vec::with_capacity(block.len());

        // emit executable expressions
        for expression in block.iter_expressions() {
            if let Some(statement) = self.emit_statement(expression)? {
                statements.push(statement);
            }
        }

        // attach the block scope
        let block = js::Block { statements };
        let output = self.insert_from_source(block, block_id);

        Ok(output)
    }
}
