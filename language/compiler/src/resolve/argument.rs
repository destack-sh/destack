use dyst_dir::{Argument, LocalNodeId, Module, NodeTree, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Argument.
    pub(super) fn resolve_argument(
        &self,
        module: &Module,
        argument_id: LocalNodeId<Argument>,
        tree: &mut NodeTree,
        _symbols: &SymbolTable,
    ) -> ResolveResult<()> {
        let _argument = tree.get(argument_id);
        Err(ResolveError::UnsupportedNode {
            node: argument_id.into_global_any(module.id),
        })
    }
}
