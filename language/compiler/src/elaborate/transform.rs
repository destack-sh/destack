use destack_dir::{Expression, LocalNodeId, NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult};

#[allow(clippy::single_match)]
impl Compiler {
    /// Transform a module with semantic desugaring:
    /// - Pattern destructuring → explicit bindings
    /// - Pattern matching → decision if/else trees
    /// - If/let expressions -> explicit bindings
    /// - Expressions as values -> explicit bindings
    /// - Maybe/Must → explicit error handling (if not overloaded)
    pub(super) fn elaborate_module_transform(&self, module_id: ModuleId) -> ElaborateResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.dir.tree.write();
        let symbols = module.dir.symbols.read();
        let types = module.dir.types.read();

        // transform expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.transform_expression(expression_id, &mut tree, &symbols, &types)?;
        }

        Ok(())
    }

    /// Transform an expression (post-analysis).
    pub(super) fn transform_expression(
        &self,
        _expression_id: LocalNodeId<Expression>,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: implement semantic elaborate / transforms
        // - Maybe/Must → explicit error handling
        Ok(())
    }
}
