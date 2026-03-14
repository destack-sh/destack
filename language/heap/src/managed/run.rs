use std::sync::Arc;

use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

use super::super::DynamicBitmap;
use super::ReferenceMapId;

/// One immutable managed run trace layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ManagedRunTraceImage {
    /// One run-wide trace id shared by every occupied slot.
    Monomorphic(u32),
    /// One per-slot trace id table.
    Polymorphic(Arc<[u32]>),
}

/// One immutable managed run layout metadata shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ManagedRunLayoutImage {
    /// One run-wide layout id shared by every occupied slot.
    Monomorphic(u32),
    /// One per-slot layout id table.
    Polymorphic(Arc<[u32]>),
}

/// One immutable managed run image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedRunImage {
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
    /// The trace metadata for this run.
    trace_metadata: ManagedRunTraceImage,
    /// The layout metadata for this run.
    layout_metadata: ManagedRunLayoutImage,
}

impl ManagedRunImage {
    /// Report whether this run image is one vacant directory slot.
    pub(crate) fn is_vacant(&self) -> bool {
        self.slot_count == 0
    }

    /// Report whether this run image shares durable backing with another run image.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        self.size_class == other.size_class
            && self.slot_count == other.slot_count
            && Arc::ptr_eq(&self.bytes, &other.bytes)
            && self.occupied == other.occupied
            && Arc::ptr_eq(&self.lengths, &other.lengths)
            && Self::shares_trace_storage(&self.trace_metadata, &other.trace_metadata)
            && Self::shares_layout_storage(&self.layout_metadata, &other.layout_metadata)
    }

    /// Report whether two trace metadata images share backing.
    fn shares_trace_storage(left: &ManagedRunTraceImage, right: &ManagedRunTraceImage) -> bool {
        match (left, right) {
            (ManagedRunTraceImage::Monomorphic(left), ManagedRunTraceImage::Monomorphic(right)) => {
                left == right
            }
            (ManagedRunTraceImage::Polymorphic(left), ManagedRunTraceImage::Polymorphic(right)) => {
                Arc::ptr_eq(left, right)
            }
            _ => false,
        }
    }

    /// Report whether two layout metadata images share backing.
    fn shares_layout_storage(left: &ManagedRunLayoutImage, right: &ManagedRunLayoutImage) -> bool {
        match (left, right) {
            (
                ManagedRunLayoutImage::Monomorphic(left),
                ManagedRunLayoutImage::Monomorphic(right),
            ) => left == right,
            (
                ManagedRunLayoutImage::Polymorphic(left),
                ManagedRunLayoutImage::Polymorphic(right),
            ) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
}

/// One owned managed run trace layout.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ManagedRunTraceOwned {
    /// One run-wide trace id shared by every occupied slot.
    Monomorphic(u32),
    /// One per-slot trace id table.
    Polymorphic(Box<[u32]>),
}

/// One owned managed run layout metadata shape.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ManagedRunLayoutOwned {
    /// One run-wide layout id shared by every occupied slot.
    Monomorphic(u32),
    /// One per-slot layout id table.
    Polymorphic(Box<[u32]>),
}

/// One owned managed run payload.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedRunOwned {
    /// The packed slot payload bytes.
    bytes: Box<[u8]>,
    /// The occupied slots in this run.
    occupied: DynamicBitmap,
    /// The logical byte length for each slot.
    lengths: Box<[u16]>,
    /// The trace metadata for this run.
    trace_metadata: ManagedRunTraceOwned,
    /// The layout metadata for this run.
    layout_metadata: ManagedRunLayoutOwned,
}

/// One live managed run storage state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ManagedRunStorage {
    /// Owned mutable run storage.
    Owned(ManagedRunOwned),
    /// Shared immutable run storage.
    Shared(ManagedRunImage),
}

/// One live managed run for one size class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedRun {
    /// The slot payload size in bytes.
    size_class: usize,
    /// The number of slots in this run.
    slot_count: usize,
    /// The number of occupied slots in this run.
    occupied_count: usize,
    /// The next likely free slot.
    next_free_slot: usize,
    /// The live run storage.
    storage: ManagedRunStorage,
    /// The live mark bitmap keyed by slot.
    marked: DynamicBitmap,
    /// The live pinned slots keyed by slot.
    pinned: DynamicBitmap,
    /// Overflow pin counts for slots pinned more than once.
    overflow_pin_counts: Vec<(u16, u16)>,
    /// The number of active pins in this run.
    active_pins: usize,
}

