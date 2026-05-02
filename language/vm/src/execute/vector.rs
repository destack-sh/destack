use super::access;
use super::element::element_access_for_type;
use super::index::word_to_usize;
use super::scalar::{
    ReduceOperator, ScalarConvertMode, ScalarLayout, binary_operator, convert_scalar_value,
    reduce_operator, scalar_layout,
};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    Instruction, Transfer, VectorCompare, VectorConvert, VectorExtract, VectorInsert, VectorReduce,
    VectorSelect, VectorShuffle, VectorSplat,
};
use destack_mir as mir;

/// Return the vector element count for one vector value id.
fn vector_element_count(
    state: &DispatchState<'_, '_>,
    value_id: mir::Value,
) -> Result<usize, Error> {
    let vector_type = state.value_type(value_id)?;

    match state.tree().get(vector_type) {
        mir::Type::Vector {
            lanes: elements, ..
        } => Ok(*elements as usize),
        _ => Err(Error::TypeMismatch {
            expected: "vector type".to_string(),
            actual: format!("{vector_type:?}"),
        }),
    }
}

/// Return the scalar element type for one vector value.
fn vector_element_type(
    state: &DispatchState<'_, '_>,
    value_id: mir::Value,
) -> Result<ScalarLayout, Error> {
    let vector_type = state.value_type(value_id)?;
    let element = match state.tree().get(vector_type) {
        mir::Type::Vector { element, .. } => {
            element.ty().ok_or_else(|| Error::MissingRepresentation {
                context: "vector element type".to_string(),
            })?
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: "vector type".to_string(),
                actual: format!("{vector_type:?}"),
            });
        }
    };

    scalar_layout(state.tree(), element)
}

/// Load one vector element through indexed access.
fn load_vector_element_at(
    state: &mut DispatchState<'_, '_>,
    vector: Word,
    vector_type: mir::LocalNodeId<mir::Type>,
    element_index: usize,
) -> Result<Word, Error> {
    let index = u32::try_from(element_index).map_err(|_| Error::TypeMismatch {
        expected: "vector element index".to_string(),
        actual: element_index.to_string(),
    })?;
    let (element, element_count) = element_access_for_type(state, vector_type, index.into())?;

    access::load_frame_element(state, vector, index.into(), element_count, element)
}

/// Store one vector result into frame bytes.
fn store_vector_elements<F>(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    mut element_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut DispatchState<'_, '_>, usize) -> Result<Word, Error>,
{
    super::frame::store_frame_elements(state, dest, |state, element_index, _value_type| {
        element_value(state, element_index)
    })
}

