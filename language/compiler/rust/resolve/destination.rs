use dyst_dir::{Destination, NodeIdAny};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Destination.
    pub fn resolve_destination(
        &mut self,
        scope_id: NodeIdAny,
        destination: Destination,
    ) -> ResolveResult<Destination> {
        todo!("resolve_destination({destination:?})")
    }
}
