use serde::{Deserialize, Serialize};

use std::sync::Arc;

use super::{Allocation, AllocationImage, RawPointerRecord, RawSpace, Span, SpanImage};
use crate::alloc::{Arena, SizeClassTable};

/// One frozen raw-space root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawSpaceImage {
    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The configured local page width.
    page_bytes: usize,
    /// The captured raw spans.
    spans: Box<[SpanImage]>,
    /// The captured raw allocations in large space.
    allocations: Box<[AllocationImage]>,
    /// Dense raw pointer metadata keyed by allocation id minus one.
    pointers: Box<[RawPointerRecord]>,
    /// The next raw allocation id to allocate.
    next_unused_pointer_id: u64,
    /// The next raw allocation id to allocate in large space.
    next_unused_allocation_id: u64,
    /// The number of live raw allocations.
    allocated_count: usize,
    /// The number of live raw bytes.
    allocated_bytes: u64,
}

/// One serialized raw-space snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawSpaceSnapshot {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured small-space span width.
    pub(crate) small_bytes: usize,
    /// The configured local page width.
    pub(crate) page_bytes: usize,
    /// The serialized raw spans.
    pub(crate) spans: Box<[SpanImage]>,
    /// The serialized raw allocations in large space.
    pub(crate) allocations: Box<[AllocationImage]>,
    /// Dense raw pointer metadata keyed by allocation id minus one.
    pub(crate) pointers: Box<[RawPointerRecord]>,
    /// The next raw allocation id to allocate.
    pub(crate) next_unused_pointer_id: u64,
    /// The next raw allocation id to allocate in large space.
    pub(crate) next_unused_allocation_id: u64,
    /// The number of live raw allocations.
    pub(crate) allocated_count: usize,
    /// The number of live raw bytes.
    pub(crate) allocated_bytes: u64,
}

impl RawSpaceImage {
    /// Create one frozen raw-space root.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        size_classes: SizeClassTable,
        small_bytes: usize,
        page_bytes: usize,
        spans: Box<[SpanImage]>,
        allocations: Box<[AllocationImage]>,
        pointers: Box<[RawPointerRecord]>,
        next_unused_pointer_id: u64,
        next_unused_allocation_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            page_bytes,
            spans,
            allocations,
            pointers,
            next_unused_pointer_id,
            next_unused_allocation_id,
            allocated_count,
            allocated_bytes,
        }
    }

    /// Build one frozen raw-space root from one serialized snapshot.
    pub(crate) fn from_snapshot(snapshot: &RawSpaceSnapshot) -> Self {
        Self {
            size_classes: snapshot.size_classes.clone(),
            small_bytes: snapshot.small_bytes,
            page_bytes: snapshot.page_bytes,
            spans: snapshot.spans.clone(),
            allocations: snapshot.allocations.clone(),
            pointers: snapshot.pointers.clone(),
            next_unused_pointer_id: snapshot.next_unused_pointer_id,
            next_unused_allocation_id: snapshot.next_unused_allocation_id,
            allocated_count: snapshot.allocated_count,
            allocated_bytes: snapshot.allocated_bytes,
        }
    }

    /// Flatten one frozen raw-space root into one serialized snapshot.
    pub(crate) fn snapshot(&self) -> RawSpaceSnapshot {
        RawSpaceSnapshot {
            size_classes: self.size_classes.clone(),
            small_bytes: self.small_bytes,
            page_bytes: self.page_bytes,
            spans: self.spans.clone(),
            allocations: self.allocations.clone(),
            pointers: self.pointers.clone(),
            next_unused_pointer_id: self.next_unused_pointer_id,
            next_unused_allocation_id: self.next_unused_allocation_id,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
        }
    }

    /// Return the captured raw spans.
    pub(crate) fn spans(&self) -> &[SpanImage] {
        &self.spans
    }

    /// Return the captured raw allocations in large space.
    pub(crate) fn allocations(&self) -> &[AllocationImage] {
        &self.allocations
    }

    /// Return the configured size-class table.
    pub(crate) fn size_classes(&self) -> &SizeClassTable {
        &self.size_classes
    }

    /// Return the configured small-space span width.
    pub(crate) const fn small_bytes(&self) -> usize {
        self.small_bytes
    }

    /// Return the configured local page width.
    pub(crate) const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the captured raw pointer table.
    pub(crate) fn pointers(&self) -> &[RawPointerRecord] {
        &self.pointers
    }

    /// Return the next raw allocation id.
    pub(crate) const fn next_unused_pointer_id(&self) -> u64 {
        self.next_unused_pointer_id
    }

    /// Return the next raw allocation id in large space.
    pub(crate) const fn next_unused_allocation_id(&self) -> u64 {
        self.next_unused_allocation_id
    }

    /// Return the number of live raw allocations.
    pub(crate) const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the number of live raw bytes.
    pub(crate) const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }
}

impl RawSpace {
    /// Restore one raw space from one frozen raw-space root.
    pub(crate) fn from_image(arena: Arc<Arena>, image: &RawSpaceImage) -> Self {
        // retain the shared backing first
        Self::retain_image_pages(&arena, image);

        // rebuild the dense metadata tables
        let pointers = Self::restore_pointer_table(image);
        let free_pointer_ids = Self::free_pointer_ids(image);

        // rebuild each live raw storage partition
        let small = Self::restore_small_space(image);
        let large = Self::restore_large_space(image);

        // rebuild the live root over the shared arena
        Self {
            arena,
            small,
            large,
            pointers,
            free_pointer_ids,
            next_unused_pointer_id: image.next_unused_pointer_id(),
            allocated_count: image.allocated_count(),
            allocated_bytes: image.allocated_bytes(),
        }
    }

