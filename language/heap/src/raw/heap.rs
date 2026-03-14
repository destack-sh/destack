use std::borrow::Cow;
use std::mem::size_of;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{RawExtent, RawExtentId, RawExtentImage, RawImage, RawRun, RawRunImage};
use crate::heap::{HeapLayoutOptions, RawSpaceUsage, SizeClassTable, TreeVector};
use crate::value::{RawPointer, Value};

/// The first non-null raw allocation id.
const FIRST_ALLOCATED_RAW_ID: u64 = 1;

/// The first non-null raw extent id.
const FIRST_ALLOCATED_EXTENT_ID: u64 = 1;

/// One stable raw run slot location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRunSlot {
    /// The containing run index.
    run_index: u32,
    /// The slot index inside the run.
    slot_index: u32,
}

impl RawRunSlot {
    /// Create one raw run slot location.
    pub(crate) const fn new(run_index: usize, slot_index: usize) -> Self {
        Self {
            run_index: run_index as u32,
            slot_index: slot_index as u32,
        }
    }

    /// Return the containing run index.
    pub(crate) const fn run_index(self) -> usize {
        self.run_index as usize
    }

    /// Return the slot index inside the run.
    pub(crate) const fn slot_index(self) -> usize {
        self.slot_index as usize
    }
}

/// One stable raw allocation location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawLocation {
    /// One vacant directory slot.
    Vacant,
    /// One run-backed allocation.
    Run(RawRunSlot),
    /// One extent-backed allocation.
    Extent(RawExtentId),
}

/// One live raw allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured run width.
    pub(crate) run_bytes: usize,
    /// The configured extent chunk width.
    pub(crate) chunk_bytes: usize,
    /// The live raw runs.
    runs: Vec<RawRun>,
    /// The reusable non-full runs per size class.
    available_runs: Vec<Vec<usize>>,
    /// The vacant run slots available for reuse.
    free_run_ids: Vec<usize>,
    /// The live raw extents.
    extents: Vec<RawExtent>,
    /// The free raw extent ids available for reuse.
    free_extent_ids: Vec<u64>,
    /// Stable raw locations keyed by allocation id minus one.
    locations: Vec<RawLocation>,
    /// The free raw allocation ids available for reuse.
    free_ids: Vec<u64>,
    /// The next raw allocation id to allocate.
    next_unused_id: u64,
    /// The next raw extent id to allocate.
    next_unused_extent_id: u64,
    /// The number of live raw allocations.
    allocated_count: usize,
    /// The number of live raw bytes.
    allocated_bytes: u64,
    /// The exact retained raw bytes.
    retained_bytes: u64,
}

impl Default for RawSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl RawSpace {
    /// Create one raw space with the default layout options.
    pub fn new() -> Self {
        Self::with_layout(&HeapLayoutOptions::default())
    }

    /// Create one raw space with explicit layout options.
    pub fn with_layout(options: &HeapLayoutOptions) -> Self {
        let mut space = Self {
            size_classes: options.size_classes.clone(),
            run_bytes: options.raw_run_bytes,
            chunk_bytes: options.chunk_bytes,
            runs: Vec::new(),
            available_runs: vec![Vec::new(); options.size_classes.classes.len()],
            free_run_ids: Vec::new(),
            extents: Vec::new(),
            free_extent_ids: Vec::new(),
            locations: Vec::new(),
            free_ids: Vec::new(),
            next_unused_id: FIRST_ALLOCATED_RAW_ID,
            next_unused_extent_id: FIRST_ALLOCATED_EXTENT_ID,
            allocated_count: 0,
            allocated_bytes: 0,
            retained_bytes: 0,
        };

        space.recompute_retained_bytes();

        space
    }

