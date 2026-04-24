use super::prelude::*;

// TODO #Performance: improve VM vector performance

/// Return the lane count for one vector value id.
fn vector_lane_count(state: &StepState<'_, '_>, value_id: mir::Value) -> Result<usize, Error> {
    let vector_type = state.value_type(value_id)?;

    match state.tree().get(vector_type) {
        mir::Type::Vector { lanes, .. } => Ok(*lanes as usize),
        _ => Err(Error::TypeMismatch {
            expected: "vector type".to_string(),
            actual: format!("{vector_type:?}"),
        }),
    }
}

/// Load one vector lane through indexed storage.
fn vector_lane_value(
    state: &mut StepState<'_, '_>,
    vector: Value,
    vector_type: mir::LocalNodeId<mir::Type>,
    lane_index: usize,
) -> Result<Value, Error> {
    let index = u32::try_from(lane_index).map_err(|_| Error::TypeMismatch {
        expected: "vector lane index".to_string(),
        actual: lane_index.to_string(),
    })?;
    let (element, element_count) =
        access::element_access_for_type(state, vector_type, index.into())?;

    access::get_element(state, vector, index.into(), element_count, Some(element))
}

/// Materialize one vector result lane by lane.
fn materialize_vector_by_lane<F>(
    state: &mut StepState<'_, '_>,
    dest: mir::Value,
    mut lane_value: F,
) -> Result<Value, Error>
where
    F: FnMut(&mut StepState<'_, '_>, usize) -> Result<Value, Error>,
{
    materialize_composite_by_index(state, dest, |state, lane_index, _value_type| {
        let lane_index = usize::try_from(lane_index).map_err(|_| Error::TypeMismatch {
            expected: "vector lane index".to_string(),
            actual: lane_index.to_string(),
        })?;

        lane_value(state, lane_index)
    })
}

/// Step vector.splat.
pub(crate) fn step_vector_splat(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::VectorSplat { dest, value } = &block[pc].immediate else {
        unreachable!()
    };

    let lane_value = state.get(*value);

    // materialize the result one lane at a time
    let result =
        match materialize_vector_by_lane(state, *dest, |_state, _lane_index| Ok(lane_value)) {
            Ok(result) => result,
            Err(error) => return Transfer::Error(error),
        };
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
    // decode instruction immediate
    let Immediate::VectorExtract {
        dest,
        vector,
        index,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve inputs
    let vec_value = state.get(*vector);
    let index_value = match value_to_usize(state.get(*index)) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let lane_count = match vector_lane_count(state, *vector) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let vector_type = match state.value_type(*vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // extract lane
    if index_value >= lane_count {
        return Transfer::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: lane_count as u64,
        });
    }
    let result = match vector_lane_value(state, vec_value, vector_type, index_value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
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
    // decode instruction immediate
    let Immediate::VectorInsert {
        dest,
        vector,
        index,
        value,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve inputs
    let vec_value = state.get(*vector);
    let index_value = match value_to_usize(state.get(*index)) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let lane_count = match vector_lane_count(state, *vector) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let vector_type = match state.value_type(*vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // reject out of bounds lane indices
    if index_value >= lane_count {
        return Transfer::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: lane_count as u64,
        });
    }

    let inserted_value = state.get(*value);

    // materialize the updated vector one lane at a time
    let result = match materialize_vector_by_lane(state, *dest, |state, lane_index| {
        if lane_index == index_value {
            return Ok(inserted_value);
        }

        vector_lane_value(state, vec_value, vector_type, lane_index)
    }) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };
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
    // decode instruction immediate
    let Immediate::VectorShuffle {
        dest,
        left,
        right,
        mask,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve lane sources
    let left_value = state.get(*left);
    let right_value = state.get(*right);
    let left_lane_count = match vector_lane_count(state, *left) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let right_lane_count = match vector_lane_count(state, *right) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let left_type = match state.value_type(*left) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let right_type = match state.value_type(*right) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // materialize the shuffled lanes directly
    let result = match materialize_vector_by_lane(state, *dest, |state, lane_index| {
        let idx = *mask.get(lane_index).ok_or(Error::IndexOutOfBounds {
            index: lane_index as u64,
            length: mask.len() as u64,
        })? as usize;

        if idx < left_lane_count {
            return vector_lane_value(state, left_value, left_type, idx);
        }

        let rhs = idx - left_lane_count;
        if rhs >= right_lane_count {
            return Err(Error::IndexOutOfBounds {
                index: idx as u64,
                length: (left_lane_count + right_lane_count) as u64,
            });
        }

        vector_lane_value(state, right_value, right_type, rhs)
    }) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };
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
    let Immediate::VectorSelect {
        dest,
        mask,
        then_value,
        else_value,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    let mask_value = state.get(*mask);
    let then_vector = state.get(*then_value);
    let else_vector = state.get(*else_value);
    let mask_lane_count = match vector_lane_count(state, *mask) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let then_lane_count = match vector_lane_count(state, *then_value) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let else_lane_count = match vector_lane_count(state, *else_value) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let mask_type = match state.value_type(*mask) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let then_type = match state.value_type(*then_value) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let else_type = match state.value_type(*else_value) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    if mask_lane_count != then_lane_count || mask_lane_count != else_lane_count {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching vector lanes".to_string(),
            actual: format!("{mask_lane_count} vs {then_lane_count} vs {else_lane_count}"),
        });
    }

    // materialize the selected lanes directly
    let result = match materialize_vector_by_lane(state, *dest, |state, lane_index| {
        let mask_lane = match vector_lane_value(state, mask_value, mask_type, lane_index) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        if !matches!(mask_lane.tag(), ValueTag::Bool) {
            return Err(Error::TypeMismatch {
                expected: "vector.select mask lane to be bool".to_string(),
                actual: format!("{mask_lane:?}"),
            });
        }
        let then_lane = match vector_lane_value(state, then_vector, then_type, lane_index) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let else_lane = match vector_lane_value(state, else_vector, else_type, lane_index) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let select = mask_lane.raw_data() != 0;
        Ok(if select { then_lane } else { else_lane })
    }) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };
    state.set(*dest, result);
    next!(state, block, pc)
}

