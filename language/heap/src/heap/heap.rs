use std::sync::Arc;

use crate::alloc::Arena;
use crate::managed::ManagedSpace;
use crate::raw::RawSpace;
use crate::value::ManagedReference;
use crate::{GcState, GcStats, HeapLimits, HeapOptions, HeapResult};

/// One live local heap rooted in one shared arena.
#[derive(Debug)]
pub struct Heap {
    /// The shared page arena for every local byte payload.
    pub(super) arena: Arc<Arena>,
    /// The configured heap options.
    pub(super) options: HeapOptions,
    /// The managed local allocation space.
    pub(super) managed: ManagedSpace,
    /// The raw local allocation space.
    pub(crate) raw: RawSpace,
    /// Exact hard limits for this heap.
    pub(super) limits: HeapLimits,
}

impl Heap {
    /// Create one heap with the default limits and options.
    pub fn new() -> HeapResult<Self> {
        Self::with_limits_and_options(HeapLimits::default(), HeapOptions::default())
    }

    /// Create one heap with explicit limits and options.
    pub fn with_limits_and_options(limits: HeapLimits, options: HeapOptions) -> HeapResult<Self> {
        let arena = Arc::new(Arena::try_new(
            options.page_bytes,
            options.arena_segment_bytes,
        )?);

        options.validate()?;
        options.validate_arena(&arena)?;

        Self::build_with_options(arena, limits, options)
    }

    /// Create one heap from one checked shared arena, limits, and options.
    fn build_with_options(
        arena: Arc<Arena>,
        limits: HeapLimits,
        options: HeapOptions,
    ) -> HeapResult<Self> {
        Ok(Self {
            managed: ManagedSpace::build_with_options(arena.clone(), &options)?,
            raw: RawSpace::with_options(arena.clone(), &options)?,
            arena,
            options,
            limits,
        })
    }

    /// Return the shared page arena.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the heap options.
    pub fn options(&self) -> &HeapOptions {
        &self.options
    }

    /// Return the managed local allocation space.
    pub fn managed(&self) -> &ManagedSpace {
        &self.managed
    }

    /// Return the raw local allocation space.
    pub fn raw(&self) -> &RawSpace {
        &self.raw
    }

    /// Return the currently live managed references.
    pub fn live_references(&self) -> HeapResult<Vec<ManagedReference>> {
        self.managed.live_references()
    }

    /// Return the current collector state.
    pub fn gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Perform one young managed collection over explicit roots.
    pub fn collect_young_managed_references(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<GcStats> {
        self.managed.collect_young_references(roots)
    }

    /// Perform one full managed collection over explicit roots.
    pub fn collect_managed_references(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<GcStats> {
        self.managed.collect_references(roots)
    }
}
