use std::borrow::Cow;
use std::mem::size_of;
use std::sync::Arc;

use destack_mir::LayoutId;

use super::{
    GcState, ManagedExtent, ManagedExtentId, ManagedExtentImage, ManagedImage, ManagedRun,
    ManagedRunImage, ReferenceMap, ReferenceMapId, ReferenceMapTable,
};
use crate::heap::{
    HeapCaptureError, HeapLayoutOptions, ManagedSpaceUsage, SizeClassTable, TreeVector,
};
use crate::value::{ManagedReference, Value};

/// The first non-null managed reference id.
pub(crate) const FIRST_ALLOCATED_REFERENCE_ID: u64 = 1;

/// The first non-null managed extent id.
const FIRST_ALLOCATED_EXTENT_ID: u64 = 1;

/// One stable run slot location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ManagedRunSlot {
    /// The containing run index.
    run_index: u32,
    /// The slot index inside the run.
    slot_index: u32,
}

impl ManagedRunSlot {
    /// Create one run slot location.
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

/// One stable managed allocation location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ManagedLocation {
    /// One vacant directory slot.
    Vacant,
    /// One run-backed allocation.
    Run(ManagedRunSlot),
    /// One extent-backed allocation.
    Extent(ManagedExtentId),
}

/// One live managed allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured run width.
    pub(crate) run_bytes: usize,
    /// The configured extent chunk width.
    pub(crate) chunk_bytes: usize,
    /// The interned managed reference maps.
    pub(crate) reference_map_table: ReferenceMapTable,
    /// The live managed runs.
    pub(crate) runs: Vec<ManagedRun>,
    /// The reusable non-full runs per size class.
    available_runs: Vec<Vec<usize>>,
    /// The vacant run slots available for reuse.
    free_run_ids: Vec<usize>,
    /// The live managed extents.
    pub(crate) extents: Vec<ManagedExtent>,
    /// The free managed extent ids available for reuse.
    free_extent_ids: Vec<u64>,
    /// Stable allocation locations keyed by reference id minus one.
    pub(crate) locations: Vec<ManagedLocation>,
    /// The free managed reference ids available for reuse.
    pub(crate) free_ids: Vec<u64>,
    /// The next managed reference id to allocate.
    pub(crate) next_unused_id: u64,
    /// The next managed extent id to allocate.
    next_unused_extent_id: u64,
    /// The number of allocated managed references.
    pub(crate) allocated_count: usize,
    /// The number of allocated managed bytes.
    pub(crate) allocated_bytes: u64,
    /// The exact retained managed bytes.
    pub(crate) retained_bytes: u64,
    /// The live GC state.
    pub(crate) gc_state: GcState,
    /// The pending mark queue.
    pub(crate) mark_queue: Vec<ManagedReference>,
}

impl Default for ManagedSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedSpace {
    /// Create one managed space with the default layout options.
    pub fn new() -> Self {
        Self::with_layout(&HeapLayoutOptions::default())
    }

    /// Create one managed space with explicit layout options.
    pub fn with_layout(options: &HeapLayoutOptions) -> Self {
        let mut space = Self {
            size_classes: options.size_classes.clone(),
            run_bytes: options.managed_run_bytes,
            chunk_bytes: options.chunk_bytes,
            reference_map_table: ReferenceMapTable::new(),
            runs: Vec::new(),
            available_runs: vec![Vec::new(); options.size_classes.classes.len()],
            free_run_ids: Vec::new(),
            extents: Vec::new(),
            free_extent_ids: Vec::new(),
            locations: Vec::new(),
            free_ids: Vec::new(),
            next_unused_id: FIRST_ALLOCATED_REFERENCE_ID,
            next_unused_extent_id: FIRST_ALLOCATED_EXTENT_ID,
            allocated_count: 0,
            allocated_bytes: 0,
            retained_bytes: 0,
            gc_state: GcState::default(),
            mark_queue: Vec::new(),
        };

        // exact retained bytes
        space.recompute_retained_bytes();

        space
    }

