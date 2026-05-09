use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_core::StringPool;
use destack_mir::Tree;
use destack_vm::{Isolate, IsolateId};

/// Build one empty VM engine for low-level runtime control operations.
pub(crate) fn empty_vm_engine() -> RuntimeResult<Isolate> {
    let tree = Tree::new();
    let strings = StringPool::new();

    Isolate::build(IsolateId::new(1), tree, strings).map_err(Box::<RuntimeError>::from)
}
