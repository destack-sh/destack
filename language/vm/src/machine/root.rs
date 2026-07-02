use destack_heap::{HeapResult, RootSlot};

use super::Frame;
use crate::diagnostic::Error;
use destack_program::{FrameLayout, FrameMaterialization, FrameSlot, Program};

impl Frame {
    /// Visit mutable heap root slots in this frame.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout = frame_layout(program, self)?;

        visit_frame_slots(program, layout, |slot| {
            visit_frame_slot_root_slots(program, slot, self.slot_bytes_mut(slot), visit)
        })
    }
}

/// Visit each full-frame root slot.
fn visit_frame_slots(
    program: &Program,
    layout: &FrameLayout,
    mut visit: impl FnMut(&FrameSlot) -> Result<(), Error>,
) -> Result<(), Error> {
    // ssa values
    for slot in program.frame_value_slots(layout) {
        visit(slot)?;
    }

    // locals
    for slot in program.frame_local_slots(layout) {
        visit(slot)?;
    }

    // function environment
    if let Some(slot) = program.frame_environment_slot(layout) {
        visit(slot)?;
    }

    Ok(())
}

/// Visit each materialized frame slot once.
pub(crate) fn visit_materialized_slots(
    program: &Program,
    layout: &FrameLayout,
    materialization: &FrameMaterialization,
    mut visit: impl FnMut(&FrameSlot) -> Result<(), Error>,
) -> Result<(), Error> {
    for slot in program.frame_copied_slots(materialization) {
        let slot = program
            .frame_slot(layout, *slot)
            .ok_or(Error::invalid_continuation())?;
        visit(slot)?;
    }

    Ok(())
}

/// Visit mutable heap roots stored in one frame slot byte range.
pub(crate) fn visit_frame_slot_root_slots(
    program: &Program,
    slot: &FrameSlot,
    bytes: &mut [u8],
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> Result<(), Error> {
    // aggregate slots may contain several heap roots
    let ty = slot.ty;
    if !program.frame_slot_is_cell(slot) {
        program.visit_byte_root_slots(ty, bytes, visit)?;

        return Ok(());
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
fn frame_layout<'a>(program: &'a Program, frame: &Frame) -> Result<&'a FrameLayout, Error> {
    let frame_layout = frame.frame_layout();

    program.frame_layout_by_id(frame_layout).ok_or_else(|| {
        Error::internal(format!(
            "missing frame layout for root scan: {frame_layout:?}"
        ))
    })
}
