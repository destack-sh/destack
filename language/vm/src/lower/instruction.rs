use destack_mir as mir;

use crate::program::{Instruction, Op};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::pool::Pool;
impl<'a> BlockLowerer<'a> {
    /// Lower one MIR instruction into zero or more VM instructions.
    pub(super) fn lower_instructions(
        &self,
        inst: &mir::Instruction,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Vec<Instruction>> {
        match inst {
            mir::Instruction::Struct {
                destination,
                fields,
                ..
            } => self.lower_frame_constructor(*destination, *fields),
            mir::Instruction::Tuple {
                destination,
                elements,
                ..
            } => self.lower_frame_constructor(*destination, *elements),
            mir::Instruction::Array {
                destination,
                elements,
                ..
            } => self.lower_frame_constructor(*destination, *elements),
            mir::Instruction::FieldSet {
                destination,
                aggregate: base,
                index,
                value,
            } => self.lower_field_update(*destination, *base, *index, *value),
            mir::Instruction::FieldGet { .. } => self.lower_field_read(inst),
            mir::Instruction::ElementGet { .. } => self.lower_element_read(inst),
            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => self.lower_element_update(*destination, *array, *index, *value),
            mir::Instruction::Drop { value } => self.lower_drop(pool, *value),
            _ => Ok(vec![self.lower_instruction(inst, pool)?]),
        }
    }

    /// Convert a MIR instruction to lowered interpreter form.
    pub(super) fn lower_instruction(
        &self,
        inst: &mir::Instruction,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        Ok(match inst {
            mir::Instruction::Error => {
                return Err(Error::MissingRepresentation {
                    context: "instruction".to_string(),
                });
            }
            mir::Instruction::Const { destination, value } => {
                self.lower_const(pool, *destination, value)?
            }

            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => self.lower_binary(pool, *destination, *operator, *left, *right)?,

            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => self.lower_unary(pool, *destination, *operator, *argument)?,

            mir::Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => self.lower_cast(*destination, *operator, *argument, *to_type)?,

            mir::Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => self.lower_select(pool, *destination, *condition, *then_value, *else_value)?,

            mir::Instruction::Call {
                destination,
                function,
                call,
                ..
            } => self.lower_call(*destination, *function, call, pool)?,

            mir::Instruction::CallClass {
                destination,
                receiver,
                slot: method,
                call,
                ..
            } => self.lower_class_call(*destination, *receiver, *method, call, pool)?,

            mir::Instruction::CallInterface {
                destination,
                receiver,
                slot: method,
                call,
                ..
            } => self.lower_interface_call(*destination, *receiver, *method, call, pool)?,

            mir::Instruction::CallIndirect {
                destination,
                callee,
                call,
                ..
            } => self.lower_indirect_call(*destination, *callee, call, pool)?,

            mir::Instruction::LocalGet { destination, local } => {
                self.lower_local_get(*destination, *local)?
            }

            mir::Instruction::LocalAddr {
                destination, local, ..
            } => self.lower_local_addr(*destination, *local)?,

            mir::Instruction::LocalSet { local, value } => self.lower_local_set(*local, *value)?,

            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => self.lower_static_addr(*destination, *global)?,

            mir::Instruction::FunctionAddr {
                destination,
                function,
            } => self.lower_function_addr(*destination, *function)?,
            mir::Instruction::CallableBind {
                destination,
                function,
                environment,
            } => self.lower_callable_bind(pool, *destination, *function, *environment)?,
            mir::Instruction::CallableEnvironment { destination } => {
                self.lower_callable_environment(*destination)?
            }
            mir::Instruction::Load {
                destination,
                pointer,
                ..
            } => self.lower_load(pool, *destination, *pointer)?,

            mir::Instruction::Store { pointer, value } => {
                self.lower_store(pool, *pointer, *value)?
            }

            mir::Instruction::Pin {
                destination, value, ..
            } => self.lower_pin(*destination, *value)?,

            mir::Instruction::Unpin { value } => self.lower_unpin(*value)?,

            mir::Instruction::Drop { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::Assume { condition: _ } => Instruction::new(Op::Assume, 0, 0, 0, 0),

            mir::Instruction::FieldGet { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::FieldAddr {
                destination,
                aggregate: base,
                index,
                ..
            } => self.lower_field_addr(*destination, *base, *index)?,

            mir::Instruction::FieldSet { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::ElementGet { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => self.lower_element_addr(pool, *destination, *array, *index)?,

            mir::Instruction::ElementSet { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::Struct { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::Tuple { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::Array { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::VectorSplat { destination, value } => {
                self.lower_vector_splat(pool, *destination, *value)?
            }

            mir::Instruction::VectorExtract {
                destination,
                vector,
                index,
            } => self.lower_vector_extract(pool, *destination, *vector, *index)?,

            mir::Instruction::VectorInsert {
                destination,
                vector,
                index,
                value,
            } => self.lower_vector_insert(pool, *destination, *vector, *index, *value)?,

            mir::Instruction::VectorShuffle {
                destination,
                left,
                right,
                mask,
            } => self.lower_vector_shuffle(pool, *destination, *left, *right, mask)?,
            mir::Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => self.lower_vector_select(pool, *destination, *mask, *then_value, *else_value)?,

            mir::Instruction::VectorReduce {
                destination,
                operator,
                vector,
            } => self.lower_vector_reduce(pool, *destination, *operator, *vector)?,

            mir::Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => self.lower_vector_compare(pool, *destination, *operator, *left, *right)?,

            mir::Instruction::VectorConvert {
                destination,
                mode,
                vector,
            } => self.lower_vector_convert(pool, *destination, *mode, *vector)?,

            inst if is_tensor_instruction(inst) => self.lower_tensor(inst, pool)?,

            mir::Instruction::New {
                destination,
                layout,
                ..
            } => self.lower_new(pool, *destination, *layout)?,

            mir::Instruction::NewSlice {
                destination,
                element,
                length,
                result_type,
                ..
            } => self.lower_new_slice(pool, *destination, *element, *length, *result_type)?,

            mir::Instruction::RawAlloc {
                destination,
                layout,
                ..
            } => self.lower_raw_alloc(*destination, *layout)?,

            mir::Instruction::RawFree { pointer } => self.lower_raw_free(*pointer)?,

            mir::Instruction::StackAlloc {
                destination,
                layout,
                ..
            } => self.lower_stack_alloc(*destination, *layout)?,

            mir::Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => self.lower_intrinsic(*destination, *intrinsic, *arguments, pool)?,

            mir::Instruction::AtomicLoad {
                destination,
                pointer,
                access,
                ..
            } => self.lower_atomic_load(pool, *destination, *pointer, *access)?,

            mir::Instruction::AtomicStore {
                pointer,
                value,
                access,
            } => self.lower_atomic_store(pool, *pointer, *value, *access)?,

            mir::Instruction::AtomicCompareExchange {
                destination,
                pointer,
                expected,
                new_value,
                is_weak,
                access,
            } => self.lower_atomic_compare_exchange(
                pool,
                *destination,
                *pointer,
                *expected,
                *new_value,
                *is_weak,
                *access,
            )?,

            mir::Instruction::AtomicRmw {
                destination,
                operator,
                pointer,
                value,
                access,
            } => self.lower_atomic_rmw(pool, *destination, *operator, *pointer, *value, *access)?,

            mir::Instruction::AtomicFence { access } => self.lower_atomic_fence(pool, *access),

            mir::Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => self.lower_barrier_write(*object, *offset, *byte_len)?,
            _ => return Err(Error::InvalidInstruction),
        })
    }
}

/// Return whether one MIR instruction belongs to the tensor domain.
fn is_tensor_instruction(inst: &mir::Instruction) -> bool {
    matches!(
        inst,
        mir::Instruction::TensorSplat { .. }
            | mir::Instruction::TensorLoad { .. }
            | mir::Instruction::TensorExtract { .. }
            | mir::Instruction::TensorStore { .. }
            | mir::Instruction::TensorFill { .. }
            | mir::Instruction::TensorCopy { .. }
            | mir::Instruction::TensorReshape { .. }
            | mir::Instruction::TensorBroadcast { .. }
            | mir::Instruction::TensorTranspose { .. }
            | mir::Instruction::TensorSlice { .. }
            | mir::Instruction::TensorPad { .. }
            | mir::Instruction::TensorConcat { .. }
            | mir::Instruction::TensorReduce { .. }
            | mir::Instruction::TensorDot { .. }
            | mir::Instruction::TensorConvolution { .. }
            | mir::Instruction::TensorGather { .. }
            | mir::Instruction::TensorScatter { .. }
            | mir::Instruction::TensorCompare { .. }
            | mir::Instruction::TensorSelect { .. }
            | mir::Instruction::TensorConvert { .. }
            | mir::Instruction::TensorCast { .. }
            | mir::Instruction::TensorView { .. }
    )
}
