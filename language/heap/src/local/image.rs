use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::Heap;
use crate::allocator::{Allocator, AllocatorImage, PageId, PageRun};
use crate::local::raw::{RawSpace, RawSpaceImage};
use crate::local::space::{GcState, HeapSpace, HeapSpaceImage};
use crate::{HeapError, HeapLimits, HeapOptions, HeapResult};

/// One frozen heap image over one shared allocator.
#[derive(Debug, Clone)]
pub struct HeapImage {
    /// The retained heap image state.
    inner: Arc<HeapImageState>,
}

/// One retained heap image state.
#[derive(Debug)]
struct HeapImageState {
    /// The shared allocator backing every captured page.
    allocator: Arc<Allocator>,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The captured heap-space image.
    heap: HeapSpaceImage,
    /// The captured raw-space image.
    raw: RawSpaceImage,
    /// The page runs owned by this image.
    page_runs: Box<[PageRun]>,
}

/// One serialized heap snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// The serialized allocator pages reachable from this heap image.
    allocator: AllocatorImage,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The serialized heap-space image.
    heap: HeapSpaceImage,
    /// The serialized raw-space image.
    raw: RawSpaceImage,
}

impl Drop for HeapImageState {
    fn drop(&mut self) {
        let _ = self.allocator.release_page_runs(&self.page_runs);
    }
}

impl HeapImage {
    /// Create one frozen heap image.
    pub(crate) fn new(
        allocator: Arc<Allocator>,
        options: HeapOptions,
        heap: HeapSpaceImage,
        raw: RawSpaceImage,
    ) -> HeapResult<Self> {
        let page_runs = heap.page_runs().into_boxed_slice();
        let state = HeapImageState {
            allocator,
            options,
            heap,
            raw,
            page_runs,
        };

        Ok(Self {
            inner: Arc::new(state),
        })
    }

    /// Build one heap image from one serialized payload.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Result<Self, HeapError> {
        let allocator = Arc::new(Allocator::try_new(
            snapshot.allocator.page_bytes as usize,
            snapshot.allocator.chunk_bytes as usize,
        )?);

