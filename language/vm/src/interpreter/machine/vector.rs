use super::prelude::*;

// TODO #Performance: improve VM vector performance

/// Step vector.splat.
pub(crate) fn step_vector_splat(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::VectorSplat { dest, value, lanes } = &block[pc].data else {
        unreachable!()
    };

    // build lane values
    let lane_value = state.get(*value);
    let mut lanes_vec = Vec::with_capacity(*lanes as usize);
    for _ in 0..*lanes {
        lanes_vec.push(lane_value);
    }

    // allocate the vector aggregate
    let result = state.allocate_aggregate(lanes_vec);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step vector.extract.
pub(crate) fn step_vector_extract(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::VectorExtract {
        dest,
        vector,
        index,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve inputs
    let vec_value = state.get(*vector);
    let index_value = match value_to_usize(state.get(*index)) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };

    // extract lane
    let result = {
        let vec_slots = match aggregate_slots(state, vec_value) {
            Ok(slots) => slots,
            Err(error) => return Transfer::Error(error),
        };

        // reject out of bounds lane indices
        if index_value >= vec_slots.len() {
            return Transfer::Error(Error::IndexOutOfBounds {
                index: index_value as u64,
                length: vec_slots.len() as u64,
            });
        }
        match vec_slots.get(index_value) {
            Some(value) => value,
            None => unreachable!("lane bounds were validated above"),
        }
    };
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step vector.insert.
pub(crate) fn step_vector_insert(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::VectorInsert {
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
    let mut vec_slots = match aggregate_slots_vec(state, vec_value) {
        Ok(slots) => slots,
        Err(error) => return Transfer::Error(error),
    };
    let index_value = match value_to_usize(state.get(*index)) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };

    // reject out of bounds lane indices
    if index_value >= vec_slots.len() {
        return Transfer::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: vec_slots.len() as u64,
        });
    }

    // update lane
    vec_slots[index_value] = state.get(*value);
    let result = state.allocate_aggregate(vec_slots);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step vector.shuffle.
pub(crate) fn step_vector_shuffle(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::VectorShuffle {
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
    let left_slots = match aggregate_slots_vec(state, left_value) {
        Ok(slots) => slots,
        Err(error) => return Transfer::Error(error),
    };
    let right_slots = match aggregate_slots_vec(state, right_value) {
        Ok(slots) => slots,
        Err(error) => return Transfer::Error(error),
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
                return Transfer::Error(Error::IndexOutOfBounds {
                    index: idx as u64,
                    length: (left_slots.len() + right_slots.len()) as u64,
                });
            }
            result.push(right_slots[rhs]);
        }
    }

    // allocate result aggregate
    let result = state.allocate_aggregate(result);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step vector.select.
pub(crate) fn step_vector_select(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let InstructionData::VectorSelect {
        dest,
        mask,
        then_value,
        else_value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let mask_value = state.get(*mask);
    let then_value = state.get(*then_value);
    let else_value = state.get(*else_value);
    let mask_slots = match aggregate_slots(state, mask_value) {
        Ok(slots) => slots,
        Err(error) => return Transfer::Error(error),
    };
    let then_slots = match aggregate_slots(state, then_value) {
        Ok(slots) => slots,
        Err(error) => return Transfer::Error(error),
    };
    let else_slots = match aggregate_slots(state, else_value) {
        Ok(slots) => slots,
        Err(error) => return Transfer::Error(error),
    };

    if mask_slots.len() != then_slots.len() || mask_slots.len() != else_slots.len() {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching vector lanes".to_string(),
            actual: format!(
                "{} vs {} vs {}",
                mask_slots.len(),
                then_slots.len(),
                else_slots.len()
            ),
        });
    }

    // select one lane from the matching then or else vectors
    let mut output = Vec::with_capacity(mask_slots.len());
    for ((mask_value, then_lane), else_lane) in mask_slots
        .iter()
        .zip(then_slots.iter())
        .zip(else_slots.iter())
    {
        if !matches!(mask_value.tag(), ValueTag::Bool) {
            return Transfer::Error(Error::TypeMismatch {
                expected: "vector.select mask lane to be bool".to_string(),
                actual: format!("{mask_value:?}"),
            });
        }
        let select = mask_value.raw_data() != 0;
        output.push(if select { then_lane } else { else_lane });
    }

    // allocate the result vector aggregate
    let result = state.allocate_aggregate(output);
    state.set(*dest, result);
    next!(state, block, pc)
}