/// Step vector.reduce.
pub(crate) fn step_vector_reduce(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::VectorReduce {
        dest,
        operator,
        vector,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve vector lanes
    let vec_value = state.get(*vector);
    let lane_count = match vector_lane_count(state, *vector) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let vector_type = match state.value_type(*vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let result = if lane_count == 0 {
        None
    } else {
        let mut result = match vector_lane_value(state, vec_value, vector_type, 0) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        let op = ReduceOperator::from(*operator);
        for lane_index in 1..lane_count {
            let lane = match vector_lane_value(state, vec_value, vector_type, lane_index) {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            };
            match apply_reduce_operator(op, result, lane) {
                Ok(value) => result = value,
                Err(error) => return Transfer::Error(error),
            }
        }
        Some(result)
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
    // decode instruction immediate
    let Immediate::VectorCompare {
        dest,
        operator,
        left,
        right,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve vector slots
    let left_value = state.get(*left);
    let right_value = state.get(*right);
    let left_lane_count = match vector_lane_count(state, *left) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let right_lane_count = match vector_lane_count(state, *right) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    let left_type = match state.value_type(*left) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let right_type = match state.value_type(*right) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // validate lane counts
    if left_lane_count != right_lane_count {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching vector lanes".to_string(),
            actual: format!("{left_lane_count} vs {right_lane_count}"),
        });
    }

    // compare the vectors lane by lane
    let result = match materialize_vector_by_lane(state, *dest, |state, lane_index| {
        let lhs = match vector_lane_value(state, left_value, left_type, lane_index) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let rhs = match vector_lane_value(state, right_value, right_type, lane_index) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        operator::execute_binary(*operator, lhs, rhs)
    }) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };
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
    // decode instruction immediate
    let Immediate::VectorConvert {
        dest,
        mode,
        vector,
        source_type,
        dest_type,
    } = &block[pc].immediate
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
    let vector_type = match state.value_type(*vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let source_lane_count = match vector_lane_count(state, *vector) {
        Ok(lanes) => lanes,
        Err(error) => return Transfer::Error(error),
    };
    if source_lane_count != dest_lanes {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching vector lanes".to_string(),
            actual: format!("{source_lane_count} vs {dest_lanes}"),
        });
    }

    let Some(source_element) = source_element.ty() else {
        return Transfer::Error(Error::ConcreteMirRequired {
            context: "vector convert source element".to_string(),
        });
    };
    let Some(dest_element) = dest_element.ty() else {
        return Transfer::Error(Error::ConcreteMirRequired {
            context: "vector convert destination element".to_string(),
        });
    };
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
    let result = match materialize_vector_by_lane(state, *dest, |state, lane_index| {
        let value = vector_lane_value(state, vector_value, vector_type, lane_index)?;

        convert_scalar_value(value, source_info, dest_info, convert_mode)
    }) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
