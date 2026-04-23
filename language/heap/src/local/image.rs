use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Heap;
use crate::allocator::{Allocator, AllocatorImage, PageId};
use crate::local::raw::{RawSpace, RawSpaceImage};
use crate::local::space::{
    GcState, HeapLocation, HeapPageOwner, HeapSpace, HeapSpaceImage, HeapStorage, live_page_views,
};
use crate::{HeapError, HeapLimits, HeapOptions, HeapReference, HeapResult};

/// One frozen heap root over one shared allocator.
#[derive(Debug, Clone)]
pub struct HeapImage {
    /// The shared allocator backing every captured page.
    allocator: Arc<Allocator>,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The captured heap-space root.
    heap: HeapSpaceImage,
    /// The captured raw-space root.
    raw: RawSpaceImage,
}

/// One serialized heap snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeapSnapshot {
    /// The serialized allocator pages reachable from this heap root.
    allocator: AllocatorImage,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The serialized heap-space root.
    heap: HeapSpaceImage,
    /// The serialized raw-space root.
    raw: RawSpaceImage,
}

impl HeapImage {
    /// Create one frozen heap root.
    pub(crate) fn new(
        allocator: Arc<Allocator>,
        options: HeapOptions,
        heap: HeapSpaceImage,
        raw: RawSpaceImage,
    ) -> Self {
        Self {
            allocator,
            options,
            heap,
            raw,
        }
    }

    /// Build one heap image from one serialized payload.
    fn from_snapshot(snapshot: &HeapSnapshot) -> Result<Self, HeapError> {
        let allocator = Arc::new(Allocator::from_image(&snapshot.allocator)?);
        let heap = snapshot.heap.clone();
        let raw = snapshot.raw.clone();

        Ok(Self {
            allocator,
            options: snapshot.options.clone(),
            heap,
            raw,
        })
    }

    /// Flatten one heap image into one serialized snapshot.
    fn snapshot(&self) -> Result<HeapSnapshot, HeapError> {
        // collect the reachable allocator pages once
        let page_ids = self.image_page_ids();

        Ok(HeapSnapshot {
            allocator: self.allocator.image_pages_from_ids(&page_ids)?,
            options: self.options.clone(),
            heap: self.heap.clone(),
            raw: self.raw.clone(),
        })
    }