    /// Return one frozen raw-space root.
    pub(crate) fn image(&self) -> RawSpaceImage {
        // capture the live raw storage directly
        let spans = self.capture_span_images();
        let allocations = self.capture_allocation_images();
        let pointers = self.capture_pointer_table();

        // freeze the current raw root
        RawSpaceImage::new(
            self.small.size_classes.clone(),
            self.small.span_bytes,
            self.large.page_bytes,
            spans,
            allocations,
            pointers,
            self.next_unused_pointer_id,
            self.large.next_unused_allocation_id,
            self.allocated_count,
            self.allocated_bytes,
        )
    }

    /// Retain every arena page reachable from one frozen raw-space root.
    fn retain_image_pages(arena: &Arc<Arena>, image: &RawSpaceImage) {
        // retain every captured span root
        for span in image.spans() {
            arena.retain_pages(&span.pages);
        }

        // retain every captured large allocation root
        for allocation in image.allocations() {
            arena.retain_pages(&allocation.pages);
        }
    }

    /// Rebuild the dense raw pointer table from one frozen image.
    fn restore_pointer_table(image: &RawSpaceImage) -> Vec<RawPointerRecord> {
        image.pointers().to_vec()
    }

    /// Restore the raw small-allocation space from one frozen image.
    fn restore_small_space(image: &RawSpaceImage) -> super::SmallSpace {
        // restore the captured span roots first
        let spans = image
            .spans()
            .iter()
            .map(Self::restore_span)
            .collect::<Vec<_>>();

        let mut small = super::SmallSpace {
            size_classes: image.size_classes().clone(),
            span_bytes: image.small_bytes(),
            spans,
            available_spans: vec![Vec::new(); image.size_classes().classes.len()],
        };

        // rebuild the derived span occupancy state
        Self::restore_available_spans(&mut small);

        small
    }

    /// Rebuild the derived reusable-span state for one restored small space.
    fn restore_available_spans(small: &mut super::SmallSpace) {
        for (span_index, span) in small.spans.iter_mut().enumerate() {
            // rebuild the derived per-span occupancy counters
            span.occupied_count = span.occupied.count_ones();
            span.next_free_slot = span.occupied.first_clear_from(0).unwrap_or(span.slot_count);

            // requeue every non-full span under its size class
            if span.occupied_count >= span.slot_count {
                continue;
            }

            let Some(class_index) = small.size_classes.class_index_for(span.size_class) else {
                continue;
            };

            small.available_spans[class_index].push(span_index);
        }
    }

    /// Restore one raw span from one frozen span root.
    fn restore_span(span: &SpanImage) -> Span {
        Span {
            size_class: span.size_class,
            slot_count: span.slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            lengths: span.lengths.clone(),
            occupied: span.occupied.clone(),
            pages: span.pages.clone(),
        }
    }

    /// Restore the raw large space from one frozen image.
    fn restore_large_space(image: &RawSpaceImage) -> super::LargeSpace {
        // rebuild the captured allocation roots first
        let allocations = image
            .allocations()
            .iter()
            .map(Self::restore_allocation)
            .collect();

        // rebuild the reusable allocation ids from the frozen table
        let free_allocation_ids = Self::free_allocation_ids(image);

        super::LargeSpace {
            page_bytes: image.page_bytes(),
            allocations,
            free_allocation_ids,
            next_unused_allocation_id: image.next_unused_allocation_id(),
        }
    }

    /// Restore one raw allocation from one frozen allocation root.
    fn restore_allocation(allocation: &AllocationImage) -> Allocation {
        Allocation {
            is_allocated: allocation.is_allocated,
            len: allocation.len,
            pages: allocation.pages.clone(),
        }
    }

    /// Return the reusable raw allocation ids from one frozen image.
    fn free_allocation_ids(image: &RawSpaceImage) -> Vec<u64> {
        image
            .allocations()
            .iter()
            .enumerate()
            .filter_map(|(index, allocation)| {
                (!allocation.is_allocated).then_some(index as u64 + 1)
            })
            .collect()
    }

    /// Return the reusable raw pointer ids from one frozen image.
    fn free_pointer_ids(image: &RawSpaceImage) -> Vec<u64> {
        image
            .pointers()
            .iter()
            .enumerate()
            .filter_map(|(index, record)| record.is_vacant().then_some(index as u64 + 1))
            .collect()
    }

    /// Capture the dense raw pointer table.
    fn capture_pointer_table(&self) -> Box<[RawPointerRecord]> {
        self.pointers.clone().into_boxed_slice()
    }

    /// Capture every live raw span image.
    fn capture_span_images(&self) -> Box<[SpanImage]> {
        self.small
            .spans
            .iter()
            .map(Self::capture_span_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live raw span image.
    fn capture_span_image(span: &Span) -> SpanImage {
        SpanImage {
            size_class: span.size_class,
            slot_count: span.slot_count,
            lengths: span.lengths.clone(),
            occupied: span.occupied.clone(),
            pages: span.pages.clone(),
        }
    }

    /// Capture every live raw allocation image in large space.
    fn capture_allocation_images(&self) -> Box<[AllocationImage]> {
        self.large
            .allocations
            .iter()
            .map(Self::capture_allocation_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live raw allocation image in large space.
    fn capture_allocation_image(allocation: &Allocation) -> AllocationImage {
        AllocationImage {
            is_allocated: allocation.is_allocated,
            len: allocation.len,
            pages: allocation.pages.clone(),
        }
    }
}
