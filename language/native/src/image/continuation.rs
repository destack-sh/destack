use serde::{Deserialize, Serialize};

use destack_heap::{HeapResult, RootSlot};
use destack_mir as mir;
use destack_program::Program;

use crate::{Error, NativeFrameImage};

/// Native execution state captured at managed safepoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuation {
    /// Captured native frames from outermost to innermost.
    pub frames: Vec<FrameImage>,
}

/// Captured native frame state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameImage {
    /// The captured frame state.
    pub frame_state: mir::FrameStateId,
    /// The caller return frame state.
    pub return_state: Option<mir::FrameStateId>,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}

impl Continuation {
    /// Create one native continuation.
    pub fn new(frames: Vec<FrameImage>) -> Self {
        Self { frames }
    }

    /// Visit mutable heap root slots referenced by this native continuation.
    pub fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        for frame in &mut self.frames {
            frame.visit_root_slots(program, visit)?;
        }

        Ok(())
    }

    /// Project this continuation into ABI frame records for one native call.
    pub fn abi_frames(&self) -> Vec<NativeFrameImage> {
        self.frames.iter().map(FrameImage::to_abi).collect()
    }
}

impl FrameImage {
    /// Project this frame image into one ABI frame record.
    fn to_abi(&self) -> NativeFrameImage {
        let return_state = self.return_state.map(|state| state.0).unwrap_or(0);
        let return_state_is_present = u32::from(self.return_state.is_some());

        NativeFrameImage {
            frame_state: self.frame_state.0,
            return_state_is_present,
            return_state,
            bytes: self.bytes.as_ptr(),
            byte_len: self.bytes.len(),
        }
    }

    /// Visit mutable heap root slots referenced by this native frame image.
    fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let materialization = program.frame_materialization(self.frame_state).ok_or(
            Error::InvalidContinuationFrame {
                frame_state: self.frame_state,
            },
        )?;
        let layout = program
            .frame_layout_by_id(materialization.frame_layout)
            .ok_or(Error::InvalidContinuationFrame {
                frame_state: self.frame_state,
            })?;

        for slot in materialization.copied_slots() {
            let slot = layout.slot(slot).ok_or(Error::InvalidContinuationFrame {
                frame_state: self.frame_state,
            })?;
            let start = slot.offset as usize;
            let end = start + slot.byte_len as usize;
            let bytes = self
                .bytes
                .get_mut(start..end)
                .ok_or(Error::InvalidContinuationFrame {
                    frame_state: self.frame_state,
                })?;

            program
                .visit_byte_root_slots(program.types().type_id(slot.ty), bytes, visit)
                .map_err(|_| Error::InvalidContinuationFrame {
                    frame_state: self.frame_state,
                })?;
        }

        Ok(())
    }
}
