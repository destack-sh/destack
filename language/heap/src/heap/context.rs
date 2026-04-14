use super::{Heap, SharedBudget};
use crate::{SharedLimitError, SharedLimits, SharedPointer, SharedSpace};

/// One agent-local execution memory view across local and shared memory.
#[derive(Debug)]
pub struct MemoryContext<'a> {
    /// The agent-local heap.
    heap: &'a mut Heap,
    /// The world-shared memory space.
    shared: &'a mut SharedSpace,
    /// The live shared-memory admission budget.
    shared_budget: SharedBudget,
}

impl<'a> MemoryContext<'a> {
    /// Create one memory context from one local heap and one shared space.
    pub fn new(heap: &'a mut Heap, shared: &'a mut SharedSpace) -> Self {
        Self::with_shared_limits(heap, shared, Default::default())
    }

    /// Create one memory context from one local heap, one shared space, and one explicit limit set.
    pub fn with_shared_limits(
        heap: &'a mut Heap,
        shared: &'a mut SharedSpace,
        shared_limits: SharedLimits,
    ) -> Self {
        let active_bytes = shared.active_bytes();

        Self {
            heap,
            shared,
            shared_budget: SharedBudget::new(shared_limits, active_bytes),
        }
    }

    /// Return the local heap.
    pub fn heap(&mut self) -> &mut Heap {
        self.heap
    }

    /// Return the mutable local heap.
    pub fn heap_mut(&mut self) -> &mut Heap {
        self.heap
    }

    /// Return the local heap by shared reference.
    pub fn heap_ref(&self) -> &Heap {
        self.heap
    }

    /// Return the shared space.
    pub fn shared(&self) -> &SharedSpace {
        self.shared
    }

    /// Return the mutable shared space.
    pub fn shared_mut(&mut self) -> &mut SharedSpace {
        self.shared
    }

    /// Return the shared space by shared reference.
    pub fn shared_ref(&self) -> &SharedSpace {
        self.shared
    }

    /// Return the current shared-memory budget.
    pub fn shared_budget(&self) -> SharedBudget {
        self.shared_budget
    }

    /// Allocate one shared-memory region.
    pub fn allocate_shared_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<SharedPointer, SharedLimitError> {
        self.shared_budget
            .check_active_reservation(bytes.len() as i64)?;

        let pointer = self.shared.allocate_bytes(bytes);
        self.shared_budget.refresh(self.shared.active_bytes());

        Ok(pointer)
    }

    /// Replace one shared-memory region payload.
    pub fn replace_shared_bytes(
        &mut self,
        pointer: SharedPointer,
        bytes: &[u8],
    ) -> Result<bool, SharedLimitError> {
        let previous_len = self
            .shared
            .bytes_to_vec(pointer)
            .map(|bytes| bytes.len())
            .unwrap_or(0);
        let reservation = bytes.len() as i64 - previous_len as i64;
        self.shared_budget.check_active_reservation(reservation)?;

        if !self.shared.replace_bytes(pointer, bytes) {
            return Ok(false);
        }

        self.shared_budget.refresh(self.shared.active_bytes());

        Ok(true)
    }

    /// Reborrow this memory context for one nested call.
    pub fn reborrow(&mut self) -> MemoryContext<'_> {
        // execution reborrows
        let heap = unsafe { &mut *(self.heap as *mut Heap) };
        let shared = unsafe { &mut *(self.shared as *mut SharedSpace) };

        MemoryContext {
            heap,
            shared,
            shared_budget: self.shared_budget,
        }
    }
}
