use serde::{Deserialize, Serialize};

use std::sync::Arc;

use destack_memory::AddressSpace;

use super::{
    LargeBlock, LargeBlockId, LargeBlockImage, RawPageMapEntry, RawSpace, SmallSpan, SmallSpanImage,
};
use crate::allocator::{Allocator, PageSpan, PageSpanCache, SizeClassTable};
use crate::{AllocationUsage, CowTable, HeapConfigurationError, HeapError, HeapResult};

/// One frozen raw-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawSpaceImage {
    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured raw spans.
    spans: Box<[SmallSpanImage]>,
    /// The configured local page width.
    page_size_bytes: usize,
    /// The reserved virtual byte capacity for raw space.
    space_size_bytes: usize,
    /// The captured raw blocks in large space.
    blocks: Box<[LargeBlockImage]>,

    /// The next raw block id to allocate in large space.
    next_unused_large_block_id: u64,
    /// The next unused byte offset in raw space.
    next_offset: usize,

    /// The number of live raw blocks.
    allocated_count: usize,
    /// The number of live raw bytes.
    allocated_bytes: u64,
}

impl RawSpaceImage {
    /// Create one frozen raw-space image.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SmallSpanImage]>,
        page_size_bytes: usize,
        space_size_bytes: usize,
        blocks: Box<[LargeBlockImage]>,
        next_unused_large_block_id: u64,
        next_offset: usize,
        allocated_count: usize,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            spans,
            page_size_bytes,
            space_size_bytes,
            blocks,
            next_unused_large_block_id,
            next_offset,
            allocated_count,
            allocated_bytes,
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

    /// Return the captured raw spans.
    pub(crate) fn spans(&self) -> &[SmallSpanImage] {
        &self.spans
    }

    /// Return the configured local page width.
    pub(crate) const fn page_size_bytes(&self) -> usize {
        self.page_size_bytes
    }

    /// Return the reserved virtual byte capacity for raw space.
    pub(crate) const fn space_size_bytes(&self) -> usize {
        self.space_size_bytes
    }

    /// Return the captured raw blocks in large space.
    pub(crate) fn blocks(&self) -> &[LargeBlockImage] {
        &self.blocks
    }

    /// Return the next raw block id in large space.
    pub(crate) const fn next_unused_large_block_id(&self) -> u64 {
        self.next_unused_large_block_id
    }

    /// Return the next unused byte offset in raw space.
    pub(crate) const fn next_offset(&self) -> usize {
        self.next_offset
    }

    /// Return the number of live raw blocks.
    pub(crate) const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the number of live raw bytes.
    pub(crate) const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the number of pages needed to restore this image.
    pub(crate) fn page_count(&self) -> usize {
        let span_pages = self
            .spans()
            .iter()
            .map(|span| span.bytes.len().div_ceil(self.page_size_bytes))
            .sum::<usize>();
        let block_pages = self
            .blocks()
            .iter()
            .filter(|block| block.is_live)
            .map(|block| block.bytes.len().div_ceil(self.page_size_bytes))
            .sum::<usize>();

        span_pages + block_pages
    }

    /// Return this image with one explicit size-class table.
    #[cfg(test)]
    pub(crate) fn with_size_classes(mut self, size_classes: SizeClassTable) -> Self {
        self.size_classes = size_classes;

        self
    }
}

impl RawSpace {
    /// Fork one raw space over the same allocator.
    ///
    /// Call this only from a safepoint where the raw space cannot mutate.
    pub(crate) fn fork(&mut self) -> Result<Self, HeapError> {
        self.flush_branch_boundary()?;
        let mapping = self.mapping.fork_lazy()?;
        let spans = self.fork_spans()?;
        let blocks = self.fork_large_blocks()?;

        let mut space = Self {
            allocator: self.allocator.clone(),
            page_span_cache: PageSpanCache::new(self.allocator.pages_per_chunk()),
            small: super::SmallSpace {
                size_classes: self.small.size_classes.clone(),
                span_size_bytes: self.small.span_size_bytes,
                spans: CowTable::from_vec(spans),
                partial_spans: Default::default(),
            },
            large: super::LargeSpace {
                blocks: CowTable::from_vec(blocks),
                free_large_block_ids: self.large.free_large_block_ids.clone(),
                next_unused_large_block_id: self.large.next_unused_large_block_id,
            },
            page_map: Vec::new(),
            next_offset: self.next_offset,
            mapping,
            usage: self.usage,
        };

        Self::restore_partial_spans(&mut space.small)?;
        space.rebuild_page_map()?;

        Ok(space)
    }

