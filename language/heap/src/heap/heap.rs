use std::sync::Arc;

use crate::alloc::Arena;
use crate::managed::{ManagedCollectError, ManagedSpace};
use crate::raw::RawSpace;
use crate::value::ManagedReference;
use crate::{GcState, GcStats, HeapLayout, HeapLayoutError, HeapLimits};

/// One live local heap rooted in one shared arena.
#[derive(Debug, Clone)]
pub struct Heap {
    /// The shared page arena for every local byte payload.
    pub(super) arena: Arc<Arena>,
    /// The configured heap layout.
    pub(super) layout: HeapLayout,
    /// The managed local allocation space.
    pub(super) managed: ManagedSpace,
    /// The raw local allocation space.
    pub(super) raw: RawSpace,
    /// Exact hard limits for this heap.
    pub(super) limits: HeapLimits,
}

impl Heap {
    /// Create one heap with the default limits and layout.
    pub fn new() -> Self {
        let layout = HeapLayout::default();
        let arena = Arc::new(Arena::with_page_bytes(layout.page_bytes));

        Self::build_with_layout(arena, HeapLimits::default(), layout)
    }

    /// Create one heap with one explicit shared arena, limits, and layout.
    pub fn with_arena_limits_and_layout(
        arena: Arc<Arena>,
        limits: HeapLimits,
        layout: HeapLayout,
    ) -> Result<Self, HeapLayoutError> {
        layout.check()?;

        Ok(Self::build_with_layout(arena, limits, layout))
    }

    /// Create one heap with explicit limits and layout.
    pub fn with_limits_and_layout(
        limits: HeapLimits,
        layout: HeapLayout,
    ) -> Result<Self, HeapLayoutError> {
        let arena = Arc::new(Arena::with_page_bytes(layout.page_bytes));

        Self::with_arena_limits_and_layout(arena, limits, layout)
    }

    /// Create one heap from one checked shared arena, limits, and layout.
    fn build_with_layout(arena: Arc<Arena>, limits: HeapLimits, layout: HeapLayout) -> Self {
        Self {
            managed: ManagedSpace::build_with_layout(arena.clone(), &layout),
            raw: RawSpace::with_layout(arena.clone(), &layout),
            arena,
            layout,
            limits,
        }
    }

    /// Return the shared page arena.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the heap layout.
    pub fn layout(&self) -> &HeapLayout {
        &self.layout
    }

    /// Return the managed local allocation space.
    pub fn managed(&self) -> &ManagedSpace {
        &self.managed
    }

    /// Return the raw local allocation space.
    pub fn raw(&self) -> &RawSpace {
        &self.raw
    }

    /// Return the currently allocated managed references.
    pub fn allocated_references(&self) -> Vec<ManagedReference> {
        self.managed.allocated_references()
    }

    /// Return the current managed collector state.
    pub fn managed_gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Perform one young managed collection over explicit roots.
    pub fn collect_young_managed_references<I>(
        &mut self,
        roots: I,
    ) -> Result<GcStats, ManagedCollectError>
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        self.managed.collect_young_references(roots)
    }

    /// Perform one full managed collection over explicit roots.
    pub fn collect_managed_references<I>(
        &mut self,
        roots: I,
    ) -> Result<GcStats, ManagedCollectError>
    where
        I: IntoIterator<Item = ManagedReference>,
    {
        self.managed.collect_references(roots)
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}
