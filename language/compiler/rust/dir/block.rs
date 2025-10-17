use dyst_ast as ast;
use dyst_dir::{Block, Destination, NodeId};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a label to a DIR destination.
    pub fn lower_label(
        &mut self,
        source_id: SourceId,
        _ast: &ast::NodeTree,
        label: ast::StringId,
    ) -> Destination {
        let label = self.intern_string(source_id, label);
        Destination::UnevaluatedString { label }
    }

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
            .insert_from_ast(Block { label, expressions }, source_id, block_id)
    }
}
