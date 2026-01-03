use destack_dir::{Expression, LocalNodeId, NodeTree, SymbolTable, TypeTable};

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Reify an operator into resolved method calls ("deload").
    /// - Builtin: keep as primitive operator
    /// - Static: emit direct method call `a.add(b)`
    /// - Dynamic: emit type dispatch match expression
    pub(super) fn reify_operator_expression(
        &self,
        _expression_id: LocalNodeId<Expression>,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: reify operators based on Resolutions
        //  - static: a + b → a.add(b)
        //  - dynamic: a + b → match typeof(..) { T1 => ..., T2 => ... }
        Ok(())
    }
}
