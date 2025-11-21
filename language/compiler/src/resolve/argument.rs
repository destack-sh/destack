use dyst_dir::{Argument, ModuleId, LocalNodeId, NodeTree};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Argument.
    pub(super) fn resolve_argument(
        &self,
        _module_id: ModuleId,
        argument_id: LocalNodeId<Argument>,
        tree: &mut NodeTree,
    ) -> ResolveResult<()> {
        let _argument = tree.get(argument_id);
        Err(ResolveError::UnsupportedNode {
            node: argument_id.into(),
        })
    }
}
