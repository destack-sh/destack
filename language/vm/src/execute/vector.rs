use super::access;
use super::index::word_to_usize;
use super::scalar::{
    convert_scalar_exact, convert_scalar_round_ceil, convert_scalar_round_floor,
    convert_scalar_round_ties_even, convert_scalar_round_toward_zero, convert_scalar_saturate,
    reduce_add, reduce_and, reduce_max, reduce_min, reduce_multiply, reduce_or, reduce_xor,
};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    ElementAccess, Instruction, ScalarLayout, Transfer, VectorBinary, VectorConvert, VectorExtract,
    VectorInsert, VectorReduce, VectorSelect, VectorShuffle, VectorSplat, VectorUnary,
};

macro_rules! vector_binary_executor {
    ($function:ident, $operation:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
            execute_vector_binary(machine, instruction, super::scalar::$operation)
        }
    };
}

macro_rules! vector_unary_executor {
    ($function:ident, $operation:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
            execute_vector_unary(machine, instruction, super::scalar::$operation)
        }
    };
}

/// Load one vector element through a precomputed element access.
#[inline(always)]
fn load_vector_element(
    machine: &mut Machine<'_, '_>,
    vector_offset: u32,
    element: ElementAccess,
    element_index: usize,
) -> Result<Word, Error> {
    let element_offset = element.byte_stride * element_index;
    let pointer = machine
        .frame_pointer_at(vector_offset)
        .add_bytes(element_offset);

    access::load_frame_scalar_by_layout(machine, pointer, element.into())
}

/// Store one vector result into frame bytes.
fn store_vector_elements<F>(
    machine: &mut Machine<'_, '_>,
    dest_offset: u32,
    dest_element: ElementAccess,
    element_count: u32,
    mut element_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut Machine<'_, '_>, usize) -> Result<Word, Error>,
{
    for element_index in 0..element_count as usize {
        let value = element_value(machine, element_index)?;
        let element_offset = dest_element.byte_stride * element_index;
        let pointer = machine
            .frame_pointer_at(dest_offset)
            .add_bytes(element_offset);

        access::store_frame_scalar_by_layout(machine, pointer, dest_element.into(), value)?;
    }

    Ok(())
}

