use destack_engine as engine;
use destack_heap::{HeapResult, RootSlot};

use super::Frame;
use crate::diagnostic::Error;
use crate::program::{Layout, Program};

impl Frame {
    /// Visit mutable heap root slots in this frame.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout = frame_layout(program, self)?;

        visit_frame_slots(layout, |slot| {
            visit_frame_slot_root_slots(program, slot, self.slot_bytes_mut(slot), visit)
        })
    }

    /// Visit mutable heap root slots materialized at one safepoint.
    pub(crate) fn visit_materialized_root_slots(
        &mut self,
        program: &Program,
        materialization: &engine::FrameMaterialization,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout = materialized_layout(program, materialization)?;

        visit_materialized_slots(layout, materialization, |slot| {
            visit_frame_slot_root_slots(program, slot, self.slot_bytes_mut(slot), visit)
        })
    }
}

/// Visit each full-frame root slot.
fn visit_frame_slots(
    layout: &engine::FrameLayout,
    mut visit: impl FnMut(&engine::FrameSlot) -> Result<(), Error>,
) -> Result<(), Error> {
    // ssa values
    for slot in layout.values() {
        visit(slot)?;
    }

    // locals
    for slot in layout.locals() {
        visit(slot)?;
    }

    // callable environment
    if let Some(slot) = layout.environment() {
        visit(slot)?;
    }

    Ok(())
}

/// Visit each materialized frame slot once.
pub(crate) fn visit_materialized_slots(
    layout: &engine::FrameLayout,
    materialization: &engine::FrameMaterialization,
    mut visit: impl FnMut(&engine::FrameSlot) -> Result<(), Error>,
) -> Result<(), Error> {
    for slot in materialization.copied_slots() {
        let slot = layout.slot(slot).ok_or(Error::invalid_continuation())?;
        visit(slot)?;
    }

    Ok(())
}

/// Visit mutable heap roots stored in one frame slot byte range.
pub(crate) fn visit_frame_slot_root_slots(
    program: &Program,
    slot: &engine::FrameSlot,
    bytes: &mut [u8],
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> Result<(), Error> {
    let ty = program.type_for_value_layout(slot.layout);
    let layout = frame_slot_layout(program, slot)?;

    // aggregate slots may contain several heap roots
    if !layout.is_word() {
        return program.visit_byte_root_slots(ty, bytes, visit);
    }

    // local scalar root
    if program.is_local_root_type(ty)? {
        visit(RootSlot::HeapBytes(bytes)).map_err(Error::from)?;
    }
    // shared scalar root
    else if program.is_shared_root_type(ty)? {
        visit(RootSlot::SharedHeapBytes(bytes)).map_err(Error::from)?;
    }

    Ok(())
}

/// Return the physical frame layout for one live frame.
fn frame_layout<'a>(program: &'a Program, frame: &Frame) -> Result<&'a engine::FrameLayout, Error> {
    let function = frame.function();

    program.frame_layout(function).ok_or_else(|| {
        Error::internal(format!(
            "missing frame layout for root scan: function={function:?}"
        ))
    })
}

/// Return the physical frame layout for one materialization.
fn materialized_layout<'a>(
    program: &'a Program,
    materialization: &engine::FrameMaterialization,
) -> Result<&'a engine::FrameLayout, Error> {
    program
        .frame_layout_by_id(materialization.frame_layout)
        .ok_or_else(|| {
            Error::internal(format!(
                "missing materialized frame layout for root scan: {:?}",
                materialization.frame_layout
            ))
        })
}

/// Return the physical layout for one frame slot.
fn frame_slot_layout<'a>(
    program: &'a Program,
    slot: &engine::FrameSlot,
) -> Result<&'a Layout, Error> {
    program.layout_for_value_id(slot.layout).ok_or_else(|| {
        Error::internal(format!(
            "missing frame slot layout for root scan: layout={:?}",
            slot.layout
        ))
    })
}
