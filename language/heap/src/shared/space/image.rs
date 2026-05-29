use std::collections::BTreeMap;
use std::sync::Arc;

use destack_memory::AddressSpace;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{
    SharedHeapLargeBlockImage, SharedHeapPageMapEntry, SharedHeapSmallSpanImage, SharedHeapSpace,
    SharedLargeBlock, SharedLargeBlockId, SharedSmallSpan, SpanList,
};
use crate::allocator::PageSpanCache;
use crate::shared::gc::SharedGcState;
use crate::shared::space::{
    SharedHeapAccounting, SharedHeapState, SharedLargeSpace, SharedSmallSpace,
};
use crate::{Allocator, GcState, HeapResult, PageSpan, SizeClassTable};

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
    page_size_bytes: usize,
    /// The reserved virtual byte capacity for shared heap space.
    space_size_bytes: usize,
    /// The captured shared heap blocks in large space.
    blocks: Box<[SharedHeapLargeBlockImage]>,
    /// The captured free shared heap large-block ids.
    free_large_block_ids: Box<[u64]>,
    /// The next shared heap large-block id to allocate.
    next_unused_large_block_id: u64,
    /// The next unused byte offset in shared heap space.
    next_offset: usize,
    /// The number of allocated shared heap-space blocks.
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
        page_size_bytes: usize,
        space_size_bytes: usize,
        blocks: Box<[SharedHeapLargeBlockImage]>,
        free_large_block_ids: Box<[u64]>,
        next_unused_large_block_id: u64,
        next_offset: usize,
        allocated_count: usize,
        allocated_bytes: u64,
        gc_state: GcState,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            spans,
            page_size_bytes,
            space_size_bytes,
            blocks,
            free_large_block_ids,
            next_unused_large_block_id,
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
    pub const fn page_size_bytes(&self) -> usize {
        self.page_size_bytes
    }

    /// Return the reserved virtual byte capacity for shared heap space.
    pub const fn space_size_bytes(&self) -> usize {
        self.space_size_bytes
    }

    /// Return the captured shared heap blocks in large space.
    pub fn blocks(&self) -> &[SharedHeapLargeBlockImage] {
        &self.blocks
    }

    /// Return the next shared heap large-block id.
    pub const fn next_unused_large_block_id(&self) -> u64 {
        self.next_unused_large_block_id
    }

    /// Return the next unused byte offset in shared heap space.
    pub const fn next_offset(&self) -> usize {
        self.next_offset
    }

    /// Return the number of allocated shared heap-space blocks.
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

    /// Return the retained frozen page count.
    pub fn page_count(&self) -> usize {
        let mut page_count = 0;

        // count retained small-span bytes
        for span in self.spans() {
            page_count += span.bytes.len().div_ceil(self.page_size_bytes());
        }

        // count retained large-block bytes
        for block in self.blocks() {
            page_count += block.bytes.len().div_ceil(self.page_size_bytes());
        }

        page_count
    }
}

