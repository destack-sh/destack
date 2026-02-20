use std::time::Instant;

use destack_heap::{GcState, GcStats, SharedHeap};
use destack_workspace::{GcLogging, GcOptions};

use super::{Gc, RootSet, RootVisitor};

/// Managed heap and GC coordination for the runtime.
#[derive(Default)]
pub struct Heap {
    /// Runtime GC controller for this heap.
    pub gc: Gc,
    /// Shared heap storage for managed and raw allocations.
    pub store: SharedHeap,
    /// Root visitors contributing GC roots.
    pub root_visitors: Vec<Box<dyn RootVisitor>>,
}

impl std::fmt::Debug for Heap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let store = self.store.borrow();
        f.debug_struct("Heap")
            .field("gc", &self.gc)
            .field("gc_state", store.managed.gc_state())
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
    pub fn configure_gc(&mut self, options: GcOptions) {
        self.gc.configure(options);
    }

    /// Check whether the heap should trigger a GC cycle.
    pub fn should_collect(&mut self) -> bool {
        // read the current heap size
        let heap_bytes = self.store.borrow_read().managed.heap_bytes();

        // evaluate runtime gc pacing policy
        self.gc.should_collect(heap_bytes)
    }

    /// Run garbage collection using the current root set.
    pub fn collect(&mut self) -> GcStats {
        // skip collection when gc is disabled
        if !self.gc.is_enabled() {
            return GcStats::default();
        }

        // gather managed heap handles from root visitors
        let roots = self.collect_roots();
        let handles = roots.managed_handles();

        // run collection and capture cycle duration
        let cycle_start = Instant::now();
        let mut store = self.store.borrow();
        let stats = store.managed.collect(&handles);
        let duration_ns = cycle_start.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
        drop(store);

        // update runtime gc pacing from cycle results
        self.gc.on_cycle_complete(stats);

        // emit configured gc logs
        self.emit_gc_log(stats, duration_ns);

        stats
    }

    /// Emit one GC cycle log entry according to configured verbosity.
    fn emit_gc_log(&self, stats: GcStats, duration_ns: u64) {
        // skip logging when gc logging is disabled
        if self.gc.options.logging == GcLogging::Off {
            return;
        }

        // read cycle counter for this collection result
        let cycle_count = self.store.borrow_read().managed.gc_state().cycles;

        // emit cycle summary metrics
        tracing::info!(
            cycle_count,
            duration_ns,
            freed_cells = stats.freed_cells,
            live_cells = stats.live_cells,
            freed_bytes = stats.freed_bytes,
            live_bytes = stats.live_bytes,
            heap_bytes = stats.heap_bytes,
            "runtime gc cycle"
        );

        // include pacing details for verbose mode
        if self.gc.options.logging == GcLogging::Verbose {
            tracing::info!(
                cycle_count,
                trigger_bytes = self.gc.pacer.trigger_bytes,
                heap_goal_bytes = self.gc.pacer.heap_goal_bytes,
                tracked_live_bytes = self.gc.pacer.live_bytes,
                root_visitor_count = self.root_visitors.len(),
                "runtime gc pacing"
            );
        }
    }
}
