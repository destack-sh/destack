use dyst_dir::{Argument, NodeId};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Argument.
    pub fn resolve_argument(&mut self, argument_id: NodeId<Argument>) -> ResolveResult<()> {
        let _argument = self.tree.get(argument_id);
        // todo!("resolve_argument({argument:?})");
        Ok(())
    }
}
