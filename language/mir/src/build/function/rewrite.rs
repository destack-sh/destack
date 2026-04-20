use crate::build::FunctionBuilder;
use crate::{
    Block, CheckConstraint, Instruction, LocalNodeId, Parameter, Terminator, Value, ValueReference,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Replace all uses of a value with another value in the current function.
    pub(crate) fn replace_value(&mut self, from: Value, to: Value) {
        // skip no op replacements
        if from == to {
            return;
        }

        // update variable definitions
        for value in self.variable_definitions.values_mut() {
            Self::replace_plain_value(value, from, to);
        }

        // update incomplete phis
        for phis in self.incomplete_phis.values_mut() {
            for (_variable, phi_value) in phis {
                Self::replace_plain_value(phi_value, from, to);
            }
        }

        // update blocks
        let blocks = self.blocks.clone();
        for block in blocks {
            self.replace_value_in_block(block, from, to);
        }
    }

    /// Replace a value in a block.
    fn replace_value_in_block(&mut self, block: LocalNodeId<Block>, from: Value, to: Value) {
        // update instructions
        let instruction_ids = self.tree.get(block).instructions.clone();
        for instruction_id in instruction_ids {
            self.replace_value_in_instruction(instruction_id, from, to);
        }

        // update parameters and terminator
        let terminator_id = self.tree.get(block).terminator;
        let block_data = self.tree.get_mut(block);
        Self::replace_values_in_parameters(&mut block_data.parameters, from, to);

        let terminator = self.tree.get_mut(terminator_id);
        Self::replace_value_in_terminator(terminator, from, to);
    }

    /// Replace a value in an instruction.
    fn replace_value_in_instruction(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        from: Value,
        to: Value,
    ) {
        // update inline operands
        let argument_slice = {
            let instruction = self.tree.get_mut(instruction_id);
            let argument_slice = instruction.argument_slice();
            match instruction {
                Instruction::Error => {}
                Instruction::Const { .. }
                | Instruction::LocalGet { .. }
                | Instruction::LocalAddr { .. }
                | Instruction::GlobalAddr { .. }
                | Instruction::GlobalConst { .. }
                | Instruction::FunctionAddr { .. }
                | Instruction::ManagedAlloc { .. }
                | Instruction::RawAlloc { .. }
                | Instruction::StackAlloc { .. } => {}
                Instruction::FunctionBind { environment, .. } => {
                    Self::replace_value_in_slot(environment, from, to);
                }
                Instruction::FunctionEnvironment { .. } => {}
                Instruction::Binary { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::Unary { argument, .. }
                | Instruction::Cast { argument, .. }
                | Instruction::VectorSplat {
                    value: argument, ..
                }
                | Instruction::VectorReduce {
                    vector: argument, ..
                }
                | Instruction::VectorConvert {
                    vector: argument, ..
                }
                | Instruction::TensorReshape {
                    tensor: argument, ..
                }
                | Instruction::TensorBroadcast {
                    tensor: argument, ..
                }
                | Instruction::TensorTranspose {
                    tensor: argument, ..
                }
                | Instruction::TensorCast {
                    tensor: argument, ..
                }
                | Instruction::TensorSlice {
                    tensor: argument, ..
                }
                | Instruction::TensorConvert {
                    tensor: argument, ..
                }
                | Instruction::RawFree { pointer: argument }
                | Instruction::Dispose { value: argument }
                | Instruction::AsyncDispose { value: argument }
                | Instruction::Pin { value: argument }
                | Instruction::Unpin { value: argument }
                | Instruction::Drop { value: argument }
                | Instruction::AtomicLoad {
                    pointer: argument, ..
                } => {
                    Self::replace_value_in_slot(argument, from, to);
                }
                Instruction::CallIndirect { callee, .. } => {
                    Self::replace_value_in_slot(callee, from, to);
                }
                Instruction::VectorExtract { vector, index, .. } => {
                    Self::replace_value_in_slot(vector, from, to);
                    Self::replace_value_in_slot(index, from, to);
                }
                Instruction::VectorInsert {
                    vector,
                    index,
                    value,
                    ..
                } => {
                    Self::replace_value_in_slot(vector, from, to);
                    Self::replace_value_in_slot(index, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::VectorShuffle { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::VectorSelect {
                    mask,
                    then_value,
                    else_value,
                    ..
                } => {
                    Self::replace_value_in_slot(mask, from, to);
                    Self::replace_value_in_slot(then_value, from, to);
                    Self::replace_value_in_slot(else_value, from, to);
                }
                Instruction::VectorCompare { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::TensorLoad { view, .. } => {
                    Self::replace_value_in_slot(view, from, to);
                }
                Instruction::TensorView { view, .. } => {
                    Self::replace_value_in_slot(view, from, to);
                }
                Instruction::TensorStore { view, value, .. } => {
                    Self::replace_value_in_slot(view, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::TensorFill { view, value } => {
                    Self::replace_value_in_slot(view, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::TensorCopy { target, source } => {
                    Self::replace_value_in_slot(target, from, to);
                    Self::replace_value_in_slot(source, from, to);
                }
                Instruction::TensorPad { tensor, value, .. } => {
                    Self::replace_value_in_slot(tensor, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::TensorReduce {
                    tensor, initial, ..
                } => {
                    Self::replace_value_in_slot(tensor, from, to);
                    Self::replace_value_in_slot(initial, from, to);
                }
                Instruction::TensorDot { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::TensorConvolution { input, kernel, .. } => {
                    Self::replace_value_in_slot(input, from, to);
                    Self::replace_value_in_slot(kernel, from, to);
                }
                Instruction::TensorGather {
                    operand, indices, ..
                } => {
                    Self::replace_value_in_slot(operand, from, to);
                    Self::replace_value_in_slot(indices, from, to);
                }
                Instruction::TensorScatter {
                    operand,
                    indices,
                    updates,
                    ..
                } => {
                    Self::replace_value_in_slot(operand, from, to);
                    Self::replace_value_in_slot(indices, from, to);
                    Self::replace_value_in_slot(updates, from, to);
                }
                Instruction::TensorCompare { left, right, .. } => {
                    Self::replace_value_in_slot(left, from, to);
                    Self::replace_value_in_slot(right, from, to);
                }
                Instruction::TensorSelect {
                    mask,
                    then_value,
                    else_value,
                    ..
                } => {
                    Self::replace_value_in_slot(mask, from, to);
                    Self::replace_value_in_slot(then_value, from, to);
                    Self::replace_value_in_slot(else_value, from, to);
                }
                Instruction::CallVirtual { receiver, .. }
                | Instruction::CallInterface { receiver, .. } => {
                    Self::replace_value_in_slot(receiver, from, to);
                }
                Instruction::Select {
                    condition,
                    then_value,
                    else_value,
                    ..
                } => {
                    Self::replace_value_in_slot(condition, from, to);
                    Self::replace_value_in_slot(then_value, from, to);
                    Self::replace_value_in_slot(else_value, from, to);
                }
                Instruction::Load { pointer, .. } => {
                    Self::replace_value_in_slot(pointer, from, to);
                }
                Instruction::LocalSet { value, .. } => {
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::FieldGet { aggregate, .. }
                | Instruction::FieldAddr { aggregate, .. } => {
                    Self::replace_value_in_slot(aggregate, from, to);
                }
                Instruction::FieldSet {
                    aggregate, value, ..
                } => {
                    Self::replace_value_in_slot(aggregate, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::ElementGet { array, index, .. }
                | Instruction::ElementAddr { array, index, .. } => {
                    Self::replace_value_in_slot(array, from, to);
                    Self::replace_value_in_slot(index, from, to);
                }
                Instruction::ElementSet {
                    array,
                    index,
                    value,
                    ..
                } => {
                    Self::replace_value_in_slot(array, from, to);
                    Self::replace_value_in_slot(index, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::ManagedAllocArray { length, .. } => {
                    Self::replace_value_in_slot(length, from, to);
                }
                Instruction::AtomicStore { pointer, value, .. }
                | Instruction::Store { pointer, value } => {
                    Self::replace_value_in_slot(pointer, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::AtomicCompareExchange {
                    pointer,
                    expected,
                    new_value,
                    ..
                } => {
                    Self::replace_value_in_slot(pointer, from, to);
                    Self::replace_value_in_slot(expected, from, to);
                    Self::replace_value_in_slot(new_value, from, to);
                }
                Instruction::AtomicRmw { pointer, value, .. } => {
                    Self::replace_value_in_slot(pointer, from, to);
                    Self::replace_value_in_slot(value, from, to);
                }
                Instruction::AtomicFence { .. } | Instruction::Barrier { .. } => {}
                Instruction::Assume { condition } => {
                    Self::replace_value_in_slot(condition, from, to);
                }

                // arguments stored externally
                Instruction::Struct { .. }
                | Instruction::Tuple { .. }
                | Instruction::Array { .. }
                | Instruction::Call { .. }
                | Instruction::TensorConcat { .. }
                | Instruction::Intrinsic { .. } => {}
            }
            argument_slice
        };

        // update external arguments
        if let Some(argument_slice) = argument_slice {
            let start = argument_slice.start as usize;
            let end = start + argument_slice.count as usize;
            Self::replace_values_in_slice(
                &mut self.tree.instruction_arguments[start..end],
                from,
                to,
            );
        }
    }

    /// Replace a value in a terminator.
    fn replace_value_in_terminator(terminator: &mut Terminator, from: Value, to: Value) {
        // update terminator operands
        match terminator {
            Terminator::Error => {}
            Terminator::Return { value } => {
                if let Some(value) = value {
                    Self::replace_value_in_slot(value, from, to);
                }
            }
            Terminator::Jump { target } => {
                Self::replace_values_in_slice(&mut target.arguments, from, to);
            }
            Terminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => {
                Self::replace_value_in_slot(condition, from, to);
                Self::replace_values_in_slice(&mut then_target.arguments, from, to);
                Self::replace_values_in_slice(&mut else_target.arguments, from, to);
            }
            Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                Self::replace_values_in_check_kind(constraint, from, to);
                Self::replace_values_in_slice(&mut success.arguments, from, to);
                Self::replace_values_in_slice(&mut failure.arguments, from, to);
            }
            Terminator::Switch {
                value,
                default,
                cases,
                ..
            } => {
                Self::replace_value_in_slot(value, from, to);
                Self::replace_values_in_slice(&mut default.arguments, from, to);
                for case in cases {
                    Self::replace_values_in_slice(&mut case.target.arguments, from, to);
                }
            }
            Terminator::Yield { value, resume } => {
                Self::replace_value_in_slot(value, from, to);
                Self::replace_values_in_slice(&mut resume.arguments, from, to);
            }
            Terminator::Invoke {
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                Self::replace_values_in_slice(&mut call.arguments, from, to);
                Self::replace_values_in_slice(&mut normal_target.arguments, from, to);
                Self::replace_values_in_slice(&mut unwind_target.arguments, from, to);
            }
            Terminator::InvokeIndirect {
                callee,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                Self::replace_value_in_slot(callee, from, to);
                Self::replace_values_in_slice(&mut call.arguments, from, to);
                Self::replace_values_in_slice(&mut normal_target.arguments, from, to);
                Self::replace_values_in_slice(&mut unwind_target.arguments, from, to);
            }
            Terminator::InvokeVirtual {
                receiver,
                call,
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeInterface {
                receiver,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                Self::replace_value_in_slot(receiver, from, to);
                Self::replace_values_in_slice(&mut call.arguments, from, to);
                Self::replace_values_in_slice(&mut normal_target.arguments, from, to);
                Self::replace_values_in_slice(&mut unwind_target.arguments, from, to);
            }
            Terminator::Throw { value } => {
                Self::replace_value_in_slot(value, from, to);
            }
            Terminator::Trap { payload, .. } => {
                if let Some(payload) = payload {
                    Self::replace_value_in_slot(payload, from, to);
                }
            }
            Terminator::Unreachable => {}
            Terminator::TailCall { call, .. } => {
                Self::replace_values_in_slice(&mut call.arguments, from, to);
            }
            Terminator::TailCallIndirect { callee, call, .. } => {
                Self::replace_value_in_slot(callee, from, to);
                Self::replace_values_in_slice(&mut call.arguments, from, to);
            }
            Terminator::TailCallVirtual { receiver, call, .. }
            | Terminator::TailCallInterface { receiver, call, .. } => {
                Self::replace_value_in_slot(receiver, from, to);
                Self::replace_values_in_slice(&mut call.arguments, from, to);
            }
        }
    }

    /// Replace a value in a slot.
    fn replace_plain_value(value: &mut Value, from: Value, to: Value) {
        if *value == from {
            *value = to;
        }
    }

    /// Replace a value in a recoverable slot.
    fn replace_value_in_slot(value: &mut ValueReference, from: Value, to: Value) {
        if *value == ValueReference::Value(from) {
            *value = ValueReference::Value(to);
        }
    }

    /// Replace values referenced by a check kind.
    fn replace_values_in_check_kind(kind: &mut CheckConstraint, from: Value, to: Value) {
        // update values stored in the check kind
        match kind {
            CheckConstraint::Bounds {
                index,
                length,
                collection,
                ..
            } => {
                Self::replace_value_in_slot(index, from, to);
                Self::replace_value_in_slot(length, from, to);
                Self::replace_value_in_slot(collection, from, to);
            }
            CheckConstraint::Null { value } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::DivZero { divisor } => {
                Self::replace_value_in_slot(divisor, from, to);
            }
            CheckConstraint::ShiftRange { value, .. } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::Narrow { value, .. } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::Overflow { left, right, .. } => {
                Self::replace_value_in_slot(left, from, to);
                Self::replace_value_in_slot(right, from, to);
            }
            CheckConstraint::Type { value, .. } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::Union { value, .. } => {
                Self::replace_value_in_slot(value, from, to);
            }
            CheckConstraint::ReceiverType { receiver, .. } => {
                Self::replace_value_in_slot(receiver, from, to);
            }
            CheckConstraint::Implements { receiver, .. } => {
                Self::replace_value_in_slot(receiver, from, to);
            }
        }
    }

    /// Replace values in a slice.
    fn replace_values_in_slice(values: &mut [ValueReference], from: Value, to: Value) {
        // update each value
        for value in values {
            Self::replace_value_in_slot(value, from, to);
        }
    }

    /// Replace values in a parameter list.
    fn replace_values_in_parameters(parameters: &mut [Parameter], from: Value, to: Value) {
        // update each parameter value
        for parameter in parameters {
            Self::replace_value_in_slot(&mut parameter.value, from, to);
        }
    }
}
