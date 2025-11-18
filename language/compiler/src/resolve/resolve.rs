use crate::{Compiler, ResolveResult};

use dyst_dir::{ModuleId, NodeIdAny};

/// Task to statically resolve something in-place.
#[derive(Debug, Clone)]
pub enum ResolveTask {
    /// Resolve a Node.
    ResolveNode { module: ModuleId, node: NodeIdAny },
}

impl<'a> Compiler<'a> {
    /// Generate tasks for all unresolved nodes.
    pub fn queue_all_unresolved(&mut self) {

    }

    /// Generate tasks for all unresolved nodes in a module.
    pub(super) fn queue_all_unresolved_in_module(&mut self, module: ModuleId) {
    }

    /// Resolve a node.
    pub fn process_resolve(&mut self, task: ResolveTask) -> ResolveResult<()> {
        todo!("process_resolve({task:?})")
    }
}
