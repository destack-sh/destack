use dyst_ast::{self as ast};
use dyst_dir::{Block, BlockTarget, Module, NodeId};
use dyst_source::StringId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower a label to a DIR block target.
    pub(super) fn lower_label(&mut self, module: &Module, label: StringId) -> BlockTarget {
        let label = self.session.strings.intern_from(&module.strings, label);
        BlockTarget::UnresolvedString { label }
    }

    /// Lower a block to a DIR block.
    pub(super) fn lower_block(
        &mut self,
        module: &Module,
        block_id: ast::NodeId<ast::Block>,
    ) -> NodeId<Block> {
        let block = module.get(block_id);
        let label = block
            .label
            .map(|label| self.session.strings.intern_from(&module.strings, label));
        let expressions = block
            .expressions
            .iter()
            .map(|expression| self.lower_expression(module, *expression))
            .collect();
        self.session
            .tree
            .insert_from_ast(Block { label, expressions }, module.id, block_id)
    }
}
