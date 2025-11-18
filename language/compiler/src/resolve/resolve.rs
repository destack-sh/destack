use crate::{Compiler, ResolveResult};

use dyst_dir::{ModuleId, NodeIdAny};

/// Task to statically resolve something in-place.
#[derive(Debug, Clone)]
pub enum ResolveTask {
    /// Resolve a Node.
    ResolveNode { module: ModuleId, node: NodeIdAny },
}

impl<'a> Compiler<'a> {
    /// Resolve a node.
    pub fn process_resolve(&mut self, task: ResolveTask) -> ResolveResult<()> {
        // nocheckin #Broken: revisit Compiler is_resolved/resolve logic (after load, ...)
        // (also see all the :Unresolved* variants, and Type::Definition, ...)
        todo!("process_resolve({task:?})")
    }
}
