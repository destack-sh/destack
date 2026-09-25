use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one MIR terminator.
    pub(super) fn emit_terminator(
        &mut self,
        block: mir::BlockId,
        terminator: &mir::Terminator,
    ) -> Result<(), EmitError> {
        match terminator {
            mir::Terminator::Return { value } => {
                let range = value.map(|value| self.register(value)).transpose()?;
                let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::RETURN);
                instruction.span(range.unwrap_or_else(bytecode::RegisterSpan::empty));

                self.encode(instruction, &[])
            }
            mir::Terminator::Jump { target } => {
                self.emit_jump(terminator, mir::Successor::Jump, target)
            }
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => self.emit_branch(terminator, *condition, then_target, else_target),
            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => self.emit_check(terminator, constraint, success, failure),
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => self.emit_switch(terminator, *value, default, *cases),
            mir::Terminator::VariantSwitch {
                value,
                default,
                cases,
            } => self.emit_variant_switch(terminator, *value, default.as_ref(), *cases),
            mir::Terminator::Invoke {
                call,
                target,
                unwind,
            } => self.emit_invoke(terminator, call, target, unwind),
            mir::Terminator::Panic { payload } => {
                let opcode = if payload.is_some() {
                    bytecode::Opcode::PANIC_VALUE
                } else {
                    bytecode::Opcode::PANIC
                };
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                if let Some(payload) = payload {
                    let ty = self
                        .function
                        .value_type(*payload)
                        .ok_or_else(|| self.internal("missing panic payload type"))?;
                    let ty = self.types.type_id(ty)?;
                    instruction.relocation(bytecode::RelocationTag::TYPE, ty.0);
                    instruction.span(self.register(*payload)?);
                }

                self.encode(instruction, &[])
            }
            mir::Terminator::UnwindResume => self.emit_empty(bytecode::Opcode::UNWIND_RESUME),
            mir::Terminator::TailCall { call } => self.emit_tail_call(call),
            mir::Terminator::Abort { .. } => {
                let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::TRAP);
                instruction.u16(bytecode::Trap::Abort as u16);

                self.encode(instruction, &[])
            }
            mir::Terminator::Unreachable => self.emit_empty(bytecode::Opcode::UNREACHABLE),

            mir::Terminator::NewZeroedTry {
                success, failure, ..
            } => self.emit_new_try(
                block,
                terminator,
                bytecode::NewKind::Value,
                bytecode::Initialization::Zeroed,
                None,
                success,
                failure,
            ),
            mir::Terminator::NewUninitTry {
                success, failure, ..
            } => self.emit_new_try(
                block,
                terminator,
                bytecode::NewKind::Value,
                bytecode::Initialization::Uninit,
                None,
                success,
                failure,
            ),
            mir::Terminator::NewSliceZeroedTry {
                length,
                success,
                failure,
                ..
            } => self.emit_new_try(
                block,
                terminator,
                bytecode::NewKind::Slice,
                bytecode::Initialization::Zeroed,
                Some(*length),
                success,
                failure,
            ),
            mir::Terminator::NewSliceUninitTry {
                length,
                success,
                failure,
                ..
            } => self.emit_new_try(
                block,
                terminator,
                bytecode::NewKind::Slice,
                bytecode::Initialization::Uninit,
                Some(*length),
                success,
                failure,
            ),
            mir::Terminator::Error => Err(self.internal("invalid terminator")),
        }
    }
}
