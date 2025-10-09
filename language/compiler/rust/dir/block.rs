use dyst_ast as ast;
use dyst_dir::{Block, Expression, Mutability, NodeId, Runtime, ScopedMutability, Visibility};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a block to a DIR block.
    pub fn lower_block(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        block_id: ast::NodeId<ast::Block>,
    ) -> NodeId<Block> {
        // let block = ast.get(block_id);
        // self.tree.allocate(block, source_id, block_id)
        todo!("Compiler::lower_block")
    }
}
