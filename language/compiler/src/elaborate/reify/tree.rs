use destack_dir::{Expression, LocalNodeId, NodeTree, SymbolTable, TypeTable};

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Reify a tree literal into constructor calls.
    pub(super) fn reify_tree_expression(
        &self,
        _expression_id: LocalNodeId<Expression>,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: reify tree literals
        // <Div>{children}</Div> → createElement(Div, null, children)
        Ok(())
    }
}
