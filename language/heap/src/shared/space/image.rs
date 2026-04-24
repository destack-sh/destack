use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{
    SharedHeapLargeAllocationImage, SharedHeapPageOwner, SharedHeapSmallSpanImage, SharedHeapSpace,
    SharedLargeAllocation, SharedLargeAllocationId, SharedSmallSpan,
};
use crate::allocator::PageRunCache;
use crate::shared::gc::SharedGcState;
use crate::shared::space::{SharedHeapState, SharedLargeSpace, SharedSmallSpace, find_free_cursor};
use crate::{AllocationUsage, Allocator, GcState, HeapResult, PageId, SizeClassTable};

/// One frozen shared heap-space root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedHeapSpaceImage {
    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured shared heap spans.
    spans: Box<[SharedHeapSmallSpanImage]>,
    /// The configured shared page width.
    page_bytes: usize,
    /// The captured shared heap allocations in large space.
    allocations: Box<[SharedHeapLargeAllocationImage]>,
    /// The captured free shared heap large-allocation ids.
    free_large_allocation_ids: Box<[u64]>,
    /// The next shared heap large-allocation id to allocate.
    next_unused_large_allocation_id: u64,
    /// The number of allocated shared heap-space allocations.
    allocated_count: usize,
    /// The number of allocated shared heap-space bytes.
    allocated_bytes: u64,
    /// The captured shared heap collector state.
    gc_state: GcState,
}

impl SharedHeapSpaceImage {
    /// Create one frozen shared heap-space root.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SharedHeapSmallSpanImage]>,
        page_bytes: usize,
        allocations: Box<[SharedHeapLargeAllocationImage]>,
        free_large_allocation_ids: Box<[u64]>,
        next_unused_large_allocation_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
        gc_state: GcState,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            spans,
            page_bytes,
            allocations,
            free_large_allocation_ids,
            next_unused_large_allocation_id,
            allocated_count,
            allocated_bytes,
            gc_state,
        }
    }

    /// Return the configured size-class table.
    pub fn size_classes(&self) -> &SizeClassTable {
        &self.size_classes
    }

    /// Return the configured small-space span width.
    pub const fn small_bytes(&self) -> usize {
        self.small_bytes
    }

    /// Return the captured shared heap spans.
    pub fn spans(&self) -> &[SharedHeapSmallSpanImage] {
        &self.spans
    }

    /// Return the configured shared page width.
    pub const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the captured shared heap allocations in large space.
    pub fn allocations(&self) -> &[SharedHeapLargeAllocationImage] {
        &self.allocations
    }

    /// Return the next shared heap large-allocation id.
    pub const fn next_unused_large_allocation_id(&self) -> u64 {
        self.next_unused_large_allocation_id
    }

    /// Return the number of allocated shared heap-space allocations.
    pub const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated shared heap-space bytes.
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the captured shared heap collector state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Return every allocator page reachable from this shared heap-space image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // span pages
        for span in &*self.spans {
            pages.extend(span.pages.page_ids());
        }

        // large-allocation pages
        for allocation in &*self.allocations {
            pages.extend(allocation.pages.page_ids());
        }

        pages
    }
}

impl SharedHeapSpace {
    /// Fork one shared heap-space root over the same shared allocator.
    pub fn fork(&self) -> HeapResult<Self> {
        let store = self.state.read();
        let retained_page_views = self.allocator.retain_page_views(
            store
                .small
                .spans
                .iter()
                .map(|span| span.read().pages.clone())
                .chain(
                    store
                        .large
                        .allocations
                        .iter()
                        .map(|allocation| allocation.read().pages.clone()),
                ),
        )?;

        let mut cloned_store = SharedHeapState {
            page_run_cache: PageRunCache::new(self.allocator.pages_per_arena()),
            small: SharedSmallSpace {
                size_classes: store.small.size_classes.clone(),
                span_bytes: store.small.span_bytes,
                spans: store
                    .small
                    .spans
                    .iter()
                    .map(|span| Arc::new(RwLock::new(span.read().clone())))
                    .collect(),
                partial_spans: store.small.partial_spans.clone(),
            },
            large: SharedLargeSpace {
                page_bytes: store.large.page_bytes,
                allocations: store
                    .large
                    .allocations
                    .iter()
                    .map(|allocation| Arc::new(RwLock::new(allocation.read().clone())))
                    .collect(),
                free_large_allocation_ids: store.large.free_large_allocation_ids.clone(),
                next_unused_large_allocation_id: store.large.next_unused_large_allocation_id,
            },
            page_owners: Vec::new(),
            usage: store.usage,
            gc: store.gc.clone(),
        };

        if let Err(error) = Self::rebuild_page_owners(&mut cloned_store) {
            self.allocator.release_page_views(&retained_page_views)?;

            return Err(error);
        }

        Ok(Self {
            allocator: self.allocator.clone(),
            state: RwLock::new(cloned_store),
            gc: SharedGcState::default(),
        })
    }

