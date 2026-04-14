use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Heap;
use crate::alloc::{Arena, ArenaSnapshot, PageId};
use crate::managed::{
    GcState, ManagedLocation, ManagedSpace, ManagedSpaceImage, ManagedSpaceSnapshot,
};
use crate::raw::{RawLocation, RawSpace, RawSpaceImage, RawSpaceSnapshot};
use crate::value::{ManagedReference, RawPointer};
use crate::{HeapLayout, HeapLayoutError};

/// One frozen heap root over one shared arena.
#[derive(Debug, Clone)]
pub struct HeapImage {
    /// The shared arena backing every captured page.
    arena: Arc<Arena>,
    /// The heap layout used by this image.
    layout: HeapLayout,
    /// The captured managed-space root.
    managed: ManagedSpaceImage,
    /// The captured raw-space root.
    raw: RawSpaceImage,
}

/// One serialized heap snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapSnapshot {
    /// The serialized arena pages reachable from this heap root.
    pub arena: ArenaSnapshot,
    /// The heap layout used by this snapshot.
    pub layout: HeapLayout,
    /// The serialized managed-space root.
    pub(crate) managed: ManagedSpaceSnapshot,
    /// The serialized raw-space root.
    pub(crate) raw: RawSpaceSnapshot,
}

impl HeapImage {
    /// Create one frozen heap root.
    pub(crate) fn new(
        arena: Arc<Arena>,
        layout: HeapLayout,
        managed: ManagedSpaceImage,
        raw: RawSpaceImage,
    ) -> Self {
        Self {
            arena,
            layout,
            managed,
            raw,
        }
    }

    /// Build one heap image from one serialized snapshot.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Self {
        let arena = Arc::new(Arena::from_snapshot(&snapshot.arena));
        let managed = ManagedSpaceImage::from_snapshot(&snapshot.managed);
        let raw = RawSpaceImage::from_snapshot(&snapshot.raw);

