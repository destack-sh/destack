use std::collections::{HashMap, HashSet};

use destack_core::{EntryStore, SectionPacker};
use destack_mir as mir;
use destack_program::vm::{Cell, FrameEntryBuilder, MoveSlot};
use destack_program::{
    FrameLayout, FrameLayoutId, FrameMaterialization, FrameSlot, FrameSlotId, FrameStateId,
    FrameTable,
};

use super::super::ProgramLinker;
use super::StorageLayout;
use crate::LinkResult;

/// Link VM frame layouts and frame materialization entries.
#[derive(Debug)]
pub(crate) struct FrameLinker<'a> {
    /// MIR tree being linked.
    tree: &'a mir::Tree,
    /// Program linker owning dense program id projection.
    program: &'a ProgramLinker,
    /// Lowered value storage layouts keyed by MIR type id.
    storage: &'a HashMap<mir::TypeId, StorageLayout>,
    /// Linked frame layouts.
    layouts: Vec<FrameLayout>,
    /// Linked frame materializations.
    materializations: Vec<FrameMaterialization>,
    /// Linked frame slots.
    slots: EntryStore<FrameSlot>,
    /// Linked copied frame slots.
    copied_slots: EntryStore<FrameSlotId>,
}

impl<'a> FrameLinker<'a> {
    /// Create one frame linker.
    pub(crate) fn new(
        tree: &'a mir::Tree,
        program: &'a ProgramLinker,
        storage: &'a HashMap<mir::TypeId, StorageLayout>,
    ) -> Self {
        Self {
            tree,
            program,
            storage,
            layouts: Vec::new(),
            materializations: Vec::new(),
            slots: EntryStore::new(),
            copied_slots: EntryStore::new(),
        }
    }

    /// Return the next frame layout id.
    pub(crate) fn next_layout_id(&self) -> FrameLayoutId {
        FrameLayoutId::from(self.layouts.len() as u32)
    }

    /// Finish the frame table.
    pub(crate) fn finish(self, sections: &mut SectionPacker) -> FrameTable {
        let Self {
            layouts,
            materializations,
            slots,
            copied_slots,
            ..
        } = self;

        FrameTable::from_entries(
            sections,
            &layouts,
            &materializations,
            slots.entries(),
            copied_slots.entries(),
        )
    }

    /// Append one frame layout.
    pub(crate) fn push_layout(&mut self, layout: FrameLayout) {
        self.layouts.push(layout);
    }

    /// Return whether one program frame slot is stored as one VM cell.
    pub(crate) fn slot_is_cell(&self, slot: &FrameSlot) -> bool {
        self.program
            .type_by_id(slot.ty)
            .and_then(|ty| self.storage.get(&ty))
            .is_some_and(StorageLayout::is_cell)
    }

    /// Return one value slot inside one frame layout.
    pub(crate) fn value_slot(&self, layout: &FrameLayout, value: u32) -> Option<&FrameSlot> {
        let slots = layout.slots(self.slots.entries());

        layout.value(slots, value)
    }

    /// Return one local slot inside one frame layout.
    pub(crate) fn local_slot(&self, layout: &FrameLayout, local: u32) -> Option<&FrameSlot> {
        let slots = layout.slots(self.slots.entries());

        layout.local(slots, local)
    }

    /// Return the lowered frame slot for one SSA value.
    pub(crate) fn move_slot(
        &self,
        frame_layout: &FrameLayout,
        value: mir::Value,
    ) -> LinkResult<MoveSlot> {
        let slots = frame_layout.slots(self.slots.entries());
        let slot = frame_layout
            .value(slots, value.0)
            .ok_or_else(|| self.program.invalid_instruction("frame value slot"))?;
        let is_cell = self.slot_is_cell(slot);

        Ok(MoveSlot::new(
            slot.ty,
            slot.offset,
            slot.byte_len(),
            is_cell,
        ))
    }

    /// Build one byte frame layout for one function.
    pub(crate) fn build_layout(
        &mut self,
        function: &mir::Function,
        value_types: &[mir::TypeId],
    ) -> LinkResult<FrameLayout> {
        let mut byte_len = 0usize;

        let slot_count = value_types.len()
            + function.locals().len()
            + usize::from(function.environment.is_some());
        let mut slots = Vec::with_capacity(slot_count);

        // allocate SSA value slots
        for value_type in value_types {
            let slot = self.slot(*value_type, &mut byte_len)?;
            slots.push(slot);
        }

        // allocate local slots
        let value_count = u32::try_from(value_types.len())
            .map_err(|_| self.program.layout_overflow("frame value slots"))?;
        for local_id in function.locals() {
            let local = self.tree.get(*local_id);
            let slot = self.slot(local.ty, &mut byte_len)?;
            slots.push(slot);
        }

        // allocate optional closure environment slot
        let local_count = u32::try_from(function.locals().len())
            .map_err(|_| self.program.layout_overflow("frame local slots"))?;
        let environment_slot = function
            .environment
            .as_ref()
            .map(|environment| self.slot(*environment, &mut byte_len))
            .transpose()?;
        let environment_slot = environment_slot.map(|slot| {
            let id = (slots.len() as u32).into();
            slots.push(slot);

            id
        });

        let slots = self.slots.append(slots);

        Ok(FrameLayout {
            slots,
            value_count,
            local_count,
            environment_slot: environment_slot.into(),
            byte_len: u32::try_from(byte_len)
                .map_err(|_| self.program.layout_overflow("frame byte length"))?,
        })
    }

