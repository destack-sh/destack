use destack_mir as mir;
use mir::{Instruction, Terminator, ValueReference};

/// Return every value read by one instruction.
pub(super) fn instruction_uses(instruction: &Instruction, tree: &mir::Tree) -> Vec<ValueReference> {
    let mut values = instruction.uses().to_vec();

    if let Some(arguments) = instruction.argument_slice() {
        push_arguments(&mut values, tree.get_arguments(arguments));
    }

    values
}

/// Return values consumed by one instruction.
pub(super) fn instruction_consumes(
    instruction: &Instruction,
    tree: &mir::Tree,
) -> Vec<ValueReference> {
    let mut values = Vec::new();

    match instruction {
        Instruction::LocalSet { value, .. }
        | Instruction::Store { value, .. }
        | Instruction::TensorStore { value, .. }
        | Instruction::TensorFill { value, .. }
        | Instruction::Drop { value } => values.push(*value),
        Instruction::AtomicStore { value, .. } | Instruction::AtomicRmw { value, .. } => {
            values.push(*value)
        }
        Instruction::AtomicCompareExchange {
            expected,
            new_value,
            ..
        } => {
            values.push(*expected);
            values.push(*new_value);
        }
        Instruction::FieldSet {
            aggregate, value, ..
        }
        | Instruction::ElementSet {
            array: aggregate,
            value,
            ..
        } => {
            values.push(*aggregate);
            values.push(*value);
        }
        Instruction::CallableBind { environment, .. }
        | Instruction::VectorSplat {
            value: environment, ..
        }
        | Instruction::TensorSplat {
            value: environment, ..
        } => values.push(*environment),
        Instruction::VectorInsert { vector, value, .. }
        | Instruction::TensorPad {
            tensor: vector,
            value,
            ..
        } => {
            values.push(*vector);
            values.push(*value);
        }
        Instruction::TensorExtract { tensor, .. }
        | Instruction::TensorReshape { tensor, .. }
        | Instruction::TensorBroadcast { tensor, .. }
        | Instruction::TensorTranspose { tensor, .. }
        | Instruction::TensorCast { tensor, .. }
        | Instruction::TensorSlice { tensor, .. }
        | Instruction::TensorReduce { tensor, .. }
        | Instruction::TensorConvert { tensor, .. } => values.push(*tensor),
        Instruction::TensorDot { left, right, .. }
        | Instruction::TensorCompare { left, right, .. } => {
            values.push(*left);
            values.push(*right);
        }
        Instruction::TensorConvolution { input, kernel, .. } => {
            values.push(*input);
            values.push(*kernel);
        }
        Instruction::TensorGather {
            operand, indices, ..
        } => {
            values.push(*operand);
            values.push(*indices);
        }
        Instruction::TensorScatter {
            operand,
            indices,
            updates,
            ..
        } => {
            values.push(*operand);
            values.push(*indices);
            values.push(*updates);
        }
        Instruction::Struct { .. }
        | Instruction::Tuple { .. }
        | Instruction::Array { .. }
        | Instruction::TensorConcat { .. } => {
            if let Some(arguments) = instruction.argument_slice() {
                push_arguments(&mut values, tree.get_arguments(arguments));
            }
        }
        Instruction::Call { call, .. } => {
            push_arguments(&mut values, tree.get_arguments(call.arguments));
        }
        Instruction::CallClass { receiver, call, .. }
        | Instruction::CallInterface { receiver, call, .. } => {
            values.push(*receiver);
            push_arguments(&mut values, tree.get_arguments(call.arguments));
        }
        Instruction::CallIndirect { callee, call, .. } => {
            values.push(*callee);
            push_arguments(&mut values, tree.get_arguments(call.arguments));
        }
        Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => {
            let arguments = tree.get_arguments(*arguments);
            for &index in intrinsic.consumed_arguments() {
                let Some(&value) = arguments.get(index as usize) else {
                    continue;
                };

                values.push(value.into());
            }
        }
        _ => {}
    }

    values
}

/// Return values consumed by one terminator.
pub(super) fn terminator_consumes(terminator: &Terminator) -> Vec<ValueReference> {
    let mut values = Vec::new();

    match terminator {
        Terminator::Return { value: Some(value) }
        | Terminator::Throw { value }
        | Terminator::Yield { value, .. }
        | Terminator::Trap {
            payload: Some(value),
            ..
        } => values.push(*value),
        Terminator::Invoke { call, .. } | Terminator::TailCall { call, .. } => {
            push_arguments(&mut values, &call.arguments);
        }
        Terminator::InvokeIndirect { callee, call, .. } => {
            values.push(*callee);
            push_arguments(&mut values, &call.arguments);
        }
        Terminator::InvokeClass { receiver, call, .. }
        | Terminator::InvokeInterface { receiver, call, .. }
        | Terminator::TailCallClass { receiver, call, .. }
        | Terminator::TailCallInterface { receiver, call, .. } => {
            values.push(*receiver);
            push_arguments(&mut values, &call.arguments);
        }
        Terminator::TailCallIndirect { callee, call, .. } => {
            values.push(*callee);
            push_arguments(&mut values, &call.arguments);
        }
        _ => {}
    }

    values
}

/// Append SSA values as value references.
fn push_arguments(values: &mut Vec<ValueReference>, arguments: &[ValueReference]) {
    values.extend(arguments.iter().copied());
}
