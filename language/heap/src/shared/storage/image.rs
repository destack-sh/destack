use std::collections::BTreeMap;
use std::sync::Arc;

use destack_memory::{MemoryMap, MemoryRange};
use destack_serde::Reflect;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{
    HeapStorage, LargeBlock, LargeBlockId, LargeBlockImage, PageOwner, SmallSpan, SmallSpanImage,
    SpanList,
};
use crate::shared::gc::CollectorState;
use crate::shared::storage::{HeapAccounting, HeapState, LargeStorage, SmallStorage};
use crate::{GcState, HeapResult, PageTable, SizeClassTable};

/// One frozen shared heap storage metadata image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct HeapStorageImage {
    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured shared heap spans.
    spans: Box<[SmallSpanImage]>,
    /// The configured shared page width.
    page_size_bytes: usize,
    /// The captured shared heap blocks in large space.
    blocks: Box<[Option<LargeBlockImage>]>,
    /// The captured free shared heap large-block ids.
    free_large_block_ids: Box<[u64]>,
    /// The next shared heap large-block id to allocate.
    next_unused_large_block_id: u64,
    /// The number of allocated shared heap storage blocks.
    allocated_count: usize,
    /// The number of allocated shared heap storage bytes.
    allocated_bytes: u64,
    /// The captured shared heap collector state.
    gc_state: GcState,
}

impl HeapStorageImage {
    /// Create one frozen shared heap storage image.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SmallSpanImage]>,
        page_size_bytes: usize,
        blocks: Box<[Option<LargeBlockImage>]>,
        free_large_block_ids: Box<[u64]>,
        next_unused_large_block_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
        gc_state: GcState,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            spans,
            page_size_bytes,
            blocks,
            free_large_block_ids,
            next_unused_large_block_id,
            allocated_count,
            allocated_bytes,
            gc_state,
        }
    }

    /// Return the configured size-class table.
    pub(crate) fn size_classes(&self) -> &SizeClassTable {
        &self.size_classes
    }

    /// Return the configured small-space span width.
    pub(crate) const fn small_bytes(&self) -> usize {
        self.small_bytes
    }

    /// Return the captured shared heap spans.
    pub(crate) fn spans(&self) -> &[SmallSpanImage] {
        &self.spans
    }

    /// Return the configured shared page width.
    pub(crate) const fn page_size_bytes(&self) -> usize {
        self.page_size_bytes
    }

    /// Return the captured shared heap blocks in large space.
    pub(crate) fn blocks(&self) -> &[Option<LargeBlockImage>] {
        &self.blocks
    }

    /// Return the next shared heap large-block id.
    pub(crate) const fn next_unused_large_block_id(&self) -> u64 {
        self.next_unused_large_block_id
    }

    /// Return the allocated shared heap bytes.
    pub(crate) const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the captured shared heap collector state.
    pub(crate) fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Return the retained shared heap page count represented by this image.
    pub(crate) fn page_count(&self) -> usize {
        let mut page_count = 0;

        // count retained small-span bytes
        for span in self.spans() {
            page_count += span
                .class
                .span_size_bytes()
                .div_ceil(self.page_size_bytes());
        }

        // count retained large-block bytes
        for block in self.blocks().iter().flatten() {
            page_count += block.byte_len.div_ceil(self.page_size_bytes());
        }

        page_count
    }
}

impl HeapStorage {
    /// Fork one shared heap storage over the same shared memory.
    ///
    /// Call this only from a safepoint where shared heap mutators are stopped.
    pub(crate) fn fork(&self, memory: Arc<MemoryMap>) -> HeapResult<Self> {
        let store = self.state.read();

        // clone small span metadata for independent branch mutation
        let spans = store
            .small
            .spans
            .iter()
            .map(|span| {
                let pages = span.pages();
                let occupied = span.occupied_snapshot();
                let local_reference_bits = span.local_reference_snapshot();
                let shared_reference_bits = span.shared_reference_snapshot();
                let retained = span.retained_snapshot();
                let empty = span.empty_snapshot();
                let span = SmallSpan::from_image(
                    span.first_offset,
                    span.class,
                    span.slot_count,
                    &occupied,
                    &local_reference_bits,
                    &shared_reference_bits,
                    &retained,
                    &empty,
                    pages,
                    span.list.load(),
                );

                Arc::new(span)
            })
            .collect();

        // clone large block metadata for independent branch mutation
        let blocks = store
            .large
            .blocks
            .iter()
            .map(|block| {
                let Some(block) = block else {
                    return None;
                };
                let block = block.read().clone();

                Some(Arc::new(RwLock::new(block)))
            })
            .collect();

        let cloned_store = HeapState {
            small: SmallStorage {
                size_classes: store.small.size_classes.clone(),
                span_size_bytes: store.small.span_size_bytes,
                spans,
                partial_spans: BTreeMap::new(),
            },
            large: LargeStorage {
                blocks,
                free_large_block_ids: store.large.free_large_block_ids.clone(),
                next_unused_large_block_id: store.large.next_unused_large_block_id,
            },
            page_table: store.page_table.clone(),
            gc: store.gc.clone(),
        };

        let space = Self {
            memory,
            page_size_bytes: self.page_size_bytes,
            accounting: HeapAccounting::from_state(&cloned_store),
            state: RwLock::new(cloned_store),
            constant: self.constant,
            gc: CollectorState::default(),
        };

        space.rebuild_small_availability()?;

        Ok(space)
    }