    /// Restore one raw space from one immutable image.
    pub fn from_image(image: &RawImage) -> Self {
        let mut space = Self {
            size_classes: image.size_classes.clone(),
            run_bytes: image.run_bytes,
            chunk_bytes: image.chunk_bytes,
            runs: image.runs.iter().cloned().map(RawRun::from_image).collect(),
            available_runs: vec![Vec::new(); image.size_classes.classes.len()],
            free_run_ids: Vec::new(),
            extents: image
                .extents
                .iter()
                .map(RawExtent::from_extent_image)
                .collect(),
            free_extent_ids: image.free_extent_ids.iter().copied().collect(),
            locations: image.locations.iter().copied().collect(),
            free_ids: image.free_ids.iter().copied().collect(),
            next_unused_id: image.next_unused_id,
            next_unused_extent_id: image.next_unused_extent_id,
            allocated_count: image.allocated_count,
            allocated_bytes: image.allocated_bytes,
            retained_bytes: 0,
        };

        space.rebuild_run_directories();
        space.recompute_retained_bytes();

        space
    }

    /// Return the number of live raw allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the exact retained raw bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Return the exact live usage for this raw space.
    pub fn usage(&self) -> RawSpaceUsage {
        RawSpaceUsage {
            allocation_count: self.allocated_count,
            allocation_bytes: self.allocated_bytes,
            retained_bytes: self.retained_bytes,
        }
    }

    /// Capture one immutable raw-space image.
    pub fn image(&mut self, base: Option<&RawImage>) -> RawImage {
        let runs = self.runs.iter_mut().map(RawRun::image).collect::<Vec<_>>();
        let extents = self
            .extents
            .iter_mut()
            .map(RawExtent::extent_image)
            .collect::<Vec<_>>();

        RawImage {
            size_classes: self.size_classes.clone(),
            run_bytes: self.run_bytes,
            chunk_bytes: self.chunk_bytes,
            runs: TreeVector::from_values_by(
                &runs,
                base.map(|image| &image.runs),
                RawRunImage::shares_storage_with,
            ),
            extents: TreeVector::from_values_by(
                &extents,
                base.map(|image| &image.extents),
                RawExtentImage::shares_storage_with,
            ),
            locations: Arc::from(self.locations.as_slice()),
            free_ids: Arc::from(self.free_ids.as_slice()),
            free_extent_ids: Arc::from(self.free_extent_ids.as_slice()),
            next_unused_id: self.next_unused_id,
            next_unused_extent_id: self.next_unused_extent_id,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
        }
    }

    /// Allocate one raw byte allocation.
    pub fn allocate_bytes(&mut self, bytes: &[u8]) -> RawPointer {
        let location = if let Some(class_index) = self.size_classes.class_index_for(bytes.len()) {
            RawLocation::Run(self.allocate_run_slot(class_index, bytes))
        } else {
            RawLocation::Extent(self.allocate_extent(bytes))
        };

        let pointer = self.allocate_location(location);
        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);
        self.recompute_retained_bytes();