        Self {
            arena,
            layout: snapshot.layout.clone(),
            managed,
            raw,
        }
    }

    /// Flatten one heap image into one serialized snapshot.
    pub fn snapshot(&self) -> HeapSnapshot {
        // collect the reachable arena pages once
        let page_ids = self.snapshot_page_ids();

        HeapSnapshot {
            arena: self.arena.snapshot_pages_from_ids(&page_ids),
            layout: self.layout.clone(),
            managed: self.managed.snapshot(),
            raw: self.raw.snapshot(),
        }
    }

    /// Return the shared arena for this image.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return a copy of this image rebound onto one explicit arena.
    pub fn with_arena(&self, arena: Arc<Arena>) -> Self {
        Self {
            arena,
            layout: self.layout.clone(),
            managed: self.managed.clone(),
            raw: self.raw.clone(),
        }
    }

    /// Return the heap layout for this image.
    pub fn layout(&self) -> &HeapLayout {
        &self.layout
    }

    /// Return the managed-space root.
    pub(crate) fn managed(&self) -> &ManagedSpaceImage {
        &self.managed
    }

    /// Return the raw-space root.
    pub(crate) fn raw(&self) -> &RawSpaceImage {
        &self.raw
    }

    /// Return the captured managed collector state.
    pub fn managed_gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Return the total page count reachable from this heap image.
    pub fn page_count(&self) -> usize {
        let young_pages = self.managed.young().pages().len();
        let managed_span_pages =
            Self::page_len_sum(self.managed.spans().iter().map(|span| &span.pages));
        let managed_allocation_pages = Self::page_len_sum(
            self.managed
                .allocations()
                .iter()
                .map(|allocation| &allocation.pages),
        );
        let raw_span_pages = Self::page_len_sum(self.raw.spans().iter().map(|span| &span.pages));
        let raw_allocation_pages = Self::page_len_sum(
            self.raw
                .allocations()
                .iter()
                .map(|allocation| &allocation.pages),
        );

        young_pages
            + managed_span_pages
            + managed_allocation_pages
            + raw_span_pages
            + raw_allocation_pages
    }

    /// Return the raw page count reachable from this heap image.
    pub fn raw_page_count(&self) -> usize {
        let raw_span_pages = Self::page_len_sum(self.raw.spans().iter().map(|span| &span.pages));
        let raw_allocation_pages = Self::page_len_sum(
            self.raw
                .allocations()
                .iter()
                .map(|allocation| &allocation.pages),
        );

        raw_span_pages + raw_allocation_pages
    }

    /// Return the total local allocated bytes captured by this image.
    pub fn local_allocated_bytes(&self) -> u64 {
        self.managed.allocated_bytes() + self.raw.allocated_bytes()
    }

    /// Return whether one managed allocation shares arena storage with another heap root.
    #[doc(hidden)]
    pub fn shares_managed_allocation_with(
        &self,
        other: &Self,
        reference: ManagedReference,
    ) -> bool {
        // resolve the captured managed records first
        let Some((record, other_record)) = self.managed_record_pair(other, reference) else {
            return false;
        };

        // sharing only makes sense inside one shared arena
        if !Arc::ptr_eq(&self.arena, &other.arena) {
            return false;
        }

        // compare the location-specific page maps
        match (record.location(), other_record.location()) {
            (ManagedLocation::Vacant, ManagedLocation::Vacant) => false,
            (ManagedLocation::Young(_), ManagedLocation::Young(_)) => {
                self.managed.young().pages() == other.managed.young().pages()
            }
            (ManagedLocation::Small(slot), ManagedLocation::Small(other_slot)) => {
                let Some(span) = self.managed.spans().get(slot.span_index()) else {
                    return false;
                };
                let Some(other_span) = other.managed.spans().get(other_slot.span_index()) else {
                    return false;
                };

                slot == other_slot && span.pages == other_span.pages
            }
            (ManagedLocation::Large(allocation), ManagedLocation::Large(other_allocation)) => {
                let Some(allocation) = self.managed.allocations().get(allocation.index()) else {
                    return false;
                };
                let Some(other_allocation) =
                    other.managed.allocations().get(other_allocation.index())
                else {
                    return false;
                };

                allocation.pages == other_allocation.pages
            }
            _ => false,
        }
    }

    /// Return whether one raw allocation shares arena storage with another heap root.
    #[doc(hidden)]
    pub fn shares_raw_allocation_with(&self, other: &Self, pointer: RawPointer) -> bool {
        // resolve the captured raw records first
        let Some((record, other_record)) = self.raw_record_pair(other, pointer) else {
            return false;
        };

        // sharing only makes sense inside one shared arena
        if !Arc::ptr_eq(&self.arena, &other.arena) {
            return false;
        }

        // compare the location-specific page maps
        match (record.location, other_record.location) {
            (RawLocation::Vacant, RawLocation::Vacant) => false,
            (RawLocation::Small(slot), RawLocation::Small(other_slot)) => {
                let Some(span) = self.raw.spans().get(slot.span_index()) else {
                    return false;
                };
                let Some(other_span) = other.raw.spans().get(other_slot.span_index()) else {
                    return false;
                };

                slot == other_slot && span.pages == other_span.pages
            }
            (RawLocation::Large(allocation), RawLocation::Large(other_allocation)) => {
                let Some(allocation) = self.raw.allocations().get(allocation.index()) else {
                    return false;
                };
                let Some(other_allocation) = other.raw.allocations().get(other_allocation.index())
                else {
                    return false;
                };

                allocation.pages == other_allocation.pages
            }
            _ => false,
        }
    }

    /// Return every arena page reachable from this heap image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // collect the nursery pages first
        pages.extend(self.managed.young().pages().page_ids());

        // collect every managed span and allocation page
        for span in self.managed.spans() {
            pages.extend(span.pages.page_ids());
        }

        for allocation in self.managed.allocations() {
            pages.extend(allocation.pages.page_ids());
        }

        // collect every raw span and allocation page
        for span in self.raw.spans() {
            pages.extend(span.pages.page_ids());
        }

        for allocation in self.raw.allocations() {
            pages.extend(allocation.pages.page_ids());
        }

        pages
    }

    /// Return the deduplicated page ids for one serialized snapshot.
    fn snapshot_page_ids(&self) -> Vec<PageId> {
        self.page_ids()
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Sum the page counts for one iterator of page maps.
    fn page_len_sum<'a>(page_maps: impl Iterator<Item = &'a crate::alloc::PageMap>) -> usize {
        page_maps.map(crate::alloc::PageMap::len).sum()
    }

    /// Return one pair of live managed records captured by two heap images.
    fn managed_record_pair<'a>(
        &'a self,
        other: &'a Self,
        reference: ManagedReference,
    ) -> Option<(
        &'a crate::managed::ManagedReferenceRecord,
        &'a crate::managed::ManagedReferenceRecord,
    )> {
        // resolve the stable reference slot first
        let reference_index = reference.id().checked_sub(1)? as usize;
        let record = self.managed.references().get(reference_index)?;
        let other_record = other.managed.references().get(reference_index)?;

        // skip vacant captured records
        if record.is_vacant() || other_record.is_vacant() {
            return None;
        }

        Some((record, other_record))
    }

    /// Return one pair of live raw records captured by two heap images.
    fn raw_record_pair<'a>(
        &'a self,
        other: &'a Self,
        pointer: RawPointer,
    ) -> Option<(
        &'a crate::raw::RawPointerRecord,
        &'a crate::raw::RawPointerRecord,
    )> {
        // resolve the stable pointer slot first
        let pointer_index = pointer.id().checked_sub(1)? as usize;
        let record = self.raw.pointers().get(pointer_index)?;
        let other_record = other.raw.pointers().get(pointer_index)?;

        // skip vacant captured records
        if record.is_vacant() || other_record.is_vacant() {
            return None;
        }

        Some((record, other_record))
    }
}

