use destack_serde::Reflect;
use std::sync::Arc;

use crate::TraceView;
use serde::{Deserialize, Serialize};

use super::Heap;
use crate::local::storage::{GcState, HeapStorage, HeapStorageImage};
use crate::{HeapError, HeapLimits, HeapOptions};
use destack_memory::MemoryMap;

/// One frozen heap image over one shared memory.
#[derive(Debug, Clone)]
pub struct HeapImage {
    /// The retained heap image state.
    state: Arc<ImageState>,
}

/// One retained heap image state.
#[derive(Debug)]
struct ImageState {
    /// The heap options used by this image.
    options: HeapOptions,

    /// The captured heap storage image.
    storage: HeapStorageImage,
}

/// One serialized heap snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct HeapSnapshot {
    /// The heap options used by this image.
    options: HeapOptions,

    /// The serialized heap storage image.
    storage: HeapStorageImage,
}

impl HeapImage {
    /// Create one frozen heap image.
    pub(crate) fn new(options: HeapOptions, storage: HeapStorageImage) -> Self {
        let state = ImageState { options, storage };

        Self {
            state: Arc::new(state),
        }
    }

    /// Build one heap image from one serialized payload and memory.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Self {
        let storage = snapshot.storage.clone();
        let state = ImageState {
            options: snapshot.options.clone(),
            storage,
        };

        Self {
            state: Arc::new(state),
        }
    }

    /// Flatten one heap image into one serialized snapshot.
    pub fn snapshot(&self) -> HeapSnapshot {
        HeapSnapshot {
            options: self.options().clone(),
            storage: self.storage().clone(),
        }
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
    /// Fork one live heap over the same shared memory.
    ///
    /// Call this only from a safepoint where the heap cannot mutate.
    pub fn fork(
        &mut self,
        memory: Arc<MemoryMap>,
        trace_view: TraceView<'_>,
    ) -> Result<Self, HeapError> {
        let storage = self.storage.fork(memory, trace_view)?;

        Ok(Self {
            options: self.options.clone(),
            gc_pacer: self.gc_pacer,
            gc_request: self.gc_request,
            storage,
            limits: self.limits,
        })
    }

    /// Create one heap from one serialized heap snapshot, memory, and explicit hard limits.
    pub fn from_snapshot(
        snapshot: &HeapSnapshot,
        memory: Arc<MemoryMap>,
        limits: HeapLimits,
        trace_view: TraceView<'_>,
    ) -> Result<Self, HeapError> {
        let image = HeapImage::from_snapshot(snapshot);

        Self::from_image(&image, memory, limits, trace_view)
    }

    /// Create one heap from one frozen heap image and explicit hard limits.
    pub fn from_image(
        image: &HeapImage,
        memory: Arc<MemoryMap>,
        limits: HeapLimits,
        trace_view: TraceView<'_>,
    ) -> Result<Self, HeapError> {
        image.options().validate_local()?;

        let storage = HeapStorage::from_image(memory, image.storage(), trace_view)?;

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

        Ok(HeapImage::new(self.options().clone(), storage))
    }

    /// Restore this heap from one frozen heap image.
    pub fn restore_image(
        &mut self,
        image: &HeapImage,
        memory: Arc<MemoryMap>,
        trace_view: TraceView<'_>,
    ) -> Result<(), HeapError> {
        self.storage.check_branch_boundary()?;

        let limits = self.limits;
        *self = Self::from_image(image, memory, limits, trace_view)?;

        Ok(())
    }
}
