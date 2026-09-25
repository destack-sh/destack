use tspp_bytecode as bytecode;
use tspp_program::object::{FramePlace, FramePoint, FrameState};

use crate::EmitError;

use super::FunctionEmitter;

/// One logical frame state and its physical bytecode registers.
#[derive(Debug)]
pub(crate) struct FrameEmission {
    /// The object-local logical point.
    pub(crate) point: FramePoint,
    /// Physical register spans in acquisition order.
    pub(crate) registers: Vec<bytecode::RegisterSpan>,
}

impl<'a> FunctionEmitter<'a> {
    /// Project one logical frame state into physical bytecode registers.
    pub(super) fn frame(&self, state: &FrameState) -> Result<FrameEmission, EmitError> {
        let mut registers = Vec::new();

        // map every canonical logical place onto its assigned register range
        for slot in &state.slots {
            let registers_for_slot = match slot.place {
                FramePlace::Environment => {
                    let ty = self.types.register_type(slot.ty)?;

                    bytecode::RegisterSpan::new(bytecode::RegisterId(0), ty.word_count())
                }
                FramePlace::Local(local) => self.local(local)?,
                FramePlace::Value(value) => self.register(value)?,
            };
            registers.push(registers_for_slot);
        }

        Ok(FrameEmission {
            point: state.point,
            registers,
        })
    }
}
