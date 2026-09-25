use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use destack_mir as mir;
use destack_native as native;
use destack_program::object::FramePoint;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl<'a> FunctionEmitter<'a> {
    /// Emit one MIR instruction.
    pub(super) fn emit_instruction(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<bool, EmitError> {
        match instruction {
            mir::Instruction::Error => {
                return Err(self.invalid("native emission received an invalid instruction"));
            }
            mir::Instruction::Const { destination, value } => {
                let value = self.emit_constant(value, builder)?;
                self.set(*destination, Value::Direct(value))?;
            }
            mir::Instruction::Copy { destination, value } => {
                let value_type = self.types.value(self.value_type(*value)?)?;
                let value = match self.value(*value)? {
                    Value::Address(address) => self.load(address, value_type, builder)?,
                    value => value,
                };
                self.set(*destination, value)?;
            }
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let ty = self.optimized.tree.storage_type(self.value_type(*left)?);
                let ty = match self.optimized.tree.type_definition(ty) {
                    mir::Type::Vector { element, .. } => *element,
                    _ => ty,
                };
                let ty = self.optimized.tree.storage_type(ty);
                let left = self.scalar(*left)?;
                let right = self.scalar(*right)?;
                let value = self.emit_binary(*operator, left, right, ty, builder)?;
                self.set(*destination, Value::Direct(value))?;
            }
            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let ty = self
                    .optimized
                    .tree
                    .storage_type(self.value_type(*argument)?);
                let is_float = self
                    .optimized
                    .tree
                    .type_definition(ty)
                    .is_float(&self.optimized.tree);
                let argument = self.scalar(*argument)?;
                let value = match (*operator, is_float) {
                    (mir::UnaryOperator::Negate, false) => builder.ins().ineg(argument),
                    (mir::UnaryOperator::Negate, true) => builder.ins().fneg(argument),
                    (mir::UnaryOperator::Not, false) => builder.ins().bnot(argument),
                    (mir::UnaryOperator::Not, true) => {
                        return Err(self.invalid("native floating point NOT is invalid"));
                    }
                };
                self.set(*destination, Value::Direct(value))?;
            }
            mir::Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => {
                let argument_type = self.value_type(*argument)?;
                let argument = self.scalar(*argument)?;
                let target = self
                    .types
                    .value(*to_type)?
                    .direct()
                    .ok_or_else(|| self.invalid("native cast target is not scalar"))?;
                let value = self.emit_cast(
                    *operator,
                    argument,
                    argument_type,
                    *to_type,
                    target,
                    builder,
                )?;
                self.set(*destination, Value::Direct(value))?;
            }
            mir::Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => {
                let condition = self.scalar(*condition)?;
                let then_value = self.scalar(*then_value)?;
                let else_value = self.scalar(*else_value)?;
                let value = builder.ins().select(condition, then_value, else_value);
                self.set(*destination, Value::Direct(value))?;
            }
            mir::Instruction::FunctionAddr {
                destination,
                function,
                ..
            } => self.emit_function_address(*destination, *function, builder)?,
            mir::Instruction::FunctionBind {
                destination,
                function,
                environment,
                ..
            } => self.emit_function_bind(*destination, *function, *environment, builder)?,
            mir::Instruction::FunctionEnvironment {
                destination,
                function,
            } => self.emit_function_environment(*destination, *function)?,
            mir::Instruction::FunctionEnvironmentCurrent { destination } => {
                self.emit_function_environment_current(*destination)?;
            }
            mir::Instruction::ContextCurrent { destination } => {
                self.emit_context_current(*destination, builder)?;
            }
            mir::Instruction::ContextReplace {
                destination,
                context,
            } => self.emit_context_replace(*destination, *context, builder)?,
            mir::Instruction::ContextBind {
                destination,
                context,
                variable,
                value,
                node_type,
                result_type,
            } => self.emit_context_bind(
                instruction_id,
                *destination,
                *context,
                *variable,
                *value,
                *node_type,
                *result_type,
                builder,
            )?,
            mir::Instruction::ContextGet {
                destination,
                context,
                variable,
                default,
                node_type,
                result_type,
            } => self.emit_context_get(
                *destination,
                *context,
                *variable,
                *default,
                *node_type,
                *result_type,
                builder,
            )?,
            mir::Instruction::Load {
                destination, place, ..
            } => self.emit_load(*destination, place, false, builder)?,
            mir::Instruction::Address {
                destination,
                place,
                result_type,
            } => self.emit_address(*destination, place, *result_type, builder)?,
            mir::Instruction::Store { place, value } => {
                self.emit_store(place, *value, false, builder)?
            }
            mir::Instruction::Aggregate {
                destination,
                values,
            } => self.emit_aggregate(*destination, *values, builder)?,
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                field,
                ..
            }
            | mir::Instruction::ElementGet {
                destination,
                aggregate,
                index: field,
                ..
            } => self.emit_projection(*destination, *aggregate, *field, builder)?,
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                field,
                value,
            } => self.emit_field_set(*destination, *aggregate, *field, *value, builder)?,
            mir::Instruction::ElementSet {
                destination,
                aggregate,
                index,
                value,
            } => self.emit_element_set(*destination, *aggregate, *index, *value, builder)?,
            mir::Instruction::VariantNew {
                destination,
                case,
                payload,
                result_type,
            } => self.emit_variant_new(*destination, *case, *payload, *result_type, builder)?,
            mir::Instruction::VariantTag {
                destination,
                variant,
            } => self.emit_variant_tag(*destination, *variant, builder)?,
            mir::Instruction::VariantTagLoad { destination, place } => {
                self.emit_variant_tag_load(*destination, place, builder)?
            }
            mir::Instruction::VariantPayload {
                destination,
                variant,
                case,
                ..
            } => self.emit_variant_payload(*destination, *variant, *case, builder)?,
            mir::Instruction::SliceLength { destination, slice } => {
                self.emit_slice_length(*destination, *slice)?
            }
            mir::Instruction::DynamicBind {
                destination,
                payload,
                concrete,
            } => self.emit_dynamic_bind(*destination, *payload, *concrete, builder)?,
            mir::Instruction::DynamicPayload {
                destination,
                dynamic,
                ..
            } => self.emit_dynamic_payload(*destination, *dynamic)?,
            mir::Instruction::DynamicType {
                destination,
                dynamic,
            } => self.emit_dynamic_type(*destination, *dynamic, builder)?,
            mir::Instruction::DynamicRead {
                destination,
                dynamic,
                slot,
                result_type,
            } => self.emit_dynamic_read(*destination, *dynamic, *slot, *result_type, builder)?,
            mir::Instruction::DynamicFind { .. } => {
                return Err(Self::internal(
                    self.module,
                    "dynamic property lookup requires executable string representation",
                ));
            }
            mir::Instruction::Drop { value } => self.emit_drop(*value, builder)?,
            mir::Instruction::NewZeroed {
                destination,
                result_type,
                space,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                *space,
                native::abi::AllocationInitialization::Zeroed,
                None,
                builder,
            )?,
            mir::Instruction::NewUninit {
                destination,
                result_type,
                space,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                *space,
                native::abi::AllocationInitialization::Uninit,
                None,
                builder,
            )?,
            mir::Instruction::NewComplete {
                destination, value, ..
            } => {
                let value = self.value(*value)?;
                self.set(*destination, value)?;
            }
            mir::Instruction::NewSliceZeroed {
                destination,
                length,
                result_type,
                space,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                *space,
                native::abi::AllocationInitialization::Zeroed,
                Some(*length),
                builder,
            )?,
            mir::Instruction::NewSliceUninit {
                destination,
                length,
                result_type,
                space,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                *space,
                native::abi::AllocationInitialization::Uninit,
                Some(*length),
                builder,
            )?,
            mir::Instruction::Release { value } => {
                self.emit_release(instruction_id, *value, builder)?
            }
            mir::Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => {
                let object = self.materialize_pointer(*object, builder)?;
                let offset = self.scalar(*offset)?;
                let byte_len = self.scalar(*byte_len)?;
                self.emit_runtime(
                    native::abi::Operation::WriteBarrier,
                    &[object, offset, byte_len],
                    builder,
                )?;
            }
            mir::Instruction::AtomicLoad {
                destination,
                place,
                result_type,
                access,
            } => self.emit_atomic_load(*destination, place, *result_type, *access, builder)?,
            mir::Instruction::AtomicStore {
                place,
                value,
                access,
            } => self.emit_atomic_store(place, *value, *access, builder)?,
            mir::Instruction::AtomicCompareExchange {
                destination,
                place,
                expected,
                new_value,
                is_weak,
                access,
            } => self.emit_atomic_compare_exchange(
                *destination,
                place,
                *expected,
                *new_value,
                *is_weak,
                *access,
                builder,
            )?,
            mir::Instruction::AtomicRmw {
                destination,
                operator,
                place,
                value,
                access,
            } => self.emit_atomic_rmw(*destination, *operator, place, *value, *access, builder)?,
            mir::Instruction::AtomicFence { access } => self.emit_atomic_fence(*access, builder)?,
            mir::Instruction::Assume { .. } => {}
            mir::Instruction::ProfileIncrement { counter } => {
                self.emit_profile_increment(*counter, builder)?
            }
            mir::Instruction::ProfileSample { sampler, value } => {
                self.emit_profile_sample(*sampler, *value, builder)?
            }
            mir::Instruction::VectorSplat { destination, value } => {
                self.emit_vector_splat(*destination, *value, builder)?
            }
            mir::Instruction::VectorExtract {
                destination,
                vector,
                index,
            } => self.emit_vector_extract(*destination, *vector, *index, builder)?,
            mir::Instruction::VectorInsert {
                destination,
                vector,
                index,
                value,
            } => self.emit_vector_insert(*destination, *vector, *index, *value, builder)?,
            mir::Instruction::VectorShuffle {
                destination,
                left,
                right,
                mask,
            } => self.emit_vector_shuffle(*destination, *left, *right, *mask, builder)?,
            mir::Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => self.emit_vector_select(*destination, *mask, *then_value, *else_value, builder)?,
            mir::Instruction::VectorReduce {
                destination,
                operator,
                vector,
            } => self.emit_vector_reduce(*destination, *operator, *vector, builder)?,
            mir::Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => self.emit_vector_compare(*destination, *operator, *left, *right, builder)?,
            mir::Instruction::VectorConvert {
                destination,
                mode,
                vector,
            } => self.emit_vector_convert(*destination, *mode, *vector, builder)?,
            mir::Instruction::Call { destination, call } => {
                let point = self.object.instruction_point(instruction_id);
                let frame = self.stack_map(FramePoint::operation(point), builder)?;
                self.emit_call(*destination, call, frame, builder)?;
            }
            mir::Instruction::Poll => {
                let point = self.object.instruction_point(instruction_id);
                self.emit_poll(point, builder)?;
            }
            mir::Instruction::Breakpoint => {
                let point = self.object.instruction_point(instruction_id);
                let frame = self.stack_map(FramePoint::operation(point.next()), builder)?;
                let id = self.frame_map_id(frame.id, builder)?;
                let operation = builder
                    .ins()
                    .iconst(cir::types::I32, point.operation as i64);
                let call = self.emit_runtime(
                    native::abi::Operation::Stop,
                    &[id, operation, frame.anchor],
                    builder,
                )?;
                Self::attach_stack_map(frame.entries, call, builder);
                Self::terminate_runtime(builder);

                return Ok(false);
            }
            mir::Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => self.emit_intrinsic(*destination, *intrinsic, *arguments, builder)?,
        }

        Ok(true)
    }
}