/// Execute a vector binary operation.
fn execute_vector_binary<F>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: F,
) -> Transfer
where
    F: Fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
{
    // decode fixed fields
    let VectorBinary {
        dest_offset,
        left_offset,
        right_offset,
        dest_element,
        left_element,
        right_element,
        element_layout,
        element_count,
    } = machine.side::<VectorBinary>(instruction);

    // apply the scalar operation to each vector element
    if let Err(error) = store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |machine, element_index| {
            let left = load_vector_element(machine, *left_offset, *left_element, element_index)?;
            let right = load_vector_element(machine, *right_offset, *right_element, element_index)?;

            operation(*element_layout, left, right)
        },
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute a vector unary operation.
fn execute_vector_unary<F>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: F,
) -> Transfer
where
    F: Fn(ScalarLayout, Word) -> Result<Word, Error>,
{
    // decode fixed fields
    let VectorUnary {
        dest_offset,
        argument_offset,
        dest_element,
        argument_element,
        element_layout,
        element_count,
    } = machine.side::<VectorUnary>(instruction);

    // apply the scalar operation to each vector element
    if let Err(error) = store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |machine, element_index| {
            let value =
                load_vector_element(machine, *argument_offset, *argument_element, element_index)?;

            operation(*element_layout, value)
        },
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

vector_binary_executor!(
    execute_vector_and_bool,
    and_bool,
    "Execute vector boolean and."
);
vector_binary_executor!(
    execute_vector_or_bool,
    or_bool,
    "Execute vector boolean or."
);
vector_binary_executor!(
    execute_vector_xor_bool,
    xor_bool,
    "Execute vector boolean xor."
);
vector_binary_executor!(
    execute_vector_add_int,
    add_int,
    "Execute vector integer add."
);
vector_binary_executor!(
    execute_vector_sub_int,
    sub_int,
    "Execute vector integer subtract."
);
vector_binary_executor!(
    execute_vector_mul_int,
    mul_int,
    "Execute vector integer multiply."
);
vector_binary_executor!(
    execute_vector_div_int,
    div_int,
    "Execute vector signed integer divide."
);
vector_binary_executor!(
    execute_vector_div_uint,
    div_uint,
    "Execute vector unsigned integer divide."
);
vector_binary_executor!(
    execute_vector_rem_int,
    rem_int,
    "Execute vector signed integer remainder."
);
vector_binary_executor!(
    execute_vector_rem_uint,
    rem_uint,
    "Execute vector unsigned integer remainder."
);
vector_binary_executor!(
    execute_vector_and_int,
    and_int,
    "Execute vector integer and."
);
vector_binary_executor!(execute_vector_or_int, or_int, "Execute vector integer or.");
vector_binary_executor!(
    execute_vector_xor_int,
    xor_int,
    "Execute vector integer xor."
);
vector_binary_executor!(
    execute_vector_shl_int,
    shl_int,
    "Execute vector integer shift left."
);
vector_binary_executor!(
    execute_vector_shr_int,
    shr_int,
    "Execute vector signed integer shift right."
);
vector_binary_executor!(
    execute_vector_shr_uint,
    shr_uint,
    "Execute vector unsigned integer shift right."
);
vector_binary_executor!(
    execute_vector_add_f32,
    add_f32,
    "Execute vector float32 add."
);
vector_binary_executor!(
    execute_vector_add_f64,
    add_f64,
    "Execute vector float64 add."
);
vector_binary_executor!(
    execute_vector_sub_f32,
    sub_f32,
    "Execute vector float32 subtract."
);
vector_binary_executor!(
    execute_vector_sub_f64,
    sub_f64,
    "Execute vector float64 subtract."
);
vector_binary_executor!(
    execute_vector_mul_f32,
    mul_f32,
    "Execute vector float32 multiply."
);
vector_binary_executor!(
    execute_vector_mul_f64,
    mul_f64,
    "Execute vector float64 multiply."
);
vector_binary_executor!(
    execute_vector_div_f32,
    div_f32,
    "Execute vector float32 divide."
);
vector_binary_executor!(
    execute_vector_div_f64,
    div_f64,
    "Execute vector float64 divide."
);
vector_binary_executor!(
    execute_vector_eq_int,
    eq_int,
    "Execute vector integer equality."
);
vector_binary_executor!(
    execute_vector_eq_bool,
    eq_bool,
    "Execute vector boolean equality."
);
vector_binary_executor!(
    execute_vector_ne_int,
    ne_int,
    "Execute vector integer inequality."
);
vector_binary_executor!(
    execute_vector_ne_bool,
    ne_bool,
    "Execute vector boolean inequality."
);
vector_binary_executor!(
    execute_vector_lt_int,
    lt_int,
    "Execute vector signed integer less than."
);
vector_binary_executor!(
    execute_vector_lt_uint,
    lt_uint,
    "Execute vector unsigned integer less than."
);
vector_binary_executor!(
    execute_vector_le_int,
    le_int,
    "Execute vector signed integer less than or equal."
);
vector_binary_executor!(
    execute_vector_le_uint,
    le_uint,
    "Execute vector unsigned integer less than or equal."
);
vector_binary_executor!(
    execute_vector_gt_int,
    gt_int,
    "Execute vector signed integer greater than."
);
vector_binary_executor!(
    execute_vector_gt_uint,
    gt_uint,
    "Execute vector unsigned integer greater than."
);
vector_binary_executor!(
    execute_vector_ge_int,
    ge_int,
    "Execute vector signed integer greater than or equal."
);
vector_binary_executor!(
    execute_vector_ge_uint,
    ge_uint,
    "Execute vector unsigned integer greater than or equal."
);
vector_binary_executor!(
    execute_vector_eq_f32,
    eq_f32,
    "Execute vector float32 equality."
);
vector_binary_executor!(
    execute_vector_eq_f64,
    eq_f64,
    "Execute vector float64 equality."
);
vector_binary_executor!(
    execute_vector_ne_f32,
    ne_f32,
    "Execute vector float32 inequality."
);
vector_binary_executor!(
    execute_vector_ne_f64,
    ne_f64,
    "Execute vector float64 inequality."
);
vector_binary_executor!(
    execute_vector_lt_f32,
    lt_f32,
    "Execute vector float32 less than."
);
vector_binary_executor!(
    execute_vector_lt_f64,
    lt_f64,
    "Execute vector float64 less than."
);
vector_binary_executor!(
    execute_vector_le_f32,
    le_f32,
    "Execute vector float32 less than or equal."
);
vector_binary_executor!(
    execute_vector_le_f64,
    le_f64,
    "Execute vector float64 less than or equal."
);
vector_binary_executor!(
    execute_vector_gt_f32,
    gt_f32,
    "Execute vector float32 greater than."
);
vector_binary_executor!(
    execute_vector_gt_f64,
    gt_f64,
    "Execute vector float64 greater than."
);
vector_binary_executor!(
    execute_vector_ge_f32,
    ge_f32,
    "Execute vector float32 greater than or equal."
);
vector_binary_executor!(
    execute_vector_ge_f64,
    ge_f64,
    "Execute vector float64 greater than or equal."
);
vector_unary_executor!(
    execute_vector_neg_int,
    neg_int,
    "Execute vector integer negation."
);
vector_unary_executor!(
    execute_vector_not_int,
    not_int,
    "Execute vector integer inversion."
);
vector_unary_executor!(
    execute_vector_neg_f32,
    neg_f32,
    "Execute vector float32 negation."
);
vector_unary_executor!(
    execute_vector_neg_f64,
    neg_f64,
    "Execute vector float64 negation."
);
vector_unary_executor!(
    execute_vector_not_bool,
    not_bool,
    "Execute vector boolean inversion."
);

/// Execute vector.splat.
pub(crate) fn execute_vector_splat(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let VectorSplat {
        dest_offset,
        value_offset,
        dest_element,
        element_count,
    } = machine.side::<VectorSplat>(instruction);

    let element_value = machine.get_word_at(*value_offset);

    // store the same value into each element
    if let Err(error) = store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |_machine, _element_index| Ok(element_value),
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.extract.
pub(crate) fn execute_vector_extract(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let VectorExtract {
        dest_offset,
        vector_offset,
        index_offset,
        vector_element,
        element_count,
    } = machine.side::<VectorExtract>(instruction);

    // resolve inputs
    let index_value = match word_to_usize(machine.get_word_at(*index_offset)) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let element_count = *element_count as usize;
    if index_value >= element_count {
        return Transfer::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: element_count as u64,
        });
    }

    // extract element
    let result = match load_vector_element(machine, *vector_offset, *vector_element, index_value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word_at(*dest_offset, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.insert.
pub(crate) fn execute_vector_insert(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let VectorInsert {
        dest_offset,
        vector_offset,
        index_offset,
        value_offset,
        dest_element,
        vector_element,
        element_count,
    } = machine.side::<VectorInsert>(instruction);

    // resolve inputs
    let index_value = match word_to_usize(machine.get_word_at(*index_offset)) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let element_count = *element_count;
    let element_count_usize = element_count as usize;

    // reject out of bounds element indices
    if index_value >= element_count_usize {
        return Transfer::Error(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: element_count as u64,
        });
    }

    let inserted_value = machine.get_word_at(*value_offset);

    // write the updated vector one element at a time
    if let Err(error) = store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        element_count,
        |machine, element_index| {
            if element_index == index_value {
                return Ok(inserted_value);
            }

            load_vector_element(machine, *vector_offset, *vector_element, element_index)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.shuffle.
pub(crate) fn execute_vector_shuffle(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let VectorShuffle {
        dest_offset,
        left_offset,
        right_offset,
        mask,
        dest_element,
        left_element,
        right_element,
        left_count,
        right_count,
    } = machine.side::<VectorShuffle>(instruction);
    let table = machine.side_table_ptr();
    let mask = unsafe { (*table).u32_range(*mask) };

    // resolve element sources
    let left_count = *left_count as usize;
    let right_count = *right_count as usize;

    // write the shuffled elements directly
    if let Err(error) = store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        mask.len() as u32,
        |machine, element_index| {
            let index = *mask.get(element_index).ok_or(Error::IndexOutOfBounds {
                index: element_index as u64,
                length: mask.len() as u64,
            })? as usize;

            if index < left_count {
                return load_vector_element(machine, *left_offset, *left_element, index);
            }

            let right_index = index - left_count;
            if right_index >= right_count {
                return Err(Error::IndexOutOfBounds {
                    index: index as u64,
                    length: (left_count + right_count) as u64,
                });
            }

            load_vector_element(machine, *right_offset, *right_element, right_index)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute vector.select.
pub(crate) fn execute_vector_select(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let VectorSelect {
        dest_offset,
        mask_offset,
        then_offset,
        else_offset,
        dest_element,
        mask_element,
        then_element,
        else_element,
        element_count,
    } = machine.side::<VectorSelect>(instruction);

    // write the selected elements directly
    if let Err(error) = store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |machine, element_index| {
            let mask = load_vector_element(machine, *mask_offset, *mask_element, element_index)?;
            let then_value =
                load_vector_element(machine, *then_offset, *then_element, element_index)?;
            let else_value =
                load_vector_element(machine, *else_offset, *else_element, element_index)?;
            let select = mask.as_bool();

            Ok(if select { then_value } else { else_value })
        },
    ) {
        return Transfer::Error(error);
    }
    Transfer::Continue
}

macro_rules! vector_reduce_executor {
    ($function:ident, $operation:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
            execute_vector_reduce(machine, instruction, $operation)
        }
    };
}

/// Execute vector reduction.
fn execute_vector_reduce(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
) -> Transfer {
    let VectorReduce {
        dest_offset,
        vector_offset,
        vector_element,
        element_layout,
        element_count,
    } = machine.side::<VectorReduce>(instruction);

    let element_count = *element_count as usize;
    if element_count == 0 {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let mut result = match load_vector_element(machine, *vector_offset, *vector_element, 0) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    for element_index in 1..element_count {
        let value =
            match load_vector_element(machine, *vector_offset, *vector_element, element_index) {
                Ok(value) => value,
                Err(error) => return Transfer::Error(error),
            };
        match operation(*element_layout, result, value) {
            Ok(value) => result = value,
            Err(error) => return Transfer::Error(error),
        }
    }

    // store result
    machine.set_word_at(*dest_offset, result);

    // continue to next instruction
    Transfer::Continue
}

vector_reduce_executor!(
    execute_vector_reduce_add,
    reduce_add,
    "Execute vector add reduction."
);
vector_reduce_executor!(
    execute_vector_reduce_multiply,
    reduce_multiply,
    "Execute vector multiply reduction."
);
vector_reduce_executor!(
    execute_vector_reduce_min,
    reduce_min,
    "Execute vector minimum reduction."
);
vector_reduce_executor!(
    execute_vector_reduce_max,
    reduce_max,
    "Execute vector maximum reduction."
);
vector_reduce_executor!(
    execute_vector_reduce_and,
    reduce_and,
    "Execute vector bitwise and reduction."
);
vector_reduce_executor!(
    execute_vector_reduce_or,
    reduce_or,
    "Execute vector bitwise or reduction."
);
vector_reduce_executor!(
    execute_vector_reduce_xor,
    reduce_xor,
    "Execute vector bitwise xor reduction."
);

/// Execute vector.convert.
fn execute_vector_convert(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    convert: fn(Word, ScalarLayout, ScalarLayout) -> Result<Word, Error>,
) -> Transfer {
    // decode fixed fields
    let VectorConvert {
        dest_offset,
        vector_offset,
        dest_element,
        source_element,
        dest_layout,
        source_layout,
        element_count,
    } = machine.side::<VectorConvert>(instruction);

    // convert the elements one by one
    if let Err(error) = store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |machine, element_index| {
            let value =
                load_vector_element(machine, *vector_offset, *source_element, element_index)?;

            convert(value, *source_layout, *dest_layout)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute exact vector conversion.
pub(crate) fn execute_vector_convert_exact(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_vector_convert(machine, instruction, convert_scalar_exact)
}

/// Execute vector conversion with round to nearest even.
pub(crate) fn execute_vector_convert_round_ties_even(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_vector_convert(machine, instruction, convert_scalar_round_ties_even)
}

/// Execute vector conversion with round toward zero.
pub(crate) fn execute_vector_convert_round_toward_zero(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_vector_convert(machine, instruction, convert_scalar_round_toward_zero)
}

/// Execute vector conversion with round toward negative infinity.
pub(crate) fn execute_vector_convert_round_floor(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_vector_convert(machine, instruction, convert_scalar_round_floor)
}

/// Execute vector conversion with round toward positive infinity.
pub(crate) fn execute_vector_convert_round_ceil(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_vector_convert(machine, instruction, convert_scalar_round_ceil)
}

/// Execute vector conversion with saturation.
pub(crate) fn execute_vector_convert_saturate(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_vector_convert(machine, instruction, convert_scalar_saturate)
}
