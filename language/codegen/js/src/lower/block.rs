use crate::{
    Block, CodegenJsError, CodegenJsResult, CodegenJsResultExt, LocalNodeId, ModuleLowerer,
    Statement,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a block from DIR into JS AST.
    pub fn lower_block(
        &mut self,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> CodegenJsResult<LocalNodeId<Block>> {
        let block = self.dir_tree.get(block_id);
        let label = block
            .label
            .map(|label| self.strings.intern_from(&self.program.strings, label));
        let statements = block
            .expressions
            .iter()
            .map(|statement| {
                self.lower_expression(*statement)
                    .expect_node::<Statement>(statement.into_global_any(self.module.id), self)
            })
            .collect::<Result<Vec<_>, CodegenJsError>>()?;
        let block = Block { label, statements };
        Ok(self
            .tree
            .insert_from_source(block, self.module.id, block_id))
    }
}
