use std::sync::Arc;

use destack_heap as heap;
use destack_workspace::ExecutionMode;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::{SharedCollector, SharedCollectorMode};

use super::BranchId;

/// Live memory services shared by one world.
#[derive(Clone, Debug)]
pub(crate) struct WorldMemory {
    /// Page allocator backing runtime and worker heaps.
    pub(crate) allocator: Arc<heap::Allocator>,
    /// Shared GC scheduler for live runtimes.
    pub(crate) shared_collector: Arc<SharedCollector>,
}

impl WorldMemory {
    /// Create live memory services for one world.
    pub(crate) fn new(execution_mode: ExecutionMode, branch_id: BranchId) -> RuntimeResult<Self> {
        let allocator = Arc::new(
            heap::Allocator::try_new(
                heap::DEFAULT_PAGE_SIZE_BYTES,
                heap::DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES,
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        // name collector after its root branch
        let collector_mode = SharedCollectorMode::from_execution_mode(execution_mode);
        let shared_collector = SharedCollector::new(
            collector_mode,
            format!("destack.collector.{}", branch_id.get()),
        )?;

        Ok(Self {
            allocator,
            shared_collector,
        })
    }
}
