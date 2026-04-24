use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_core::LocalStringPool;
use destack_mir::NodeTree;
use destack_vm::{Isolate, IsolateId};

/// Build one empty VM engine for low-level runtime control operations.
pub(crate) fn empty_vm_engine() -> RuntimeResult<Isolate> {
    let tree = NodeTree::new();
    let strings = LocalStringPool::new().into_immutable();

    Isolate::build(IsolateId::new(1), tree, strings).map_err(Box::<RuntimeError>::from)
}
