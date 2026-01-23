use super::*;

// FUGU #Performance: improve VM vector performance

/// Handle vector.splat.
pub(crate) fn handle_vector_splat(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::VectorSplat { dest, value, lanes } = &block[pc].data else {
        unreachable!()
    };

    // build lane values
    let lane_value = state.get(*value);
    let mut lanes_vec = Vec::with_capacity(*lanes as usize);
    for _ in 0..*lanes {
        lanes_vec.push(lane_value);
    }

    // allocate the vector aggregate
    let result = state.interpreter.allocate_aggregate(lanes_vec);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle vector.extract.
pub(crate) fn handle_vector_extract(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::VectorExtract {
        dest,
        vector,
        index,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve inputs
    let vec_value = state.get(*vector);
    let vec_slots = match aggregate_slots(state, vec_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };
    let index_value = match value_to_usize(state.get(*index)) {
        Ok(index) => index,
        Err(error) => return ControlFlow::Error(error),
    };
    if index_value >= vec_slots.len() {
        return ControlFlow::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: vec_slots.len() as u64,
        });
    }

    // extract lane
    let result = vec_slots[index_value];
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle vector.insert.
pub(crate) fn handle_vector_insert(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::VectorInsert {
        dest,
        vector,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve inputs
    let vec_value = state.get(*vector);
    let mut vec_slots = match aggregate_slots(state, vec_value) {
        Ok(slots) => slots.to_vec(),
        Err(error) => return ControlFlow::Error(error),
    };
    let index_value = match value_to_usize(state.get(*index)) {
        Ok(index) => index,
        Err(error) => return ControlFlow::Error(error),
    };
    if index_value >= vec_slots.len() {
        return ControlFlow::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: vec_slots.len() as u64,
        });
    }

    // update lane
    vec_slots[index_value] = state.get(*value);
    let result = state.interpreter.allocate_aggregate(vec_slots);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle vector.shuffle.
pub(crate) fn handle_vector_shuffle(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::VectorShuffle {
        dest,
        left,
        right,
        mask,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve lane sources
    let left_value = state.get(*left);
    let right_value = state.get(*right);
    let left_slots = match aggregate_slots(state, left_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };
    let right_slots = match aggregate_slots(state, right_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply mask
    let mut result = Vec::with_capacity(mask.len());
    for index in mask {
        let idx = *index as usize;
        if idx < left_slots.len() {
            result.push(left_slots[idx]);
        } else {
            let rhs = idx - left_slots.len();
            if rhs >= right_slots.len() {
                return ControlFlow::Error(Error::IndexOutOfBounds {
                    index: idx as u64,
                    length: (left_slots.len() + right_slots.len()) as u64,
                });
            }
            result.push(right_slots[rhs]);
        }
    }

    // allocate result aggregate
    let result = state.interpreter.allocate_aggregate(result);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle vector.reduce.
pub(crate) fn handle_vector_reduce(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::VectorReduce {
        dest,
        operator,
        vector,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve vector lanes
    let vec_value = state.get(*vector);
    let vec_slots = match aggregate_slots(state, vec_value) {
        Ok(slots) => slots,
        Err(error) => return ControlFlow::Error(error),
    };
    if vec_slots.is_empty() {
        state.set(*dest, Value::VOID);
        next!(state, block, pc)
    }

    // reduce lanes
    let mut result = vec_slots[0];
    let op = ReduceOperator::from(*operator);
    for lane in &vec_slots[1..] {
        match apply_reduce_operator(op, result, *lane) {
            Ok(value) => result = value,
            Err(error) => return ControlFlow::Error(error),
        }
    }

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
