use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::super::DynamicBitmap;

/// One immutable raw run image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRunImage {
    /// The size class for this run in bytes.
    pub size_class: usize,
    /// The number of slots in this run.
    pub slot_count: usize,
    /// The packed slot payload bytes.
    pub bytes: Arc<[u8]>,
    /// The occupied slots in this run.
    pub occupied: DynamicBitmap,
    /// The logical byte length for each slot.
    pub lengths: Arc<[u16]>,
}

impl RawRunImage {
    /// Report whether this run image is one vacant directory slot.
    pub(crate) fn is_vacant(&self) -> bool {
        self.slot_count == 0
    }

    /// Report whether this run image shares durable backing with another run image.
    pub(crate) fn shares_storage_with(&self, other: &Self) -> bool {
        self.size_class == other.size_class
            && self.slot_count == other.slot_count
            && Arc::ptr_eq(&self.bytes, &other.bytes)
            && self.occupied == other.occupied
            && Arc::ptr_eq(&self.lengths, &other.lengths)
    }
}

/// One owned raw run payload.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawRunOwned {
    /// The packed slot payload bytes.
    bytes: Box<[u8]>,
    /// The occupied slots in this run.
    occupied: DynamicBitmap,
    /// The logical byte length for each slot.
    lengths: Box<[u16]>,
}

/// One live raw run storage state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RawRunStorage {
    /// Owned mutable run storage.
    Owned(RawRunOwned),
    /// Shared immutable run storage.
    Shared(RawRunImage),
}

/// One live raw run for one size class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawRun {
    /// The slot payload size in bytes.
    size_class: usize,
    /// The number of slots in this run.
    slot_count: usize,
    /// The number of occupied slots in this run.
    occupied_count: usize,
    /// The next likely free slot.
    next_free_slot: usize,
    /// The live storage for this run.
    storage: RawRunStorage,
}

impl RawRun {
    /// Create one vacant raw run slot with no backing storage.
    pub(crate) fn vacant() -> Self {
        Self {
            size_class: 0,
            slot_count: 0,
            occupied_count: 0,
            next_free_slot: 0,
            storage: RawRunStorage::Owned(RawRunOwned {
                bytes: Vec::new().into_boxed_slice(),
                occupied: DynamicBitmap::with_capacity(0),
                lengths: Vec::new().into_boxed_slice(),
            }),
        }
    }

    /// Create one empty raw run for the given size class and run byte width.
    pub(crate) fn new(size_class: usize, run_bytes: usize) -> Self {
        let slot_count = (run_bytes / size_class).max(1);
        let bytes = vec![0; slot_count * size_class].into_boxed_slice();
        let lengths = vec![0; slot_count].into_boxed_slice();

        Self {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            storage: RawRunStorage::Owned(RawRunOwned {
                bytes,
                occupied: DynamicBitmap::with_capacity(slot_count),
                lengths,
            }),
        }
    }

    /// Restore one raw run from one immutable image.
    pub(crate) fn from_image(image: RawRunImage) -> Self {
        if image.is_vacant() {
            return Self::vacant();
        }

        let occupied_count = image.occupied.count_ones();
        let slot_count = image.slot_count;
        let next_free_slot = image.occupied.first_clear_from(0).unwrap_or(slot_count);

        Self {
            size_class: image.size_class,
            slot_count,
            occupied_count,
            next_free_slot,
            storage: RawRunStorage::Shared(image),
        }
    }

    /// Report whether this run is one vacant directory slot.
    pub(crate) fn is_vacant(&self) -> bool {
        self.slot_count == 0
    }

    /// Report whether this run has no live allocations.
    pub(crate) fn is_empty(&self) -> bool {
        self.occupied_count == 0
    }

    /// Report whether this run has at least one free slot.
    pub(crate) fn has_free_slot(&self) -> bool {
        !self.is_vacant() && self.occupied_count < self.slot_count
    }

    /// Return the size class for this run in bytes.
    pub(crate) fn size_class(&self) -> usize {
        self.size_class
    }

    /// Return whether one slot is occupied.
    pub(crate) fn is_occupied(&self, slot: usize) -> bool {
        self.occupied().contains(slot)
    }

    /// Return the first free slot in this run.
    pub(crate) fn first_free_slot(&self) -> Option<usize> {
        if !self.has_free_slot() {
            return None;
        }

        let occupied = self.occupied();
        occupied
            .first_clear_from(self.next_free_slot)
            .or_else(|| occupied.first_clear_from(0))
    }

    /// Return the logical byte length for one occupied slot.
    pub(crate) fn len(&self, slot: usize) -> Option<usize> {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return None;
        }

