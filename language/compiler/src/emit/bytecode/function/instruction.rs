use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::{EmitError, TensorEmitter};

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one MIR instruction.
    pub(super) fn emit_instruction(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> Result<(), EmitError> {
        match instruction {
            mir::Instruction::Const { destination, value } => {
                self.emit_constant(*destination, value)
            }
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => self.emit_binary(*destination, *operator, *left, *right),
            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => self.emit_unary(*destination, *operator, *argument),
            mir::Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => self.emit_cast(*destination, *operator, *argument, *to_type),
            mir::Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => self.emit_select(*destination, *condition, *then_value, *else_value),
            mir::Instruction::LocalGet { destination, local } => {
                self.emit_local_get(*destination, *local)
            }
            mir::Instruction::LocalAddr {
                destination, local, ..
            } => self.emit_local_address(*destination, *local),
            mir::Instruction::LocalSet { local, value } => self.emit_local_set(*local, *value),
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => self.emit_global_address(*destination, *global),
            mir::Instruction::FunctionAddr {
                destination,
                function,
            } => self.emit_function_address(*destination, *function),
            mir::Instruction::FunctionBind {
                destination,
                function,
                environment,
            } => self.emit_function_bind(*destination, *function, *environment),
            mir::Instruction::FunctionEnvironment {
                destination,
                function,
            } => self.emit_function_environment(*destination, *function),
            mir::Instruction::FunctionEnvironmentCurrent { destination } => {
                self.emit_function_environment_current(*destination)
            }
            mir::Instruction::Load {
                destination,
                pointer,
                result_type,
            } => self.emit_load(*destination, *pointer, *result_type),
            mir::Instruction::Store { pointer, value } => self.emit_store(*pointer, *value),
            mir::Instruction::Aggregate {
                destination,
                values,
            } => self.emit_aggregate(*destination, *values),
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } => self.emit_field_get(*destination, *aggregate, *field),
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                field,
                value,
            } => self.emit_field_set(*destination, *aggregate, *field, *value),
            mir::Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } => self.emit_element_get(*destination, *aggregate, *index),
            mir::Instruction::ElementSet {
                destination,
                aggregate,
                index,
                value,
            } => self.emit_element_set(*destination, *aggregate, *index, *value),
            mir::Instruction::VariantNew {
                destination,
                case,
                payload,
                result_type,
            } => self.emit_variant_new(*destination, *case, *payload, *result_type),
            mir::Instruction::VariantTag {
                destination,
                variant,
            } => self.emit_variant_tag(*destination, *variant),
            mir::Instruction::VariantTagLoad {
                destination,
                variant,
            } => self.emit_variant_tag_load(*destination, *variant),
            mir::Instruction::VariantPayload {
                destination,
                variant,
                case,
            } => self.emit_variant_payload(*destination, *variant, *case),
            mir::Instruction::VariantPayloadAddr {
                destination,
                variant,
                case,
                ..
            } => self.emit_variant_payload_address(*destination, *variant, *case),
            mir::Instruction::ContextCurrent { destination } => {
                self.emit_context_current(*destination)
            }
            mir::Instruction::ContextReplace {
                destination,
                context,
            } => self.emit_context_replace(*destination, *context),
            mir::Instruction::ContextBind {
                destination,
                context,
                variable,
                value,
                node_type,
                ..
            } => self.emit_context_bind(
                instruction_id,
                *destination,
                *context,
                *variable,
                *value,
                *node_type,
            ),
            mir::Instruction::ContextGet {
                destination,
                context,
                variable,
                default,
                node_type,
                ..
            } => self.emit_context_get(*destination, *context, *variable, *default, *node_type),
            mir::Instruction::Call { destination, call } => self.emit_call(*destination, call),
            mir::Instruction::CallDetach { thunk } => self.emit_call_detach(*thunk),
            mir::Instruction::Drop { value } => self.emit_drop(*value),
            mir::Instruction::NewZeroed {
                destination,
                result_type,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                bytecode::NewKind::Value,
                bytecode::Initialization::Zeroed,
                None,
            ),
            mir::Instruction::NewUninit {
                destination,
                result_type,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                bytecode::NewKind::Value,
                bytecode::Initialization::Uninit,
                None,
            ),
            mir::Instruction::NewComplete {
                destination, value, ..
            } => self.emit_new_complete(*destination, *value),
            mir::Instruction::NewSliceZeroed {
                destination,
                length,
                result_type,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                bytecode::NewKind::Slice,
                bytecode::Initialization::Zeroed,
                Some(*length),
            ),
            mir::Instruction::NewSliceUninit {
                destination,
                length,
                result_type,
                ..
            } => self.emit_new(
                instruction_id,
                *destination,
                *result_type,
                bytecode::NewKind::Slice,
                bytecode::Initialization::Uninit,
                Some(*length),
            ),
            mir::Instruction::Poll => self.emit_empty(bytecode::Opcode::POLL),
            mir::Instruction::Breakpoint => self.emit_empty(bytecode::Opcode::BREAKPOINT),

            mir::Instruction::Error => Err(self.internal("invalid instruction")),

            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                field,
                ..
            } => self.emit_field_address(*destination, *aggregate, *field),
            mir::Instruction::ElementAddr {
                destination,
                base,
                index,
                ..
            } => self.emit_element_address(*destination, *base, *index),
            mir::Instruction::SliceView {
                destination,
                source,
                start,
                length,
                ..
            } => self.emit_slice_view(*destination, *source, *start, *length),
            mir::Instruction::SliceLength { destination, slice } => {
                self.emit_slice_length(*destination, *slice)
            }
            mir::Instruction::DynamicBind {
                destination,
                payload,
                concrete,
            } => self.emit_dynamic_bind(*destination, *payload, *concrete),
            mir::Instruction::DynamicPayload {
                destination,
                dynamic,
                ..
            } => self.emit_dynamic_payload(*destination, *dynamic),
            mir::Instruction::DynamicType {
                destination,
                dynamic,
            } => self.emit_dynamic_type(*destination, *dynamic),
            mir::Instruction::DynamicFind { .. } => {
                Err(self
                    .internal("dynamic property lookup requires executable string representation"))
            }
            mir::Instruction::VectorSplat { destination, value } => {
                self.emit_vector_splat(*destination, *value)
            }
            mir::Instruction::VectorExtract {
                destination,
                vector,
                index,
            } => self.emit_vector_extract(*destination, *vector, *index),
            mir::Instruction::VectorInsert {
                destination,
                vector,
                index,
                value,
            } => self.emit_vector_insert(*destination, *vector, *index, *value),
            mir::Instruction::VectorShuffle {
                destination,
                left,
                right,
                mask,
            } => self.emit_vector_shuffle(*destination, *left, *right, *mask),
            mir::Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => self.emit_vector_select(*destination, *mask, *then_value, *else_value),
            mir::Instruction::VectorReduce {
                destination,
                operator,
                vector,
            } => self.emit_vector_reduce(*destination, *operator, *vector),
            mir::Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => self.emit_vector_compare(*destination, *operator, *left, *right),
            mir::Instruction::VectorConvert {
                destination,
                mode,
                vector,
            } => self.emit_vector_convert(*destination, *mode, *vector),
            instruction @ (mir::Instruction::TensorSplat { .. }
            | mir::Instruction::TensorLoad { .. }
            | mir::Instruction::TensorExtract { .. }
            | mir::Instruction::TensorStore { .. }
            | mir::Instruction::TensorFill { .. }
            | mir::Instruction::TensorCopy { .. }
            | mir::Instruction::TensorReshape { .. }
            | mir::Instruction::TensorBroadcast { .. }
            | mir::Instruction::TensorTranspose { .. }
            | mir::Instruction::TensorCast { .. }
            | mir::Instruction::TensorView { .. }
            | mir::Instruction::TensorSlice { .. }
            | mir::Instruction::TensorPad { .. }
            | mir::Instruction::TensorConcat { .. }
            | mir::Instruction::TensorCompare { .. }
            | mir::Instruction::TensorSelect { .. }
            | mir::Instruction::TensorReduce { .. }
            | mir::Instruction::TensorIndexReduce { .. }
            | mir::Instruction::TensorDot { .. }
            | mir::Instruction::TensorConvolution { .. }
            | mir::Instruction::TensorGather { .. }
            | mir::Instruction::TensorScatter { .. }
            | mir::Instruction::TensorConvert { .. }) => {
                let command = TensorEmitter::new(
                    self.module,
                    self.optimized,
                    self.object,
                    self.function,
                    &self.values,
                )
                .emit(instruction_id, instruction)?;
                let destinations = command.destination.into_iter().collect::<Vec<_>>();

                self.encode(command.instruction, &destinations)
            }
            mir::Instruction::Free { value } => self.emit_free(*value),
            mir::Instruction::Pin {
                destination, value, ..
            } => self.emit_pin(*destination, *value),
            mir::Instruction::Unpin { value } => self.emit_unpin(*value),
            mir::Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => self.emit_barrier(*object, *offset, *byte_len),
            mir::Instruction::AtomicLoad {
                destination,
                pointer,
                access,
                ..
            } => self.emit_atomic_load(*destination, *pointer, *access),
            mir::Instruction::AtomicStore {
                pointer,
                value,
                access,
            } => self.emit_atomic_store(*pointer, *value, *access),
            mir::Instruction::AtomicCompareExchange {
                destination,
                pointer,
                expected,
                new_value,
                is_weak,
                access,
            } => self.emit_atomic_compare_exchange(
                *destination,
                *pointer,
                *expected,
                *new_value,
                *is_weak,
                *access,
            ),
            mir::Instruction::AtomicRmw {
                destination,
                operator,
                pointer,
                value,
                access,
            } => self.emit_atomic_rmw(*destination, *operator, *pointer, *value, *access),
            mir::Instruction::AtomicFence { access } => self.emit_atomic_fence(*access),
            mir::Instruction::Assume { .. } => Ok(()),
            mir::Instruction::ProfileIncrement { counter } => self.emit_profile_increment(*counter),
            mir::Instruction::ProfileSample { sampler, value } => {
                self.emit_profile_sample(*sampler, *value)
            }
            mir::Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => match intrinsic {
                mir::Intrinsic::Memcpy
                | mir::Intrinsic::Memmove
                | mir::Intrinsic::Memset
                | mir::Intrinsic::Memcmp
                | mir::Intrinsic::PrefetchRead
                | mir::Intrinsic::PrefetchWrite
                | mir::Intrinsic::PointerByteOffsetFrom
                | mir::Intrinsic::VolatileLoad
                | mir::Intrinsic::VolatileStore => {
                    self.emit_memory_intrinsic(*destination, *intrinsic, *arguments)
                }
                _ => self.emit_scalar_intrinsic(*destination, *intrinsic, *arguments),
            },
        }
    }
}