impl SharedHeapSpace {
    /// Fork one shared heap space over the same shared allocator.
    ///
    /// Call this only from a safepoint where shared heap mutators are stopped.
    pub fn fork(&self) -> HeapResult<Self> {
        let store = self.state.read();
        let mapping = self.mapping.fork_lazy()?;

        // share small span page metadata with the fork
        let spans = store
            .small
            .spans
            .iter()
            .map(|span| -> HeapResult<Arc<SharedSmallSpan>> {
                let pages = self.allocator.share_page_span(span.pages())?;
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

        // share large block page metadata with the fork
        let blocks = store
            .large
            .blocks
            .iter()
            .map(|block| -> HeapResult<Arc<RwLock<SharedLargeBlock>>> {
                let block = block.read();
                let pages = if block.is_live {
                    self.allocator.share_page_span(block.pages)?
                } else {
                    PageSpan::empty()
                };
                let mut block = block.clone();
                block.pages = pages;

                Ok(Arc::new(RwLock::new(block)))
            })
            .collect::<HeapResult<Vec<_>>>()?;

        let mut cloned_store = SharedHeapState {
            page_span_cache: PageSpanCache::new(self.allocator.pages_per_chunk()),
            small: SharedSmallSpace {
                size_classes: store.small.size_classes.clone(),
                span_size_bytes: store.small.span_size_bytes,
                spans,
                partial_spans: BTreeMap::new(),
            },
            large: SharedLargeSpace {
                blocks,
                free_large_block_ids: store.large.free_large_block_ids.clone(),
                next_unused_large_block_id: store.large.next_unused_large_block_id,
            },
            page_map: Vec::new(),
            next_offset: store.next_offset,
            gc: store.gc.clone(),
        };

        Self::rebuild_page_map(&mut cloned_store, self.allocator.page_size_bytes());

        let space = Self {
            allocator: self.allocator.clone(),
            mapping,
            accounting: SharedHeapAccounting::from_state(
                &cloned_store,
                self.allocator.page_size_bytes(),
            ),
            state: RwLock::new(cloned_store),
            gc: SharedGcState::default(),
        };

        space.rebuild_small_availability()?;

        Ok(space)
    }

    /// Create one shared heap space from one frozen image over one shared allocator.
    pub fn from_image_with_allocator(
        allocator: Arc<Allocator>,
        image: &SharedHeapSpaceImage,
    ) -> HeapResult<Self> {
        let mut mapping = AddressSpace::reserve(image.space_size_bytes(), image.page_size_bytes())?;
        restore_shared_mapping(image, &mut mapping)?;
        let mut store = Self::restore_state(image, allocator.as_ref())?;

        Self::rebuild_page_map(&mut store, allocator.page_size_bytes());

        let space = Self {
            accounting: SharedHeapAccounting::from_state(&store, allocator.page_size_bytes()),
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
        let mut blocks = Vec::with_capacity(store.large.blocks.len());

        // capture small spans from the live mapping
        for span in &store.small.spans {
            let byte_len = span.page_count() * self.allocator.page_size_bytes();
            let bytes = self.mapping.read_bytes(span.first_offset, byte_len)?;

            spans.push(SharedHeapSmallSpanImage {
                first_offset: span.first_offset,
                class: span.class,
                slot_count: span.slot_count,
                occupied: span.occupied_snapshot(),
                local_reference_bits: span.local_reference_snapshot(),
                shared_reference_bits: span.shared_reference_snapshot(),
                bytes: bytes.into_boxed_slice(),
            });
        }

        // capture large blocks from the live mapping
        for block in &store.large.blocks {
            let block = block.read();
            let bytes = if block.is_live {
                self.mapping
                    .read_bytes(block.first_offset, block.byte_len)?
                    .into_boxed_slice()
            } else {
                Box::new([])
            };

            blocks.push(SharedHeapLargeBlockImage {
                is_live: block.is_live,
                first_offset: block.first_offset,
                byte_len: block.byte_len,
                bytes,
                trace_map: block.trace_map.clone(),
            });
        }

        let allocation_count = self.accounting.allocation_count();
        let allocated_bytes = self.accounting.allocated_bytes();

        Ok(SharedHeapSpaceImage::new(
            store.small.size_classes.clone(),
            store.small.span_size_bytes,
            spans.into_boxed_slice(),
            self.allocator.page_size_bytes(),
            self.mapping.byte_len(),
            blocks.into_boxed_slice(),
            store.large.free_large_block_ids.clone().into_boxed_slice(),
            store.large.next_unused_large_block_id,
            store.next_offset,
            allocation_count,
            allocated_bytes,
            store.gc.clone(),
        ))
    }

    /// Restore one shared heap state into fresh page spans.
    fn restore_state(
        image: &SharedHeapSpaceImage,
        allocator: &Allocator,
    ) -> HeapResult<SharedHeapState> {
        Ok(SharedHeapState {
            page_span_cache: PageSpanCache::new(allocator.pages_per_chunk()),
            small: SharedSmallSpace {
                size_classes: image.size_classes().clone(),
                span_size_bytes: image.small_bytes(),
                spans: image
                    .spans()
                    .iter()
                    .map(|span| -> HeapResult<Arc<SharedSmallSpan>> {
                        let pages = allocator.allocate_pages(span.bytes.len())?;

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
                partial_spans: BTreeMap::new(),
            },
            large: SharedLargeSpace {
                blocks: image
                    .blocks()
                    .iter()
                    .map(|block| -> HeapResult<Arc<RwLock<SharedLargeBlock>>> {
                        let pages = if block.is_live {
                            allocator.allocate_pages(block.bytes.len())?
                        } else {
                            PageSpan::empty()
                        };

                        Ok(Arc::new(RwLock::new(SharedLargeBlock {
                            is_live: block.is_live,
                            first_offset: block.first_offset,
                            byte_len: block.byte_len,
                            pages,
                            trace_map: block.trace_map.clone(),
                            mark_epoch: 0,
                        })))
                    })
                    .collect::<HeapResult<Vec<_>>>()?,
                free_large_block_ids: image.free_large_block_ids.to_vec(),
                next_unused_large_block_id: image.next_unused_large_block_id(),
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
            let class;

            {
                let span = store.small.spans[span_index].clone();
                span.reset_free_cursor();
                occupied_count = span.occupied_count();
                slot_count = span.slot_count;
                class = span.class;
                class.validate(
                    &store.small.size_classes,
                    self.allocator.page_size_bytes(),
                    store.small.span_size_bytes,
                )?;

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

            store
                .small
                .partial_spans
                .entry(class)
                .or_default()
                .push(span_index);
        }

        drop(store);

        Ok(())
    }

    /// Rebuild the page-map table from live shared blocks.
    fn rebuild_page_map(store: &mut SharedHeapState, page_size_bytes: usize) {
        store.page_map.clear();

        for span_index in 0..store.small.spans.len() {
            let span = store.small.spans[span_index].clone();

            for logical_page_index in 0..span.page_count() {
                let page_index = span.first_offset / page_size_bytes + logical_page_index;

                if store.page_map.len() <= page_index {
                    store.page_map.resize(page_index + 1, None);
                }

                store.page_map[page_index] = Some(SharedHeapPageMapEntry::SmallSpan {
                    span_index,
                    logical_page_index,
                });
            }
        }

        for block_index in 0..store.large.blocks.len() {
            let block_id = SharedLargeBlockId::new(block_index as u64 + 1);
            let block = store.large.blocks[block_index].read();
            if !block.is_live {
                continue;
            }

            for logical_page_index in 0..block.pages.len() {
                let page_index = block.first_offset / page_size_bytes + logical_page_index;

                if store.page_map.len() <= page_index {
                    store.page_map.resize(page_index + 1, None);
                }

                store.page_map[page_index] = Some(SharedHeapPageMapEntry::LargeBlock {
                    block_id,
                    logical_page_index,
                });
            }
        }
    }
}

/// Restore one shared heap mapping from one image.
fn restore_shared_mapping(
    image: &SharedHeapSpaceImage,
    mapping: &mut AddressSpace,
) -> HeapResult<()> {
    // restore each captured small span range
    for span in image.spans() {
        if span.bytes.is_empty() {
            continue;
        }

        mapping.write_bytes(span.first_offset, &span.bytes)?;
    }

    // restore each captured large block range
    for block in image.blocks() {
        if !block.is_live || block.byte_len == 0 {
            continue;
        }

        mapping.write_bytes(block.first_offset, &block.bytes)?;
    }

    Ok(())
}
