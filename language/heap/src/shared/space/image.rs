use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{
    SharedHeapLargeAllocationImage, SharedHeapPageMapEntry, SharedHeapSmallSpanImage,
    SharedHeapSpace, SharedLargeAllocation, SharedLargeAllocationId, SharedSmallSpan, SpanList,
};
use crate::allocator::{AddressSpace, PageRunCache};
use crate::shared::gc::SharedGcState;
use crate::shared::space::{SharedHeapState, SharedLargeSpace, SharedSmallSpace};
use crate::{Allocator, GcState, HeapResult, PageId, PageRun, SizeClassTable};

/// One frozen shared heap-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapSpaceImage {
    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured shared heap spans.
    spans: Box<[SharedHeapSmallSpanImage]>,
    /// The configured shared page width.
    page_bytes: usize,
    /// The reserved virtual byte capacity for shared heap space.
    space_bytes: usize,
    /// The captured shared heap allocations in large space.
    allocations: Box<[SharedHeapLargeAllocationImage]>,
    /// The captured free shared heap large-allocation ids.
    free_large_allocation_ids: Box<[u64]>,
    /// The next shared heap large-allocation id to allocate.
    next_unused_large_allocation_id: u64,
    /// The next unused byte offset in shared heap space.
    next_offset: usize,
    /// The number of allocated shared heap-space allocations.
    allocated_count: usize,
    /// The number of allocated shared heap-space bytes.
    allocated_bytes: u64,
    /// The captured shared heap collector state.
    gc_state: GcState,
}

impl SharedHeapSpaceImage {
    /// Create one frozen shared heap-space image.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SharedHeapSmallSpanImage]>,
        page_bytes: usize,
        space_bytes: usize,
        allocations: Box<[SharedHeapLargeAllocationImage]>,
        free_large_allocation_ids: Box<[u64]>,
        next_unused_large_allocation_id: u64,
        next_offset: usize,
        allocated_count: usize,
        allocated_bytes: u64,
        gc_state: GcState,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            spans,
            page_bytes,
            space_bytes,
            allocations,
            free_large_allocation_ids,
            next_unused_large_allocation_id,
            next_offset,
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

    /// Return the reserved virtual byte capacity for shared heap space.
    pub const fn space_bytes(&self) -> usize {
        self.space_bytes
    }

    /// Return the captured shared heap allocations in large space.
    pub fn allocations(&self) -> &[SharedHeapLargeAllocationImage] {
        &self.allocations
    }

    /// Return the next shared heap large-allocation id.
    pub const fn next_unused_large_allocation_id(&self) -> u64 {
        self.next_unused_large_allocation_id
    }

