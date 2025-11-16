use crate::{TranspileError, TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};
use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Block, NodeId, Statement};

impl<'a> Transpiler<'a> {
    /// Transpile a block from DIR into JS AST.
    pub fn transpile_block(
        &self,
        module: &'a Module,
        block_id: dir::NodeId<dir::Block>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Block>> {
        let block = self.session.tree.get(block_id);
        let label = block
            .label
            .map(|label| unit.strings.intern_from(&self.session.strings, label));
        let statements = block
            .expressions
            .iter()
            .map(|statement| {
                self.transpile_expression(module, *statement, unit)
                    .expect_node::<Statement>(statement.into_any(), unit)
            })
            .collect::<Result<Vec<_>, TranspileError>>()?;
        let block = Block { label, statements };
        Ok(unit.ast.insert_from_source(block, module.id, block_id))
    }
}
