use super::{ManagedHeap, ManagedLargeSpan};
use crate::page::{ManagedPage, ManagedPageImage, PageSlot, ValuePage, ValuePageImage};
use crate::{GcPhase, GcState, HeapCaptureError, ManagedHeapSnapshot, ManagedSpan, Value, Vector};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Immutable managed heap image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManagedImage {
    /// Captured managed pages.
    pub pages: Vector<ManagedPageImage>,
    /// Captured value pages.
    pub value_pages: Vector<Arc<ValuePageImage>>,
    /// Captured dedicated large managed spans.
    pub large_spans: Vector<Arc<[Value]>>,
    /// The stable location for each allocated managed reference id.
    pub locations: Arc<[PageSlot]>,

    /// The next managed reference id to allocate.
    pub next_unused_id: u64,
    /// The next managed page slot to allocate for one new id.
    pub next_unused_slot: u64,
    /// The next managed span index to allocate.
    pub next_unused_span_index: u64,
    /// The next large managed span id to allocate.
    pub next_unused_large_span_id: u64,

    /// The captured free managed reference ids.
    pub free_ids: Arc<[u64]>,
    /// The captured free managed spans.
    pub free_spans: Arc<[ManagedSpan]>,
    /// The captured free large managed span ids.
    pub free_large_span_ids: Arc<[u64]>,
    /// The number of allocated managed references.
    pub allocated_count: usize,
    /// The number of allocated managed bytes.
    pub allocated_bytes: u64,
    /// The captured gc state.
    pub gc_state: GcState,
    /// The captured dedicated large-span threshold in values.
    pub large_span_values: usize,
}

impl ManagedImage {
    /// Build one managed heap image from one managed heap snapshot.
    pub(crate) fn from_snapshot(snapshot: &ManagedHeapSnapshot) -> Self {
        Self {
            pages: Vector::from_values_by(
                &snapshot.pages,
                None,
                ManagedPageImage::shares_storage_with,
            ),
            value_pages: Vector::from_shared(
                &snapshot
                    .value_pages
                    .iter()
                    .cloned()
                    .map(Arc::new)
                    .collect::<Vec<_>>(),
                None,
            ),
            large_spans: Vector::from_shared(
                &snapshot
                    .large_spans
                    .iter()
                    .cloned()
                    .map(|values| Arc::from(values.into_boxed_slice()))
                    .collect::<Vec<_>>(),
                None,
            ),
            locations: Arc::from(snapshot.locations.as_slice()),
            next_unused_id: snapshot.next_unused_id,
            next_unused_slot: snapshot.next_unused_slot,
            next_unused_span_index: snapshot.next_unused_span_index,
            next_unused_large_span_id: snapshot.next_unused_large_span_id,
            free_ids: Arc::from(snapshot.free_ids.as_slice()),
            free_spans: Arc::from(snapshot.free_spans.as_slice()),
            free_large_span_ids: Arc::from(snapshot.free_large_span_ids.as_slice()),
            allocated_count: snapshot.allocated_count,
            allocated_bytes: snapshot.allocated_bytes,
            gc_state: snapshot.gc_state.clone(),
            large_span_values: snapshot.large_span_values,
        }
    }

    /// Flatten one managed heap image into one managed heap snapshot.
    pub(crate) fn snapshot(&self) -> ManagedHeapSnapshot {
        ManagedHeapSnapshot {
            pages: self.pages.iter().cloned().collect(),
            value_pages: self
                .value_pages
                .iter()
                .map(|page| (**page).clone())
                .collect(),
            large_spans: self
                .large_spans
                .iter()
                .map(|values| values.as_ref().to_vec())
                .collect(),
            locations: self.locations.iter().copied().collect(),
            next_unused_id: self.next_unused_id,
            next_unused_slot: self.next_unused_slot,
            next_unused_span_index: self.next_unused_span_index,
            next_unused_large_span_id: self.next_unused_large_span_id,
            free_ids: self.free_ids.iter().copied().collect(),
            free_spans: self.free_spans.iter().copied().collect(),
            free_large_span_ids: self.free_large_span_ids.iter().copied().collect(),
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            gc_state: self.gc_state.clone(),
            large_span_values: self.large_span_values,
        }
    }
}

