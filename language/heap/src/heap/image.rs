use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::Value;
use crate::managed::ManagedImage;
use crate::page::{ManagedPageImage, RawPageImage, ValuePageImage};
use crate::raw::RawImage;

/// Immutable in-memory heap image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapImage {
    /// Captured managed heap state.
    pub(crate) managed: ManagedImage,
    /// Captured raw heap state.
    pub(crate) raw: RawImage,
}

impl HeapImage {
    /// Build one heap image from one serialized snapshot.
    pub(crate) fn from_snapshot(snapshot: &HeapSnapshot) -> Self {
        Self {
            managed: ManagedImage::from_snapshot(&snapshot.managed),
            raw: RawImage::from_snapshot(&snapshot.raw),
        }
    }

    /// Flatten one heap image into one serialized snapshot.
    pub(crate) fn snapshot(&self) -> HeapSnapshot {
        HeapSnapshot {
            managed: self.managed.snapshot(),
            raw: self.raw.snapshot(),
        }
    }

    /// Return one managed page image by index.
    pub fn managed_page(&self, index: usize) -> Option<&ManagedPageImage> {
        self.managed.pages.get(index)
    }

    /// Return one managed value page image by index.
    pub fn managed_value_page(&self, index: usize) -> Option<&Arc<ValuePageImage>> {
        self.managed.value_pages.get(index)
    }

    /// Return one dedicated large managed span by index.
    pub fn managed_large_span(&self, index: usize) -> Option<&Arc<[Value]>> {
        self.managed.large_spans.get(index)
    }

    /// Return one raw page image by index.
    pub fn raw_page(&self, index: usize) -> Option<&Arc<RawPageImage>> {
        self.raw.pages.get(index)
    }

    /// Return the bytes for one raw span image by index.
    pub fn raw_span_bytes(&self, index: usize) -> Option<&[u8]> {
        Some(self.raw.spans.get(index)?.as_ref())
    }

    /// Return the bytes for one dedicated large raw span by index.
    pub fn raw_large_span_bytes(&self, index: usize) -> Option<&[u8]> {
        Some(self.raw.large_spans.get(index)?.as_ref())
    }

    /// Return the dedicated managed large-span threshold in values.
    pub fn managed_large_span_value_threshold(&self) -> usize {
        self.managed.large_span_values
    }

    /// Return the dedicated raw large-span threshold in bytes.
    pub fn raw_large_span_byte_threshold(&self) -> usize {
        self.raw.large_span_bytes
    }

    /// Return the captured managed gc state.
    pub fn managed_gc_state(&self) -> &crate::GcState {
        &self.managed.gc_state
    }

    /// Return the number of allocated managed bytes.
    pub fn managed_allocated_bytes(&self) -> u64 {
        self.managed.allocated_bytes
    }

    /// Return the number of allocated raw bytes.
    pub fn raw_allocated_bytes(&self) -> u64 {
        self.raw.allocated_bytes
    }

    /// Report whether one raw span image shares durable backing with another heap image.
    pub fn raw_span_shares_with(&self, other: &Self, index: usize) -> bool {
        let Some(left) = self.raw.spans.get(index) else {
            return false;
        };
        let Some(right) = other.raw.spans.get(index) else {
            return false;
        };

        Arc::ptr_eq(left, right)
    }

    /// Return the number of immutable managed leaves in this image.
    pub fn managed_leaf_count(&self) -> usize {
        self.managed.pages.len() + self.managed.value_pages.len() + self.managed.large_spans.len()
    }

    /// Return the total raw payload bytes captured in span-backed storage.
    pub fn raw_payload_bytes(&self) -> u64 {
        self.raw
            .spans
            .iter()
            .chain(self.raw.large_spans.iter())
            .map(|bytes| bytes.len() as u64)
            .sum()
    }

    /// Return the number of immutable raw leaves in this image.
    pub fn raw_leaf_count(&self) -> usize {
        self.raw.pages.len() + self.raw.spans.len() + self.raw.large_spans.len()
    }

    /// Return the total captured allocation bytes in this heap image.
    pub fn heap_bytes(&self) -> u64 {
        self.managed
            .allocated_bytes
            .saturating_add(self.raw.allocated_bytes)
    }

    /// Return the number of immutable leaf images in this heap image.
    pub fn leaf_count(&self) -> usize {
        self.managed_leaf_count() + self.raw_leaf_count()
    }
}

/// Serialized managed heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedHeapSnapshot {
    /// Flattened managed anchor pages.
    pub pages: Vec<ManagedPageImage>,
    /// Flattened managed value pages.
    pub value_pages: Vec<ValuePageImage>,
    /// Flattened dedicated large managed spans.
    pub large_spans: Vec<Vec<Value>>,
    /// Flattened managed locations.
    pub locations: Vec<crate::page::PageSlot>,
    /// The next managed reference id to allocate.
    pub next_unused_id: u64,
    /// The next managed page slot to allocate.
    pub next_unused_slot: u64,
    /// The next managed span index to allocate.
    pub next_unused_span_index: u64,
    /// The next large managed span id to allocate.
    pub next_unused_large_span_id: u64,
    /// Flattened managed free ids.
    pub free_ids: Vec<u64>,
    /// Flattened managed free spans.
    pub free_spans: Vec<crate::ManagedSpan>,
    /// Flattened free large managed span ids.
    pub free_large_span_ids: Vec<u64>,
    /// The number of allocated managed references.
    pub allocated_count: usize,
    /// The number of allocated managed bytes.
    pub allocated_bytes: u64,
    /// The captured managed GC state.
    pub gc_state: crate::GcState,
    /// The captured dedicated managed large-span threshold in values.
    pub large_span_values: usize,
}

/// Serialized raw heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawHeapSnapshot {
    /// Flattened raw value pages.
    pub pages: Vec<RawPageImage>,
    /// Flattened raw spans.
    pub spans: Vec<Vec<u8>>,
    /// Flattened dedicated large raw spans.
    pub large_spans: Vec<Vec<u8>>,
    /// Flattened raw locations.
    pub locations: Vec<crate::page::PageSlot>,
    /// The next raw allocation id to allocate.
    pub next_unused_id: u64,
    /// The next raw span id to allocate.
    pub next_unused_span_id: u64,
    /// The next large raw span id to allocate.
    pub next_unused_large_span_id: u64,
    /// The next raw page slot to allocate.
    pub next_unused_slot: u64,
    /// Flattened raw free ids.
    pub free_ids: Vec<u64>,
    /// Flattened raw free span ids.
    pub free_span_ids: Vec<u64>,
    /// Flattened raw free large span ids.
    pub free_large_span_ids: Vec<u64>,
    /// The number of allocated raw allocations.
    pub allocated_count: usize,
    /// The number of allocated raw bytes.
    pub allocated_bytes: u64,
    /// The captured dedicated raw large-span threshold in bytes.
    pub large_span_bytes: usize,
}

/// Serialized heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// Flattened managed heap image.
    pub managed: ManagedHeapSnapshot,
    /// Flattened raw heap image.
    pub raw: RawHeapSnapshot,
}
