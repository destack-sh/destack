use crate::{
    Block, LocalNodeId, Statement, TranspileError, TranspileResult, TranspileResultExt, Transpiler,
    TranspilerUnit,
};
use destack_dir::{self as dir, Module, NodeTree, SymbolTable, TypeTable};

impl Transpiler {
    /// Lower a block from DIR into JS AST.
    pub fn lower_block(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        block_id: dir::LocalNodeId<dir::Block>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Block>> {
        let block = tree.get(block_id);
        let label = block
            .label
            .map(|label| unit.strings.intern_from(&self.program.strings, label));
        let statements = block
            .expressions
            .iter()
            .map(|statement| {
                self.lower_expression(module, tree, symbols, types, *statement, unit)
                    .expect_node::<Statement>(statement.into_global_any(module.id), unit)
            })
            .collect::<Result<Vec<_>, TranspileError>>()?;
        let block = Block { label, statements };
        Ok(unit.ast.insert_from_source(block, module.id, block_id))
    }
}
