use serde::{Deserialize, Serialize};

use std::sync::Arc;

use super::{
    LargeAllocation, LargeAllocationId, LargeAllocationImage, RawPageOwner, RawSpace, SmallSpan,
    SmallSpanImage,
};
use crate::allocator::{Allocator, PageRunCache, SizeClassTable};
use crate::{AllocationUsage, CowTable, HeapError, HeapResult};

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
    page_bytes: usize,
    /// The captured raw allocations in large space.
    allocations: Box<[LargeAllocationImage]>,

    /// The next raw allocation id to allocate in large space.
    next_unused_large_allocation_id: u64,

    /// The number of live raw allocations.
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
        page_bytes: usize,
        allocations: Box<[LargeAllocationImage]>,
        next_unused_large_allocation_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            spans,
            page_bytes,
            allocations,
            next_unused_large_allocation_id,
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
    pub(crate) const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the captured raw allocations in large space.
    pub(crate) fn allocations(&self) -> &[LargeAllocationImage] {
        &self.allocations
    }

    /// Return the next raw allocation id in large space.
    pub(crate) const fn next_unused_large_allocation_id(&self) -> u64 {
        self.next_unused_large_allocation_id
    }

    /// Return the number of live raw allocations.
    pub(crate) const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the number of live raw bytes.
    pub(crate) const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    #[cfg(test)]
    /// Return this image with one explicit size-class table.
    pub(crate) fn with_size_classes(mut self, size_classes: SizeClassTable) -> Self {
        self.size_classes = size_classes;

        self
    }
}

impl RawSpace {
    /// Fork one raw space over the same allocator.
    pub(crate) fn fork(&mut self) -> Result<Self, HeapError> {
        let image = self.image()?;

        Self::from_image(self.allocator.clone(), &image)
    }

    /// Restore one raw space from one frozen raw-space image.
    pub(crate) fn from_image(
        allocator: Arc<Allocator>,
        image: &RawSpaceImage,
    ) -> Result<Self, HeapError> {
        let mut space = Self {
            allocator: allocator.clone(),
            page_run_cache: PageRunCache::new(allocator.pages_per_arena()),
            small: super::SmallSpace {
                size_classes: image.size_classes().clone(),
                span_bytes: image.small_bytes(),
                spans: CowTable::new(),
                partial_spans: Default::default(),
            },
            large: super::LargeSpace {
                page_bytes: image.page_bytes(),
                allocations: CowTable::new(),
                free_large_allocation_ids: Vec::new(),
                next_unused_large_allocation_id: image.next_unused_large_allocation_id(),
            },
            page_owners: Vec::new(),
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
        };

        // restore raw small space first
        for span in image.spans() {
            let span = space.restore_span(span)?;

            space.small.spans.push(span)?;
        }

        // rebuild the derived small-span state
        Self::restore_partial_spans(&mut space.small)?;

        // restore raw large space next
        for allocation in image.allocations() {
            let allocation = space.restore_large_allocation(allocation)?;

            space.large.allocations.push(allocation)?;
        }

        // rebuild the reusable large-allocation ids
        space.large.free_large_allocation_ids = Self::free_large_allocation_ids(image);

        // rebuild the live root over fresh raw place
        space.rebuild_page_owners()?;

        Ok(space)
    }

    /// Return one frozen raw-space image.
    pub(crate) fn image(&mut self) -> HeapResult<RawSpaceImage> {
        self.flush_branch_boundary()?;

        // capture the live raw allocations directly
        let spans = self.capture_span_images();
        let allocations = self.capture_large_allocation_images();

        // freeze the current raw root
        Ok(RawSpaceImage::new(
            self.small.size_classes.clone(),
            self.small.span_bytes,
            spans,
            self.large.page_bytes,
            allocations,
            self.large.next_unused_large_allocation_id,
            self.usage.allocation_count(),
            self.usage.allocated_bytes(),
        ))
    }

