use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_memory::MemoryMap;

use super::Heap;
use crate::local::storage::{HeapStorage, HeapStorageImage};
use crate::{GcState, HeapError, HeapLimits, HeapOptions, TraceView};

/// One frozen local heap metadata image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapImage {
    /// The retained heap image state.
    state: Arc<ImageState>,
}

/// One retained local heap metadata state.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ImageState {
    /// The heap options used by this image.
    options: HeapOptions,

    /// The captured heap storage metadata.
    storage: HeapStorageImage,
}

impl HeapImage {
    /// Create one frozen local heap metadata image.
    pub(crate) fn new(options: HeapOptions, storage: HeapStorageImage) -> Self {
        let state = ImageState { options, storage };

        Self {
            state: Arc::new(state),
        }
    }

    /// Return the heap options for this image.
    pub fn options(&self) -> &HeapOptions {
        &self.state.options
    }

    /// Return the local heap storage metadata.
    pub(crate) fn storage(&self) -> &HeapStorageImage {
        &self.state.storage
    }

    /// Return the captured collector state.
    pub fn gc_state(&self) -> &GcState {
        &self.storage().gc_state
    }

    /// Return the retained local heap page count.
    pub fn page_count(&self) -> usize {
        self.storage().page_count()
    }

    /// Return the allocated local heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.storage().allocated_bytes
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
            is_gc_requested: self.is_gc_requested,
            storage,
            limits: self.limits,
        })
    }

    /// Restore one heap image over its captured world memory and explicit hard limits.
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
            is_gc_requested: false,
            storage,
            limits,
        };

        heap.refresh_gc_request();

        Ok(heap)
    }

    /// Capture one frozen local heap metadata image.
    pub fn image(&mut self) -> Result<HeapImage, HeapError> {
        let storage = self.storage.image()?;

        Ok(HeapImage::new(self.options().clone(), storage))
    }

    /// Restore this heap image over its captured world memory.
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
