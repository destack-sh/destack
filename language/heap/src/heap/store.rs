use serde::{Deserialize, Serialize};

use super::{ManagedHeap, ManagedHeapSnapshot, RawHeap, RawHeapSnapshot};
use crate::value::ManagedPointer;

/// Immutable heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// Captured managed heap state.
    pub managed: ManagedHeapSnapshot,
    /// Captured raw heap state.
    pub raw: RawHeapSnapshot,
}

/// Heap for managed and raw allocations.
#[derive(Debug, Default)]
pub struct Heap {
    /// Managed heap used for GC tracked allocations.
    managed: ManagedHeap,
    /// Raw heap used for manual allocations.
    raw: RawHeap,
}

impl Heap {
    /// Create a heap from explicit heap instances.
    pub fn new(managed: ManagedHeap, raw: RawHeap) -> Self {
        Self { managed, raw }
    }

    /// Create a heap from an immutable snapshot.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Self {
        Self::new(
            ManagedHeap::restore(&snapshot.managed),
            RawHeap::restore(&snapshot.raw),
        )
    }

    /// Return the managed heap.
    pub fn managed(&self) -> &ManagedHeap {
        &self.managed
    }

    /// Return the raw heap.
    pub fn raw(&self) -> &RawHeap {
        &self.raw
    }

    /// Return the managed heap mutably.
    pub fn managed_mut(&mut self) -> &mut ManagedHeap {
        &mut self.managed
    }

    /// Return the raw heap mutably.
    pub fn raw_mut(&mut self) -> &mut RawHeap {
        &mut self.raw
    }

    /// Return both heap regions.
    pub fn parts(&self) -> (&ManagedHeap, &RawHeap) {
        (&self.managed, &self.raw)
    }

    /// Return both heap regions mutably.
    pub fn parts_mut(&mut self) -> (&mut ManagedHeap, &mut RawHeap) {
        (&mut self.managed, &mut self.raw)
    }

    /// Capture one immutable heap snapshot.
    pub fn snapshot(&mut self) -> HeapSnapshot {
        HeapSnapshot {
            managed: self.managed.snapshot(),
            raw: self.raw.snapshot(),
        }
    }

    /// Restore this heap from one immutable snapshot.
    pub fn restore(&mut self, snapshot: &HeapSnapshot) {
        self.managed = ManagedHeap::restore(&snapshot.managed);
        self.raw = RawHeap::restore(&snapshot.raw);
    }

    /// Return the currently allocated managed pointers.
    pub fn allocated_pointers(&self) -> Vec<ManagedPointer> {
        self.managed.allocated_pointers()
    }
}
