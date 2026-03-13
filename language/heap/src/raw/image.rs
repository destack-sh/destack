use super::RawSpan;
use super::heap::RawHeap;
use crate::page::{PageSlot, RawPage, RawPageImage};
use crate::{RawHeapSnapshot, Vector};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Immutable raw heap image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawImage {
    /// Captured raw pages.
    pub pages: Vector<Arc<RawPageImage>>,
    /// Captured raw spans.
    pub spans: Vector<Arc<[u8]>>,
    /// Captured dedicated large raw spans.
    pub large_spans: Vector<Arc<[u8]>>,
    /// The stable location for each allocated raw allocation id.
    pub locations: Arc<[PageSlot]>,
    /// The next raw allocation id to allocate.
    pub next_unused_id: u64,
    /// The next raw span id to allocate.
    pub next_unused_span_id: u64,
    /// The next large raw span id to allocate.
    pub next_unused_large_span_id: u64,
    /// The next raw page slot to allocate for one new id.
    pub next_unused_slot: u64,
    /// The captured free raw allocation ids.
    pub free_ids: Arc<[u64]>,
    /// The captured free raw spans.
    pub free_span_ids: Arc<[u64]>,
    /// The captured free large raw spans.
    pub free_large_span_ids: Arc<[u64]>,
    /// The number of allocated raw allocations.
    pub allocated_count: usize,
    /// The number of allocated raw bytes.
    pub allocated_bytes: u64,
    /// The captured dedicated large-span threshold in bytes.
    pub large_span_bytes: usize,
}

impl RawImage {
    /// Build one raw heap image from one raw heap snapshot.
    pub(crate) fn from_snapshot(snapshot: &RawHeapSnapshot) -> Self {
        Self {
            pages: Vector::from_shared(
                &snapshot
                    .pages
                    .iter()
                    .cloned()
                    .map(Arc::new)
                    .collect::<Vec<_>>(),
                None,
            ),
            spans: Vector::from_shared(
                &snapshot
                    .spans
                    .iter()
                    .cloned()
                    .map(|bytes| Arc::from(bytes.into_boxed_slice()))
                    .collect::<Vec<_>>(),
                None,
            ),
            large_spans: Vector::from_shared(
                &snapshot
                    .large_spans
                    .iter()
                    .cloned()
                    .map(|bytes| Arc::from(bytes.into_boxed_slice()))
                    .collect::<Vec<_>>(),
                None,
            ),
            locations: Arc::from(snapshot.locations.as_slice()),
            next_unused_id: snapshot.next_unused_id,
            next_unused_span_id: snapshot.next_unused_span_id,
            next_unused_large_span_id: snapshot.next_unused_large_span_id,
            next_unused_slot: snapshot.next_unused_slot,
            free_ids: Arc::from(snapshot.free_ids.as_slice()),
            free_span_ids: Arc::from(snapshot.free_span_ids.as_slice()),
            free_large_span_ids: Arc::from(snapshot.free_large_span_ids.as_slice()),
            allocated_count: snapshot.allocated_count,
            allocated_bytes: snapshot.allocated_bytes,
            large_span_bytes: snapshot.large_span_bytes,
        }
    }

    /// Flatten one raw heap image into one raw heap snapshot.
    pub(crate) fn snapshot(&self) -> RawHeapSnapshot {
        RawHeapSnapshot {
            pages: self.pages.iter().map(|page| (**page).clone()).collect(),
            spans: self
                .spans
                .iter()
                .map(|bytes| bytes.as_ref().to_vec())
                .collect(),
            large_spans: self
                .large_spans
                .iter()
                .map(|bytes| bytes.as_ref().to_vec())
                .collect(),
            locations: self.locations.iter().copied().collect(),
            next_unused_id: self.next_unused_id,
            next_unused_span_id: self.next_unused_span_id,
            next_unused_large_span_id: self.next_unused_large_span_id,
            next_unused_slot: self.next_unused_slot,
            free_ids: self.free_ids.iter().copied().collect(),
            free_span_ids: self.free_span_ids.iter().copied().collect(),
            free_large_span_ids: self.free_large_span_ids.iter().copied().collect(),
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            large_span_bytes: self.large_span_bytes,
        }
    }
}

impl RawHeap {
    /// Create one raw heap directly from one raw heap image.
    pub(crate) fn from_image(image: &RawImage) -> Self {
        let pages = image
            .pages
            .iter()
            .cloned()
            .map(RawPage::from_image)
            .collect::<Vec<_>>();
        let spans = image
            .spans
            .iter()
            .cloned()
            .map(RawSpan::from_image)
            .collect();
        let large_spans = image
            .large_spans
            .iter()
            .cloned()
            .map(RawSpan::from_image)
            .collect();
        let locations = image.locations.iter().copied().collect();
        let free_ids = image.free_ids.iter().copied().collect();
        let free_span_ids = image.free_span_ids.iter().copied().collect();
        let free_large_span_ids = image.free_large_span_ids.iter().copied().collect();

        Self {
            pages,
            spans,
            large_spans,
            locations,
            free_ids,
            free_span_ids,
            free_large_span_ids,
            next_unused_id: image.next_unused_id,
            next_unused_span_id: image.next_unused_span_id,
            next_unused_large_span_id: image.next_unused_large_span_id,
            next_unused_slot: image.next_unused_slot,
            allocated_count: image.allocated_count,
            allocated_bytes: image.allocated_bytes,
            retained_bytes: 0,
            large_span_bytes: image.large_span_bytes,
        }
        .with_retained_bytes()
    }

    /// Capture one immutable raw heap section.
    pub(crate) fn image(&mut self, base: Option<&RawImage>) -> RawImage {
        // capture page contents
        let pages = Vector::from_shared(
            &self
                .pages
                .iter_mut()
                .map(RawPage::image)
                .collect::<Vec<_>>(),
            base.map(|image| &image.pages),
        );
        let spans = Vector::from_shared(
            &self
                .spans
                .iter_mut()
                .map(RawSpan::image)
                .collect::<Vec<_>>(),
            base.map(|image| &image.spans),
        );
        let large_spans = Vector::from_shared(
            &self
                .large_spans
                .iter_mut()
                .map(RawSpan::image)
                .collect::<Vec<_>>(),
            base.map(|image| &image.large_spans),
        );
        let locations = Arc::from(self.locations.as_slice());

        // capture allocator state
        let free_ids = Arc::from(self.free_ids.as_slice());
        let free_span_ids = Arc::from(self.free_span_ids.as_slice());
        let free_large_span_ids = Arc::from(self.free_large_span_ids.as_slice());

        RawImage {
            pages,
            spans,
            large_spans,
            locations,
            next_unused_id: self.next_unused_id,
            next_unused_span_id: self.next_unused_span_id,
            next_unused_large_span_id: self.next_unused_large_span_id,
            next_unused_slot: self.next_unused_slot,
            free_ids,
            free_span_ids,
            free_large_span_ids,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            large_span_bytes: self.large_span_bytes,
        }
    }
}