    /// Create one shared heap-space root from one frozen image over one shared allocator.
    pub fn from_image_with_allocator(
        allocator: Arc<Allocator>,
        image: &SharedHeapSpaceImage,
    ) -> HeapResult<Self> {
        let retained_page_views = allocator.retain_page_views(
            image.spans().iter().map(|span| span.pages.clone()).chain(
                image
                    .allocations()
                    .iter()
                    .map(|allocation| allocation.pages.clone()),
            ),
        )?;

        let mut store =
            match Self::restore_state_with_retained_pages(image, allocator.pages_per_arena()) {
                Ok(store) => store,
                Err(error) => {
                    allocator.release_page_views(&retained_page_views)?;

                    return Err(error);
                }
            };

        if let Err(error) = Self::rebuild_page_owners(&mut store) {
            allocator.release_page_views(&retained_page_views)?;

            return Err(error);
        }

        let root = Self {
            allocator,
            state: RwLock::new(store),
            gc: SharedGcState::default(),
        };

        root.rebuild_small_availability()?;

        Ok(root)
    }

    /// Return one frozen shared heap-space root.
    pub fn image(&self) -> SharedHeapSpaceImage {
        let store = self.state.read();

        SharedHeapSpaceImage::new(
            store.small.size_classes.clone(),
            store.small.span_bytes,
            store
                .small
                .spans
                .iter()
                .map(|span| {
                    let span = span.read();

                    SharedHeapSmallSpanImage {
                        class: span.class.clone(),
                        slot_count: span.slot_count,
                        occupied: span.occupied.clone(),
                        local_reference_bits: span.local_reference_bits.clone(),
                        shared_reference_bits: span.shared_reference_bits.clone(),
                        pages: span.pages.clone(),
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            store.large.page_bytes,
            store
                .large
                .allocations
                .iter()
                .map(|allocation| {
                    let allocation = allocation.read();

                    SharedHeapLargeAllocationImage {
                        is_live: allocation.is_live,
                        len: allocation.len,
                        pages: allocation.pages.clone(),
                        reference_map: allocation.reference_map.clone(),
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            store
                .large
                .free_large_allocation_ids
                .clone()
                .into_boxed_slice(),
            store.large.next_unused_large_allocation_id,
            store.usage.allocation_count(),
            store.usage.allocated_bytes(),
            store.gc.clone(),
        )
    }

    /// Return every allocator page reachable from this live shared heap space.
    pub fn page_ids(&self) -> Vec<PageId> {
        let store = self.state.read();
        let mut pages = Vec::new();

        // span pages
        for span in &store.small.spans {
            pages.extend(span.read().pages.page_ids());
        }

        // large-allocation pages
        for allocation in &store.large.allocations {
            pages.extend(allocation.read().pages.page_ids());
        }

        pages
    }

    /// Restore one shared heap state after retaining every image page view.
    fn restore_state_with_retained_pages(
        image: &SharedHeapSpaceImage,
        pages_per_arena: usize,
    ) -> HeapResult<SharedHeapState> {
        Ok(SharedHeapState {
            page_run_cache: PageRunCache::new(pages_per_arena),
            small: SharedSmallSpace {
                size_classes: image.size_classes().clone(),
                span_bytes: image.small_bytes(),
                spans: image
                    .spans()
                    .iter()
                    .map(|span| -> HeapResult<Arc<RwLock<SharedSmallSpan>>> {
                        Ok(Arc::new(RwLock::new(SharedSmallSpan {
                            class: span.class.clone(),
                            slot_count: span.slot_count,
                            occupied_count: span.occupied.count_ones(),
                            free_cursor: 0,
                            occupied: span.occupied.clone(),
                            local_reference_bits: span.local_reference_bits.clone(),
                            shared_reference_bits: span.shared_reference_bits.clone(),
                            marked: crate::Bitmap::with_capacity(span.slot_count),
                            scanned: crate::Bitmap::with_capacity(span.slot_count),
                            is_queued_for_scan: false,
                            pages: span.pages.clone(),
                        })))
                    })
                    .collect::<HeapResult<Vec<_>>>()?,
                partial_spans: vec![
                    Vec::new();
                    crate::SmallSpanClass::bucket_count(image.size_classes())
                ],
            },
            large: SharedLargeSpace {
                page_bytes: image.page_bytes(),
                allocations: image
                    .allocations()
                    .iter()
                    .map(
                        |allocation| -> HeapResult<Arc<RwLock<SharedLargeAllocation>>> {
                            Ok(Arc::new(RwLock::new(SharedLargeAllocation {
                                is_live: allocation.is_live,
                                len: allocation.len,
                                pages: allocation.pages.clone(),
                                reference_map: allocation.reference_map.clone(),
                                is_marked: false,
                            })))
                        },
                    )
                    .collect::<HeapResult<Vec<_>>>()?,
                free_large_allocation_ids: image.free_large_allocation_ids.to_vec(),
                next_unused_large_allocation_id: image.next_unused_large_allocation_id(),
            },
            page_owners: Vec::new(),
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            gc: image.gc_state().clone(),
        })
    }

    /// Rebuild shared small-span availability after cloning or restore.
    fn rebuild_small_availability(&self) -> HeapResult<()> {
        let mut store = self.state.write();

        for span_index in 0..store.small.spans.len() {
            let occupied_count;
            let slot_count;
            let bucket_index;

            {
                let mut span = store.small.spans[span_index].write();
                span.free_cursor = find_free_cursor(&span.occupied, 0, span.slot_count);
                occupied_count = span.occupied_count;
                slot_count = span.slot_count;
                bucket_index = span.class.bucket_index(&store.small.size_classes)?;
            }

            if occupied_count == 0 || occupied_count >= slot_count {
                continue;
            }

            store.small.partial_spans[bucket_index].push(span_index);
        }

        drop(store);

        Ok(())
    }

    /// Rebuild the page-owner table from live shared allocations.
    fn rebuild_page_owners(store: &mut SharedHeapState) -> HeapResult<()> {
        store.page_owners.clear();

        for span_index in 0..store.small.spans.len() {
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(crate::HeapError::MissingSpan { span_index });
            };
            let span = span.read();

            for logical_page_index in 0..span.pages.len() {
                let Some(page_id) = span.pages.page(logical_page_index) else {
                    return Err(crate::HeapError::MissingLogicalPage {
                        page_index: logical_page_index,
                    });
                };
                let page_index = page_id.index();

                if store.page_owners.len() <= page_index {
                    store.page_owners.resize(page_index + 1, None);
                }

                store.page_owners[page_index] = Some(SharedHeapPageOwner::Small {
                    span_index,
                    logical_page_index,
                });
            }
        }

        for allocation_index in 0..store.large.allocations.len() {
            let allocation_id = SharedLargeAllocationId::new(allocation_index as u64 + 1);
            let Some(allocation) = store.large.allocations.get(allocation_index).cloned() else {
                continue;
            };
            let allocation = allocation.read();
            if !allocation.is_live {
                continue;
            }

            for logical_page_index in 0..allocation.pages.len() {
                let Some(page_id) = allocation.pages.page(logical_page_index) else {
                    return Err(crate::HeapError::MissingLogicalPage {
                        page_index: logical_page_index,
                    });
                };
                let page_index = page_id.index();

                if store.page_owners.len() <= page_index {
                    store.page_owners.resize(page_index + 1, None);
                }

                store.page_owners[page_index] = Some(SharedHeapPageOwner::Large {
                    allocation_id,
                    logical_page_index,
                });
            }
        }

        Ok(())
    }
}