    /// Restore one raw space from one frozen raw-space image.
    pub(crate) fn from_image(
        allocator: Arc<Allocator>,
        image: &RawSpaceImage,
    ) -> Result<Self, HeapError> {
        let mapping = AddressSpace::reserve(image.space_size_bytes(), image.page_size_bytes())?;
        let mut space = Self {
            allocator: allocator.clone(),
            page_span_cache: PageSpanCache::new(allocator.pages_per_chunk()),
            small: super::SmallSpace {
                size_classes: image.size_classes().clone(),
                span_size_bytes: image.small_bytes(),
                spans: CowTable::new(),
                partial_spans: Default::default(),
            },
            large: super::LargeSpace {
                blocks: CowTable::new(),
                free_large_block_ids: Vec::new(),
                next_unused_large_block_id: image.next_unused_large_block_id(),
            },
            page_map: Vec::new(),
            next_offset: image.next_offset(),
            mapping,
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
        };

        // restore raw small space first
        for span in image.spans() {
            let span = space.restore_span(span)?;

            space.small.spans.push(span);
        }

        // rebuild the derived small-span state
        Self::restore_partial_spans(&mut space.small)?;

        // restore raw large space next
        for block in image.blocks() {
            let block = space.restore_large_block(block)?;

            space.large.blocks.push(block);
        }

        // rebuild the reusable large-block ids
        space.large.free_large_block_ids = Self::free_large_block_ids(image);

        // rebuild the live space over fresh raw storage
        space.rebuild_page_map()?;

        Ok(space)
    }

    /// Return one frozen raw-space image.
    pub(crate) fn image(&mut self) -> HeapResult<RawSpaceImage> {
        self.flush_branch_boundary()?;

        // capture the live raw blocks directly
        let spans = self.capture_span_images()?;
        let blocks = self.capture_large_block_images()?;

        // freeze the current raw image
        Ok(RawSpaceImage::new(
            self.small.size_classes.clone(),
            self.small.span_size_bytes,
            spans,
            self.allocator.page_size_bytes(),
            self.mapping.byte_len(),
            blocks,
            self.large.next_unused_large_block_id,
            self.next_offset,
            self.usage.allocation_count(),
            self.usage.allocated_bytes(),
        ))
    }

    /// Rebuild the derived reusable-span state for one restored small space.
    fn restore_partial_spans(small: &mut super::SmallSpace) -> Result<(), HeapError> {
        for span_index in 0..small.spans.len() {
            let Some(span) = small.spans.get_mut(span_index) else {
                return Err(HeapError::internal("missing span"));
            };

            // rebuild the derived per-span occupancy counters
            span.occupied_count = span.occupied.count_ones();
            span.free_cursor = span.occupied.first_clear_from(0).unwrap_or(span.slot_count);

            // requeue every non-full span under its size class
            if span.occupied_count >= span.slot_count {
                continue;
            }

            let Some(class_index) = small.size_classes.class_index_for(span.class.size_class)
            else {
                return Err(HeapError::configuration(
                    HeapConfigurationError::InvalidSizeClass {
                        class_bytes: span.class.size_class,
                    },
                ));
            };
            let configured_size_class = small.size_classes.classes[class_index].bytes;
            if configured_size_class != span.class.size_class
                || span.class.byte_len > span.class.size_class
            {
                return Err(HeapError::configuration(
                    HeapConfigurationError::InvalidSizeClass {
                        class_bytes: span.class.size_class,
                    },
                ));
            }

            small
                .partial_spans
                .entry(span.class.clone())
                .or_default()
                .push(span_index);
        }

        Ok(())
    }

    /// Restore one raw span from one frozen span image.
    fn restore_span(&mut self, span: &SmallSpanImage) -> Result<SmallSpan, HeapError> {
        let pages = self.allocate_page_span(span.bytes.len())?;
        self.mapping.write_bytes(span.first_offset, &span.bytes)?;

        Ok(SmallSpan {
            first_offset: span.first_offset,
            class: span.class.clone(),
            slot_count: span.slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: span.occupied.clone(),
            pages,
        })
    }