    /// Append one frame materialization entry.
    pub(crate) fn append_materialization(
        &mut self,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
        frame_state: FrameStateId,
        frame_entry: Option<&FrameEntryBuilder>,
        source_point: Option<u32>,
    ) -> LinkResult<()> {
        let materialized_values =
            self.materialized_values(frame_layout, liveness, block, frame_entry, source_point)?;
        let materialized_locals = self.materialized_locals(liveness, block);
        let copied_slots: Vec<_> = frame_layout
            .slot_ids()
            .filter(|slot| {
                self.is_materialized_slot(
                    frame_layout,
                    *slot,
                    &materialized_values,
                    &materialized_locals,
                )
            })
            .collect();
        let copied_slots = self.copied_slots.append(copied_slots);

        let materialization = FrameMaterialization {
            frame_layout: frame_layout_id,
            copied_slots,
        };
        debug_assert_eq!(self.materializations.len(), frame_state.0 as usize);
        self.materializations.push(materialization);

        Ok(())
    }

    /// Allocate one typed slot inside a frame layout.
    fn slot(&self, ty: mir::TypeId, byte_len: &mut usize) -> LinkResult<FrameSlot> {
        let layout = self.storage.get(&ty).ok_or_else(|| {
            self.program
                .invalid_input(format!("frame layout for {ty:?}"))
        })?;
        let slot_alignment = if layout.is_cell() {
            layout.alignment().max(Cell::BYTE_LEN)
        } else {
            layout.alignment()
        };
        let slot_len = if layout.is_cell() {
            layout.byte_len().max(Cell::BYTE_LEN)
        } else {
            layout.byte_len()
        };

        let offset = align_offset(*byte_len, slot_alignment);
        let end = offset + slot_len;
        *byte_len = end;

        Ok(FrameSlot {
            offset: u32::try_from(offset)
                .map_err(|_| self.program.layout_overflow("frame slot offset"))?,
            byte_len: u32::try_from(slot_len)
                .map_err(|_| self.program.layout_overflow("frame slot byte length"))?,
            alignment: u16::try_from(slot_alignment)
                .map_err(|_| self.program.layout_overflow("frame slot alignment"))?,
            ty: self.program.type_id(ty),
        })
    }

    /// Return the values materialized at one resume state.
    fn materialized_values(
        &self,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
        frame_entry: Option<&FrameEntryBuilder>,
        source_point: Option<u32>,
    ) -> LinkResult<HashSet<mir::Value>> {
        let Some(frame_entry) = frame_entry else {
            let source_point = source_point
                .ok_or_else(|| self.program.invalid_instruction("frame source point"))?;

            return Ok(liveness.value_live_before_instruction(
                self.tree,
                block,
                source_point as usize,
            ));
        };

        // materialize live in values that survive the entry edge
        let live_in = liveness.value_live_in(block);
        let entry_block = self.tree.get(block);
        let parameter_values = entry_block
            .parameters
            .iter()
            .map(|parameter| parameter.value)
            .collect::<HashSet<_>>();
        let mut values = live_in
            .difference(&parameter_values)
            .copied()
            .collect::<HashSet<_>>();

        // entry bindings keep their source values live
        for binding in &frame_entry.bindings {
            let source = frame_layout
                .value_for_slot(binding.source)
                .ok_or_else(|| self.program.invalid_instruction("frame binding source"))?;
            let source = mir::Value::new(source);

            values.insert(source);
        }

        Ok(values)
    }

    /// Return the locals materialized at one resume state.
    fn materialized_locals(
        &self,
        liveness: &mir::FunctionLiveness,
        block: mir::LocalNodeId<mir::Block>,
    ) -> HashSet<mir::LocalId> {
        liveness.local_live_in(block).clone()
    }

    /// Return whether one frame slot is materialized at this resume state.
    fn is_materialized_slot(
        &self,
        layout: &FrameLayout,
        slot: FrameSlotId,
        materialized_values: &HashSet<mir::Value>,
        materialized_locals: &HashSet<mir::LocalId>,
    ) -> bool {
        if let Some(value) = layout.value_for_slot(slot) {
            return materialized_values.contains(&mir::Value::new(value));
        }

        if let Some(local) = layout.local_for_slot(slot) {
            return materialized_locals.contains(&mir::LocalNodeId::new(local));
        }

        layout.is_environment_slot(slot)
    }
}

/// Align one byte offset up to the requested byte alignment.
fn align_offset(offset: usize, alignment: usize) -> usize {
    if alignment <= 1 {
        return offset;
    }

    let remainder = offset % alignment;
    if remainder == 0 {
        return offset;
    }

    offset + alignment - remainder
}