    /// Return the shared allocator for this image.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
    }

    /// Return the heap options for this image.
    pub fn options(&self) -> &HeapOptions {
        &self.options
    }

    /// Return the heap-space root.
    pub(crate) fn heap(&self) -> &HeapSpaceImage {
        &self.heap
    }

    /// Return the raw-space root.
    pub(crate) fn raw(&self) -> &RawSpaceImage {
        &self.raw
    }

    /// Return the captured collector state.
    pub fn gc_state(&self) -> &GcState {
        self.heap.gc_state()
    }

    /// Return the total page count reachable from this heap image.
    pub fn page_count(&self) -> usize {
        self.image_page_ids().len()
    }

    /// Return the total local allocated bytes captured by this image.
    pub fn local_allocated_bytes(&self) -> HeapResult<u64> {
        self.heap
            .allocated_bytes()
            .checked_add(self.raw.allocated_bytes())
            .ok_or(HeapError::InvariantOverflow {
                context: "heap image allocated bytes",
            })
    }

    /// Return whether one heap entry shares allocator storage with another heap root.
    #[doc(hidden)]
    pub fn shares_heap_allocation_with(&self, other: &Self, reference: HeapReference) -> bool {
        // resolve the captured heap locations first
        let Some(location) = image_heap_location(&self.allocator, &self.heap, reference) else {
            return false;
        };
        let Some(other_location) = image_heap_location(&other.allocator, &other.heap, reference)
        else {
            return false;
        };

        // sharing only makes sense inside one shared allocator
        if !Arc::ptr_eq(&self.allocator, &other.allocator) {
            return false;
        }

        // compare the location-specific page maps
        match (location.storage, other_location.storage) {
            (HeapStorage::Young(_), HeapStorage::Young(_)) => {
                self.heap.young().pages() == other.heap.young().pages()
            }
            (HeapStorage::Small(slot), HeapStorage::Small(other_slot)) => {
                let Some(span) = self.heap.spans().get(slot.span_index()) else {
                    return false;
                };
                let Some(other_span) = other.heap.spans().get(other_slot.span_index()) else {
                    return false;
                };

                slot == other_slot && span.pages == other_span.pages
            }
            (HeapStorage::Large(entry), HeapStorage::Large(other_entry)) => {
                let Ok(entry_index) = entry.index() else {
                    return false;
                };
                let Ok(other_entry_index) = other_entry.index() else {
                    return false;
                };
                let Some(entry) = self.heap.entries().get(entry_index) else {
                    return false;
                };
                let Some(other_entry) = other.heap.entries().get(other_entry_index) else {
                    return false;
                };

                entry.pages == other_entry.pages
            }
            _ => false,
        }
    }

    /// Return every allocator page reachable from this heap image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // collect the young-space pages first
        pages.extend(self.heap.young().pages().page_ids());

        // collect every heap span and entry page
        for span in self.heap.spans() {
            pages.extend(span.pages.page_ids());
        }

        for entry in self.heap.entries() {
            pages.extend(entry.pages.page_ids());
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

impl Heap {
    /// Fork one live heap over the same shared allocator.
    pub fn fork(&mut self) -> Result<Self, HeapError> {
        let heap = self.heap.fork()?;
        let raw = match self.raw.fork() {
            Ok(raw) => raw,
            Err(error) => {
                for page_view in live_page_views(&heap).into_iter().rev() {
                    self.allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }
        };

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

    /// Create one heap from one frozen heap root and explicit hard limits.
    pub fn from_image_with_limits(
        image: &HeapImage,
        limits: HeapLimits,
    ) -> Result<Self, HeapError> {
        image.options().validate_local()?;

        let heap = HeapSpace::from_image(image.allocator().clone(), image.heap())?;
        let raw = match RawSpace::from_image(image.allocator().clone(), image.raw()) {
            Ok(raw) => raw,
            Err(error) => {
                for page_view in live_page_views(&heap).into_iter().rev() {
                    image.allocator().release_page_view(&page_view)?;
                }

                return Err(error);
            }
        };

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
        let raw = self.raw.image();

        Ok(HeapImage::new(
            self.allocator().clone(),
            self.options().clone(),
            heap,
            raw,
        ))
    }

    /// Restore this heap from one frozen heap root.
    pub fn restore_image(&mut self, image: &HeapImage) -> Result<(), HeapError> {
        self.heap.check_branch_boundary()?;

        let limits = self.limits;
        *self = Self::from_image_with_limits(image, limits)?;

        Ok(())
    }
}

impl Serialize for HeapImage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let snapshot = self.snapshot().map_err(serde::ser::Error::custom)?;

        snapshot.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HeapImage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let snapshot = HeapSnapshot::deserialize(deserializer)?;

        Self::from_snapshot(&snapshot).map_err(serde::de::Error::custom)
    }
}

/// Return the resolved heap page owner for one captured page.
fn image_heap_page_owner(image: &HeapSpaceImage, page_id: PageId) -> Option<HeapPageOwner> {
    for logical_page_index in 0..image.young().pages().len() {
        let image_page_id = image.young().pages().page(logical_page_index)?;
        if image_page_id == page_id {
            return Some(HeapPageOwner::Young { logical_page_index });
        }
    }

    for (span_index, span) in image.spans().iter().enumerate() {
        for logical_page_index in 0..span.pages.len() {
            let image_page_id = span.pages.page(logical_page_index)?;
            if image_page_id == page_id {
                return Some(HeapPageOwner::Small {
                    span_index,
                    logical_page_index,
                });
            }
        }
    }

    for (entry_index, entry) in image.entries().iter().enumerate() {
        for logical_page_index in 0..entry.pages.len() {
            let image_page_id = entry.pages.page(logical_page_index)?;
            if image_page_id == page_id {
                return Some(HeapPageOwner::Large {
                    entry_id: crate::local::space::LargeEntryId::new(entry_index as u64 + 1),
                    logical_page_index,
                });
            }
        }
    }

    None
}

/// Return the resolved heap location for one captured heap reference.
fn image_heap_location(
    allocator: &Allocator,
    image: &HeapSpaceImage,
    reference: HeapReference,
) -> Option<HeapLocation> {
    let (page_id, page_offset) = allocator.address_page_position(reference.address())?;
    let owner = image_heap_page_owner(image, page_id)?;

    match owner {
        HeapPageOwner::Young { logical_page_index } => {
            let logical_byte_offset = logical_page_index
                .checked_mul(image.young().page_bytes())?
                .checked_add(page_offset)?;

            for (entry_index, entry) in image.young().entries().iter().enumerate() {
                if !entry.is_live {
                    continue;
                }

                let entry_offset = entry.first_page as usize * image.young().page_bytes()
                    + entry.first_offset as usize;
                let entry_end = entry_offset.checked_add(entry.byte_len)?;
                if logical_byte_offset < entry_offset || logical_byte_offset >= entry_end {
                    continue;
                }

                let base_address = allocator
                    .page_view_ptr(image.young().pages(), entry_offset)
                    .ok()? as *mut u8 as usize;
                let byte_offset = logical_byte_offset.checked_sub(entry_offset)?;

                return Some(HeapLocation {
                    storage: HeapStorage::Young(crate::local::space::HeapYoungId::new(
                        image.young().generation(),
                        entry_index as u32,
                    )),
                    base: HeapReference::new(base_address),
                    byte_offset,
                    byte_len: entry.byte_len,
                });
            }

            None
        }
        HeapPageOwner::Small {
            span_index,
            logical_page_index,
        } => {
            let span = image.spans().get(span_index)?;
            let logical_byte_offset = logical_page_index
                .checked_mul(image.page_bytes())?
                .checked_add(page_offset)?;
            let slot_index = logical_byte_offset / span.class.size_class;
            let slot_offset = logical_byte_offset % span.class.size_class;

            if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
                return None;
            }

            let byte_len = *span.byte_lens.get(slot_index)?;
            if slot_offset >= byte_len {
                return None;
            }

            let slot_base_offset = slot_index.checked_mul(span.class.size_class)?;
            let base_address = allocator
                .page_view_ptr(&span.pages, slot_base_offset)
                .ok()? as *mut u8 as usize;
            let slot = crate::allocator::SpanSlot::new(span_index, slot_index).ok()?;

            Some(HeapLocation {
                storage: HeapStorage::Small(slot),
                base: HeapReference::new(base_address),
                byte_offset: slot_offset,
                byte_len,
            })
        }
        HeapPageOwner::Large {
            entry_id,
            logical_page_index,
        } => {
            let entry = image.entries().get(entry_id.index().ok()?)?;
            let logical_byte_offset = logical_page_index
                .checked_mul(image.page_bytes())?
                .checked_add(page_offset)?;

            if logical_byte_offset >= entry.len {
                return None;
            }

            let base_address = allocator.page_view_ptr(&entry.pages, 0).ok()? as *mut u8 as usize;

            Some(HeapLocation {
                storage: HeapStorage::Large(entry_id),
                base: HeapReference::new(base_address),
                byte_offset: logical_byte_offset,
                byte_len: entry.len,
            })
        }
    }
}