impl Heap {
    /// Create one heap from one frozen heap root.
    pub fn from_image(image: &HeapImage) -> Result<Self, HeapLayoutError> {
        Ok(Self {
            arena: image.arena().clone(),
            layout: image.layout().clone(),
            managed: ManagedSpace::from_image(image.arena().clone(), image.managed())?,
            raw: RawSpace::from_image(image.arena().clone(), image.raw()),
            limits: super::HeapLimits::default(),
        })
    }

    /// Create one heap from one serialized snapshot.
    pub fn from_snapshot(snapshot: &HeapSnapshot) -> Result<Self, HeapLayoutError> {
        Self::from_image(&HeapImage::from_snapshot(snapshot))
    }

    /// Capture one frozen heap root.
    pub fn image(&mut self) -> Result<HeapImage, HeapCaptureError> {
        let managed = self.managed.image()?;
        let raw = self.raw.image();

        Ok(HeapImage::new(
            self.arena().clone(),
            self.layout().clone(),
            managed,
            raw,
        ))
    }

    /// Restore this heap from one frozen heap root.
    pub fn restore_image(&mut self, image: &HeapImage) -> Result<(), HeapLayoutError> {
        *self = Self::from_image(image)?;

        Ok(())
    }

    /// Capture one serialized heap snapshot.
    pub fn snapshot(&mut self) -> Result<HeapSnapshot, HeapCaptureError> {
        Ok(self.image()?.snapshot())
    }

    /// Restore this heap from one serialized snapshot.
    pub fn restore_snapshot(&mut self, snapshot: &HeapSnapshot) -> Result<(), HeapLayoutError> {
        *self = Self::from_snapshot(snapshot)?;

        Ok(())
    }
}

impl Serialize for HeapImage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.snapshot().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HeapImage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let snapshot = HeapSnapshot::deserialize(deserializer)?;

        Ok(Self::from_snapshot(&snapshot))
    }
}

/// Heap image capture failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapCaptureError {
    /// The managed collector still has in-flight work.
    GcActive,
    /// One managed allocation is still pinned for raw exposure.
    PinnedManagedReferences,
}

impl fmt::Display for HeapCaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GcActive => write!(f, "heap capture requires idle gc state"),
            Self::PinnedManagedReferences => {
                write!(f, "heap capture requires all managed pins to be released")
            }
        }
    }
}

impl Error for HeapCaptureError {}
