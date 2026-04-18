use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Heap;
use crate::arena::{Arena, ArenaImage, PageId};
use crate::core::sum_bytes;
use crate::local::managed::{
    GcState, ManagedLocation, ManagedSpace, ManagedSpaceImage, live_page_views,
};
use crate::local::raw::{RawLocation, RawSpace, RawSpaceImage};
use crate::value::{ManagedReference, RawPointer};
use crate::{HeapError, HeapLimits, HeapOptions, HeapResult};

/// One frozen heap root over one shared arena.
#[derive(Debug, Clone)]
pub struct HeapImage {
    /// The shared arena backing every captured page.
    arena: Arc<Arena>,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The captured managed-space root.
    managed: ManagedSpaceImage,
    /// The captured raw-space root.
    raw: RawSpaceImage,
}

/// One serialized heap snapshot payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HeapSnapshot {
    /// The serialized arena pages reachable from this heap root.
    arena: ArenaImage,

    /// The heap options used by this image.
    options: HeapOptions,

    /// The serialized managed-space root.
    managed: ManagedSpaceImage,
    /// The serialized raw-space root.
    raw: RawSpaceImage,
}

impl HeapImage {
    /// Create one frozen heap root.
    pub(crate) fn new(
        arena: Arc<Arena>,
        options: HeapOptions,
        managed: ManagedSpaceImage,
        raw: RawSpaceImage,
    ) -> Self {
        Self {
            arena,
            options,
            managed,
            raw,
        }
    }

    /// Build one heap image from one serialized payload.
    fn from_snapshot(snapshot: &HeapSnapshot) -> Result<Self, HeapError> {
        let arena = Arc::new(Arena::from_image(&snapshot.arena)?);
        let managed = snapshot.managed.clone();
        let raw = snapshot.raw.clone();

        Ok(Self {
            arena,
            options: snapshot.options.clone(),
            managed,
            raw,
        })
    }

    /// Flatten one heap image into one serialized snapshot.
    fn snapshot(&self) -> Result<HeapSnapshot, HeapError> {
        // collect the reachable arena pages once
        let page_ids = self.image_page_ids();

        Ok(HeapSnapshot {
            arena: self.arena.image_pages_from_ids(&page_ids)?,
            options: self.options.clone(),
            managed: self.managed.clone(),
            raw: self.raw.clone(),
        })
    }

    /// Return the shared arena for this image.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the heap options for this image.
    pub fn options(&self) -> &HeapOptions {
        &self.options
    }

    /// Return the managed-space root.
    pub(crate) fn managed(&self) -> &ManagedSpaceImage {
        &self.managed
    }

    /// Return the raw-space root.
    pub(crate) fn raw(&self) -> &RawSpaceImage {
        &self.raw
    }

    /// Return the captured collector state.
    pub fn gc_state(&self) -> &GcState {
        self.managed.gc_state()
    }

    /// Return the total page count reachable from this heap image.
    pub fn page_count(&self) -> usize {
        self.image_page_ids().len()
    }