impl ManagedHeap {
    /// Create one managed heap directly from one managed heap image.
    pub(crate) fn from_image(image: &ManagedImage) -> Self {
        let pages = image
            .pages
            .iter()
            .cloned()
            .map(ManagedPage::from_image)
            .collect::<Vec<_>>();
        let value_pages = image
            .value_pages
            .iter()
            .cloned()
            .map(ValuePage::from_image)
            .collect::<Vec<_>>();
        let large_spans = image
            .large_spans
            .iter()
            .cloned()
            .map(ManagedLargeSpan::from_image)
            .collect::<Vec<_>>();
        let locations = image.locations.iter().copied().collect();
        let free_ids = image.free_ids.iter().copied().collect();
        let free_spans = image.free_spans.iter().copied().collect();
        let free_large_span_ids = image.free_large_span_ids.iter().copied().collect();

        Self {
            pages,
            value_pages,
            large_spans,
            locations,
            free_ids,
            free_spans,
            free_large_span_ids,
            next_unused_id: image.next_unused_id,
            next_unused_slot: image.next_unused_slot,
            next_unused_span_index: image.next_unused_span_index,
            next_unused_large_span_id: image.next_unused_large_span_id,
            allocated_count: image.allocated_count,
            allocated_bytes: image.allocated_bytes,
            retained_bytes: 0,
            gc_state: image.gc_state.clone(),
            mark_queue: Vec::new(),
            large_span_values: image.large_span_values,
        }
        .with_retained_bytes()
    }

    /// Capture one immutable managed heap section.
    pub(crate) fn image(
        &mut self,
        base: Option<&ManagedImage>,
    ) -> Result<ManagedImage, HeapCaptureError> {
        // capture barrier: gc work must not be in flight
        if self.gc_state.phase != GcPhase::Idle || !self.mark_queue.is_empty() {
            return Err(HeapCaptureError::GcActive);
        }

        // capture barrier: externally observed managed addresses must be released
        if self.pages.iter().any(|page| page.active_pins() > 0) {
            return Err(HeapCaptureError::PinnedManagedReferences);
        }

        // capture page contents
        let pages = Vector::from_values_by(
            &self
                .pages
                .iter_mut()
                .map(ManagedPage::image)
                .collect::<Vec<_>>(),
            base.map(|image| &image.pages),
            ManagedPageImage::shares_storage_with,
        );
        let value_pages = Vector::from_shared(
            &self
                .value_pages
                .iter_mut()
                .map(ValuePage::image)
                .collect::<Vec<_>>(),
            base.map(|image| &image.value_pages),
        );
        let large_spans = Vector::from_shared(
            &self
                .large_spans
                .iter_mut()
                .map(ManagedLargeSpan::image)
                .collect::<Vec<_>>(),
            base.map(|image| &image.large_spans),
        );
        let locations = Arc::from(self.locations.as_slice());

        // capture allocator and gc state
        let free_ids = Arc::from(self.free_ids.as_slice());
        let free_spans = Arc::from(self.free_spans.as_slice());
        let free_large_span_ids = Arc::from(self.free_large_span_ids.as_slice());

        Ok(ManagedImage {
            pages,
            value_pages,
            large_spans,
            locations,
            next_unused_id: self.next_unused_id,
            next_unused_slot: self.next_unused_slot,
            next_unused_span_index: self.next_unused_span_index,
            next_unused_large_span_id: self.next_unused_large_span_id,
            free_ids,
            free_spans,
            free_large_span_ids,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            gc_state: self.gc_state.clone(),
            large_span_values: self.large_span_values,
        })
    }
}