    /// Restore one managed space from one immutable image.
    pub fn from_image(image: &ManagedImage) -> Self {
        let mut space = Self {
            size_classes: image.size_classes.clone(),
            run_bytes: image.run_bytes,
            chunk_bytes: image.chunk_bytes,
            reference_map_table: ReferenceMapTable::from_maps(image.reference_maps.clone()),
            runs: image
                .runs
                .iter()
                .cloned()
                .map(ManagedRun::from_image)
                .collect(),
            available_runs: vec![Vec::new(); image.size_classes.classes.len()],
            free_run_ids: Vec::new(),
            extents: image
                .extents
                .iter()
                .map(ManagedExtent::from_extent_image)
                .collect(),
            free_extent_ids: image.free_extent_ids.iter().copied().collect(),
            locations: image.locations.iter().copied().collect(),
            free_ids: image.free_ids.iter().copied().collect(),
            next_unused_id: image.next_unused_id,
            next_unused_extent_id: image.next_unused_extent_id,
            allocated_count: image.allocated_count,
            allocated_bytes: image.allocated_bytes,
            retained_bytes: 0,
            gc_state: image.gc_state.clone(),
            mark_queue: Vec::new(),
        };

        // rebuild run directories
        space.rebuild_run_directories();

        // exact retained bytes
        space.recompute_retained_bytes();

        space
    }

    /// Return the number of live managed allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the logical live managed allocation bytes.
    pub fn allocation_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact retained managed bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Return the current GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Return the exact live usage for this managed space.
    pub fn usage(&self) -> ManagedSpaceUsage {
        ManagedSpaceUsage {
            allocation_count: self.allocated_count,
            allocation_bytes: self.allocated_bytes,
            retained_bytes: self.retained_bytes,
        }
    }

    /// Capture one immutable managed-space image.
    pub fn image(&mut self, base: Option<&ManagedImage>) -> Result<ManagedImage, HeapCaptureError> {
        // reject active collection work
        if self.gc_state.phase != super::GcPhase::Idle {
            return Err(HeapCaptureError::GcActive);
        }

        // reject active pins
        if self.active_pins() > 0 {
            return Err(HeapCaptureError::PinnedManagedReferences);
        }

        // capture runs
        let runs = self
            .runs
            .iter_mut()
            .enumerate()
            .map(|(index, run)| {
                let _ = base.and_then(|image| image.runs.get(index));
                run.image()
            })
            .collect::<Vec<_>>();

        // capture extents
        let extents = self
            .extents
            .iter_mut()
            .enumerate()
            .map(|(index, extent)| {
                let _ = base.and_then(|image| image.extents.get(index));
                extent.extent_image()
            })
            .collect::<Vec<_>>();

        Ok(ManagedImage {
            size_classes: self.size_classes.clone(),
            run_bytes: self.run_bytes,
            chunk_bytes: self.chunk_bytes,
            runs: TreeVector::from_values_by(
                &runs,
                base.map(|image| &image.runs),
                ManagedRunImage::shares_storage_with,
            ),
            extents: TreeVector::from_values_by(
                &extents,
                base.map(|image| &image.extents),
                ManagedExtentImage::shares_storage_with,
            ),
            locations: Arc::from(self.locations.as_slice()),
            free_ids: Arc::from(self.free_ids.as_slice()),
            free_extent_ids: Arc::from(self.free_extent_ids.as_slice()),
            next_unused_id: self.next_unused_id,
            next_unused_extent_id: self.next_unused_extent_id,
            allocated_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            reference_maps: self.reference_map_table.snapshot(),
            gc_state: self.gc_state.clone(),
        })
    }