/// Step vector.reduce.
pub(crate) fn step_vector_reduce(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::VectorReduce {
        dest,
        operator,
        vector,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve vector lanes
    let vec_value = state.get(*vector);
    let result = {
        let vec_slots = match aggregate_slots(state, vec_value) {
            Ok(slots) => slots,
            Err(error) => return Transfer::Error(error),
        };

        // return void for empty vectors and otherwise reduce left to right
        if vec_slots.is_empty() {
            None
        } else {
            let mut result = vec_slots
                .get(0)
                .expect("non-empty vectors should have a first lane");
            let op = ReduceOperator::from(*operator);
            for lane_index in 1..vec_slots.len() {
                let lane = vec_slots
                    .get(lane_index)
                    .expect("lane index should stay within bounds");
                match apply_reduce_operator(op, result, lane) {
                    Ok(value) => result = value,
                    Err(error) => return Transfer::Error(error),
                }
            }
            Some(result)
        }
    };

    // store result
    state.set(*dest, result.unwrap_or(Value::VOID));

    // continue to next instruction
    next!(state, block, pc)
}

/// Step vector.compare.
pub(crate) fn step_vector_compare(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::VectorCompare {
        dest,
        operator,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve vector slots
    let left_value = state.get(*left);
    let right_value = state.get(*right);
    let left_slots = match aggregate_slots_vec(state, left_value) {
        Ok(slots) => slots,
        Err(error) => return Transfer::Error(error),
    };
    let right_slots = match aggregate_slots_vec(state, right_value) {
        Ok(slots) => slots,
        Err(error) => return Transfer::Error(error),
    };

    // validate lane counts
    if left_slots.len() != right_slots.len() {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching vector lanes".to_string(),
            actual: format!("{} vs {}", left_slots.len(), right_slots.len()),
        });
    }

    // compare lane values
    // compare the vectors lane by lane
    let mut output = Vec::with_capacity(left_slots.len());
    for (lhs, rhs) in left_slots.iter().zip(right_slots.iter()) {
        let value = match operator::execute_binary(*operator, *lhs, *rhs) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        output.push(value);
    }

    // allocate result aggregate
    let result = state.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step vector.convert.
pub(crate) fn step_vector_convert(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::VectorConvert {
        dest,
        mode,
        vector,
        source_type,
        dest_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve vector element types
    let source_element = match state.tree().get(*source_type) {
        mir::Type::Vector { element, .. } => *element,
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "vector type".to_string(),
                actual: format!("{source_type:?}"),
            });
        }
    };
    let dest_vector = state.tree().get(*dest_type);
    let (dest_element, dest_lanes) = match dest_vector {
        mir::Type::Vector { element, lanes, .. } => (*element, *lanes as usize),
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "vector type".to_string(),
                actual: format!("{dest_type:?}"),
            });
        }
    };

    // resolve lane values
    let vector_value = state.get(*vector);
    let output = {
        let source_slots = match aggregate_slots(state, vector_value) {
            Ok(slots) => slots,
            Err(error) => return Transfer::Error(error),
        };

        // validate lane counts
        if source_slots.len() != dest_lanes {
            return Transfer::Error(Error::TypeMismatch {
                expected: "matching vector lanes".to_string(),
                actual: format!("{} vs {}", source_slots.len(), dest_lanes),
            });
        }

        // resolve scalar conversion metadata once for the whole vector
        let source_info = match scalar_type_info(state.tree(), source_element) {
            Ok(info) => info,
            Err(error) => return Transfer::Error(error),
        };
        let dest_info = match scalar_type_info(state.tree(), dest_element) {
            Ok(info) => info,
            Err(error) => return Transfer::Error(error),
        };
        let convert_mode = ScalarConvertMode::from(*mode);

        // convert the lanes one by one
        let mut output = Vec::with_capacity(source_slots.len());
        for value in source_slots.iter() {
            let converted = match convert_scalar_value(value, source_info, dest_info, convert_mode)
            {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            };
            output.push(converted);
        }

        output
    };

    // allocate result aggregate
    let result = state.allocate_aggregate(output);
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
