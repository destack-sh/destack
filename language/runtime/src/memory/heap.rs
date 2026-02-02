use destack_heap::{GcOptions, GcState, GcStats, SharedHeap};

use super::{RootSet, RootVisitor};

/// Managed heap and GC coordination for the runtime.
pub struct Heap {
    /// Whether GC is enabled for this heap.
    pub gc_enabled: bool,
    /// Shared heap storage for managed and raw allocations.
    pub store: SharedHeap,
    /// Root visitors contributing GC roots.
    pub root_visitors: Vec<Box<dyn RootVisitor>>,
}

impl Default for Heap {
    fn default() -> Self {
        Self {
            gc_enabled: true,
            store: SharedHeap::default(),
            root_visitors: Vec::new(),
        }
    }
}

impl std::fmt::Debug for Heap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let store = self.store.borrow();
        f.debug_struct("Heap")
            .field("gc_enabled", &self.gc_enabled)
            .field("gc", store.managed.gc_state())
            .field("managed", &store.managed)
            .field("raw", &store.raw)
            .field("root_visitors", &self.root_visitors.len())
            .finish()
    }
}

impl Heap {
    /// Register a root visitor for GC coordination.
    pub fn register_root_visitor(&mut self, visitor: Box<dyn RootVisitor>) {
        self.root_visitors.push(visitor);
    }

    /// Collect roots from all registered providers.
    pub fn collect_roots(&self) -> RootSet {
        let mut roots = RootSet::new();
        for visitor in &self.root_visitors {
            visitor.collect_roots(&mut roots);
        }
        roots
    }

    /// Return the shared heap store.
    pub fn store(&self) -> SharedHeap {
        self.store.clone()
    }

    /// Return a snapshot of the GC state.
    pub fn gc_state(&self) -> GcState {
        self.store.borrow().managed.gc_state().clone()
    }

    /// Configure GC enablement and options.
    pub fn configure_gc(&mut self, enabled: bool, options: GcOptions) {
        self.gc_enabled = enabled;
        self.store.borrow().managed.gc_state_mut().options = options;
    }

    /// Check whether the heap should trigger a GC cycle.
    pub fn should_collect(&mut self) -> bool {
        if !self.gc_enabled {
            return false;
        }
        let mut store = self.store.borrow();
        store.managed.should_collect()
    }

    /// Run garbage collection using the current root set.
    pub fn collect(&mut self) -> GcStats {
        if !self.gc_enabled {
            return GcStats::default();
        }

        // gather managed heap handles from root visitors
        let roots = self.collect_roots();
        let handles = roots.managed_handles();

        // run collection on the managed heap
        let mut store = self.store.borrow();
        store.managed.collect(&handles)
    }
}
