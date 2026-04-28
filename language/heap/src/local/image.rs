use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::Heap;
use crate::allocator::{Allocator, AllocatorImage, PageId, PageRun};
use crate::local::raw::{RawSpace, RawSpaceImage};
use crate::local::space::{
    GcState, HeapLocation, HeapPageMapEntry, HeapPlace, HeapSpace, HeapSpaceImage,
};
use crate::{HeapError, HeapLimits, HeapOptions, HeapReference, HeapResult};

/// One frozen heap root over one shared allocator.
#[derive(Debug, Clone)]
pub struct HeapImage {
    /// The retained heap image root.
    inner: Arc<HeapImageRoot>,
}

/// One retained heap image root.
#[derive(Debug)]
struct HeapImageRoot {
    /// The shared allocator backing every captured page.
    allocator: Arc<Allocator>,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The captured heap-space root.
    heap: HeapSpaceImage,
    /// The captured raw-space root.
    raw: RawSpaceImage,
    /// The page runs owned by this image.
    page_runs: Box<[PageRun]>,
}

/// One serialized heap snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// The serialized allocator pages reachable from this heap root.
    allocator: AllocatorImage,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The serialized heap-space root.
    heap: HeapSpaceImage,
    /// The serialized raw-space root.
    raw: RawSpaceImage,
}

impl Drop for HeapImageRoot {
    fn drop(&mut self) {
        let _ = self.allocator.release_page_runs(&self.page_runs);
    }
}

