use destack_heap::{HeapResult, RootSlot};
use destack_program::native::NativeFrameImage;
use destack_program::{ContinuationImage, Program};
use serde::{Deserialize, Serialize};

use super::Error;

/// Native execution state captured at managed safepoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuation {
    /// Durable continuation image.
    pub image: ContinuationImage,
}

impl Continuation {
    /// Create one native continuation.
    pub fn new(image: ContinuationImage) -> Self {
        Self { image }
    }

    /// Visit mutable heap root slots referenced by this native continuation.
    pub fn visit_root_slots(
        &mut self,
        program: &Program,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        for frame in &mut self.image.frames {
            let materialization = program.frame_materialization(frame.frame_state).ok_or(
                Error::InvalidContinuationFrame {
                    frame_state: frame.frame_state,
                },
            )?;
            let layout = program
                .frame_layout_by_id(materialization.frame_layout)
                .ok_or(Error::InvalidContinuationFrame {
                    frame_state: frame.frame_state,
                })?;

            for slot in program.frame_copied_slots(materialization) {
                let slot =
                    program
                        .frame_slot(layout, *slot)
                        .ok_or(Error::InvalidContinuationFrame {
                            frame_state: frame.frame_state,
                        })?;
                let start = slot.offset as usize;
                let end = start + slot.byte_len as usize;
                let frame_bytes = self
                    .image
                    .stack
                    .frame_bytes_mut(frame.stack_offset, frame.byte_len)
                    .ok_or(Error::InvalidContinuationFrame {
                        frame_state: frame.frame_state,
                    })?;
                let bytes =
                    frame_bytes
                        .get_mut(start..end)
                        .ok_or(Error::InvalidContinuationFrame {
                            frame_state: frame.frame_state,
                        })?;

                program
                    .visit_byte_root_slots(slot.ty, bytes, visit)
                    .map_err(|_| Error::InvalidContinuationFrame {
                        frame_state: frame.frame_state,
                    })?;
            }
        }

        Ok(())
    }

    /// Project this continuation into ABI frame records for one native call.
    pub fn abi_frames(&self) -> Result<Vec<NativeFrameImage>, Error> {
        self.image
            .frames
            .iter()
            .map(|frame| {
                let (return_state_is_present, return_state) = match frame.return_state {
                    Some(state) => (1, state.0),
                    None => (0, 0),
                };
                let bytes = self
                    .image
                    .stack
                    .frame_bytes(frame.stack_offset, frame.byte_len)
                    .ok_or(Error::InvalidContinuationFrame {
                        frame_state: frame.frame_state,
                    })?;

                Ok(NativeFrameImage {
                    frame_state: frame.frame_state.0,
                    return_state_is_present,
                    return_state,
                    bytes: bytes.as_ptr(),
                    byte_len: bytes.len(),
                })
            })
            .collect()
    }
}
