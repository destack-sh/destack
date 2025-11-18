use dyst_dir::{Argument, ModuleId, NodeId};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Argument.
    pub(super) fn resolve_argument(
        &mut self,
        _module_id: ModuleId,
        argument_id: NodeId<Argument>,
    ) -> ResolveResult<()> {
        let _argument = self.session.tree.get(argument_id);
        Err(ResolveError::UnsupportedNode {
            node: argument_id.into(),
        })
    }
}