    /// Return the currently allocated managed references.
    pub fn allocated_references(&self) -> Vec<ManagedReference> {
        let mut references = Vec::with_capacity(self.allocated_count);

        // collect live ids
        for id in FIRST_ALLOCATED_REFERENCE_ID..self.next_unused_id {
            let handle = ManagedReference::new(id);

            if self.is_allocated(handle) {
                references.push(handle);
            }
        }

        references
    }

    /// Allocate one managed byte allocation.
    pub fn allocate_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        let reference_map_id = self.reference_map_table.intern(reference_map);

        // choose run or extent
        let location = if let Some(class_index) = self.size_classes.class_index_for(bytes.len()) {
            ManagedLocation::Run(self.allocate_run_slot(
                class_index,
                bytes,
                reference_map_id,
                layout_id,
            ))
        } else {
            ManagedLocation::Extent(self.allocate_extent(bytes, reference_map_id, layout_id))
        };

        // publish stable id
        let handle = self.allocate_location(location);
        self.allocated_count += 1;
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes.len() as u64);
        self.recompute_retained_bytes();

        handle
    }

    /// Allocate one zeroed managed byte allocation.
    pub fn allocate_zeroed(
        &mut self,
        byte_len: usize,
        reference_map: ReferenceMap,
        layout_id: Option<LayoutId>,
    ) -> ManagedReference {
        self.allocate_bytes(&vec![0; byte_len], reference_map, layout_id)
    }

    /// Free one managed allocation by handle.
    pub(crate) fn free(&mut self, handle: ManagedReference) -> bool {
        let Some(location) = self.location(handle) else {
            return false;
        };

        let Some(old_len) = self.byte_len(ManagedReference::new(handle.id())) else {
            return false;
        };

        // free run or extent storage
        match location {
            ManagedLocation::Vacant => return false,
            ManagedLocation::Run(slot) => {
                let run_index = slot.run_index();
                let Some(size_class) = self.runs.get(run_index).map(ManagedRun::size_class) else {
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
                    self.runs[run_index] = ManagedRun::vacant();
                    self.free_run_ids.push(run_index);
                } else if run.has_free_slot() {
                    self.available_runs[class_index].push(run_index);
                }
            }
            ManagedLocation::Extent(extent_id) => {
                let Some(extent) = self.extent_mut(extent_id) else {
                    return false;
                };

                extent.free();
                self.free_extent_ids.push(extent_id.id());
            }
        }

        // clear stable location
        if let Some(location_slot) = self.location_entry_mut(handle.id()) {
            *location_slot = ManagedLocation::Vacant;
        }

        self.free_ids.push(handle.id());
        self.allocated_count = self.allocated_count.saturating_sub(1);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(old_len as u64);
        self.recompute_retained_bytes();

        true
    }

    /// Report whether one managed reference is currently allocated.
    pub fn is_allocated(&self, handle: ManagedReference) -> bool {
        let Some(location) = self.location(handle) else {
            return false;
        };

        match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Run(slot) => self
                .runs
                .get(slot.run_index())
                .map(|run| run.is_occupied(slot.slot_index()))
                .unwrap_or(false),
            ManagedLocation::Extent(extent_id) => self
                .extent(extent_id)
                .map(ManagedExtent::is_allocated)
                .unwrap_or(false),
        }
    }

    /// Return the logical byte length for one managed allocation.
    pub fn byte_len(&self, handle: ManagedReference) -> Option<usize> {
        let base = ManagedReference::new(handle.id());
        let offset = handle.byte_offset();
        let location = self.location(base)?;

        let len = match location {
            ManagedLocation::Vacant => return None,
            ManagedLocation::Run(slot) => {
                self.runs.get(slot.run_index())?.len(slot.slot_index())?
            }
            ManagedLocation::Extent(extent_id) => self.extent(extent_id)?.len(),
        };

        len.checked_sub(offset)
    }

    /// Return the bytes for one managed allocation.
    pub fn bytes(&self, handle: ManagedReference) -> Option<Cow<'_, [u8]>> {
        let base = ManagedReference::new(handle.id());
        let offset = handle.byte_offset();
        let location = self.location(base)?;

        let bytes = match location {
            ManagedLocation::Vacant => return None,
            ManagedLocation::Run(slot) => {
                Cow::Borrowed(self.runs.get(slot.run_index())?.bytes(slot.slot_index())?)
            }
            ManagedLocation::Extent(extent_id) => self.extent(extent_id)?.bytes(),
        };

        match bytes {
            Cow::Borrowed(bytes) => Some(Cow::Borrowed(bytes.get(offset..)?)),
            Cow::Owned(bytes) => Some(Cow::Owned(bytes.get(offset..)?.to_vec())),
        }
    }

    /// Return one owned copy of the bytes for one managed allocation.
    pub fn bytes_to_vec(&self, handle: ManagedReference) -> Option<Vec<u8>> {
        Some(self.bytes(handle)?.into_owned())
    }

    /// Return one byte by offset within one managed allocation.
    pub fn byte_at(&self, handle: ManagedReference, index: usize) -> Option<u8> {
        self.bytes(handle)?.as_ref().get(index).copied()
    }

    /// Set one byte inside one managed allocation.
    pub fn set_byte(&mut self, handle: ManagedReference, index: usize, byte: u8) -> bool {
        let base = ManagedReference::new(handle.id());
        let offset = handle.byte_offset().saturating_add(index);
        let Some(location) = self.location(base) else {
            return false;
        };

        let updated = match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Run(slot) => self
                .runs
                .get_mut(slot.run_index())
                .map(|run| run.set_byte(slot.slot_index(), offset, byte))
                .unwrap_or(false),
            ManagedLocation::Extent(extent_id) => self
                .extent_mut(extent_id)
                .map(|extent| extent.set(offset, byte))
                .unwrap_or(false),
        };

        updated
    }

    /// Return the reference map for one managed allocation.
    pub fn reference_map(&self, handle: ManagedReference) -> Option<&ReferenceMap> {
        let base = ManagedReference::new(handle.id());
        let location = self.location(base)?;
        let map_id = match location {
            ManagedLocation::Vacant => return None,
            ManagedLocation::Run(slot) => self
                .runs
                .get(slot.run_index())?
                .trace_id(slot.slot_index())?,
            ManagedLocation::Extent(extent_id) => self.extent(extent_id)?.trace_id(),
        };

        self.reference_map_table.get(map_id)
    }

    /// Return the layout id for one managed allocation.
    pub fn layout_id(&self, handle: ManagedReference) -> Option<LayoutId> {
        let base = ManagedReference::new(handle.id());
        let location = self.location(base)?;

        match location {
            ManagedLocation::Vacant => None,
            ManagedLocation::Run(slot) => self
                .runs
                .get(slot.run_index())?
                .layout_id(slot.slot_index()),
            ManagedLocation::Extent(extent_id) => self.extent(extent_id)?.layout_id(),
        }
    }

    /// Set the layout id for one managed allocation.
    pub fn set_layout_id(&mut self, handle: ManagedReference, layout_id: LayoutId) -> bool {
        let base = ManagedReference::new(handle.id());
        let Some(location) = self.location(base) else {
            return false;
        };

        let updated = match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Run(slot) => self
                .runs
                .get_mut(slot.run_index())
                .map(|run| run.set_layout_id(slot.slot_index(), layout_id))
                .unwrap_or(false),
            ManagedLocation::Extent(extent_id) => self
                .extent_mut(extent_id)
                .map(|extent| {
                    extent.set_layout_id(layout_id);
                    true
                })
                .unwrap_or(false),
        };

        if updated {
            self.recompute_retained_bytes();
        }

        updated
    }

    /// Return the number of packed values in one managed allocation.
    pub fn packed_value_count(&self, handle: ManagedReference) -> Option<usize> {
        match self.reference_map(ManagedReference::new(handle.id()))? {
            ReferenceMap::ValueArray { count } => Some(*count as usize),
            _ => None,
        }
    }

    /// Return one packed value by index.
    pub fn packed_value_at(&self, handle: ManagedReference, index: usize) -> Option<Value> {
        let value_count = self.packed_value_count(ManagedReference::new(handle.id()))?;
        if index >= value_count {
            return None;
        }

        let bytes = self.bytes(ManagedReference::new(handle.id()))?;
        let start = index.checked_mul(Value::BYTE_LEN)?;
        let end = start.checked_add(Value::BYTE_LEN)?;

        Value::from_byte_slice(bytes.as_ref().get(start..end)?)
    }

    /// Return one owned copy of the packed values.
    pub fn packed_values_to_vec(&self, handle: ManagedReference) -> Option<Vec<Value>> {
        let count = self.packed_value_count(ManagedReference::new(handle.id()))?;
        let mut values = Vec::with_capacity(count);

        // decode each packed value
        for index in 0..count {
            values.push(self.packed_value_at(handle, index)?);
        }

        Some(values)
    }

    /// Set one packed value by index.
    pub fn set_packed_value(
        &mut self,
        handle: ManagedReference,
        index: usize,
        value: Value,
    ) -> bool {
        let count = match self.packed_value_count(ManagedReference::new(handle.id())) {
            Some(count) => count,
            None => return false,
        };
        if index >= count {
            return false;
        }

        let bytes = value.to_byte_array();
        let start = index * Value::BYTE_LEN;

        // write one packed value word
        for (offset, byte) in bytes.into_iter().enumerate() {
            if !self.set_byte(handle, start + offset, byte) {
                return false;
            }
        }

        true
    }

    /// Resize one packed managed allocation.
    pub fn resize_packed_values(&mut self, handle: ManagedReference, count: usize) -> bool {
        let Some(mut values) = self.packed_values_to_vec(ManagedReference::new(handle.id())) else {
            return false;
        };
        values.resize(count, Value::VOID);

        self.replace_packed_values(handle, &values)
    }

    /// Replace one managed allocation with packed values.
    pub fn replace_packed_values(&mut self, handle: ManagedReference, values: &[Value]) -> bool {
        let Some(base) = self.location(ManagedReference::new(handle.id())) else {
            return false;
        };
        let old_len = self
            .byte_len(ManagedReference::new(handle.id()))
            .unwrap_or_default() as u64;

        let bytes = encode_values(values);
        let reference_map_id = self
            .reference_map_table
            .intern(ReferenceMap::value_array(values.len()));
        let layout_id = self.layout_id(ManagedReference::new(handle.id()));

        let replaced = match base {
            ManagedLocation::Vacant => false,
            ManagedLocation::Run(slot) => match self.size_classes.class_index_for(bytes.len()) {
                Some(class_index) => {
                    let run_size_class =
                        self.runs.get(slot.run_index()).map(ManagedRun::size_class);
                    let target_size_class = self
                        .size_classes
                        .classes
                        .get(class_index)
                        .map(|class| class.bytes);

                    if run_size_class == target_size_class {
                        self.runs
                            .get_mut(slot.run_index())
                            .map(|run| {
                                run.replace_slot(
                                    slot.slot_index(),
                                    &bytes,
                                    reference_map_id,
                                    layout_id,
                                )
                            })
                            .unwrap_or(false)
                    } else {
                        let new_slot = self.allocate_run_slot(
                            class_index,
                            &bytes,
                            reference_map_id,
                            layout_id,
                        );
                        let freed = self
                            .runs
                            .get_mut(slot.run_index())
                            .map(|run| run.free_slot(slot.slot_index()))
                            .unwrap_or(false);

                        if freed {
                            self.move_location(handle.id(), ManagedLocation::Run(new_slot));
                        }

                        freed
                    }
                }
                None => {
                    let extent_id = self.allocate_extent(&bytes, reference_map_id, layout_id);
                    let freed = self
                        .runs
                        .get_mut(slot.run_index())
                        .map(|run| run.free_slot(slot.slot_index()))
                        .unwrap_or(false);

                    if freed {
                        self.move_location(handle.id(), ManagedLocation::Extent(extent_id));
                    }

                    freed
                }
            },
            ManagedLocation::Extent(extent_id) => {
                if let Some(class_index) = self.size_classes.class_index_for(bytes.len()) {
                    let new_slot =
                        self.allocate_run_slot(class_index, &bytes, reference_map_id, layout_id);
                    if let Some(extent) = self.extent_mut(extent_id) {
                        extent.free();
                        self.free_extent_ids.push(extent_id.id());
                        self.move_location(handle.id(), ManagedLocation::Run(new_slot));
                        true
                    } else {
                        false
                    }
                } else {
                    let chunk_bytes = self.chunk_bytes;
                    self.extent_mut(extent_id)
                        .map(|extent| {
                            extent.replace(&bytes, chunk_bytes, reference_map_id, layout_id);
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

    /// Pin one managed allocation for raw exposure.
    pub fn pin(&mut self, handle: ManagedReference) -> bool {
        let Some(location) = self.location(ManagedReference::new(handle.id())) else {
            return false;
        };

        match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Run(slot) => self
                .runs
                .get_mut(slot.run_index())
                .map(|run| run.pin(slot.slot_index()))
                .unwrap_or(false),
            ManagedLocation::Extent(extent_id) => self
                .extent_mut(extent_id)
                .map(ManagedExtent::pin)
                .unwrap_or(false),
        }
    }

    /// Release one managed allocation pin.
    pub fn unpin(&mut self, handle: ManagedReference) -> bool {
        let Some(location) = self.location(ManagedReference::new(handle.id())) else {
            return false;
        };

        match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Run(slot) => self
                .runs
                .get_mut(slot.run_index())
                .map(|run| run.unpin(slot.slot_index()))
                .unwrap_or(false),
            ManagedLocation::Extent(extent_id) => self
                .extent_mut(extent_id)
                .map(ManagedExtent::unpin)
                .unwrap_or(false),
        }
    }

    // directory helpers

    fn allocate_location(&mut self, location: ManagedLocation) -> ManagedReference {
        if let Some(id) = self.free_ids.pop() {
            if let Some(slot) = self.location_entry_mut(id) {
                *slot = location;
            }

            return ManagedReference::new(id);
        }

        let id = self.next_unused_id;
        self.next_unused_id = self.next_unused_id.saturating_add(1);
        self.locations.push(location);

        ManagedReference::new(id)
    }

    pub(crate) fn location(&self, handle: ManagedReference) -> Option<ManagedLocation> {
        if handle.id() == 0 {
            return None;
        }

        self.locations.get((handle.id() - 1) as usize).copied()
    }

    pub(crate) fn location_entry_mut(&mut self, id: u64) -> Option<&mut ManagedLocation> {
        if id == 0 {
            return None;
        }

        self.locations.get_mut((id - 1) as usize)
    }

    fn move_location(&mut self, id: u64, location: ManagedLocation) {
        if let Some(location_slot) = self.location_entry_mut(id) {
            *location_slot = location;
        }
    }

    fn allocate_run_slot(
        &mut self,
        class_index: usize,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> ManagedRunSlot {
        // try one reusable run first
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

            if run.allocate_slot(slot_index, bytes, trace_id, layout_id) {
                if run.has_free_slot() {
                    self.available_runs[class_index].push(run_index);
                }

                return ManagedRunSlot::new(run_index, slot_index);
            }
        }

        // otherwise allocate a fresh run
        let size_class = self.size_classes.classes[class_index].bytes;
        let mut run = ManagedRun::new(size_class, self.run_bytes);
        let slot_index = run.first_free_slot().unwrap_or(0);
        let allocated = run.allocate_slot(slot_index, bytes, trace_id, layout_id);
        debug_assert!(allocated, "fresh managed run must accept its first slot");

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

        ManagedRunSlot::new(run_index, slot_index)
    }

    fn allocate_extent(
        &mut self,
        bytes: &[u8],
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> ManagedExtentId {
        if let Some(id) = self.free_extent_ids.pop() {
            let extent_id = ManagedExtentId::new(id);
            let chunk_bytes = self.chunk_bytes;
            if let Some(extent) = self.extent_mut(extent_id) {
                extent.replace(bytes, chunk_bytes, trace_id, layout_id);
            }

            return extent_id;
        }

        let extent_id = ManagedExtentId::new(self.next_unused_extent_id);
        self.next_unused_extent_id = self.next_unused_extent_id.saturating_add(1);
        self.extents.push(ManagedExtent::new(
            bytes,
            self.chunk_bytes,
            trace_id,
            layout_id,
        ));

        extent_id
    }

    pub(crate) fn extent(&self, extent_id: ManagedExtentId) -> Option<&ManagedExtent> {
        if extent_id.id() == 0 {
            return None;
        }

        self.extents.get((extent_id.id() - 1) as usize)
    }

    pub(crate) fn extent_mut(&mut self, extent_id: ManagedExtentId) -> Option<&mut ManagedExtent> {
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

    pub(crate) fn free_storage(&mut self, location: ManagedLocation) {
        match location {
            ManagedLocation::Vacant => {}
            ManagedLocation::Run(slot) => {
                let run_index = slot.run_index();
                let Some(size_class) = self.runs.get(run_index).map(ManagedRun::size_class) else {
                    return;
                };
                let Some(class_index) = self.class_index_for_run(size_class) else {
                    return;
                };
                let Some(run) = self.runs.get_mut(run_index) else {
                    return;
                };

                if !run.free_slot(slot.slot_index()) {
                    return;
                }

                if run.is_empty() {
                    self.runs[run_index] = ManagedRun::vacant();
                    self.free_run_ids.push(run_index);
                } else if run.has_free_slot() {
                    self.available_runs[class_index].push(run_index);
                }
            }
            ManagedLocation::Extent(extent_id) => {
                let Some(extent) = self.extent_mut(extent_id) else {
                    return;
                };

                extent.free();
                self.free_extent_ids.push(extent_id.id());
            }
        }
    }

    fn active_pins(&self) -> usize {
        let run_pins = self.runs.iter().map(ManagedRun::active_pins).sum::<usize>();
        let extent_pins = self
            .extents
            .iter()
            .map(ManagedExtent::active_pins)
            .sum::<usize>();

        run_pins.saturating_add(extent_pins)
    }

    fn rebuild_run_directories(&mut self) {
        self.available_runs = vec![Vec::new(); self.size_classes.classes.len()];
        self.free_run_ids.clear();

        // rebuild reusable run directories
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

    pub(crate) fn recompute_retained_bytes(&mut self) {
        let mut retained_bytes = 0usize;

        // core directories
        retained_bytes += self.size_classes.retained_bytes();
        retained_bytes += self.available_runs.capacity() * size_of::<Vec<usize>>();
        retained_bytes += self.free_run_ids.capacity() * size_of::<usize>();
        retained_bytes += self.extents.capacity() * size_of::<ManagedExtent>();
        retained_bytes += self.free_extent_ids.capacity() * size_of::<u64>();
        retained_bytes += self.locations.capacity() * size_of::<ManagedLocation>();
        retained_bytes += self.free_ids.capacity() * size_of::<u64>();
        retained_bytes += self.mark_queue.capacity() * size_of::<ManagedReference>();

        // run vectors
        for queue in &self.available_runs {
            retained_bytes += queue.capacity() * size_of::<usize>();
        }

        // leaf backing
        for run in &self.runs {
            retained_bytes += run.retained_bytes();
        }

        for extent in &self.extents {
            retained_bytes += extent.retained_bytes();
        }

        retained_bytes += self.reference_map_table.retained_bytes();

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