impl HeapImage {
    /// Create one frozen heap root.
    pub(crate) fn new(
        allocator: Arc<Allocator>,
        options: HeapOptions,
        heap: HeapSpaceImage,
        raw: RawSpaceImage,
    ) -> HeapResult<Self> {
        let page_runs = heap.page_runs().into_boxed_slice();
        let root = HeapImageRoot {
            allocator,
            options,
            heap,
            raw,
            page_runs,
        };

        Ok(Self {
            inner: Arc::new(root),
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

        let root = HeapImageRoot {
            allocator,
            options: snapshot.options.clone(),
            heap,
            raw,
            page_runs,
        };

        Ok(Self {
            inner: Arc::new(root),
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

    /// Return the heap-space root.
    pub(crate) fn heap(&self) -> &HeapSpaceImage {
        &self.inner.heap
    }

    /// Return the raw-space root.
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

    /// Return whether one heap allocation shares allocator place with another heap root.
    #[doc(hidden)]
    pub fn shares_heap_allocation_with(&self, other: &Self, reference: HeapReference) -> bool {
        // resolve the captured heap locations first
        let Some(location) = image_heap_location(self.allocator(), self.heap(), reference) else {
            return false;
        };
        let Some(other_location) = image_heap_location(other.allocator(), other.heap(), reference)
        else {
            return false;
        };

        // sharing only makes sense inside one shared allocator
        if !Arc::ptr_eq(self.allocator(), other.allocator()) {
            return false;
        }

        // compare the location-specific page maps
        match (location.place, other_location.place) {
            (HeapPlace::Young { .. }, HeapPlace::Young { .. }) => {
                self.heap().young().pages() == other.heap().young().pages()
            }
            (HeapPlace::Small(slot), HeapPlace::Small(other_slot)) => {
                let Some(span) = self.heap().spans().get(slot.span_index()) else {
                    return false;
                };
                let Some(other_span) = other.heap().spans().get(other_slot.span_index()) else {
                    return false;
                };

                slot == other_slot && span.pages == other_span.pages
            }
            (HeapPlace::Large(allocation), HeapPlace::Large(other_allocation)) => {
                let Ok(allocation_index) = allocation.index() else {
                    return false;
                };
                let Ok(other_allocation_index) = other_allocation.index() else {
                    return false;
                };
                let Some(allocation) = self.heap().allocations().get(allocation_index) else {
                    return false;
                };
                let Some(other_allocation) = other.heap().allocations().get(other_allocation_index)
                else {
                    return false;
                };

                allocation.pages == other_allocation.pages
            }
            _ => false,
        }
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
    pub fn fork(&mut self) -> Result<Self, HeapError> {
        let heap = self.heap.fork()?;
        let raw = self.raw.fork()?;

        Ok(Self {
            allocator: self.allocator.clone(),
            options: self.options.clone(),
            gc_pacer: self.gc_pacer,
            gc_request: self.gc_request,
            heap,
            raw,
            limits: self.limits,
        })
    }

    /// Create one heap from one frozen heap root.
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

    /// Create one heap from one frozen heap root and explicit hard limits.
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
            gc_request: None,
            heap,
            raw,
            limits,
        };

        heap.refresh_gc_request();

        Ok(heap)
    }

    /// Capture one frozen heap root.
    pub fn image(&mut self) -> Result<HeapImage, HeapError> {
        let heap = self.heap.image()?;
        let raw = self.raw.image()?;

        HeapImage::new(self.allocator().clone(), self.options().clone(), heap, raw)
    }

    /// Restore this heap from one frozen heap root.
    pub fn restore_image(&mut self, image: &HeapImage) -> Result<(), HeapError> {
        self.heap.check_branch_boundary()?;

        let limits = self.limits;
        *self = Self::from_image_with_limits(image, limits)?;

        Ok(())
    }
}

/// Return the resolved heap page map entry for one captured page.
fn image_heap_page_entry(image: &HeapSpaceImage, page_index: usize) -> Option<HeapPageMapEntry> {
    if page_index < image.young().pages().len() {
        return Some(HeapPageMapEntry::Young {
            logical_page_index: page_index,
        });
    }

    for (span_index, span) in image.spans().iter().enumerate() {
        let first_page_index = span.first_offset / image.page_bytes();
        let end_page_index = first_page_index + span.pages.len();
        if page_index < first_page_index || page_index >= end_page_index {
            continue;
        }

        let logical_page_index = page_index - first_page_index;

        return Some(HeapPageMapEntry::Small {
            span_index,
            logical_page_index,
        });
    }

    for (allocation_index, allocation) in image.allocations().iter().enumerate() {
        let first_page_index = allocation.first_offset / image.page_bytes();
        let end_page_index = first_page_index + allocation.pages.len();
        if page_index < first_page_index || page_index >= end_page_index {
            continue;
        }

        let logical_page_index = page_index - first_page_index;

        return Some(HeapPageMapEntry::Large {
            allocation_id: crate::local::space::LargeAllocationId::new(allocation_index as u64 + 1),
            logical_page_index,
        });
    }

    None
}

/// Return the resolved heap location for one captured heap reference.
fn image_heap_location(
    allocator: &Allocator,
    image: &HeapSpaceImage,
    reference: HeapReference,
) -> Option<HeapLocation> {
    let page_bytes = allocator.page_bytes();
    let page_index = reference.offset() / page_bytes;
    let page_offset = reference.offset() % page_bytes;
    let entry = image_heap_page_entry(image, page_index)?;

    match entry {
        HeapPageMapEntry::Young { logical_page_index } => {
            let logical_byte_offset = logical_page_index * image.young().page_bytes() + page_offset;

            // young ranges are bump ordered, so offset resolution is predecessor lookup
            let range_end = image
                .young()
                .ranges()
                .partition_point(|range| range.first_offset <= logical_byte_offset);
            if range_end == 0 {
                return None;
            }

            let range_index = range_end - 1;
            let allocation = image.young().ranges().get(range_index)?;
            if !image.young().live().contains(range_index) {
                return None;
            }

            let allocation_offset = allocation.first_offset;
            let allocation_limit = allocation_offset + allocation.byte_len;
            if logical_byte_offset >= allocation_limit {
                return None;
            }

            let byte_offset = logical_byte_offset - allocation_offset;

            Some(HeapLocation {
                place: HeapPlace::Young {
                    first_offset: allocation_offset,
                },
                base: HeapReference::new(allocation_offset),
                byte_offset,
                byte_len: allocation.byte_len,
            })
        }
        HeapPageMapEntry::Small {
            span_index,
            logical_page_index,
        } => {
            let span = image.spans().get(span_index)?;
            let logical_byte_offset = logical_page_index * image.page_bytes() + page_offset;
            let slot_index = logical_byte_offset / span.class.size_class;
            let slot_offset = logical_byte_offset % span.class.size_class;

            if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
                return None;
            }

            let byte_len = span.class.size_class;
            if slot_offset >= byte_len {
                return None;
            }

            let slot_base_offset = slot_index * span.class.size_class;
            let base_offset = span.first_offset + slot_base_offset;
            let slot = crate::allocator::SpanSlot::new(span_index, slot_index).ok()?;

            Some(HeapLocation {
                place: HeapPlace::Small(slot),
                base: HeapReference::new(base_offset),
                byte_offset: slot_offset,
                byte_len,
            })
        }
        HeapPageMapEntry::Large {
            allocation_id,
            logical_page_index,
        } => {
            let allocation = image.allocations().get(allocation_id.index().ok()?)?;
            let logical_byte_offset = logical_page_index * image.page_bytes() + page_offset;

            if allocation.len == 0 {
                if logical_byte_offset != 0 {
                    return None;
                }
            } else if logical_byte_offset >= allocation.len {
                return None;
            }

            Some(HeapLocation {
                place: HeapPlace::Large(allocation_id),
                base: HeapReference::new(allocation.first_offset),
                byte_offset: logical_byte_offset,
                byte_len: allocation.len,
            })
        }
    }
}
