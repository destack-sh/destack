use destack_mir as mir;
/// Return every value read by one instruction.
pub(super) fn instruction_uses(
    instruction: &mir::Instruction,
    tree: &mir::Tree,
) -> Vec<mir::ValueReference> {
    let mut values = instruction.uses().to_vec();

    if let Some(arguments) = instruction.argument_slice() {
        push_arguments(&mut values, tree.get_arguments(arguments));
    }

    values
}

/// Return values consumed by one instruction.
pub(super) fn instruction_consumes(
    instruction: &mir::Instruction,
    tree: &mir::Tree,
) -> Vec<mir::ValueReference> {
    let mut values = Vec::new();

    match instruction {
        mir::Instruction::LocalSet { value, .. }
        | mir::Instruction::Store { value, .. }
        | mir::Instruction::NewComplete { value, .. }
        | mir::Instruction::TensorStore { value, .. }
        | mir::Instruction::TensorFill { value, .. }
        | mir::Instruction::Free { value } => values.push(*value),
        mir::Instruction::AtomicStore { value, .. } | mir::Instruction::AtomicRmw { value, .. } => {
            values.push(*value)
        }
        mir::Instruction::AtomicCompareExchange {
            expected,
            new_value,
            ..
        } => {
            values.push(*expected);
            values.push(*new_value);
        }
        mir::Instruction::FieldSet {
            aggregate, value, ..
        }
        | mir::Instruction::ElementSet {
            array: aggregate,
            value,
            ..
        } => {
            values.push(*aggregate);
            values.push(*value);
        }
        mir::Instruction::CallableBind { environment, .. }
        | mir::Instruction::VectorSplat {
            value: environment, ..
        }
        | mir::Instruction::TensorSplat {
            value: environment, ..
        } => values.push(*environment),
        mir::Instruction::VectorInsert { vector, value, .. }
        | mir::Instruction::TensorPad {
            tensor: vector,
            value,
            ..
        } => {
            values.push(*vector);
            values.push(*value);
        }
        mir::Instruction::TensorExtract { tensor, .. }
        | mir::Instruction::TensorReshape { tensor, .. }
        | mir::Instruction::TensorBroadcast { tensor, .. }
        | mir::Instruction::TensorTranspose { tensor, .. }
        | mir::Instruction::TensorCast { tensor, .. }
        | mir::Instruction::TensorSlice { tensor, .. }
        | mir::Instruction::TensorReduce { tensor, .. }
        | mir::Instruction::TensorIndexReduce { tensor, .. }
        | mir::Instruction::TensorConvert { tensor, .. } => values.push(*tensor),
        mir::Instruction::TensorDot { left, right, .. }
        | mir::Instruction::TensorCompare { left, right, .. } => {
            values.push(*left);
            values.push(*right);
        }
        mir::Instruction::TensorConvolution { input, kernel, .. } => {
            values.push(*input);
            values.push(*kernel);
        }
        mir::Instruction::TensorGather {
            operand, indices, ..
        } => {
            values.push(*operand);
            values.push(*indices);
        }
        mir::Instruction::TensorScatter {
            operand,
            indices,
            updates,
            ..
        } => {
            values.push(*operand);
            values.push(*indices);
            values.push(*updates);
        }
        mir::Instruction::Struct { .. }
        | mir::Instruction::Tuple { .. }
        | mir::Instruction::Array { .. }
        | mir::Instruction::TensorConcat { .. } => {
            if let Some(arguments) = instruction.argument_slice() {
                push_arguments(&mut values, tree.get_arguments(arguments));
            }
        }
        mir::Instruction::Call { call, .. } => {
            push_arguments(&mut values, tree.get_arguments(call.arguments));
        }
        mir::Instruction::CallClass { receiver, call, .. }
        | mir::Instruction::CallInterface { receiver, call, .. } => {
            values.push(*receiver);
            push_arguments(&mut values, tree.get_arguments(call.arguments));
        }
        mir::Instruction::CallIndirect { callee, call, .. } => {
            values.push(*callee);
            push_arguments(&mut values, tree.get_arguments(call.arguments));
        }
        mir::Instruction::Intrinsic {
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
pub(super) fn terminator_consumes(terminator: &mir::Terminator) -> Vec<mir::ValueReference> {
    let mut values = Vec::new();

    match terminator {
        mir::Terminator::Return { value: Some(value) } | mir::Terminator::Yield { value, .. } => {
            values.push(*value)
        }
        mir::Terminator::Call { call, .. } | mir::Terminator::TailCall { call, .. } => {
            push_arguments(&mut values, &call.arguments);
        }
        mir::Terminator::CallIndirect { callee, call, .. } => {
            values.push(*callee);
            push_arguments(&mut values, &call.arguments);
        }
        mir::Terminator::CallClass { receiver, call, .. }
        | mir::Terminator::CallInterface { receiver, call, .. }
        | mir::Terminator::TailCallClass { receiver, call, .. }
        | mir::Terminator::TailCallInterface { receiver, call, .. } => {
            values.push(*receiver);
            push_arguments(&mut values, &call.arguments);
        }
        mir::Terminator::TailCallIndirect { callee, call, .. } => {
            values.push(*callee);
            push_arguments(&mut values, &call.arguments);
        }
        _ => {}
    }

    values
}

/// Append SSA values as value references.
fn push_arguments(values: &mut Vec<mir::ValueReference>, arguments: &[mir::ValueReference]) {
    values.extend(arguments.iter().copied());
}
