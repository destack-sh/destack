use serde::{Deserialize, Serialize};

use std::sync::Arc;

use super::{LargeEntry, LargeEntryImage, RawPointerEntry, RawSpace, SmallSpan, SmallSpanImage};
use crate::CowTable;
use crate::alloc::{Arena, PageView, SizeClassTable};
use crate::heap::{AllocationTotals, HeapError, HeapOptions};

/// One frozen raw-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawSpaceImage {
    /// The configured entry count per metadata table chunk.
    table_chunk_len: usize,

    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured raw spans.
    spans: Box<[SmallSpanImage]>,
    /// The configured local page width.
    page_bytes: usize,
    /// The captured raw entries in large space.
    entries: Box<[LargeEntryImage]>,

    /// Dense raw pointer metadata keyed by entry id minus one.
    pointers: Box<[RawPointerEntry]>,

    /// The next raw entry id to allocate.
    next_unused_pointer_id: u64,
    /// The next raw entry id to allocate in large space.
    next_unused_large_entry_id: u64,

    /// The number of live raw entries.
    allocated_count: usize,
    /// The number of live raw bytes.
    allocated_bytes: u64,
}

impl RawSpaceImage {
    /// Create one frozen raw-space root.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        table_chunk_len: usize,
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SmallSpanImage]>,
        page_bytes: usize,
        entries: Box<[LargeEntryImage]>,
        pointers: Box<[RawPointerEntry]>,
        next_unused_pointer_id: u64,
        next_unused_large_entry_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            table_chunk_len,
            size_classes,
            small_bytes,
            spans,
            page_bytes,
            entries,
            pointers,
            next_unused_pointer_id,
            next_unused_large_entry_id,
            allocated_count,
            allocated_bytes,
        }
    }

    /// Return the configured entry count per metadata table chunk.
    pub(crate) const fn table_chunk_len(&self) -> usize {
        self.table_chunk_len
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

    /// Return the captured raw entries in large space.
    pub(crate) fn entries(&self) -> &[LargeEntryImage] {
        &self.entries
    }

    /// Return the captured raw pointer table.
    pub(crate) fn pointers(&self) -> &[RawPointerEntry] {
        &self.pointers
    }

    /// Return the next raw entry id.
    pub(crate) const fn next_unused_pointer_id(&self) -> u64 {
        self.next_unused_pointer_id
    }

    /// Return the next raw entry id in large space.
    pub(crate) const fn next_unused_large_entry_id(&self) -> u64 {
        self.next_unused_large_entry_id
    }

    /// Return the number of live raw entries.
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
    /// Fork one raw space over the same shared arena.
    pub(crate) fn fork(&self) -> Result<Self, HeapError> {
        let page_views = live_page_views(self);
        let mut retained = Vec::new();

        // retain the shared backing before cloning metadata
        for page_view in page_views {
            if let Err(error) = self.arena.retain_page_view(&page_view) {
                for page_view in retained.into_iter().rev() {
                    self.arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(page_view);
        }

        let result = (|| {
            Ok(Self {
                arena: self.arena.clone(),
                table_chunk_len: self.table_chunk_len,
                small: Self::fork_small_space(self)?,
                large: Self::fork_large_space(self)?,
                pointers: self.pointers.clone(),
                free_pointer_ids: self.free_pointer_ids.clone(),
                next_unused_pointer_id: self.next_unused_pointer_id,
                totals: self.totals,
            })
        })();

        if let Err(error) = result {
            for page_view in retained.into_iter().rev() {
                self.arena.release_page_view(&page_view)?;
            }

            return Err(error);
        }

        result
    }

    /// Restore one raw space from one frozen raw-space root.
    pub(crate) fn from_image(arena: Arc<Arena>, image: &RawSpaceImage) -> Result<Self, HeapError> {
        HeapOptions::validate_table_chunk_len(image.table_chunk_len())?;
        let page_views = image_page_views(image).collect::<Vec<_>>();
        let mut retained = Vec::new();

        // retain the shared backing first
        for page_view in page_views {
            if let Err(error) = arena.retain_page_view(&page_view) {
                for page_view in retained.into_iter().rev() {
                    arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(page_view);
        }

        let result = (|| {
            // rebuild the dense metadata tables
            let pointers = Self::restore_pointer_table(image)?;
            let free_pointer_ids = Self::free_pointer_ids(image);

            // rebuild each live raw storage partition
            let small = Self::restore_small_space(image)?;
            let large = Self::restore_large_space(image)?;

            // rebuild the live root over the shared arena
            Ok(Self {
                arena: arena.clone(),
                table_chunk_len: image.table_chunk_len(),
                small,
                large,
                pointers,
                free_pointer_ids,
                next_unused_pointer_id: image.next_unused_pointer_id(),
                totals: AllocationTotals::new(image.allocated_count(), image.allocated_bytes()),
            })
        })();

        if let Err(error) = result {
            for page_view in retained.into_iter().rev() {
                arena.release_page_view(&page_view)?;
            }

            return Err(error);
        }

        result
    }

    /// Return one frozen raw-space root.
    pub(crate) fn image(&self) -> RawSpaceImage {
        // capture the live raw storage directly
        let spans = self.capture_span_images();
        let entries = self.capture_large_entry_images();
        let pointers = self.capture_pointer_table();

        // freeze the current raw root
        RawSpaceImage::new(
            self.table_chunk_len,
            self.small.size_classes.clone(),
            self.small.span_bytes,
            spans,
            self.large.page_bytes,
            entries,
            pointers,
            self.next_unused_pointer_id,
            self.large.next_unused_large_entry_id,
            self.totals.allocation_count(),
            self.totals.allocated_bytes(),
        )
    }

    /// Rebuild the dense raw pointer table from one frozen image.
    fn restore_pointer_table(
        image: &RawSpaceImage,
    ) -> Result<CowTable<RawPointerEntry>, HeapError> {
        crate::heap::CowTable::from_vec_with_chunk_len(
            image.pointers().to_vec(),
            image.table_chunk_len(),
        )
    }

    /// Restore the raw small space from one frozen image.
    fn restore_small_space(image: &RawSpaceImage) -> Result<super::SmallSpace, HeapError> {
        // restore the captured span roots first
        let spans = image
            .spans()
            .iter()
            .map(Self::restore_span)
            .collect::<Vec<_>>();

        let mut small = super::SmallSpace {
            size_classes: image.size_classes().clone(),
            span_bytes: image.small_bytes(),
            spans: CowTable::from_vec_with_chunk_len(spans, image.table_chunk_len())?,
            available_spans: vec![Vec::new(); image.size_classes().classes.len()],
        };

        // rebuild the derived span occupancy state
        Self::restore_available_spans(&mut small)?;

        Ok(small)
    }

    /// Fork the raw small space from one live root.
    fn fork_small_space(space: &Self) -> Result<super::SmallSpace, HeapError> {
        // capture and restore spans so branch-local occupancy derives cleanly
        let spans = space
            .small
            .spans
            .iter()
            .map(Self::capture_span_image)
            .map(|span| Self::restore_span(&span))
            .collect::<Vec<_>>();

        let mut small = super::SmallSpace {
            size_classes: space.small.size_classes.clone(),
            span_bytes: space.small.span_bytes,
            spans: CowTable::from_vec_with_chunk_len(spans, space.table_chunk_len)?,
            available_spans: vec![Vec::new(); space.small.size_classes.classes.len()],
        };

        // rebuild the derived span occupancy state
        Self::restore_available_spans(&mut small)?;

        Ok(small)
    }

    /// Rebuild the derived reusable-span state for one restored small space.
    fn restore_available_spans(small: &mut super::SmallSpace) -> Result<(), HeapError> {
        for span_index in 0..small.spans.len() {
            let Some(span) = small.spans.get_mut(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            // rebuild the derived per-span occupancy counters
            span.occupied_count = span.occupied.count_ones();
            span.next_free_slot = span.occupied.first_clear_from(0).unwrap_or(span.slot_count);

            // requeue every non-full span under its size class
            if span.occupied_count >= span.slot_count {
                continue;
            }

            let Some(class_index) = small.size_classes.class_index_for(span.size_class) else {
                return Err(HeapError::InvalidSizeClass {
                    class_bytes: span.size_class,
                });
            };

            small.available_spans[class_index].push(span_index);
        }

        Ok(())
    }

    /// Restore one raw span from one frozen span root.
    fn restore_span(span: &SmallSpanImage) -> SmallSpan {
        SmallSpan {
            size_class: span.size_class,
            slot_count: span.slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            lengths: span.lengths.clone(),
            occupied: span.occupied.clone(),
            pages: span.pages,
        }
    }

    /// Restore the raw large space from one frozen image.
    fn restore_large_space(image: &RawSpaceImage) -> Result<super::LargeSpace, HeapError> {
        // rebuild the captured entry roots first
        let entries = image
            .entries()
            .iter()
            .map(Self::restore_large_entry)
            .collect::<Vec<_>>();

        // rebuild the reusable entry ids from the frozen table
        let free_large_entry_ids = Self::free_large_entry_ids(image);

        Ok(super::LargeSpace {
            page_bytes: image.page_bytes(),
            entries: CowTable::from_vec_with_chunk_len(entries, image.table_chunk_len())?,
            free_large_entry_ids,
            next_unused_large_entry_id: image.next_unused_large_entry_id(),
        })
    }

    /// Fork the raw large space from one live root.
    fn fork_large_space(space: &Self) -> Result<super::LargeSpace, HeapError> {
        let entries = space
            .large
            .entries
            .iter()
            .map(Self::capture_large_entry_image)
            .map(|entry| Self::restore_large_entry(&entry))
            .collect::<Vec<_>>();

        Ok(super::LargeSpace {
            page_bytes: space.large.page_bytes,
            entries: CowTable::from_vec_with_chunk_len(entries, space.table_chunk_len)?,
            free_large_entry_ids: space.large.free_large_entry_ids.clone(),
            next_unused_large_entry_id: space.large.next_unused_large_entry_id,
        })
    }

    /// Restore one raw entry from one frozen entry root.
    fn restore_large_entry(entry: &LargeEntryImage) -> LargeEntry {
        LargeEntry {
            is_live: entry.is_live,
            len: entry.len,
            pages: entry.pages,
        }
    }

    /// Return the reusable raw entry ids from one frozen image.
    fn free_large_entry_ids(image: &RawSpaceImage) -> Vec<u64> {
        image
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| (!entry.is_live).then_some(index as u64 + 1))
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
    fn capture_pointer_table(&self) -> Box<[RawPointerEntry]> {
        self.pointers.to_boxed_slice()
    }

    /// Capture every live raw span image.
    fn capture_span_images(&self) -> Box<[SmallSpanImage]> {
        self.small
            .spans
            .iter()
            .map(Self::capture_span_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live raw span image.
    fn capture_span_image(span: &SmallSpan) -> SmallSpanImage {
        SmallSpanImage {
            size_class: span.size_class,
            slot_count: span.slot_count,
            lengths: span.lengths.clone(),
            occupied: span.occupied.clone(),
            pages: span.pages,
        }
    }

    /// Capture every live raw entry image in large space.
    fn capture_large_entry_images(&self) -> Box<[LargeEntryImage]> {
        self.large
            .entries
            .iter()
            .map(Self::capture_large_entry_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live raw entry image in large space.
    fn capture_large_entry_image(entry: &LargeEntry) -> LargeEntryImage {
        LargeEntryImage {
            is_live: entry.is_live,
            len: entry.len,
            pages: entry.pages,
        }
    }
}

/// Return the page views reachable from one frozen raw-space root.
fn image_page_views(image: &RawSpaceImage) -> impl DoubleEndedIterator<Item = PageView> + '_ {
    image
        .spans()
        .iter()
        .map(|span| span.pages)
        .chain(image.entries().iter().map(|entry| entry.pages))
}

/// Return the page views reachable from one live raw space.
fn live_page_views(space: &RawSpace) -> Vec<PageView> {
    let mut page_views = Vec::new();

    // collect the span roots first
    page_views.extend(space.small.spans.iter().map(|span| span.pages));

    // collect the large-entry roots next
    page_views.extend(space.large.entries.iter().map(|entry| entry.pages));

    page_views
}
