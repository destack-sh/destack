use dyst_ast as ast;
use dyst_dir::{Block, NodeId};
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
        let block = ast.get(block_id);
        let label = block
            .label
            .map(|label| self.intern_string(source_id, label));
        let expressions = block
            .expressions
            .iter()
            .map(|expression| self.lower_expression(source_id, ast, *expression))
            .collect();
        self.tree
            .allocate(Block { label, expressions }, source_id, block_id)
    }
}
