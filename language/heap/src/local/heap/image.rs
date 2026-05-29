use std::sync::Arc;

use destack_mir::TraceTable;
use serde::{Deserialize, Serialize};

use super::Heap;
use crate::allocator::Allocator;
use crate::local::storage::{GcState, HeapStorage, HeapStorageImage};
use crate::{HeapError, HeapLimits, HeapOptions};

/// One frozen heap image over one shared allocator.
#[derive(Debug, Clone)]
pub struct HeapImage {
    /// The retained heap image state.
    state: Arc<ImageState>,
}

/// One retained heap image state.
#[derive(Debug)]
struct ImageState {
    /// The shared allocator backing every captured page.
    allocator: Arc<Allocator>,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The captured heap storage image.
    storage: HeapStorageImage,
}

/// One serialized heap snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// The heap options used by this image.
    options: HeapOptions,

    /// The serialized heap storage image.
    storage: HeapStorageImage,
}

impl HeapImage {
    /// Create one frozen heap image.
    pub(crate) fn new(
        allocator: Arc<Allocator>,
        options: HeapOptions,
        storage: HeapStorageImage,
    ) -> Self {
        let state = ImageState {
            allocator,
            options,
            storage,
        };

        Self {
            state: Arc::new(state),
        }
    }

    /// Build one heap image from one serialized payload.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Result<Self, HeapError> {
        let allocator = Arc::new(Allocator::try_new(
            snapshot.options.page_size_bytes,
            snapshot.options.allocator_chunk_size_bytes,
        )?);

        Self::from_snapshot_with_allocator(snapshot, allocator)
    }

    /// Build one heap image from one serialized payload and allocator.
    pub fn from_snapshot_with_allocator(
        snapshot: &HeapSnapshot,
        allocator: Arc<Allocator>,
    ) -> Result<Self, HeapError> {
        let storage = snapshot.storage.clone();
        let state = ImageState {
            allocator,
            options: snapshot.options.clone(),
            storage,
        };

        Ok(Self {
            state: Arc::new(state),
        })
    }

    /// Flatten one heap image into one serialized snapshot.
    pub fn snapshot(&self) -> HeapSnapshot {
        HeapSnapshot {
            options: self.options().clone(),
            storage: self.storage().clone(),
        }
    }

    /// Return the shared allocator for this image.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.state.allocator
    }

    /// Return the heap options for this image.
    pub fn options(&self) -> &HeapOptions {
        &self.state.options
    }

    /// Return the heap storage image.
    pub(crate) fn storage(&self) -> &HeapStorageImage {
        &self.state.storage
    }

    /// Return the captured collector state.
    pub fn gc_state(&self) -> &GcState {
        self.storage().gc_state()
    }

    /// Return the total local allocated bytes captured by this image.
    pub fn local_allocated_bytes(&self) -> u64 {
        self.storage().allocated_bytes()
    }
}

impl HeapSnapshot {
    /// Return the captured collector state.
    pub fn gc_state(&self) -> &GcState {
        self.storage.gc_state()
    }

    /// Return the total page count reachable from this heap snapshot.
    pub fn page_count(&self) -> usize {
        self.storage.page_count()
    }

    /// Return the total allocated bytes captured by this snapshot.
    pub fn allocated_bytes(&self) -> u64 {
        self.storage.allocated_bytes()
    }
}

impl Heap {
    /// Fork one live heap over the same shared allocator.
    ///
    /// Call this only from a safepoint where the heap cannot mutate.
    pub fn fork(&mut self, trace_table: &TraceTable) -> Result<Self, HeapError> {
        let storage = self.storage.fork(trace_table)?;

        Ok(Self {
            options: self.options.clone(),
            gc_pacer: self.gc_pacer,
            gc_request: self.gc_request,
            storage,
            limits: self.limits,
        })
    }

    /// Create one heap from one frozen heap image.
    pub fn from_image(image: &HeapImage, trace_table: &TraceTable) -> Result<Self, HeapError> {
        Self::from_image_with_limits(image, HeapLimits::default(), trace_table)
    }

    /// Create one heap from one serialized heap snapshot.
    pub fn from_snapshot(
        snapshot: &HeapSnapshot,
        trace_table: &TraceTable,
    ) -> Result<Self, HeapError> {
        let image = HeapImage::from_snapshot(snapshot)?;

        Self::from_image(&image, trace_table)
    }

    /// Create one heap from one serialized heap snapshot and explicit hard limits.
    pub fn from_snapshot_with_limits(
        snapshot: &HeapSnapshot,
        limits: HeapLimits,
        trace_table: &TraceTable,
    ) -> Result<Self, HeapError> {
        let image = HeapImage::from_snapshot(snapshot)?;

        Self::from_image_with_limits(&image, limits, trace_table)
    }

    /// Create one heap from one serialized heap snapshot, allocator, and explicit hard limits.
    pub fn from_snapshot_with_allocator(
        snapshot: &HeapSnapshot,
        allocator: Arc<Allocator>,
        limits: HeapLimits,
        trace_table: &TraceTable,
    ) -> Result<Self, HeapError> {
        let image = HeapImage::from_snapshot_with_allocator(snapshot, allocator)?;

        Self::from_image_with_limits(&image, limits, trace_table)
    }

    /// Create one heap from one frozen heap image and explicit hard limits.
    pub fn from_image_with_limits(
        image: &HeapImage,
        limits: HeapLimits,
        trace_table: &TraceTable,
    ) -> Result<Self, HeapError> {
        image.options().validate_local()?;

        let storage =
            HeapStorage::from_image(image.allocator().clone(), image.storage(), trace_table)?;

        let mut heap = Self {
            options: image.options().clone(),
            gc_pacer: Default::default(),
            gc_request: None,
            storage,
            limits,
        };

        heap.refresh_gc_request();

        Ok(heap)
    }

    /// Capture one frozen heap image.
    pub fn image(&mut self) -> Result<HeapImage, HeapError> {
        let storage = self.storage.image()?;

        Ok(HeapImage::new(
            self.allocator().clone(),
            self.options().clone(),
            storage,
        ))
    }

    /// Restore this heap from one frozen heap image.
    pub fn restore_image(
        &mut self,
        image: &HeapImage,
        trace_table: &TraceTable,
    ) -> Result<(), HeapError> {
        self.storage.check_branch_boundary()?;

        let limits = self.limits;
        *self = Self::from_image_with_limits(image, limits, trace_table)?;

        Ok(())
    }
}