    /// Restore one raw block from one frozen block image.
    fn restore_large_block(&mut self, block: &LargeBlockImage) -> Result<LargeBlock, HeapError> {
        let pages = if block.is_live {
            self.mapping.write_bytes(block.first_offset, &block.bytes)?;

            self.allocate_page_span(block.bytes.len())?
        } else {
            PageSpan::empty()
        };

        Ok(LargeBlock {
            is_live: block.is_live,
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            pages,
        })
    }

    /// Fork every raw span into shared metadata pages.
    fn fork_spans(&self) -> HeapResult<Vec<SmallSpan>> {
        self.small
            .spans
            .iter()
            .map(|span| {
                Ok(SmallSpan {
                    first_offset: span.first_offset,
                    class: span.class.clone(),
                    slot_count: span.slot_count,
                    occupied_count: span.occupied_count,
                    free_cursor: span.free_cursor,
                    occupied: span.occupied.clone(),
                    pages: self.allocator.share_page_span(span.pages)?,
                })
            })
            .collect()
    }

    /// Fork every raw large block into shared metadata pages.
    fn fork_large_blocks(&self) -> HeapResult<Vec<LargeBlock>> {
        self.large
            .blocks
            .iter()
            .map(|block| {
                let pages = if block.is_live {
                    self.allocator.share_page_span(block.pages)?
                } else {
                    PageSpan::empty()
                };

                Ok(LargeBlock {
                    is_live: block.is_live,
                    first_offset: block.first_offset,
                    byte_len: block.byte_len,
                    pages,
                })
            })
            .collect()
    }

    /// Return the reusable raw block ids from one frozen image.
    fn free_large_block_ids(image: &RawSpaceImage) -> Vec<u64> {
        image
            .blocks()
            .iter()
            .enumerate()
            .filter_map(|(index, block)| (!block.is_live).then_some(index as u64 + 1))
            .collect()
    }

    /// Capture every live raw span image.
    fn capture_span_images(&self) -> HeapResult<Box<[SmallSpanImage]>> {
        self.small
            .spans
            .iter()
            .map(|span| self.capture_span_image(span))
            .collect::<HeapResult<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    }

    /// Capture one live raw span image.
    fn capture_span_image(&self, span: &SmallSpan) -> HeapResult<SmallSpanImage> {
        let byte_len = span.pages.len() * self.allocator.page_size_bytes();
        let bytes = self
            .mapping
            .read_bytes(span.first_offset, byte_len)?
            .into_boxed_slice();

        Ok(SmallSpanImage {
            first_offset: span.first_offset,
            class: span.class.clone(),
            slot_count: span.slot_count,
            occupied: span.occupied.clone(),
            bytes,
        })
    }

    /// Capture every live raw block image in large space.
    fn capture_large_block_images(&self) -> HeapResult<Box<[LargeBlockImage]>> {
        self.large
            .blocks
            .iter()
            .map(|block| self.capture_large_block_image(block))
            .collect::<HeapResult<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    }

    /// Capture one live raw block image in large space.
    fn capture_large_block_image(&self, block: &LargeBlock) -> HeapResult<LargeBlockImage> {
        let bytes = if block.is_live {
            self.mapping
                .read_bytes(block.first_offset, block.byte_len)?
                .into_boxed_slice()
        } else {
            Box::new([])
        };

        Ok(LargeBlockImage {
            is_live: block.is_live,
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            bytes,
        })
    }
}

impl RawSpace {
    /// Rebuild the page-map table from live raw blocks.
    fn rebuild_page_map(&mut self) -> Result<(), HeapError> {
        self.page_map.clear();

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let pages = span.pages;

            self.map_page_span(span.first_offset, &pages, |logical_page_index| {
                RawPageMapEntry::SmallSpan {
                    span_index,
                    logical_page_index,
                }
            });
        }

        for block_index in 0..self.large.blocks.len() {
            let block_id = LargeBlockId::new(block_index as u64 + 1);
            let Some(block) = self.large_block(block_id) else {
                continue;
            };
            let pages = block.pages;

            self.map_page_span(block.first_offset, &pages, |logical_page_index| {
                RawPageMapEntry::LargeBlock {
                    block_id,
                    logical_page_index,
                }
            });
        }

        Ok(())
    }
}