        match &self.storage {
            RawRunStorage::Owned(storage) => storage.lengths.get(slot).copied(),
            RawRunStorage::Shared(image) => image.lengths.get(slot).copied(),
        }
        .map(|len| len as usize)
    }

    /// Return the bytes for one occupied slot.
    pub(crate) fn bytes(&self, slot: usize) -> Option<&[u8]> {
        let len = self.len(slot)?;
        let start = slot.checked_mul(self.size_class)?;
        let end = start.checked_add(len)?;

        match &self.storage {
            RawRunStorage::Owned(storage) => storage.bytes.get(start..end),
            RawRunStorage::Shared(image) => image.bytes.get(start..end),
        }
    }

    /// Allocate one free slot in this run.
    pub(crate) fn allocate_slot(&mut self, slot: usize, bytes: &[u8]) -> bool {
        if slot >= self.slot_count || bytes.len() > self.size_class || self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.storage_mut();
        let start = slot * size_class;
        let end = start + size_class;

        storage.bytes[start..end].fill(0);
        storage.bytes[start..start + bytes.len()].copy_from_slice(bytes);
        storage.lengths[slot] = bytes.len() as u16;
        storage.occupied.set(slot);
        self.occupied_count += 1;
        self.next_free_slot = self
            .occupied()
            .first_clear_from(slot.saturating_add(1))
            .or_else(|| self.occupied().first_clear_from(0))
            .unwrap_or(self.slot_count);
        true
    }

    /// Replace one occupied slot.
    pub(crate) fn replace_slot(&mut self, slot: usize, bytes: &[u8]) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) || bytes.len() > self.size_class {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.storage_mut();
        let start = slot * size_class;
        let end = start + size_class;

        storage.bytes[start..end].fill(0);
        storage.bytes[start..start + bytes.len()].copy_from_slice(bytes);
        storage.lengths[slot] = bytes.len() as u16;
        true
    }

    /// Write one byte inside one occupied slot.
    pub(crate) fn set_byte(&mut self, slot: usize, offset: usize, byte: u8) -> bool {
        let Some(len) = self.len(slot) else {
            return false;
        };

        if offset >= len {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.storage_mut();
        let index = slot * size_class + offset;
        storage.bytes[index] = byte;
        true
    }

    /// Free one occupied slot.
    pub(crate) fn free_slot(&mut self, slot: usize) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        let storage = self.storage_mut();
        let start = slot * size_class;
        let end = start + size_class;

        storage.bytes[start..end].fill(0);
        storage.lengths[slot] = 0;
        storage.occupied.clear(slot);
        self.occupied_count = self.occupied_count.saturating_sub(1);

        if self.occupied_count == 0 {
            self.next_free_slot = 0;
        } else {
            self.next_free_slot = self.next_free_slot.min(slot);
        }

        true
    }

    /// Capture one immutable run image.
    pub(crate) fn image(&mut self) -> RawRunImage {
        if self.is_vacant() {
            return RawRunImage {
                size_class: 0,
                slot_count: 0,
                bytes: Arc::from(Vec::<u8>::new().into_boxed_slice()),
                occupied: DynamicBitmap::with_capacity(0),
                lengths: Arc::from(Vec::<u16>::new().into_boxed_slice()),
            };
        }

        match &mut self.storage {
            RawRunStorage::Shared(image) => image.clone(),
            RawRunStorage::Owned(storage) => {
                let image = RawRunImage {
                    size_class: self.size_class,
                    slot_count: self.slot_count,
                    bytes: Arc::from(std::mem::take(&mut storage.bytes)),
                    occupied: storage.occupied.clone(),
                    lengths: Arc::from(std::mem::take(&mut storage.lengths)),
                };
                self.storage = RawRunStorage::Shared(image.clone());
                image
            }
        }
    }

    /// Return the retained bytes for this run.
    pub(crate) fn retained_bytes(&self) -> usize {
        match &self.storage {
            RawRunStorage::Owned(storage) => {
                storage.bytes.len()
                    + storage.lengths.len() * std::mem::size_of::<u16>()
                    + storage.occupied.retained_bytes()
            }
            RawRunStorage::Shared(image) => {
                image.bytes.len()
                    + image.lengths.len() * std::mem::size_of::<u16>()
                    + image.occupied.retained_bytes()
            }
        }
    }

    /// Return the occupied bitmap for this run.
    fn occupied(&self) -> &DynamicBitmap {
        match &self.storage {
            RawRunStorage::Owned(storage) => &storage.occupied,
            RawRunStorage::Shared(image) => &image.occupied,
        }
    }

    /// Return mutable owned run storage, detaching images on first write.
    fn storage_mut(&mut self) -> &mut RawRunOwned {
        if let RawRunStorage::Shared(image) = &self.storage {
            self.storage = RawRunStorage::Owned(RawRunOwned {
                bytes: image.bytes.as_ref().to_vec().into_boxed_slice(),
                occupied: image.occupied.clone(),
                lengths: image.lengths.as_ref().to_vec().into_boxed_slice(),
            });
        }

        match &mut self.storage {
            RawRunStorage::Owned(storage) => storage,
            RawRunStorage::Shared(_) => unreachable!(),
        }
    }
}