    /// Rebuild the derived reusable-span state for one restored small space.
    fn restore_partial_spans(small: &mut super::SmallSpace) -> Result<(), HeapError> {
        for span_index in 0..small.spans.len() {
            let Some(span) = small.spans.get_mut(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            // rebuild the derived per-span occupancy counters
            span.occupied_count = span.occupied.count_ones();
            span.free_cursor = span.occupied.first_clear_from(0).unwrap_or(span.slot_count);

            // requeue every non-full span under its size class
            if span.occupied_count >= span.slot_count {
                continue;
            }

            let Some(class_index) = small.size_classes.class_index_for(span.class.byte_len) else {
                return Err(HeapError::InvalidSizeClass {
                    class_bytes: span.class.byte_len,
                });
            };
            let configured_size_class = small.size_classes.classes[class_index].bytes;
            if configured_size_class != span.class.size_class {
                return Err(HeapError::InvalidSizeClass {
                    class_bytes: span.class.size_class,
                });
            }

            small
                .partial_spans
                .entry(span.class.byte_len)
                .or_default()
                .push(span_index);
        }

        Ok(())
    }

    /// Restore one raw span from one frozen span image.
    fn restore_span(&mut self, span: &SmallSpanImage) -> Result<SmallSpan, HeapError> {
        let pages = self.allocate_page_view_bytes(&span.bytes)?;

        Ok(SmallSpan {
            class: span.class.clone(),
            slot_count: span.slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: span.occupied.clone(),
            pages,
        })
    }

    /// Restore one raw allocation from one frozen allocation image.
    fn restore_large_allocation(
        &mut self,
        allocation: &LargeAllocationImage,
    ) -> Result<LargeAllocation, HeapError> {
        let pages = if allocation.is_live {
            self.allocate_page_view_bytes(&allocation.bytes)?
        } else {
            crate::allocator::PageView::empty()
        };

        Ok(LargeAllocation {
            is_live: allocation.is_live,
            len: allocation.len,
            pages,
        })
    }

    /// Return the reusable raw allocation ids from one frozen image.
    fn free_large_allocation_ids(image: &RawSpaceImage) -> Vec<u64> {
        image
            .allocations()
            .iter()
            .enumerate()
            .filter_map(|(index, allocation)| (!allocation.is_live).then_some(index as u64 + 1))
            .collect()
    }

    /// Capture every live raw span image.
    fn capture_span_images(&self) -> Box<[SmallSpanImage]> {
        self.small
            .spans
            .iter()
            .map(|span| self.capture_span_image(span))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live raw span image.
    fn capture_span_image(&self, span: &SmallSpan) -> SmallSpanImage {
        let byte_len = span.pages.len() * self.allocator.page_bytes();
        let bytes = self
            .allocator
            .read_bytes(&span.pages, byte_len)
            .expect("raw span image bytes should resolve");

        SmallSpanImage {
            class: span.class.clone(),
            slot_count: span.slot_count,
            occupied: span.occupied.clone(),
            bytes: bytes.into_boxed_slice(),
        }
    }

    /// Capture every live raw allocation image in large space.
    fn capture_large_allocation_images(&self) -> Box<[LargeAllocationImage]> {
        self.large
            .allocations
            .iter()
            .map(|allocation| self.capture_large_allocation_image(allocation))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live raw allocation image in large space.
    fn capture_large_allocation_image(&self, allocation: &LargeAllocation) -> LargeAllocationImage {
        let bytes = if allocation.is_live {
            self.allocator
                .read_bytes(&allocation.pages, allocation.len)
                .expect("raw allocation image bytes should resolve")
                .into_boxed_slice()
        } else {
            Box::new([])
        };

        LargeAllocationImage {
            is_live: allocation.is_live,
            len: allocation.len,
            bytes,
        }
    }
}

impl RawSpace {
    /// Rebuild the page-owner table from live raw allocations.
    fn rebuild_page_owners(&mut self) -> Result<(), HeapError> {
        self.page_owners.clear();

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };
            let pages = span.pages.clone();

            self.map_page_view(&pages, |logical_page_index| RawPageOwner::Small {
                span_index,
                logical_page_index,
            })?;
        }

        for allocation_index in 0..self.large.allocations.len() {
            let allocation_id = LargeAllocationId::new(allocation_index as u64 + 1);
            let Some(allocation) = self.large_allocation(allocation_id) else {
                continue;
            };
            let pages = allocation.pages.clone();

            self.map_page_view(&pages, |logical_page_index| RawPageOwner::Large {
                allocation_id,
                logical_page_index,
            })?;
        }

        Ok(())
    }
}
