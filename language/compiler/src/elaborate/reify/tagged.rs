use destack_dir::{Argument, Expression, LocalNodeId, NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify nominal constructor calls into tagged expressions.
    pub(super) fn reify_tagged_constructor_call(
        &self,
        _module_id: ModuleId,
        _profile: ProfileId,
        _expression_id: LocalNodeId<Expression>,
        _callee: LocalNodeId<Expression>,
        _static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
        _dynamic_arguments: &[LocalNodeId<Argument>],
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
        _module: &Module,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: resolve nominal constructor calls to tagged expressions
        Ok(())
    }
}
