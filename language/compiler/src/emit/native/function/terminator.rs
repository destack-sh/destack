use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;
use tspp_native as native;
use tspp_program::object::FramePoint;

use crate::EmitError;

use super::super::r#type::ValueType;
use super::FunctionEmitter;
use super::memory::AliasRegion;

impl<'a> FunctionEmitter<'a> {
    /// Emit one MIR terminator.
    pub(super) fn emit_terminator(
        &mut self,
        block: mir::BlockId,
        terminator: &mir::Terminator,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        match terminator {
            mir::Terminator::Return { value } => {
                let Some(value) = value else {
                    builder.ins().return_(&[]);

                    return Ok(());
                };
                let ty = self.value_type(*value)?;
                let value_type = self.types.value(ty)?;
                let value = self.value(*value)?;
                match value_type {
                    ValueType::Direct { .. } => {
                        let value = value
                            .direct()
                            .ok_or_else(|| self.invalid("native scalar result is indirect"))?;
                        builder.ins().return_(&[value]);
                    }
                    ValueType::ScalarPair { .. } => {
                        let mut results = Vec::new();
                        value.append_values(&mut results);
                        builder.ins().return_(&results);
                    }
                    ValueType::Indirect { .. } => {
                        let destination = self.result.ok_or_else(|| {
                            self.invalid("native indirect result address is missing")
                        })?;
                        self.store(destination, value, value_type, builder)?;
                        builder.ins().return_(&[]);
                    }
                }
            }
            mir::Terminator::Jump { target } => self.emit_jump(target, builder)?,
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let condition = self.scalar(*condition)?;
                let then_arguments = self.block_arguments(then_target)?;
                let else_arguments = self.block_arguments(else_target)?;
                builder.ins().brif(
                    condition,
                    self.blocks[&then_target.block],
                    &then_arguments,
                    self.blocks[&else_target.block],
                    &else_arguments,
                );
            }
            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => self.emit_check(constraint, success, failure, builder)?,
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => self.emit_switch(*value, default, *cases, builder)?,
            mir::Terminator::Invoke {
                call,
                target,
                unwind,
            } => {
                let point = self.object.terminator_point(block);
                self.emit_invoke(call, target, unwind, point, builder)?;
            }
            mir::Terminator::TailCall { call } => {
                let point = self.object.terminator_point(block);
                let frame = self.stack_map(FramePoint::operation(point), builder)?;
                let (call, _) = self.emit_call_values(call, &frame, builder)?;
                let values = builder.inst_results(call).to_vec();
                builder.ins().return_(&values);
            }
            mir::Terminator::Panic { payload } => {
                if let Some(payload) = payload {
                    let ty = self.value_type(*payload)?;
                    let value_type = self.types.value(ty)?;
                    let word_count = value_type.word_count();
                    let words = self.allocate_words(word_count, builder);
                    let payload = self.value(*payload)?;
                    self.store_words(words, payload, ty, value_type, builder)?;
                    let ty = u32::try_from(ty.index())
                        .map_err(|_| self.invalid("native panic type identity exceeds u32"))?;
                    let ty = self.index_u32(native::Index::Type { ty }, builder)?;

                    self.emit_runtime(native::abi::Operation::PanicValue, &[ty, words], builder)?;
                } else {
                    self.emit_runtime(native::abi::Operation::Panic, &[], builder)?;
                }
                Self::terminate_runtime(builder);
            }
            mir::Terminator::UnwindResume => {
                let slot = self.unwind_slot(builder);
                let address = builder.ins().stack_addr(self.types.pointer(), slot, 0);
                let flags = self.memory_flags(AliasRegion::World);
                let unwind = builder.ins().load(self.types.pointer(), flags, address, 0);
                self.emit_runtime(native::abi::Operation::UnwindResume, &[unwind], builder)?;
                Self::terminate_runtime(builder);
            }
            mir::Terminator::Abort { .. } => {
                let trap = u8::try_from(native::abi::Trap::Abort.code())
                    .map_err(|_| self.invalid("native abort trap exceeds u8"))?;
                let trap = cir::TrapCode::user(trap)
                    .ok_or_else(|| self.invalid("native abort trap code is reserved"))?;
                builder.ins().trap(trap);
            }
            mir::Terminator::Unreachable => {
                let trap = native::abi::Trap::Unreachable.code();
                let trap = u8::try_from(trap)
                    .map_err(|_| self.invalid("native unreachable trap exceeds u8"))?;
                let trap = cir::TrapCode::user(trap)
                    .ok_or_else(|| self.invalid("native unreachable trap code is reserved"))?;
                builder.ins().trap(trap);
            }
            _ => {
                return Err(self.invalid(&format!(
                    "native emission is missing terminator {terminator:?}"
                )));
            }
        }

        Ok(())
    }

    /// Emit one unconditional transfer.
    pub(super) fn emit_jump(
        &self,
        target: &mir::BlockTarget,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let arguments = self.block_arguments(target)?;
        builder.ins().jump(self.blocks[&target.block], &arguments);

        Ok(())
    }

    /// Return physical arguments for one MIR edge.
    pub(super) fn block_arguments(
        &self,
        target: &mir::BlockTarget,
    ) -> Result<Vec<cir::BlockArg>, EmitError> {
        // read the values passed to the destination block
        let values = self.optimized.tree.get_values(target.arguments);
        let mut arguments = Vec::with_capacity(values.len());
        for value in values {
            self.value(*value)?.append_block_arguments(&mut arguments);
        }

        Ok(arguments)
    }
}