    /// Return the next unused byte offset in shared heap space.
    pub const fn next_offset(&self) -> usize {
        self.next_offset
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

    /// Return every live allocator page run captured by this image.
    pub fn page_runs(&self) -> Vec<PageRun> {
        let mut pages = Vec::new();

        // span pages
        for span in &*self.spans {
            pages.push(span.pages);
        }

        // large-allocation pages
        for allocation in &*self.allocations {
            if allocation.is_live {
                pages.push(allocation.pages);
            }
        }

        pages
    }
}

/// Restore one shared heap mapping from one image.
fn restore_shared_mapping(
    allocator: &Allocator,
    image: &SharedHeapSpaceImage,
    mapping: &mut AddressSpace,
) -> HeapResult<()> {
    // restore each captured small span range
    for span in image.spans() {
        let byte_len = span.pages.len() * allocator.page_bytes();
        if byte_len == 0 {
            continue;
        }

        let bytes = allocator.bytes_to_vec_from(&span.pages, 0, byte_len)?;

        mapping.write(span.first_offset, &bytes)?;
    }

    // restore each captured large allocation range
    for allocation in image.allocations() {
        if !allocation.is_live || allocation.len == 0 {
            continue;
        }

        let bytes = allocator.bytes_to_vec_from(&allocation.pages, 0, allocation.len)?;

        mapping.write(allocation.first_offset, &bytes)?;
    }

    Ok(())
}

impl SharedHeapSpace {
    /// Fork one shared heap space over the same shared allocator.
    ///
    /// Call this only from a safepoint where shared heap mutators are stopped.
    pub fn fork(&self) -> HeapResult<Self> {
        let store = self.state.read();
        let mapping = self.mapping.fork()?;

        // share small span page metadata with the fork
        let spans = store
            .small
            .spans
            .iter()
            .map(|span| -> HeapResult<Arc<SharedSmallSpan>> {
                let pages = self.allocator.share_page_run(span.pages())?;
                let occupied = span.occupied_snapshot();
                let local_reference_bits = span.local_reference_snapshot();
                let shared_reference_bits = span.shared_reference_snapshot();
                let span = SharedSmallSpan::from_image(
                    span.first_offset,
                    span.class,
                    span.slot_count,
                    &occupied,
                    &local_reference_bits,
                    &shared_reference_bits,
                    pages,
                    span.list.load(),
                );

                Ok(Arc::new(span))
            })
            .collect::<HeapResult<Vec<_>>>()?;

        // share large allocation page metadata with the fork
        let allocations = store
            .large
            .allocations
            .iter()
            .map(
                |allocation| -> HeapResult<Arc<RwLock<SharedLargeAllocation>>> {
                    let allocation = allocation.read();
                    let pages = if allocation.is_live {
                        self.allocator.share_page_run(allocation.pages)?
                    } else {
                        PageRun::empty()
                    };
                    let mut allocation = allocation.clone();
                    allocation.pages = pages;

                    Ok(Arc::new(RwLock::new(allocation)))
                },
            )
            .collect::<HeapResult<Vec<_>>>()?;

        let mut cloned_store = SharedHeapState {
            page_run_cache: PageRunCache::new(self.allocator.pages_per_chunk()),
            small: SharedSmallSpace {
                size_classes: store.small.size_classes.clone(),
                span_bytes: store.small.span_bytes,
                spans,
                partial_spans: store.small.partial_spans.clone(),
            },
            large: SharedLargeSpace {
                page_bytes: store.large.page_bytes,
                allocations,
                free_large_allocation_ids: store.large.free_large_allocation_ids.clone(),
                next_unused_large_allocation_id: store.large.next_unused_large_allocation_id,
            },
            page_map: Vec::new(),
            next_offset: store.next_offset,
            gc: store.gc.clone(),
        };

        Self::rebuild_page_map(&mut cloned_store);

        Ok(Self {
            allocator: self.allocator.clone(),
            mapping,
            state: RwLock::new(cloned_store),
            gc: SharedGcState::default(),
        })
    }

    /// Create one shared heap space from one frozen image over one shared allocator.
    pub fn from_image_with_allocator(
        allocator: Arc<Allocator>,
        image: &SharedHeapSpaceImage,
    ) -> HeapResult<Self> {
        let mut mapping = AddressSpace::reserve(image.space_bytes(), image.page_bytes())?;
        restore_shared_mapping(allocator.as_ref(), image, &mut mapping)?;
        let mut store = Self::restore_state(image, allocator.as_ref())?;

        Self::rebuild_page_map(&mut store);

        let space = Self {
            allocator,
            mapping,
            state: RwLock::new(store),
            gc: SharedGcState::default(),
        };

        space.rebuild_small_availability()?;

        Ok(space)
    }

    /// Return one frozen shared heap-space image.
    pub fn image(&self) -> HeapResult<SharedHeapSpaceImage> {
        let store = self.state.read();
        let mut spans = Vec::with_capacity(store.small.spans.len());
        let mut allocations = Vec::with_capacity(store.large.allocations.len());

        // capture small spans from the live mapping
        for span in &store.small.spans {
            let byte_len = span.page_count() * self.allocator.page_bytes();
            let bytes = self.mapping.bytes(span.first_offset, byte_len)?;
            let pages = self.allocator.allocate_bytes(&bytes)?;

            spans.push(SharedHeapSmallSpanImage {
                first_offset: span.first_offset,
                class: span.class,
                slot_count: span.slot_count,
                occupied: span.occupied_snapshot(),
                local_reference_bits: span.local_reference_snapshot(),
                shared_reference_bits: span.shared_reference_snapshot(),
                pages,
            });
        }

        // capture large allocations from the live mapping
        for allocation in &store.large.allocations {
            let allocation = allocation.read();
            let pages = if allocation.is_live {
                let bytes = self
                    .mapping
                    .bytes(allocation.first_offset, allocation.len)?;

                self.allocator.allocate_bytes(&bytes)?
            } else {
                PageRun::empty()
            };

            allocations.push(SharedHeapLargeAllocationImage {
                is_live: allocation.is_live,
                first_offset: allocation.first_offset,
                len: allocation.len,
                pages,
                reference_map: allocation.reference_map.clone(),
            });
        }

        let (allocation_count, allocated_bytes) = self.live_allocated_usage(&store);

        Ok(SharedHeapSpaceImage::new(
            store.small.size_classes.clone(),
            store.small.span_bytes,
            spans.into_boxed_slice(),
            store.large.page_bytes,
            self.mapping.byte_len(),
            allocations.into_boxed_slice(),
            store
                .large
                .free_large_allocation_ids
                .clone()
                .into_boxed_slice(),
            store.large.next_unused_large_allocation_id,
            store.next_offset,
            allocation_count,
            allocated_bytes,
            store.gc.clone(),
        ))
    }

