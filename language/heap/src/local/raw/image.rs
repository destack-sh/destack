use serde::{Deserialize, Serialize};

use std::sync::Arc;

use super::{
    LargeEntry, LargeEntryId, LargeEntryImage, RawPageOwner, RawSpace, SmallSpan, SmallSpanImage,
};
use crate::allocator::{Allocator, PageRunCache, SizeClassTable};
use crate::{AllocationUsage, CowTable, HeapError};

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
    /// The captured raw entries in large space.
    entries: Box<[LargeEntryImage]>,

    /// The next raw entry id to allocate in large space.
    next_unused_large_entry_id: u64,

    /// The number of live raw entries.
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
        entries: Box<[LargeEntryImage]>,
        next_unused_large_entry_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
    ) -> Self {
        Self {
            size_classes,
            small_bytes,
            spans,
            page_bytes,
            entries,
            next_unused_large_entry_id,
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

    /// Return the captured raw entries in large space.
    pub(crate) fn entries(&self) -> &[LargeEntryImage] {
        &self.entries
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
    /// Fork one raw space over the same allocator.
    pub(crate) fn fork(&mut self) -> Result<Self, HeapError> {
        let image = self.image();

        Self::from_image(self.allocator.clone(), &image)
    }

    /// Restore one raw space from one frozen raw-space image.
    pub(crate) fn from_image(
        allocator: Arc<Allocator>,
        image: &RawSpaceImage,
    ) -> Result<Self, HeapError> {
        let mut space = Self {
            allocator: allocator.clone(),
            page_run_cache: PageRunCache::new(allocator.pages_per_segment()),
            small: super::SmallSpace {
                size_classes: image.size_classes().clone(),
                span_bytes: image.small_bytes(),
                spans: CowTable::new(),
                available_spans: vec![Vec::new(); image.size_classes().classes.len()],
            },
            large: super::LargeSpace {
                page_bytes: image.page_bytes(),
                entries: CowTable::new(),
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: image.next_unused_large_entry_id(),
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
        Self::restore_available_spans(&mut space.small)?;

        // restore raw large space next
        for entry in image.entries() {
            let entry = space.restore_large_entry(entry)?;

            space.large.entries.push(entry)?;
        }

        // rebuild the reusable large-entry ids
        space.large.free_large_entry_ids = Self::free_large_entry_ids(image);

        // rebuild the live root over fresh raw storage
        space.rebuild_page_owners()?;

        Ok(space)
    }

    /// Return one frozen raw-space image.
    pub(crate) fn image(&mut self) -> RawSpaceImage {
        self.flush_branch_boundary();

        // capture the live raw storage directly
        let spans = self.capture_span_images();
        let entries = self.capture_large_entry_images();

        // freeze the current raw root
        RawSpaceImage::new(
            self.small.size_classes.clone(),
            self.small.span_bytes,
            spans,
            self.large.page_bytes,
            entries,
            self.large.next_unused_large_entry_id,
            self.usage.allocation_count(),
            self.usage.allocated_bytes(),
        )
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

    /// Restore one raw span from one frozen span image.
    fn restore_span(&mut self, span: &SmallSpanImage) -> Result<SmallSpan, HeapError> {
        let pages = self.allocate_page_view_bytes(&span.bytes)?;

        Ok(SmallSpan {
            size_class: span.size_class,
            slot_count: span.slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            lengths: span.lengths.clone(),
            occupied: span.occupied.clone(),
            pages,
        })
    }

    /// Restore one raw entry from one frozen entry image.
    fn restore_large_entry(&mut self, entry: &LargeEntryImage) -> Result<LargeEntry, HeapError> {
        let pages = if entry.is_live {
            self.allocate_page_view_bytes(&entry.bytes)?
        } else {
            crate::allocator::PageView::empty()
        };

        Ok(LargeEntry {
            is_live: entry.is_live,
            len: entry.len,
            pages,
        })
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
            size_class: span.size_class,
            slot_count: span.slot_count,
            lengths: span.lengths.clone(),
            occupied: span.occupied.clone(),
            bytes: bytes.into_boxed_slice(),
        }
    }

    /// Capture every live raw entry image in large space.
    fn capture_large_entry_images(&self) -> Box<[LargeEntryImage]> {
        self.large
            .entries
            .iter()
            .map(|entry| self.capture_large_entry_image(entry))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live raw entry image in large space.
    fn capture_large_entry_image(&self, entry: &LargeEntry) -> LargeEntryImage {
        let bytes = if entry.is_live {
            self.allocator
                .read_bytes(&entry.pages, entry.len)
                .expect("raw entry image bytes should resolve")
                .into_boxed_slice()
        } else {
            Box::new([])
        };

        LargeEntryImage {
            is_live: entry.is_live,
            len: entry.len,
            bytes,
        }
    }
}

impl RawSpace {
    /// Rebuild the page-owner table from live raw storage.
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

        for entry_index in 0..self.large.entries.len() {
            let entry_id = LargeEntryId::new(entry_index as u64 + 1);
            let Some(entry) = self.large_entry(entry_id) else {
                continue;
            };
            let pages = entry.pages.clone();

            self.map_page_view(&pages, |logical_page_index| RawPageOwner::Large {
                entry_id,
                logical_page_index,
            })?;
        }

        Ok(())
    }
}