        pointer
    }

    /// Allocate one zeroed raw allocation.
    pub fn allocate_zeroed(&mut self, byte_len: usize) -> RawPointer {
        self.allocate_bytes(&vec![0; byte_len])
    }

    /// Return the byte length for one raw allocation.
    pub fn byte_len(&self, pointer: RawPointer) -> Option<usize> {
        let base = RawPointer::new(pointer.id());
        let offset = pointer.byte_offset();
        let location = self.location(base)?;

        let len = match location {
            RawLocation::Vacant => return None,
            RawLocation::Run(slot) => self.runs.get(slot.run_index())?.len(slot.slot_index())?,
            RawLocation::Extent(extent_id) => self.extent(extent_id)?.len(),
        };

        len.checked_sub(offset)
    }

    /// Return the bytes for one raw allocation.
    pub fn bytes(&self, pointer: RawPointer) -> Option<Cow<'_, [u8]>> {
        let base = RawPointer::new(pointer.id());
        let offset = pointer.byte_offset();
        let location = self.location(base)?;

        let bytes = match location {
            RawLocation::Vacant => return None,
            RawLocation::Run(slot) => {
                Cow::Borrowed(self.runs.get(slot.run_index())?.bytes(slot.slot_index())?)
            }
            RawLocation::Extent(extent_id) => self.extent(extent_id)?.bytes(),
        };

        match bytes {
            Cow::Borrowed(bytes) => Some(Cow::Borrowed(bytes.get(offset..)?)),
            Cow::Owned(bytes) => Some(Cow::Owned(bytes.get(offset..)?.to_vec())),
        }
    }

    /// Return one owned copy of the bytes for one raw allocation.
    pub fn bytes_to_vec(&self, pointer: RawPointer) -> Option<Vec<u8>> {
        Some(self.bytes(pointer)?.into_owned())
    }

    /// Return one byte by offset within one raw allocation.
    pub fn byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        self.bytes(pointer)?.as_ref().get(index).copied()
    }

    /// Set one byte inside one raw allocation.
    pub fn set_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> bool {
        let base = RawPointer::new(pointer.id());
        let offset = pointer.byte_offset().saturating_add(index);
        let Some(location) = self.location(base) else {
            return false;
        };

        let updated = match location {
            RawLocation::Vacant => false,
            RawLocation::Run(slot) => self
                .runs
                .get_mut(slot.run_index())
                .map(|run| run.set_byte(slot.slot_index(), offset, byte))
                .unwrap_or(false),
            RawLocation::Extent(extent_id) => self
                .extent_mut(extent_id)
                .map(|extent| extent.set(offset, byte))
                .unwrap_or(false),
        };

        updated
    }

    /// Free one raw allocation.
    pub fn free(&mut self, pointer: RawPointer) -> bool {
        let base = RawPointer::new(pointer.id());
        let Some(location) = self.location(base) else {
            return false;
        };
        let Some(old_len) = self.byte_len(base) else {
            return false;
        };

        match location {
            RawLocation::Vacant => return false,
            RawLocation::Run(slot) => {
                let run_index = slot.run_index();
                let Some(size_class) = self.runs.get(run_index).map(RawRun::size_class) else {
                    return false;
                };
                let Some(class_index) = self.class_index_for_run(size_class) else {
                    return false;
                };
                let Some(run) = self.runs.get_mut(run_index) else {
                    return false;
                };

                if !run.free_slot(slot.slot_index()) {
                    return false;
                }

                if run.is_empty() {
                    self.runs[run_index] = RawRun::vacant();
                    self.free_run_ids.push(run_index);
                } else if run.has_free_slot() {
                    self.available_runs[class_index].push(run_index);
                }
            }
            RawLocation::Extent(extent_id) => {
                let Some(extent) = self.extent_mut(extent_id) else {
                    return false;
                };

                extent.free();
                self.free_extent_ids.push(extent_id.id());
            }
        }

        if let Some(location_slot) = self.location_entry_mut(pointer.id()) {
            *location_slot = RawLocation::Vacant;
        }

        self.free_ids.push(pointer.id());
        self.allocated_count = self.allocated_count.saturating_sub(1);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(old_len as u64);
        self.recompute_retained_bytes();

        true
    }

    /// Replace one raw allocation payload.
    pub fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> bool {
        let old_len = self
            .byte_len(RawPointer::new(pointer.id()))
            .unwrap_or_default() as u64;
        let Some(location) = self.location(RawPointer::new(pointer.id())) else {
            return false;
        };

        let replaced = match location {
            RawLocation::Vacant => false,
            RawLocation::Run(slot) => {
                let Some(class_index) = self.size_classes.class_index_for(bytes.len()) else {
                    let extent_id = self.allocate_extent(bytes);
                    let freed = self
                        .runs
                        .get_mut(slot.run_index())
                        .map(|run| run.free_slot(slot.slot_index()))
                        .unwrap_or(false);
                    self.move_location(pointer.id(), RawLocation::Extent(extent_id));
                    return freed;
                };

                let run_size_class = self.runs.get(slot.run_index()).map(RawRun::size_class);
                if run_size_class
                    == self
                        .size_classes
                        .classes
                        .get(class_index)
                        .map(|class| class.bytes)
                {
                    self.runs
                        .get_mut(slot.run_index())
                        .map(|run| run.replace_slot(slot.slot_index(), bytes))
                        .unwrap_or(false)
                } else {
                    let new_slot = self.allocate_run_slot(class_index, bytes);
                    let freed = self
                        .runs
                        .get_mut(slot.run_index())
                        .map(|run| run.free_slot(slot.slot_index()))
                        .unwrap_or(false);
                    self.move_location(pointer.id(), RawLocation::Run(new_slot));
                    freed
                }
            }
            RawLocation::Extent(extent_id) => {
                if let Some(class_index) = self.size_classes.class_index_for(bytes.len()) {
                    let new_slot = self.allocate_run_slot(class_index, bytes);
                    if let Some(extent) = self.extent_mut(extent_id) {
                        extent.free();
                        self.free_extent_ids.push(extent_id.id());
                        self.move_location(pointer.id(), RawLocation::Run(new_slot));
                        true
                    } else {
                        false
                    }
                } else {
                    let chunk_bytes = self.chunk_bytes;
                    self.extent_mut(extent_id)
                        .map(|extent| {
                            extent.replace(bytes, chunk_bytes);
                            true
                        })
                        .unwrap_or(false)
                }
            }
        };

        if replaced {
            self.allocated_bytes = self
                .allocated_bytes
                .saturating_sub(old_len)
                .saturating_add(bytes.len() as u64);
            self.recompute_retained_bytes();
        }

        replaced
    }

    /// Allocate one raw packed-value payload.
    pub fn allocate_packed_values(&mut self, values: Vec<Value>) -> RawPointer {
        self.allocate_bytes(&encode_values(&values))
    }

    /// Allocate one raw packed-value payload with the given count.
    pub fn allocate_packed_value_slots(&mut self, slot_count: usize) -> RawPointer {
        self.allocate_zeroed(slot_count * Value::BYTE_LEN)
    }

    /// Return the raw allocation as decoded values.
    pub fn values_to_vec(&self, pointer: RawPointer) -> Option<Vec<Value>> {
        let bytes = self.bytes(pointer)?;
        decode_values(bytes.as_ref())
    }

    /// Set one packed raw value.
    pub fn set_value(&mut self, pointer: RawPointer, index: usize, value: Value) -> bool {
        let bytes = value.to_byte_array();
        let start = index * Value::BYTE_LEN;

        for (offset, byte) in bytes.into_iter().enumerate() {
            if !self.set_byte(pointer, start + offset, byte) {
                return false;
            }
        }

        true
    }

    /// Resize one raw packed-value payload.
    pub fn resize_values(&mut self, pointer: RawPointer, len: usize) -> bool {
        let Some(mut values) = self.values_to_vec(RawPointer::new(pointer.id())) else {
            return false;
        };
        values.resize(len, Value::VOID);
        self.replace_bytes(pointer, &encode_values(&values))
    }

    // directories

    fn allocate_location(&mut self, location: RawLocation) -> RawPointer {
        if let Some(id) = self.free_ids.pop() {
            if let Some(slot) = self.location_entry_mut(id) {
                *slot = location;
            }

            return RawPointer::new(id);
        }

        let id = self.next_unused_id;
        self.next_unused_id = self.next_unused_id.saturating_add(1);
        self.locations.push(location);

        RawPointer::new(id)
    }

    fn location(&self, pointer: RawPointer) -> Option<RawLocation> {
        if pointer.id() == 0 {
            return None;
        }

        self.locations.get((pointer.id() - 1) as usize).copied()
    }

    fn location_entry_mut(&mut self, id: u64) -> Option<&mut RawLocation> {
        if id == 0 {
            return None;
        }

        self.locations.get_mut((id - 1) as usize)
    }

    fn move_location(&mut self, id: u64, location: RawLocation) {
        if let Some(slot) = self.location_entry_mut(id) {
            *slot = location;
        }
    }

    fn allocate_run_slot(&mut self, class_index: usize, bytes: &[u8]) -> RawRunSlot {
        while let Some(run_index) = self.available_runs[class_index].pop() {
            let Some(run) = self.runs.get_mut(run_index) else {
                continue;
            };

            if !run.has_free_slot() {
                continue;
            }

            let Some(slot_index) = run.first_free_slot() else {
                continue;
            };

            if run.allocate_slot(slot_index, bytes) {
                if run.has_free_slot() {
                    self.available_runs[class_index].push(run_index);
                }

                return RawRunSlot::new(run_index, slot_index);
            }
        }

        let size_class = self.size_classes.classes[class_index].bytes;
        let mut run = RawRun::new(size_class, self.run_bytes);
        let slot_index = run.first_free_slot().unwrap_or(0);
        let allocated = run.allocate_slot(slot_index, bytes);
        debug_assert!(allocated, "fresh raw run must accept its first slot");

        let run_index = if let Some(index) = self.free_run_ids.pop() {
            self.runs[index] = run;
            index
        } else {
            self.runs.push(run);
            self.runs.len() - 1
        };

        if self.runs[run_index].has_free_slot() {
            self.available_runs[class_index].push(run_index);
        }

        RawRunSlot::new(run_index, slot_index)
    }

    fn allocate_extent(&mut self, bytes: &[u8]) -> RawExtentId {
        if let Some(id) = self.free_extent_ids.pop() {
            let extent_id = RawExtentId::new(id);
            let chunk_bytes = self.chunk_bytes;
            if let Some(extent) = self.extent_mut(extent_id) {
                extent.replace(bytes, chunk_bytes);
            }

            return extent_id;
        }

        let extent_id = RawExtentId::new(self.next_unused_extent_id);
        self.next_unused_extent_id = self.next_unused_extent_id.saturating_add(1);
        self.extents.push(RawExtent::new(bytes, self.chunk_bytes));

        extent_id
    }

    fn extent(&self, extent_id: RawExtentId) -> Option<&RawExtent> {
        if extent_id.id() == 0 {
            return None;
        }

        self.extents.get((extent_id.id() - 1) as usize)
    }

    fn extent_mut(&mut self, extent_id: RawExtentId) -> Option<&mut RawExtent> {
        if extent_id.id() == 0 {
            return None;
        }

        self.extents.get_mut((extent_id.id() - 1) as usize)
    }

    fn class_index_for_run(&self, size_class: usize) -> Option<usize> {
        self.size_classes
            .classes
            .iter()
            .position(|class| class.bytes == size_class)
    }

    fn rebuild_run_directories(&mut self) {
        self.available_runs = vec![Vec::new(); self.size_classes.classes.len()];
        self.free_run_ids.clear();

        for (index, run) in self.runs.iter().enumerate() {
            if run.is_vacant() {
                self.free_run_ids.push(index);
                continue;
            }

            if !run.has_free_slot() {
                continue;
            }

            if let Some(class_index) = self.class_index_for_run(run.size_class()) {
                self.available_runs[class_index].push(index);
            }
        }
    }

    fn recompute_retained_bytes(&mut self) {
        let mut retained_bytes = 0usize;

        retained_bytes += self.size_classes.retained_bytes();
        retained_bytes += self.available_runs.capacity() * size_of::<Vec<usize>>();
        retained_bytes += self.free_run_ids.capacity() * size_of::<usize>();
        retained_bytes += self.extents.capacity() * size_of::<RawExtent>();
        retained_bytes += self.free_extent_ids.capacity() * size_of::<u64>();
        retained_bytes += self.locations.capacity() * size_of::<RawLocation>();
        retained_bytes += self.free_ids.capacity() * size_of::<u64>();

        for queue in &self.available_runs {
            retained_bytes += queue.capacity() * size_of::<usize>();
        }

        for run in &self.runs {
            retained_bytes += run.retained_bytes();
        }

        for extent in &self.extents {
            retained_bytes += extent.retained_bytes();
        }

        self.retained_bytes = retained_bytes as u64;
    }
}

// encode one packed value vector into bytes
fn encode_values(values: &[Value]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * Value::BYTE_LEN);

    for value in values {
        bytes.extend_from_slice(&value.to_byte_array());
    }

    bytes
}

// decode one byte slice into packed values
fn decode_values(bytes: &[u8]) -> Option<Vec<Value>> {
    if bytes.len() % Value::BYTE_LEN != 0 {
        return None;
    }

    let mut values = Vec::with_capacity(bytes.len() / Value::BYTE_LEN);

    for chunk in bytes.chunks(Value::BYTE_LEN) {
        values.push(Value::from_byte_slice(chunk)?);
    }

    Some(values)
}