    /// Return every allocator page reachable from this live shared heap space.
    pub fn page_ids(&self) -> Vec<PageId> {
        let store = self.state.read();
        let mut pages = Vec::new();

        // span pages
        for span in &store.small.spans {
            pages.extend(span.pages().page_ids());
        }

        // large-allocation pages
        for allocation in &store.large.allocations {
            pages.extend(allocation.read().pages.page_ids());
        }

        pages
    }

    /// Restore one shared heap state into fresh page runs.
    fn restore_state(
        image: &SharedHeapSpaceImage,
        allocator: &Allocator,
    ) -> HeapResult<SharedHeapState> {
        Ok(SharedHeapState {
            page_run_cache: PageRunCache::new(allocator.pages_per_chunk()),
            small: SharedSmallSpace {
                size_classes: image.size_classes().clone(),
                span_bytes: image.small_bytes(),
                spans: image
                    .spans()
                    .iter()
                    .map(|span| -> HeapResult<Arc<SharedSmallSpan>> {
                        let byte_len = span.pages.len() * allocator.page_bytes();
                        let pages = allocator.allocate_pages(byte_len)?;

                        Ok(Arc::new(SharedSmallSpan::from_image(
                            span.first_offset,
                            span.class,
                            span.slot_count,
                            &span.occupied,
                            &span.local_reference_bits,
                            &span.shared_reference_bits,
                            pages,
                            SpanList::Released,
                        )))
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
                            let pages = if allocation.is_live {
                                allocator.allocate_pages(allocation.len)?
                            } else {
                                PageRun::empty()
                            };

                            Ok(Arc::new(RwLock::new(SharedLargeAllocation {
                                is_live: allocation.is_live,
                                first_offset: allocation.first_offset,
                                len: allocation.len,
                                pages,
                                reference_map: allocation.reference_map.clone(),
                                is_marked: false,
                            })))
                        },
                    )
                    .collect::<HeapResult<Vec<_>>>()?,
                free_large_allocation_ids: image.free_large_allocation_ids.to_vec(),
                next_unused_large_allocation_id: image.next_unused_large_allocation_id(),
            },
            page_map: Vec::new(),
            next_offset: image.next_offset(),
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
                let span = store.small.spans[span_index].clone();
                span.reset_free_cursor();
                occupied_count = span.occupied_count();
                slot_count = span.slot_count;
                bucket_index = span.class.bucket_index(&store.small.size_classes)?;

                if occupied_count == 0 {
                    span.list.store(SpanList::Released);
                } else if occupied_count >= slot_count {
                    span.list.store(SpanList::Full);
                } else {
                    span.list.store(SpanList::Central);
                }
            }

            if occupied_count == 0 || occupied_count >= slot_count {
                continue;
            }

            store.small.partial_spans[bucket_index].push(span_index);
        }

        drop(store);

        Ok(())
    }

    /// Rebuild the page-map table from live shared allocations.
    fn rebuild_page_map(store: &mut SharedHeapState) {
        store.page_map.clear();

        for span_index in 0..store.small.spans.len() {
            let span = store.small.spans[span_index].clone();

            for logical_page_index in 0..span.page_count() {
                let page_index = span.first_offset / store.large.page_bytes + logical_page_index;

                if store.page_map.len() <= page_index {
                    store.page_map.resize(page_index + 1, None);
                }

                store.page_map[page_index] = Some(SharedHeapPageMapEntry::Small {
                    span_index,
                    logical_page_index,
                });
            }
        }

        for allocation_index in 0..store.large.allocations.len() {
            let allocation_id = SharedLargeAllocationId::new(allocation_index as u64 + 1);
            let allocation = store.large.allocations[allocation_index].read();
            if !allocation.is_live {
                continue;
            }

            for logical_page_index in 0..allocation.pages.len() {
                let page_index =
                    allocation.first_offset / store.large.page_bytes + logical_page_index;

                if store.page_map.len() <= page_index {
                    store.page_map.resize(page_index + 1, None);
                }

                store.page_map[page_index] = Some(SharedHeapPageMapEntry::Large {
                    allocation_id,
                    logical_page_index,
                });
            }
        }
    }
}