/// Execute vector.splat.
pub(crate) fn execute_vector_splat(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let VectorSplat { dest, value } = instruction.payload_as::<VectorSplat>();

    let element_value = state.get(*value);

    // store the same value into each element
    if let Err(error) =
        store_vector_elements(state, *dest, |_state, _element_index| Ok(element_value))
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.extract.
pub(crate) fn execute_vector_extract(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let VectorExtract {
        dest,
        vector,
        index,
    } = instruction.payload_as::<VectorExtract>();

    // resolve inputs
    let vec_value = state.get(*vector);
    let index_value = match word_to_usize(state.get(*index)) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let element_count = match vector_element_count(state, *vector) {
        Ok(elements) => elements,
        Err(error) => return Transfer::Error(error),
    };
    let vector_type = match state.value_type(*vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // extract element
    if index_value >= element_count {
        return Transfer::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: element_count as u64,
        });
    }
    let result = match load_vector_element_at(state, vec_value, vector_type, index_value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.insert.
pub(crate) fn execute_vector_insert(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let VectorInsert {
        dest,
        vector,
        index,
        value,
    } = instruction.payload_as::<VectorInsert>();

    // resolve inputs
    let vec_value = state.get(*vector);
    let index_value = match word_to_usize(state.get(*index)) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let element_count = match vector_element_count(state, *vector) {
        Ok(elements) => elements,
        Err(error) => return Transfer::Error(error),
    };
    let vector_type = match state.value_type(*vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // reject out of bounds element indices
    if index_value >= element_count {
        return Transfer::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: element_count as u64,
        });
    }

    let inserted_value = state.get(*value);

    // write the updated vector one element at a time
    if let Err(error) = store_vector_elements(state, *dest, |state, element_index| {
        if element_index == index_value {
            return Ok(inserted_value);
        }

        load_vector_element_at(state, vec_value, vector_type, element_index)
    }) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.shuffle.
pub(crate) fn execute_vector_shuffle(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let VectorShuffle {
        dest,
        left,
        right,
        mask,
    } = instruction.payload_as::<VectorShuffle>();
    let table = state.operand_table_ptr();
    let mask = unsafe { (*table).u32_range(*mask) };

    // resolve element sources
    let left_value = state.get(*left);
    let right_value = state.get(*right);
    let left_element_count = match vector_element_count(state, *left) {
        Ok(elements) => elements,
        Err(error) => return Transfer::Error(error),
    };
    let right_element_count = match vector_element_count(state, *right) {
        Ok(elements) => elements,
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

    // write the shuffled elements directly
    if let Err(error) = store_vector_elements(state, *dest, |state, element_index| {
        let index = *mask.get(element_index).ok_or(Error::IndexOutOfBounds {
            index: element_index as u64,
            length: mask.len() as u64,
        })? as usize;

        if index < left_element_count {
            return load_vector_element_at(state, left_value, left_type, index);
        }

        let right_index = index - left_element_count;
        if right_index >= right_element_count {
            return Err(Error::IndexOutOfBounds {
                index: index as u64,
                length: (left_element_count + right_element_count) as u64,
            });
        }

        load_vector_element_at(state, right_value, right_type, right_index)
    }) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.select.
pub(crate) fn execute_vector_select(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let VectorSelect {
        dest,
        mask,
        then_value,
        else_value,
    } = instruction.payload_as::<VectorSelect>();

    let mask_value = state.get(*mask);
    let then_vector = state.get(*then_value);
    let else_vector = state.get(*else_value);
    let mask_element_count = match vector_element_count(state, *mask) {
        Ok(elements) => elements,
        Err(error) => return Transfer::Error(error),
    };
    let then_element_count = match vector_element_count(state, *then_value) {
        Ok(elements) => elements,
        Err(error) => return Transfer::Error(error),
    };
    let else_element_count = match vector_element_count(state, *else_value) {
        Ok(elements) => elements,
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

    if mask_element_count != then_element_count || mask_element_count != else_element_count {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching vector elements".to_string(),
            actual: format!("{mask_element_count} vs {then_element_count} vs {else_element_count}"),
        });
    }

    // write the selected elements directly
    if let Err(error) = store_vector_elements(state, *dest, |state, element_index| {
        let mask_element = match load_vector_element_at(state, mask_value, mask_type, element_index)
        {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let then_element =
            match load_vector_element_at(state, then_vector, then_type, element_index) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
        let else_element =
            match load_vector_element_at(state, else_vector, else_type, element_index) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
        let select = mask_element.as_bool();
        Ok(if select { then_element } else { else_element })
    }) {
        return Transfer::Error(error);
    }
    Transfer::Continue
}

/// Execute vector.reduce.
pub(crate) fn execute_vector_reduce(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let VectorReduce {
        dest,
        operator,
        vector,
    } = instruction.payload_as::<VectorReduce>();

    // resolve vector elements
    let vec_value = state.get(*vector);
    let element_count = match vector_element_count(state, *vector) {
        Ok(elements) => elements,
        Err(error) => return Transfer::Error(error),
    };
    let vector_type = match state.value_type(*vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    if element_count == 0 {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let mut result = match load_vector_element_at(state, vec_value, vector_type, 0) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    let op = ReduceOperator::from(*operator);
    let element_type = match vector_element_type(state, *vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    for element_index in 1..element_count {
        let element = match load_vector_element_at(state, vec_value, vector_type, element_index) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        match reduce_operator(element_type, op, result, element) {
            Ok(value) => result = value,
            Err(error) => return Transfer::Error(error),
        }
    }

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.compare.
pub(crate) fn execute_vector_compare(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let VectorCompare {
        dest,
        operator,
        left,
        right,
    } = instruction.payload_as::<VectorCompare>();

    // resolve vector elements
    let left_value = state.get(*left);
    let right_value = state.get(*right);
    let left_element_count = match vector_element_count(state, *left) {
        Ok(elements) => elements,
        Err(error) => return Transfer::Error(error),
    };
    let right_element_count = match vector_element_count(state, *right) {
        Ok(elements) => elements,
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
    let element_type = match vector_element_type(state, *left) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };

    // validate vector element counts
    if left_element_count != right_element_count {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching vector elements".to_string(),
            actual: format!("{left_element_count} vs {right_element_count}"),
        });
    }

    // compare the vectors element by element
    if let Err(error) = store_vector_elements(state, *dest, |state, element_index| {
        let left_element = match load_vector_element_at(state, left_value, left_type, element_index)
        {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let right_element =
            match load_vector_element_at(state, right_value, right_type, element_index) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };

        binary_operator(element_type, *operator, left_element, right_element)
    }) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.convert.
pub(crate) fn execute_vector_convert(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let VectorConvert {
        dest,
        mode,
        vector,
        source_type,
        dest_type,
    } = instruction.payload_as::<VectorConvert>();

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
    let (dest_element, dest_elements) = match dest_vector {
        mir::Type::Vector {
            element,
            lanes: elements,
            ..
        } => (*element, *elements as usize),
        _ => {
            return Transfer::Error(Error::TypeMismatch {
                expected: "vector type".to_string(),
                actual: format!("{dest_type:?}"),
            });
        }
    };

    // resolve element values
    let vector_value = state.get(*vector);
    let vector_type = match state.value_type(*vector) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let source_element_count = match vector_element_count(state, *vector) {
        Ok(elements) => elements,
        Err(error) => return Transfer::Error(error),
    };
    if source_element_count != dest_elements {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching vector elements".to_string(),
            actual: format!("{source_element_count} vs {dest_elements}"),
        });
    }

    let Some(source_element) = source_element.ty() else {
        return Transfer::Error(Error::MissingRepresentation {
            context: "vector convert source element".to_string(),
        });
    };
    let Some(dest_element) = dest_element.ty() else {
        return Transfer::Error(Error::MissingRepresentation {
            context: "vector convert destination element".to_string(),
        });
    };
    let source_layout = match scalar_layout(state.tree(), source_element) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let dest_layout = match scalar_layout(state.tree(), dest_element) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let convert_mode = ScalarConvertMode::from(*mode);

    // convert the elements one by one
    if let Err(error) = store_vector_elements(state, *dest, |state, element_index| {
        let value = load_vector_element_at(state, vector_value, vector_type, element_index)?;

        convert_scalar_value(value, source_layout, dest_layout, convert_mode)
    }) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}