impl ManagedRun {
    /// Create one vacant managed run slot with no backing storage.
    pub(crate) fn vacant() -> Self {
        Self {
            size_class: 0,
            slot_count: 0,
            occupied_count: 0,
            next_free_slot: 0,
            storage: ManagedRunStorage::Owned(ManagedRunOwned {
                bytes: Vec::new().into_boxed_slice(),
                occupied: DynamicBitmap::with_capacity(0),
                lengths: Vec::new().into_boxed_slice(),
                trace_metadata: ManagedRunTraceOwned::Monomorphic(0),
                layout_metadata: ManagedRunLayoutOwned::Monomorphic(0),
            }),
            marked: DynamicBitmap::with_capacity(0),
            pinned: DynamicBitmap::with_capacity(0),
            overflow_pin_counts: Vec::new(),
            active_pins: 0,
        }
    }

    /// Create one empty managed run for the given size class and run byte width.
    pub(crate) fn new(size_class: usize, run_bytes: usize) -> Self {
        let slot_count = (run_bytes / size_class).max(1);
        let bytes = vec![0; slot_count * size_class].into_boxed_slice();
        let lengths = vec![0; slot_count].into_boxed_slice();

        Self {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            storage: ManagedRunStorage::Owned(ManagedRunOwned {
                bytes,
                occupied: DynamicBitmap::with_capacity(slot_count),
                lengths,
                trace_metadata: ManagedRunTraceOwned::Monomorphic(0),
                layout_metadata: ManagedRunLayoutOwned::Monomorphic(0),
            }),
            marked: DynamicBitmap::with_capacity(slot_count),
            pinned: DynamicBitmap::with_capacity(slot_count),
            overflow_pin_counts: Vec::new(),
            active_pins: 0,
        }
    }

    /// Restore one managed run from one immutable image.
    pub(crate) fn from_image(image: ManagedRunImage) -> Self {
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
            storage: ManagedRunStorage::Shared(image),
            marked: DynamicBitmap::with_capacity(slot_count),
            pinned: DynamicBitmap::with_capacity(slot_count),
            overflow_pin_counts: Vec::new(),
            active_pins: 0,
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

    /// Report whether one slot is occupied.
    pub(crate) fn is_occupied(&self, slot: usize) -> bool {
        self.occupied().contains(slot)
    }

    /// Report whether one slot is marked.
    pub(crate) fn is_marked(&self, slot: usize) -> bool {
        self.marked.contains(slot)
    }

    /// Mark one slot.
    pub(crate) fn mark(&mut self, slot: usize) {
        self.marked.set(slot);
    }

    /// Clear all marks.
    pub(crate) fn clear_marks(&mut self) {
        self.marked.clear_all();
    }

    /// Return the logical byte length for one occupied slot.
    pub(crate) fn len(&self, slot: usize) -> Option<usize> {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return None;
        }

        match &self.storage {
            ManagedRunStorage::Owned(storage) => storage.lengths.get(slot).copied(),
            ManagedRunStorage::Shared(image) => image.lengths.get(slot).copied(),
        }
        .map(|len| len as usize)
    }

    /// Return the bytes for one occupied slot.
    pub(crate) fn bytes(&self, slot: usize) -> Option<&[u8]> {
        let len = self.len(slot)?;
        let start = slot.checked_mul(self.size_class)?;
        let end = start.checked_add(len)?;

        match &self.storage {
            ManagedRunStorage::Owned(storage) => storage.bytes.get(start..end),
            ManagedRunStorage::Shared(image) => image.bytes.get(start..end),
        }
    }

    /// Return the trace id for one occupied slot.
    pub(crate) fn trace_id(&self, slot: usize) -> Option<ReferenceMapId> {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return None;
        }