    /// Return the raw page count reachable from this heap image.
    pub fn raw_page_count(&self) -> usize {
        self.raw
            .spans()
            .iter()
            .flat_map(|span| span.pages.page_ids())
            .chain(
                self.raw
                    .entries()
                    .iter()
                    .flat_map(|entry| entry.pages.page_ids()),
            )
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Return the total local allocated bytes captured by this image.
    pub fn local_allocated_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.managed.allocated_bytes(), self.raw.allocated_bytes())
    }

    /// Return whether one managed entry shares arena storage with another heap root.
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
        let (Some(location), Some(other_location)) = (record.location(), other_record.location())
        else {
            return false;
        };

        match (location, other_location) {
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
            (ManagedLocation::Large(entry), ManagedLocation::Large(other_entry)) => {
                let Ok(entry_index) = entry.index() else {
                    return false;
                };
                let Ok(other_entry_index) = other_entry.index() else {
                    return false;
                };
                let Some(entry) = self.managed.entries().get(entry_index) else {
                    return false;
                };
                let Some(other_entry) = other.managed.entries().get(other_entry_index) else {
                    return false;
                };

                entry.pages == other_entry.pages
            }
            _ => false,
        }
    }

    /// Return whether one raw entry shares arena storage with another heap root.
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
        let (Some(location), Some(other_location)) = (record.location(), other_record.location())
        else {
            return false;
        };

        match (location, other_location) {
            (RawLocation::Small(slot), RawLocation::Small(other_slot)) => {
                let Some(span) = self.raw.spans().get(slot.span_index()) else {
                    return false;
                };
                let Some(other_span) = other.raw.spans().get(other_slot.span_index()) else {
                    return false;
                };

                slot == other_slot && span.pages == other_span.pages
            }
            (RawLocation::Large(entry), RawLocation::Large(other_entry)) => {
                let Ok(entry_index) = entry.index() else {
                    return false;
                };
                let Ok(other_entry_index) = other_entry.index() else {
                    return false;
                };
                let Some(entry) = self.raw.entries().get(entry_index) else {
                    return false;
                };
                let Some(other_entry) = other.raw.entries().get(other_entry_index) else {
                    return false;
                };

                entry.pages == other_entry.pages
            }
            _ => false,
        }
    }

    /// Return every arena page reachable from this heap image.
    pub fn page_ids(&self) -> Vec<PageId> {
        let mut pages = Vec::new();

        // collect the young-space pages first
        pages.extend(self.managed.young().pages().page_ids());

        // collect every managed span and entry page
        for span in self.managed.spans() {
            pages.extend(span.pages.page_ids());
        }

        for entry in self.managed.entries() {
            pages.extend(entry.pages.page_ids());
        }

        // collect every raw span and entry page
        for span in self.raw.spans() {
            pages.extend(span.pages.page_ids());
        }

        for entry in self.raw.entries() {
            pages.extend(entry.pages.page_ids());
        }

        pages
    }

    /// Return the deduplicated page ids for one serialized image.
    fn image_page_ids(&self) -> Vec<PageId> {
        self.page_ids()
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Return whether two heap images expose the same reachable arena pages.
    fn has_equal_page_bytes(&self, other: &Self) -> bool {
        let page_ids = self.image_page_ids();
        let other_page_ids = other.image_page_ids();
        if page_ids != other_page_ids {
            return false;
        }

        // compare each reachable page directly without snapshot materialization
        for page_id in page_ids {
            let Ok(page) = self.arena.read_page_bytes(page_id) else {
                return false;
            };
            let Ok(other_page) = other.arena.read_page_bytes(page_id) else {
                return false;
            };

            if page != other_page {
                return false;
            }
        }

        true
    }

    /// Return one pair of live managed records captured by two heap images.
    fn managed_record_pair<'a>(
        &'a self,
        other: &'a Self,
        reference: ManagedReference,
    ) -> Option<(
        &'a crate::local::managed::ManagedReferenceEntry,
        &'a crate::local::managed::ManagedReferenceEntry,
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
        &'a crate::local::raw::RawPointerEntry,
        &'a crate::local::raw::RawPointerEntry,
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
    /// Fork one live heap over the same shared arena.
    pub fn fork(&self) -> Result<Self, HeapError> {
        let managed = self.managed.fork()?;
        let raw = match self.raw.fork() {
            Ok(raw) => raw,
            Err(error) => {
                for page_view in live_page_views(&managed).into_iter().rev() {
                    self.arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }
        };

        Ok(Self {
            arena: self.arena.clone(),
            options: self.options.clone(),
            managed,
            raw,
            limits: self.limits,
        })
    }

    /// Create one heap from one frozen heap root.
    pub fn from_image(image: &HeapImage) -> Result<Self, HeapError> {
        Self::from_image_with_limits(image, HeapLimits::default())
    }

    /// Create one heap from one frozen heap root and explicit hard limits.
    pub fn from_image_with_limits(
        image: &HeapImage,
        limits: HeapLimits,
    ) -> Result<Self, HeapError> {
        image.options().validate()?;

        let managed = ManagedSpace::from_image(image.arena().clone(), image.managed())?;
        let raw = match RawSpace::from_image(image.arena().clone(), image.raw()) {
            Ok(raw) => raw,
            Err(error) => {
                for page_view in live_page_views(&managed).into_iter().rev() {
                    image.arena().release_page_view(&page_view)?;
                }

                return Err(error);
            }
        };

        Ok(Self {
            arena: image.arena().clone(),
            options: image.options().clone(),
            managed,
            raw,
            limits,
        })
    }

    /// Capture one frozen heap root.
    pub fn image(&self) -> Result<HeapImage, HeapError> {
        let managed = self.managed.image()?;
        let raw = self.raw.image();

        Ok(HeapImage::new(
            self.arena().clone(),
            self.options().clone(),
            managed,
            raw,
        ))
    }

    /// Restore this heap from one frozen heap root.
    pub fn restore_image(&mut self, image: &HeapImage) -> Result<(), HeapError> {
        self.managed.check_branch_boundary()?;

        let limits = self.limits;
        *self = Self::from_image_with_limits(image, limits)?;

        Ok(())
    }
}

impl Serialize for HeapImage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let snapshot = self.snapshot().map_err(serde::ser::Error::custom)?;

        snapshot.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HeapImage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let snapshot = HeapSnapshot::deserialize(deserializer)?;

        Self::from_snapshot(&snapshot).map_err(serde::de::Error::custom)
    }
}

impl PartialEq for HeapImage {
    fn eq(&self, other: &Self) -> bool {
        self.options == other.options
            && self.managed == other.managed
            && self.raw == other.raw
            && self.has_equal_page_bytes(other)
    }
}

impl Eq for HeapImage {}
