use dyst_dir::{BlockTarget, NodeIdAny};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a BlockTarget.
    pub fn resolve_target(
        &mut self,
        _scope_id: NodeIdAny,
        destination: BlockTarget,
    ) -> ResolveResult<BlockTarget> {
        todo!("resolve_destination({destination:?})")
    }
}