        let trace_id = match &self.storage {
            ManagedRunStorage::Owned(storage) => match &storage.trace_metadata {
                ManagedRunTraceOwned::Monomorphic(trace_id) => *trace_id,
                ManagedRunTraceOwned::Polymorphic(trace_ids) => trace_ids.get(slot).copied()?,
            },
            ManagedRunStorage::Shared(image) => match &image.trace_metadata {
                ManagedRunTraceImage::Monomorphic(trace_id) => *trace_id,
                ManagedRunTraceImage::Polymorphic(trace_ids) => trace_ids.get(slot).copied()?,
            },
        };

        Some(ReferenceMapId::new(trace_id as usize))
    }

    /// Return the layout id for one occupied slot.
    pub(crate) fn layout_id(&self, slot: usize) -> Option<LayoutId> {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return None;
        }

        let raw = match &self.storage {
            ManagedRunStorage::Owned(storage) => match &storage.layout_metadata {
                ManagedRunLayoutOwned::Monomorphic(layout_id) => *layout_id,
                ManagedRunLayoutOwned::Polymorphic(layout_ids) => layout_ids.get(slot).copied()?,
            },
            ManagedRunStorage::Shared(image) => match &image.layout_metadata {
                ManagedRunLayoutImage::Monomorphic(layout_id) => *layout_id,
                ManagedRunLayoutImage::Polymorphic(layout_ids) => layout_ids.get(slot).copied()?,
            },
        };

        if raw == 0 {
            None
        } else {
            Some(LayoutId::new(raw))
        }
    }

    /// Set the layout id for one occupied slot.
    pub(crate) fn set_layout_id(&mut self, slot: usize, layout_id: LayoutId) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) {
            return false;
        }

        let occupied = self.occupied_count;
        let slot_count = self.slot_count;
        let storage = self.storage_mut();
        Self::set_layout_metadata(
            &storage.occupied,
            &mut storage.layout_metadata,
            occupied,
            slot_count,
            slot,
            layout_id.raw(),
        );
        true
    }

    /// Allocate one free slot in this run.
    pub(crate) fn allocate_slot(
        &mut self,
        slot: usize,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> bool {
        if slot >= self.slot_count || bytes.len() > self.size_class || self.is_occupied(slot) {
            return false;
        }

        let size_class = self.size_class;
        let occupied = self.occupied_count;
        let slot_count = self.slot_count;
        let storage = self.storage_mut();
        let start = slot * size_class;
        let end = start + size_class;

        storage.bytes[start..end].fill(0);
        storage.bytes[start..start + bytes.len()].copy_from_slice(bytes);
        storage.lengths[slot] = bytes.len() as u16;
        Self::set_trace_metadata(
            &storage.occupied,
            &mut storage.trace_metadata,
            occupied,
            slot_count,
            slot,
            trace_id.index() as u32,
        );
        Self::set_layout_metadata(
            &storage.occupied,
            &mut storage.layout_metadata,
            occupied,
            slot_count,
            slot,
            layout_id.map(LayoutId::raw).unwrap_or_default(),
        );
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
    pub(crate) fn replace_slot(
        &mut self,
        slot: usize,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> bool {
        if slot >= self.slot_count || !self.is_occupied(slot) || bytes.len() > self.size_class {
            return false;
        }

        let size_class = self.size_class;
        let occupied = self.occupied_count;
        let slot_count = self.slot_count;
        let storage = self.storage_mut();
        let start = slot * size_class;
        let end = start + size_class;

        storage.bytes[start..end].fill(0);
        storage.bytes[start..start + bytes.len()].copy_from_slice(bytes);
        storage.lengths[slot] = bytes.len() as u16;
        Self::set_trace_metadata(
            &storage.occupied,
            &mut storage.trace_metadata,
            occupied,
            slot_count,
            slot,
            trace_id.index() as u32,
        );
        Self::set_layout_metadata(
            &storage.occupied,
            &mut storage.layout_metadata,
            occupied,
            slot_count,
            slot,
            layout_id.map(LayoutId::raw).unwrap_or_default(),
        );
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
        {
            let storage = self.storage_mut();
            let start = slot * size_class;
            let end = start + size_class;

            storage.bytes[start..end].fill(0);
            storage.lengths[slot] = 0;
            Self::clear_trace_metadata(&mut storage.trace_metadata, slot);
            Self::clear_layout_metadata(&mut storage.layout_metadata, slot);
            storage.occupied.clear(slot);
        }

        self.marked.clear(slot);
        self.clear_pin_state(slot);
        self.occupied_count = self.occupied_count.saturating_sub(1);

        if self.occupied_count == 0 {
            let storage = self.storage_mut();
            storage.trace_metadata = ManagedRunTraceOwned::Monomorphic(0);
            storage.layout_metadata = ManagedRunLayoutOwned::Monomorphic(0);
            self.next_free_slot = 0;
        } else {
            self.next_free_slot = self.next_free_slot.min(slot);
        }

        true
    }

    /// Return the active pin count for this run.
    pub(crate) fn active_pins(&self) -> usize {
        self.active_pins
    }

    /// Increment the pin count for one occupied slot.
    pub(crate) fn pin(&mut self, slot: usize) -> bool {
        if !self.is_occupied(slot) {
            return false;
        }

        let key = slot as u16;
        if self.pinned.contains(slot) {
            match self
                .overflow_pin_counts
                .iter_mut()
                .find(|(run_slot, _)| *run_slot == key)
            {
                Some((_, pin_count)) => *pin_count = pin_count.saturating_add(1),
                None => self.overflow_pin_counts.push((key, 2)),
            }
        } else {
            self.pinned.set(slot);
        }

        self.active_pins = self.active_pins.saturating_add(1);
        true
    }

    /// Decrement the pin count for one occupied slot.
    pub(crate) fn unpin(&mut self, slot: usize) -> bool {
        if !self.is_occupied(slot) || !self.pinned.contains(slot) {
            return false;
        }

        let key = slot as u16;
        match self
            .overflow_pin_counts
            .iter_mut()
            .find(|(run_slot, _)| *run_slot == key)
        {
            Some((_, pin_count)) if *pin_count > 2 => {
                *pin_count -= 1;
            }
            Some((_, pin_count)) if *pin_count == 2 => {
                self.overflow_pin_counts
                    .retain(|(run_slot, _)| *run_slot != key);
            }
            _ => {
                self.pinned.clear(slot);
            }
        }

        self.active_pins = self.active_pins.saturating_sub(1);
        true
    }

    /// Capture one immutable run image.
    pub(crate) fn image(&mut self) -> ManagedRunImage {
        if self.is_vacant() {
            return ManagedRunImage {
                size_class: 0,
                slot_count: 0,
                bytes: Arc::from(Vec::<u8>::new().into_boxed_slice()),
                occupied: DynamicBitmap::with_capacity(0),
                lengths: Arc::from(Vec::<u16>::new().into_boxed_slice()),
                trace_metadata: ManagedRunTraceImage::Monomorphic(0),
                layout_metadata: ManagedRunLayoutImage::Monomorphic(0),
            };
        }

        match &mut self.storage {
            ManagedRunStorage::Shared(image) => image.clone(),
            ManagedRunStorage::Owned(storage) => {
                let image = ManagedRunImage {
                    size_class: self.size_class,
                    slot_count: self.slot_count,
                    bytes: Arc::from(std::mem::take(&mut storage.bytes)),
                    occupied: storage.occupied.clone(),
                    lengths: Arc::from(std::mem::take(&mut storage.lengths)),
                    trace_metadata: Self::trace_image(&mut storage.trace_metadata),
                    layout_metadata: Self::layout_image(&mut storage.layout_metadata),
                };
                self.storage = ManagedRunStorage::Shared(image.clone());
                image
            }
        }
    }

    /// Return the retained bytes for this run.
    pub(crate) fn retained_bytes(&self) -> usize {
        if self.is_vacant() {
            return self.marked.retained_bytes()
                + self.pinned.retained_bytes()
                + self.overflow_pin_counts.capacity() * std::mem::size_of::<(u16, u16)>();
        }

        let side_bytes = self.marked.retained_bytes()
            + self.pinned.retained_bytes()
            + self.overflow_pin_counts.capacity() * std::mem::size_of::<(u16, u16)>();

        match &self.storage {
            ManagedRunStorage::Owned(storage) => {
                storage.bytes.len()
                    + storage.lengths.len() * std::mem::size_of::<u16>()
                    + storage.occupied.retained_bytes()
                    + Self::trace_metadata_retained_bytes(&storage.trace_metadata)
                    + Self::layout_metadata_retained_bytes(&storage.layout_metadata)
                    + side_bytes
            }
            ManagedRunStorage::Shared(image) => {
                image.bytes.len()
                    + image.lengths.len() * std::mem::size_of::<u16>()
                    + image.occupied.retained_bytes()
                    + Self::trace_image_retained_bytes(&image.trace_metadata)
                    + Self::layout_image_retained_bytes(&image.layout_metadata)
                    + side_bytes
            }
        }
    }

    /// Return the occupied bitmap for this run.
    fn occupied(&self) -> &DynamicBitmap {
        match &self.storage {
            ManagedRunStorage::Owned(storage) => &storage.occupied,
            ManagedRunStorage::Shared(image) => &image.occupied,
        }
    }

    /// Clear one slot's pin state.
    fn clear_pin_state(&mut self, slot: usize) {
        if self.pinned.contains(slot) {
            self.pinned.clear(slot);
        }

        let key = slot as u16;
        self.overflow_pin_counts
            .retain(|(run_slot, _)| *run_slot != key);
    }

    /// Return mutable owned run storage, detaching images on first write.
    fn storage_mut(&mut self) -> &mut ManagedRunOwned {
        if let ManagedRunStorage::Shared(image) = &self.storage {
            self.storage = ManagedRunStorage::Owned(ManagedRunOwned {
                bytes: image.bytes.as_ref().to_vec().into_boxed_slice(),
                occupied: image.occupied.clone(),
                lengths: image.lengths.as_ref().to_vec().into_boxed_slice(),
                trace_metadata: Self::trace_owned(&image.trace_metadata),
                layout_metadata: Self::layout_owned(&image.layout_metadata),
            });
        }

        match &mut self.storage {
            ManagedRunStorage::Owned(storage) => storage,
            ManagedRunStorage::Shared(_) => unreachable!(),
        }
    }

    /// Convert one shared trace image into owned storage.
    fn trace_owned(trace_metadata: &ManagedRunTraceImage) -> ManagedRunTraceOwned {
        match trace_metadata {
            ManagedRunTraceImage::Monomorphic(trace_id) => {
                ManagedRunTraceOwned::Monomorphic(*trace_id)
            }
            ManagedRunTraceImage::Polymorphic(trace_ids) => {
                ManagedRunTraceOwned::Polymorphic(trace_ids.as_ref().to_vec().into_boxed_slice())
            }
        }
    }

    /// Convert one shared layout image into owned storage.
    fn layout_owned(layout_metadata: &ManagedRunLayoutImage) -> ManagedRunLayoutOwned {
        match layout_metadata {
            ManagedRunLayoutImage::Monomorphic(layout_id) => {
                ManagedRunLayoutOwned::Monomorphic(*layout_id)
            }
            ManagedRunLayoutImage::Polymorphic(layout_ids) => {
                ManagedRunLayoutOwned::Polymorphic(layout_ids.as_ref().to_vec().into_boxed_slice())
            }
        }
    }

    /// Convert one owned trace storage into one shared image.
    fn trace_image(trace_metadata: &mut ManagedRunTraceOwned) -> ManagedRunTraceImage {
        match trace_metadata {
            ManagedRunTraceOwned::Monomorphic(trace_id) => {
                ManagedRunTraceImage::Monomorphic(*trace_id)
            }
            ManagedRunTraceOwned::Polymorphic(trace_ids) => {
                ManagedRunTraceImage::Polymorphic(Arc::from(std::mem::take(trace_ids)))
            }
        }
    }

    /// Convert one owned layout storage into one shared image.
    fn layout_image(layout_metadata: &mut ManagedRunLayoutOwned) -> ManagedRunLayoutImage {
        match layout_metadata {
            ManagedRunLayoutOwned::Monomorphic(layout_id) => {
                ManagedRunLayoutImage::Monomorphic(*layout_id)
            }
            ManagedRunLayoutOwned::Polymorphic(layout_ids) => {
                ManagedRunLayoutImage::Polymorphic(Arc::from(std::mem::take(layout_ids)))
            }
        }
    }

    /// Return the retained bytes owned by one trace metadata shape.
    fn trace_metadata_retained_bytes(trace_metadata: &ManagedRunTraceOwned) -> usize {
        match trace_metadata {
            ManagedRunTraceOwned::Monomorphic(_) => 0,
            ManagedRunTraceOwned::Polymorphic(trace_ids) => {
                trace_ids.len() * std::mem::size_of::<u32>()
            }
        }
    }

    /// Return the retained bytes owned by one layout metadata shape.
    fn layout_metadata_retained_bytes(layout_metadata: &ManagedRunLayoutOwned) -> usize {
        match layout_metadata {
            ManagedRunLayoutOwned::Monomorphic(_) => 0,
            ManagedRunLayoutOwned::Polymorphic(layout_ids) => {
                layout_ids.len() * std::mem::size_of::<u32>()
            }
        }
    }

    /// Return the retained bytes owned by one trace metadata image.
    fn trace_image_retained_bytes(trace_metadata: &ManagedRunTraceImage) -> usize {
        match trace_metadata {
            ManagedRunTraceImage::Monomorphic(_) => 0,
            ManagedRunTraceImage::Polymorphic(trace_ids) => {
                trace_ids.len() * std::mem::size_of::<u32>()
            }
        }
    }

    /// Return the retained bytes owned by one layout metadata image.
    fn layout_image_retained_bytes(layout_metadata: &ManagedRunLayoutImage) -> usize {
        match layout_metadata {
            ManagedRunLayoutImage::Monomorphic(_) => 0,
            ManagedRunLayoutImage::Polymorphic(layout_ids) => {
                layout_ids.len() * std::mem::size_of::<u32>()
            }
        }
    }

    /// Write one trace id into the run metadata.
    fn set_trace_metadata(
        occupied: &DynamicBitmap,
        trace_metadata: &mut ManagedRunTraceOwned,
        occupied_count: usize,
        slot_count: usize,
        slot: usize,
        trace_id: u32,
    ) {
        match trace_metadata {
            ManagedRunTraceOwned::Monomorphic(current)
                if occupied_count == 0 || *current == trace_id =>
            {
                *current = trace_id;
            }
            ManagedRunTraceOwned::Monomorphic(current) => {
                let mut trace_ids = vec![0; slot_count].into_boxed_slice();

                for occupied_slot in 0..slot_count {
                    if occupied.contains(occupied_slot) {
                        trace_ids[occupied_slot] = *current;
                    }
                }

                trace_ids[slot] = trace_id;
                *trace_metadata = ManagedRunTraceOwned::Polymorphic(trace_ids);
            }
            ManagedRunTraceOwned::Polymorphic(trace_ids) => {
                trace_ids[slot] = trace_id;
            }
        }
    }

    /// Write one layout id into the run metadata.
    fn set_layout_metadata(
        occupied: &DynamicBitmap,
        layout_metadata: &mut ManagedRunLayoutOwned,
        occupied_count: usize,
        slot_count: usize,
        slot: usize,
        layout_id: u32,
    ) {
        match layout_metadata {
            ManagedRunLayoutOwned::Monomorphic(current)
                if occupied_count == 0 || *current == layout_id =>
            {
                *current = layout_id;
            }
            ManagedRunLayoutOwned::Monomorphic(current) => {
                let mut layout_ids = vec![0; slot_count].into_boxed_slice();

                for occupied_slot in 0..slot_count {
                    if occupied.contains(occupied_slot) {
                        layout_ids[occupied_slot] = *current;
                    }
                }

                layout_ids[slot] = layout_id;
                *layout_metadata = ManagedRunLayoutOwned::Polymorphic(layout_ids);
            }
            ManagedRunLayoutOwned::Polymorphic(layout_ids) => {
                layout_ids[slot] = layout_id;
            }
        }
    }

    /// Clear one slot's trace metadata.
    fn clear_trace_metadata(trace_metadata: &mut ManagedRunTraceOwned, slot: usize) {
        if let ManagedRunTraceOwned::Polymorphic(trace_ids) = trace_metadata {
            trace_ids[slot] = 0;
        }
    }

    /// Clear one slot's layout metadata.
    fn clear_layout_metadata(layout_metadata: &mut ManagedRunLayoutOwned, slot: usize) {
        if let ManagedRunLayoutOwned::Polymorphic(layout_ids) = layout_metadata {
            layout_ids[slot] = 0;
        }
    }
}