    /// Restore shared heap storage metadata over captured world memory.
    pub(crate) fn from_image(memory: Arc<MemoryMap>, image: &HeapStorageImage) -> HeapResult<Self> {
        let mut store = Self::restore_state(image);

        Self::rebuild_page_table(&mut store, image.page_size_bytes());

        let space = Self {
            accounting: HeapAccounting::from_state(&store),
            memory,
            page_size_bytes: image.page_size_bytes(),
            state: RwLock::new(store),
            constant: MemoryRange::default(),
            gc: CollectorState::default(),
        };

        space.rebuild_small_availability()?;

        Ok(space)
    }

    /// Capture one frozen shared heap storage metadata image.
    pub(crate) fn image(&self) -> HeapStorageImage {
        let store = self.state.read();
        let mut spans = Vec::with_capacity(store.small.spans.len());
        let mut blocks = Vec::with_capacity(store.large.blocks.len());

        // capture small span metadata
        for span in &store.small.spans {
            spans.push(SmallSpanImage {
                first_offset: span.first_offset,
                class: span.class,
                slot_count: span.slot_count,
                occupied: span.occupied_snapshot(),
                local_reference_bits: span.local_reference_snapshot(),
                shared_reference_bits: span.shared_reference_snapshot(),
                retained: span.retained_snapshot(),
                empty: span.empty_snapshot(),
            });
        }

        // capture large block metadata
        for block in &store.large.blocks {
            let Some(block) = block else {
                blocks.push(None);

                continue;
            };
            let block = block.read();
            blocks.push(Some(LargeBlockImage {
                first_offset: block.first_offset,
                byte_len: block.byte_len,
                trace_map: (*block.trace_map).clone(),
                drop: block.drop,
                retained: block.retained,
                empty: block.empty,
            }));
        }

        let allocation_count = self.accounting.allocation_count();
        let allocated_bytes = self.accounting.allocated_bytes();

        HeapStorageImage::new(
            store.small.size_classes.clone(),
            store.small.span_size_bytes,
            spans.into_boxed_slice(),
            self.page_size_bytes,
            blocks.into_boxed_slice(),
            store.large.free_large_block_ids.clone().into_boxed_slice(),
            store.large.next_unused_large_block_id,
            allocation_count,
            allocated_bytes,
            store.gc.clone(),
        )
    }

    /// Restore shared heap metadata over captured world memory.
    fn restore_state(image: &HeapStorageImage) -> HeapState {
        HeapState {
            small: SmallStorage {
                size_classes: image.size_classes().clone(),
                span_size_bytes: image.small_bytes(),
                spans: image
                    .spans()
                    .iter()
                    .map(|span| {
                        let pages = MemoryRange {
                            offset: span.first_offset,
                            byte_len: span.class.span_size_bytes(),
                        };
                        Arc::new(SmallSpan::from_image(
                            span.first_offset,
                            span.class,
                            span.slot_count,
                            &span.occupied,
                            &span.local_reference_bits,
                            &span.shared_reference_bits,
                            &span.retained,
                            &span.empty,
                            pages,
                            SpanList::Central,
                        ))
                    })
                    .collect(),
                partial_spans: BTreeMap::new(),
            },
            large: LargeStorage {
                blocks: image
                    .blocks()
                    .iter()
                    .map(|block| {
                        let Some(block) = block else {
                            return None;
                        };
                        let pages = MemoryRange {
                            offset: block.first_offset,
                            byte_len: block.byte_len.next_multiple_of(image.page_size_bytes()),
                        };
                        Some(Arc::new(RwLock::new(LargeBlock {
                            first_offset: block.first_offset,
                            byte_len: block.byte_len,
                            pages,
                            trace_map: Arc::new(block.trace_map.clone()),
                            drop: block.drop,
                            retained: block.retained,
                            empty: block.empty,
                            mark_epoch: 0,
                        })))
                    })
                    .collect(),
                free_large_block_ids: image.free_large_block_ids.to_vec(),
                next_unused_large_block_id: image.next_unused_large_block_id(),
            },
            page_table: PageTable::new(),
            gc: image.gc_state().clone(),
        }
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
                    self.page_size_bytes,
                    store.small.span_size_bytes,
                )?;

                if occupied_count >= slot_count {
                    span.list.store(SpanList::Full);
                } else {
                    span.list.store(SpanList::Central);
                }
            }

            if occupied_count >= slot_count {
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

    /// Rebuild the page table from live shared blocks.
    fn rebuild_page_table(store: &mut HeapState, page_size_bytes: usize) {
        store.page_table.clear_all();

        for span_index in 0..store.small.spans.len() {
            let span = store.small.spans[span_index].clone();

            let page_count = span.mapped_byte_len() / page_size_bytes;
            for logical_page_index in 0..page_count {
                let page_index = span.first_offset / page_size_bytes + logical_page_index;

                store.page_table.set(
                    page_index,
                    PageOwner::SmallSpan {
                        span_index,
                        logical_page_index,
                    },
                );
            }
        }

        for block_index in 0..store.large.blocks.len() {
            let block_id = LargeBlockId::new(block_index as u64 + 1);
            let Some(block) = &store.large.blocks[block_index] else {
                continue;
            };
            let block = block.read();

            for logical_page_index in 0..block.pages.byte_len / page_size_bytes {
                let page_index = block.first_offset / page_size_bytes + logical_page_index;

                store.page_table.set(
                    page_index,
                    PageOwner::LargeBlock {
                        block_id,
                        logical_page_index,
                    },
                );
            }
        }
    }
}
