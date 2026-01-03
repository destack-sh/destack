use destack_dir::{Expression, LocalNodeId, NodeTree, SymbolTable, TypeTable};

use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify a range expression into a core range struct literal.
    pub(super) fn reify_range_expression(
        &self,
        _expression_id: LocalNodeId<Expression>,
        _start: LocalNodeId<Expression>,
        _end: LocalNodeId<Expression>,
        _is_inclusive: bool,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: reify range expressions into core range structs
        Ok(())
    }
}