        Self::from_snapshot_with_allocator(snapshot, allocator)
    }

    /// Build one heap image from one serialized payload and allocator.
    pub fn from_snapshot_with_allocator(
        snapshot: &HeapSnapshot,
        allocator: Arc<Allocator>,
    ) -> Result<Self, HeapError> {
        allocator.restore_image_pages(&snapshot.allocator)?;
        let heap = snapshot.heap.clone();
        let raw = snapshot.raw.clone();
        let page_runs = heap.page_runs().into_boxed_slice();

        allocator.restore_page_run_refs(&page_runs)?;

        let state = HeapImageState {
            allocator,
            options: snapshot.options.clone(),
            heap,
            raw,
            page_runs,
        };

        Ok(Self {
            inner: Arc::new(state),
        })
    }

    /// Flatten one heap image into one serialized snapshot.
    pub fn snapshot(&self) -> Result<HeapSnapshot, HeapError> {
        // collect the reachable allocator pages once
        let page_ids = self.image_page_ids();

        Ok(HeapSnapshot {
            allocator: self.allocator().image_pages_from_ids(&page_ids)?,
            options: self.options().clone(),
            heap: self.heap().clone(),
            raw: self.raw().clone(),
        })
    }

    /// Return the shared allocator for this image.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.inner.allocator
    }

    /// Return the heap options for this image.
    pub fn options(&self) -> &HeapOptions {
        &self.inner.options
    }

    /// Return the heap-space image.
    pub(crate) fn heap(&self) -> &HeapSpaceImage {
        &self.inner.heap
    }

    /// Return the raw-space image.
    pub(crate) fn raw(&self) -> &RawSpaceImage {
        &self.inner.raw
    }

    /// Return the captured collector state.
    pub fn gc_state(&self) -> &GcState {
        self.heap().gc_state()
    }

    /// Return the total page count reachable from this heap image.
    pub fn page_count(&self) -> usize {
        self.image_page_ids().len()
    }

    /// Return the total local allocated bytes captured by this image.
    pub fn local_allocated_bytes(&self) -> u64 {
        self.heap().allocated_bytes() + self.raw().allocated_bytes()
    }

    /// Return every allocator page reachable from this heap image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // collect the young-space pages first
        pages.extend(self.heap().young().pages().page_ids());

        // collect every heap span and allocation page
        for span in self.heap().spans() {
            pages.extend(span.pages.page_ids());
        }

        for allocation in self.heap().allocations() {
            pages.extend(allocation.pages.page_ids());
        }

        pages
    }

    /// Return the deduplicated page ids for one serialized image.
    fn image_page_ids(&self) -> Vec<PageId> {
        self.page_ids()
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

impl HeapSnapshot {
    /// Return the captured collector state.
    pub fn gc_state(&self) -> &GcState {
        self.heap.gc_state()
    }

    /// Return the total page count reachable from this heap snapshot.
    pub fn page_count(&self) -> usize {
        self.allocator.pages.len()
    }

    /// Return the total allocated bytes captured by this snapshot.
    pub fn allocated_bytes(&self) -> u64 {
        self.heap.allocated_bytes() + self.raw.allocated_bytes()
    }
}

impl Heap {
    /// Fork one live heap over the same shared allocator.
    ///
    /// Call this only from a safepoint where the heap cannot mutate.
    pub fn fork(&mut self) -> Result<Self, HeapError> {
        let heap = self.heap.fork()?;
        let raw = self.raw.fork()?;

        Ok(Self {
            allocator: self.allocator.clone(),
            options: self.options.clone(),
            gc_pacer: self.gc_pacer,
            young_trigger_bytes: self.young_trigger_bytes,
            gc_request: self.gc_request,
            heap,
            raw,
            limits: self.limits,
        })
    }

    /// Create one heap from one frozen heap image.
    pub fn from_image(image: &HeapImage) -> Result<Self, HeapError> {
        Self::from_image_with_limits(image, HeapLimits::default())
    }

    /// Create one heap from one serialized heap snapshot.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Result<Self, HeapError> {
        let image = HeapImage::from_snapshot(snapshot)?;

        Self::from_image(&image)
    }

    /// Create one heap from one serialized heap snapshot and explicit hard limits.
    pub fn from_snapshot_with_limits(
        snapshot: &HeapSnapshot,
        limits: HeapLimits,
    ) -> Result<Self, HeapError> {
        let image = HeapImage::from_snapshot(snapshot)?;

        Self::from_image_with_limits(&image, limits)
    }

    /// Create one heap from one serialized heap snapshot, allocator, and explicit hard limits.
    pub fn from_snapshot_with_allocator(
        snapshot: &HeapSnapshot,
        allocator: Arc<Allocator>,
        limits: HeapLimits,
    ) -> Result<Self, HeapError> {
        let image = HeapImage::from_snapshot_with_allocator(snapshot, allocator)?;

        Self::from_image_with_limits(&image, limits)
    }

    /// Create one heap from one frozen heap image and explicit hard limits.
    pub fn from_image_with_limits(
        image: &HeapImage,
        limits: HeapLimits,
    ) -> Result<Self, HeapError> {
        image.options().validate_local()?;

        let heap = HeapSpace::from_image(image.allocator().clone(), image.heap())?;
        let raw = RawSpace::from_image(image.allocator().clone(), image.raw())?;

        let mut heap = Self {
            allocator: image.allocator().clone(),
            options: image.options().clone(),
            gc_pacer: Default::default(),
            young_trigger_bytes: 0,
            gc_request: None,
            heap,
            raw,
            limits,
        };

        heap.young_trigger_bytes = heap.young_trigger_bytes();
        heap.refresh_gc_request();

        Ok(heap)
    }

    /// Capture one frozen heap image.
    pub fn image(&mut self) -> Result<HeapImage, HeapError> {
        let heap = self.heap.image()?;
        let raw = self.raw.image()?;

        HeapImage::new(self.allocator().clone(), self.options().clone(), heap, raw)
    }

    /// Restore this heap from one frozen heap image.
    pub fn restore_image(&mut self, image: &HeapImage) -> Result<(), HeapError> {
        self.heap.check_branch_boundary()?;

        let limits = self.limits;
        *self = Self::from_image_with_limits(image, limits)?;

        Ok(())
    }
}
