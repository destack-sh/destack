use std::collections::HashSet;
use std::ptr;

use super::access;
use super::index::word_to_u64;
use super::scalar::{
    convert_scalar_exact, convert_scalar_round_ceil, convert_scalar_round_floor,
    convert_scalar_round_ties_even, convert_scalar_round_toward_zero, convert_scalar_saturate,
    reduce_add, reduce_and, reduce_max, reduce_min, reduce_multiply, reduce_or, reduce_xor,
};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    ElementAccess, Instruction, PointeeAccess, PointerClass, ScalarLayout, TensorBinary,
    TensorBroadcast, TensorConcat, TensorConvert, TensorConvolution, TensorCopy, TensorDot,
    TensorExtract, TensorFill, TensorGather, TensorLayout, TensorLayoutId, TensorLoad, TensorPad,
    TensorReduce, TensorReshape, TensorScatter, TensorSelect, TensorSlice, TensorStore,
    TensorTranspose, TensorUnary, TensorView, Transfer, U32RangeId,
};

macro_rules! tensor_binary_executor {
    ($function:ident, $operation:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
            execute_tensor_binary(machine, instruction, super::scalar::$operation)
        }
    };
}

macro_rules! tensor_unary_executor {
    ($function:ident, $operation:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
            execute_tensor_unary(machine, instruction, super::scalar::$operation)
        }
    };
}

macro_rules! tensor_reduce_executor {
    ($function:ident, $operation:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
            execute_tensor_reduce(machine, instruction, $operation)
        }
    };
}

macro_rules! tensor_scatter_executor {
    ($function:ident, $operation:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
            execute_tensor_scatter(machine, instruction, $operation)
        }
    };
}

macro_rules! tensor_convert_executor {
    ($function:ident, $convert:ident, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
            execute_tensor_convert(machine, instruction, $convert)
        }
    };
}

/// Borrow one compiled tensor layout.
#[inline(always)]
fn tensor_layout<'iso>(machine: &Machine<'_, 'iso>, id: TensorLayoutId) -> &'iso TensorLayout {
    let table = machine.side_table_ptr();

    unsafe { (*table).tensor_layout(id) }
}

/// Return a frame value address by byte offset.
#[inline(always)]
fn frame_value(machine: &Machine<'_, '_>, offset: u32) -> Word {
    Word::frame_pointer(machine.frame_pointer_at(offset))
}

/// Return one frame offset range.
#[inline(always)]
fn frame_offsets<'a>(machine: &'a Machine<'_, '_>, range: U32RangeId) -> &'a [u32] {
    let table = machine.side_table_ptr();

    unsafe { (*table).u32_range(range) }
}

/// Read one tensor index vector from word frame offsets.
fn tensor_index_values(machine: &Machine<'_, '_>, offsets: &[u32]) -> Result<Vec<u64>, Error> {
    let mut values = Vec::with_capacity(offsets.len());
    for offset in offsets {
        let value = machine.get_word_at(*offset);
        values.push(word_to_u64(value)?);
    }

    Ok(values)
}

/// Clear one frame byte range and return its address.
fn clear_tensor_result(
    machine: &mut Machine<'_, '_>,
    offset: u32,
    layout: &TensorLayout,
) -> Result<Word, Error> {
    let pointer = machine.frame_pointer_at(offset);

    // destination tensor bytes have no meaningful old contents
    unsafe {
        ptr::write_bytes(pointer.address() as *mut u8, 0, layout.byte_len);
    }

    Ok(Word::frame_pointer(pointer))
}

/// Store one tensor result into frame bytes.
fn store_tensor_elements<F>(
    machine: &mut Machine<'_, '_>,
    dest_offset: u32,
    layout: &TensorLayout,
    mut element_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut Machine<'_, '_>, usize) -> Result<Word, Error>,
{
    let result = clear_tensor_result(machine, dest_offset, layout)?;

    // store each active tensor element
    for element_index in 0..layout.element_span_len {
        let value = element_value(machine, element_index)?;
        store_frame_tensor_element_at(machine, result, layout, element_index, value)?;
    }

    Ok(())
}

/// Load one tensor-view element through one concrete pointer class.
#[inline(always)]
fn load_view_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: ElementAccess,
) -> Result<Word, Error> {
    let access = PointeeAccess::from(element);

    match element.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => {
            access::load_heap_scalar_by_layout(machine, pointer, access)
        }
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            access::load_shared_heap_scalar_by_layout(machine, pointer, access)
        }
        PointerClass::Raw => access::load_raw_scalar_by_layout(machine, pointer, access),
        PointerClass::SharedRaw => {
            access::load_shared_raw_scalar_by_layout(machine, pointer, access)
        }
        PointerClass::Stack => {
            access::load_stack_scalar_by_layout(machine, pointer.as_stack_pointer(), access)
        }
        PointerClass::Frame => {
            access::load_frame_scalar_by_layout(machine, pointer.as_frame_pointer(), access)
        }
        PointerClass::Static => {
            access::load_static_scalar_by_layout(machine, pointer.as_static_pointer(), access)
        }
        PointerClass::Unknown => Err(access::invalid_pointer_type(pointer)),
    }
}

/// Store one tensor-view element through one concrete pointer class.
#[inline(always)]
fn store_view_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: ElementAccess,
    value: Word,
) -> Result<(), Error> {
    let access = PointeeAccess::from(element);

    match element.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => {
            access::store_heap_scalar_by_layout(machine, pointer, access, value)
        }
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            access::store_shared_heap_scalar_by_layout(machine, pointer, access, value)
        }
        PointerClass::Raw => access::store_raw_scalar_by_layout(machine, pointer, access, value),
        PointerClass::SharedRaw => {
            access::store_shared_raw_scalar_by_layout(machine, pointer, access, value)
        }
        PointerClass::Stack => {
            access::store_stack_scalar_by_layout(machine, pointer.as_stack_pointer(), access, value)
        }
        PointerClass::Frame => {
            access::store_frame_scalar_by_layout(machine, pointer.as_frame_pointer(), access, value)
        }
        PointerClass::Static => access::store_static_scalar_by_layout(
            machine,
            pointer.as_static_pointer(),
            access,
            value,
        ),
        PointerClass::Unknown => Err(access::invalid_pointer_type(pointer)),
    }
}

/// Return one frame tensor element pointer.
fn frame_tensor_element_pointer(
    tensor: Word,
    element: ElementAccess,
    element_index: usize,
    element_count: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(tensor, element, element_index, element_count)?;
    let pointer = tensor.as_frame_pointer().add_bytes(byte_offset);

    Ok(Word::frame_pointer(pointer))
}

/// Return one tensor element byte offset.
fn element_byte_offset(
    value: Word,
    element: ElementAccess,
    offset: usize,
    length: usize,
) -> Result<usize, Error> {
    // validate bounds
    if offset >= length {
        return Err(Error::IndexOutOfBounds {
            index: offset as u64,
            length: length as u64,
        });
    }

    offset
        .checked_mul(element.byte_stride)
        .ok_or(Error::InvalidPointerType {
            actual: format!("{value:?}"),
        })
}

/// Store one tensor element into one frame tensor value.
fn store_frame_tensor_element_at(
    machine: &mut Machine<'_, '_>,
    tensor: Word,
    layout: &TensorLayout,
    element_index: usize,
    value: Word,
) -> Result<(), Error> {
    let pointer = frame_tensor_element_pointer(
        tensor,
        layout.element,
        element_index,
        layout.element_span_len,
    )?;

    access::store_frame_scalar_by_layout(
        machine,
        pointer.as_frame_pointer(),
        PointeeAccess::from(layout.element),
        value,
    )
}

/// Store one tensor result from multi-dimensional output indices.
fn store_tensor_indexed_elements<F>(
    machine: &mut Machine<'_, '_>,
    dest_offset: u32,
    layout: &TensorLayout,
    mut index_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut Machine<'_, '_>, &[u64]) -> Result<Word, Error>,
{
    let result = clear_tensor_result(machine, dest_offset, layout)?;
    let mut error = None;

    // fill active tensor indices directly into the destination place
    for_each_index(&layout.shape, |output_index| {
        if error.is_some() {
            return;
        }

        let destination_offset =
            match tensor_linear_index(output_index, &layout.shape, &layout.strides) {
                Ok(offset) => offset,
                Err(current_error) => {
                    error = Some(current_error);
                    return;
                }
            };
        let value = match index_value(machine, output_index) {
            Ok(value) => value,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };

        if let Err(current_error) =
            store_frame_tensor_element_at(machine, result, layout, destination_offset, value)
        {
            error = Some(current_error);
        }
    });

    if let Some(error) = error {
        return Err(error);
    }

    Ok(())
}

/// Compute the linear index for a multi-dimensional index.
pub(crate) fn tensor_linear_index(
    indices: &[u64],
    shape: &[u64],
    strides: &[u64],
) -> Result<usize, Error> {
    // validate index length
    if indices.len() != shape.len() || shape.len() != strides.len() {
        return Err(Error::TypeMismatch {
            expected: "tensor index rank".to_string(),
            actual: format!(
                "indices={}, shape={}, strides={}",
                indices.len(),
                shape.len(),
                strides.len()
            ),
        });
    }

    // compute linear index
    let mut offset = 0u64;
    for ((index, dim), stride) in indices.iter().zip(shape.iter()).zip(strides.iter()) {
        // reject out of bounds indices before accumulating the stride
        if *index >= *dim {
            return Err(Error::IndexOutOfBounds {
                index: *index,
                length: *dim,
            });
        }

        // accumulate the linear offset in element units
        let product = index
            .checked_mul(*stride)
            .ok_or_else(|| Error::TypeMismatch {
                expected: "tensor index".to_string(),
                actual: format!("{index} * {stride}"),
            })?;
        offset = offset
            .checked_add(product)
            .ok_or_else(|| Error::TypeMismatch {
                expected: "tensor index".to_string(),
                actual: format!("{offset} + {product}"),
            })?;
    }

    // convert the final offset into host indexing
    usize::try_from(offset).map_err(|_| Error::TypeMismatch {
        expected: "tensor index".to_string(),
        actual: offset.to_string(),
    })
}

/// Load one tensor element from one frame tensor value.
pub(crate) fn load_frame_tensor_element_at(
    machine: &mut Machine<'_, '_>,
    tensor: Word,
    layout: &TensorLayout,
    element_index: usize,
) -> Result<Word, Error> {
    let pointer = frame_tensor_element_pointer(
        tensor,
        layout.element,
        element_index,
        layout.element_span_len,
    )?;

    access::load_frame_scalar_by_layout(
        machine,
        pointer.as_frame_pointer(),
        PointeeAccess::from(layout.element),
    )
}

/// Execute a tensor binary operation.
fn execute_tensor_binary<F>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: F,
) -> Transfer
where
    F: Fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
{
    // decode side record
    let TensorBinary {
        dest_offset,
        left_offset,
        right_offset,
        left_layout,
        right_layout,
        dest_layout,
        element_layout,
    } = machine.side::<TensorBinary>(instruction);

    // resolve tensor addresses
    let left_value = frame_value(machine, *left_offset);
    let right_value = frame_value(machine, *right_offset);

    // resolve compiled tensor descriptors
    let left_layout = tensor_layout(machine, *left_layout);
    let right_layout = tensor_layout(machine, *right_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // apply the scalar operation to each tensor element
    if let Err(error) = store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            let left_index =
                tensor_linear_index(output_index, &left_layout.shape, &left_layout.strides)?;
            let right_index =
                tensor_linear_index(output_index, &right_layout.shape, &right_layout.strides)?;
            let left = load_frame_tensor_element_at(machine, left_value, left_layout, left_index)?;
            let right =
                load_frame_tensor_element_at(machine, right_value, right_layout, right_index)?;

            operation(*element_layout, left, right)
        },
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute a tensor unary operation.
fn execute_tensor_unary<F>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: F,
) -> Transfer
where
    F: Fn(ScalarLayout, Word) -> Result<Word, Error>,
{
    // decode fixed fields
    let TensorUnary {
        dest_offset,
        argument_offset,
        argument_layout,
        element_layout,
        dest_layout,
    } = machine.side::<TensorUnary>(instruction);

    // resolve tensor address
    let argument_value = frame_value(machine, *argument_offset);

    // resolve compiled tensor descriptors
    let argument_layout = tensor_layout(machine, *argument_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // apply the scalar operation to each tensor element
    if let Err(error) = store_tensor_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, element_index| {
            let value = load_frame_tensor_element_at(
                machine,
                argument_value,
                argument_layout,
                element_index,
            )?;

            operation(*element_layout, value)
        },
    ) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

tensor_binary_executor!(
    execute_tensor_and_bool,
    and_bool,
    "Execute tensor boolean and."
);
tensor_binary_executor!(
    execute_tensor_or_bool,
    or_bool,
    "Execute tensor boolean or."
);
tensor_binary_executor!(
    execute_tensor_xor_bool,
    xor_bool,
    "Execute tensor boolean xor."
);
tensor_binary_executor!(
    execute_tensor_add_int,
    add_int,
    "Execute tensor integer add."
);
tensor_binary_executor!(
    execute_tensor_sub_int,
    sub_int,
    "Execute tensor integer subtract."
);
tensor_binary_executor!(
    execute_tensor_mul_int,
    mul_int,
    "Execute tensor integer multiply."
);
tensor_binary_executor!(
    execute_tensor_div_int,
    div_int,
    "Execute tensor signed integer divide."
);
tensor_binary_executor!(
    execute_tensor_div_uint,
    div_uint,
    "Execute tensor unsigned integer divide."
);
tensor_binary_executor!(
    execute_tensor_rem_int,
    rem_int,
    "Execute tensor signed integer remainder."
);
tensor_binary_executor!(
    execute_tensor_rem_uint,
    rem_uint,
    "Execute tensor unsigned integer remainder."
);
tensor_binary_executor!(
    execute_tensor_and_int,
    and_int,
    "Execute tensor integer and."
);
tensor_binary_executor!(execute_tensor_or_int, or_int, "Execute tensor integer or.");
tensor_binary_executor!(
    execute_tensor_xor_int,
    xor_int,
    "Execute tensor integer xor."
);
tensor_binary_executor!(
    execute_tensor_shl_int,
    shl_int,
    "Execute tensor integer shift left."
);
tensor_binary_executor!(
    execute_tensor_shr_int,
    shr_int,
    "Execute tensor signed integer shift right."
);
tensor_binary_executor!(
    execute_tensor_shr_uint,
    shr_uint,
    "Execute tensor unsigned integer shift right."
);
tensor_binary_executor!(
    execute_tensor_add_f32,
    add_f32,
    "Execute tensor float32 add."
);
tensor_binary_executor!(
    execute_tensor_add_f64,
    add_f64,
    "Execute tensor float64 add."
);
tensor_binary_executor!(
    execute_tensor_sub_f32,
    sub_f32,
    "Execute tensor float32 subtract."
);
tensor_binary_executor!(
    execute_tensor_sub_f64,
    sub_f64,
    "Execute tensor float64 subtract."
);
tensor_binary_executor!(
    execute_tensor_mul_f32,
    mul_f32,
    "Execute tensor float32 multiply."
);
tensor_binary_executor!(
    execute_tensor_mul_f64,
    mul_f64,
    "Execute tensor float64 multiply."
);
tensor_binary_executor!(
    execute_tensor_div_f32,
    div_f32,
    "Execute tensor float32 divide."
);
tensor_binary_executor!(
    execute_tensor_div_f64,
    div_f64,
    "Execute tensor float64 divide."
);
tensor_binary_executor!(
    execute_tensor_eq_int,
    eq_int,
    "Execute tensor integer equality."
);
tensor_binary_executor!(
    execute_tensor_eq_bool,
    eq_bool,
    "Execute tensor boolean equality."
);
tensor_binary_executor!(
    execute_tensor_ne_int,
    ne_int,
    "Execute tensor integer inequality."
);
tensor_binary_executor!(
    execute_tensor_ne_bool,
    ne_bool,
    "Execute tensor boolean inequality."
);
tensor_binary_executor!(
    execute_tensor_lt_int,
    lt_int,
    "Execute tensor signed integer less than."
);
tensor_binary_executor!(
    execute_tensor_lt_uint,
    lt_uint,
    "Execute tensor unsigned integer less than."
);
tensor_binary_executor!(
    execute_tensor_le_int,
    le_int,
    "Execute tensor signed integer less than or equal."
);
tensor_binary_executor!(
    execute_tensor_le_uint,
    le_uint,
    "Execute tensor unsigned integer less than or equal."
);
tensor_binary_executor!(
    execute_tensor_gt_int,
    gt_int,
    "Execute tensor signed integer greater than."
);
tensor_binary_executor!(
    execute_tensor_gt_uint,
    gt_uint,
    "Execute tensor unsigned integer greater than."
);
tensor_binary_executor!(
    execute_tensor_ge_int,
    ge_int,
    "Execute tensor signed integer greater than or equal."
);
tensor_binary_executor!(
    execute_tensor_ge_uint,
    ge_uint,
    "Execute tensor unsigned integer greater than or equal."
);
tensor_binary_executor!(
    execute_tensor_eq_f32,
    eq_f32,
    "Execute tensor float32 equality."
);
tensor_binary_executor!(
    execute_tensor_eq_f64,
    eq_f64,
    "Execute tensor float64 equality."
);
tensor_binary_executor!(
    execute_tensor_ne_f32,
    ne_f32,
    "Execute tensor float32 inequality."
);
tensor_binary_executor!(
    execute_tensor_ne_f64,
    ne_f64,
    "Execute tensor float64 inequality."
);
tensor_binary_executor!(
    execute_tensor_lt_f32,
    lt_f32,
    "Execute tensor float32 less than."
);
tensor_binary_executor!(
    execute_tensor_lt_f64,
    lt_f64,
    "Execute tensor float64 less than."
);
tensor_binary_executor!(
    execute_tensor_le_f32,
    le_f32,
    "Execute tensor float32 less than or equal."
);
tensor_binary_executor!(
    execute_tensor_le_f64,
    le_f64,
    "Execute tensor float64 less than or equal."
);
tensor_binary_executor!(
    execute_tensor_gt_f32,
    gt_f32,
    "Execute tensor float32 greater than."
);
tensor_binary_executor!(
    execute_tensor_gt_f64,
    gt_f64,
    "Execute tensor float64 greater than."
);
tensor_binary_executor!(
    execute_tensor_ge_f32,
    ge_f32,
    "Execute tensor float32 greater than or equal."
);
tensor_binary_executor!(
    execute_tensor_ge_f64,
    ge_f64,
    "Execute tensor float64 greater than or equal."
);
tensor_unary_executor!(
    execute_tensor_neg_int,
    neg_int,
    "Execute tensor integer negation."
);
tensor_unary_executor!(
    execute_tensor_not_int,
    not_int,
    "Execute tensor integer inversion."
);
tensor_unary_executor!(
    execute_tensor_neg_f32,
    neg_f32,
    "Execute tensor float32 negation."
);
tensor_unary_executor!(
    execute_tensor_neg_f64,
    neg_f64,
    "Execute tensor float64 negation."
);
tensor_unary_executor!(
    execute_tensor_not_bool,
    not_bool,
    "Execute tensor boolean inversion."
);

/// Execute tensor.splat.
pub(crate) fn execute_tensor_splat(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest_offset = instruction.a;
    let value = instruction.b;
    let layout = TensorLayoutId(instruction.c);

    // resolve compiled tensor descriptor
    let layout = tensor_layout(machine, layout);
    let value = machine.get_word_at(value);

    // store the same value into each active index
    if let Err(error) =
        store_tensor_indexed_elements(machine, dest_offset, layout, |_machine, _index| Ok(value))
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.extract.
pub(crate) fn execute_tensor_extract(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorExtract {
        dest_offset,
        tensor_offset,
        indices,
        tensor_layout: tensor_layout_id,
    } = machine.side::<TensorExtract>(instruction);

    // resolve compiled tensor descriptor
    let layout = tensor_layout(machine, *tensor_layout_id);

    // resolve indices
    let index_values = frame_offsets(machine, *indices);
    let index = match tensor_index_values(machine, index_values) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let element_index = match tensor_linear_index(&index, &layout.shape, &layout.strides) {
        Ok(element_index) => element_index,
        Err(error) => return Transfer::Error(error),
    };

    // load the tensor value
    let tensor_value = frame_value(machine, *tensor_offset);
    let value = match load_frame_tensor_element_at(machine, tensor_value, layout, element_index) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word_at(*dest_offset, value);

    // continue to next instruction
    Transfer::Continue
}

/// Iterate over all indices in a tensor shape.
pub(crate) fn for_each_index<F: FnMut(&[u64])>(shape: &[u64], mut f: F) {
    // scalar or empty shapes
    if shape.is_empty() {
        f(&[]);
        return;
    }
    if shape.contains(&0) {
        return;
    }

    // initialize index vector
    let mut index = vec![0u64; shape.len()];
    loop {
        // visit the current tensor index
        f(&index);

        // increment the odometer
        let mut dim = shape.len();
        while dim > 0 {
            dim -= 1;
            index[dim] += 1;
            if index[dim] < shape[dim] {
                break;
            }
            index[dim] = 0;
            if dim == 0 {
                return;
            }
        }
    }
}

/// Offset a tensor view pointer by one element index.
pub(crate) fn offset_view_pointer(
    value: Word,
    element: ElementAccess,
    offset: usize,
    length: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(value, element, offset, length)?;

    // offset the pointer according to its pointer kind
    match element.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => {
            let reference = value.as_heap_reference();
            let reference = reference.add_bytes(byte_offset);

            Ok(Word::heap_reference(reference))
        }
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            let reference = value.as_shared_heap_reference();
            let reference = reference.add_bytes(byte_offset);

            Ok(Word::shared_heap_reference(reference))
        }
        PointerClass::Raw => {
            let pointer = value.as_raw_pointer();
            let pointer = pointer.add_bytes(byte_offset);

            Ok(Word::raw_pointer(pointer))
        }
        PointerClass::SharedRaw => {
            let pointer = value.as_shared_raw_pointer();
            let pointer = pointer.add_bytes(byte_offset);

            Ok(Word::shared_raw_pointer(pointer))
        }
        PointerClass::Stack => {
            let pointer = value.as_stack_pointer();
            let pointer = pointer.add_bytes(byte_offset);

            Ok(Word::stack_pointer(pointer))
        }
        PointerClass::Frame => {
            let pointer = value.as_frame_pointer();
            let pointer = pointer.add_bytes(byte_offset);

            Ok(Word::frame_pointer(pointer))
        }
        PointerClass::Static => {
            let pointer = value.as_static_pointer();
            let pointer = pointer.add_bytes(byte_offset);

            Ok(Word::static_pointer(pointer))
        }
        PointerClass::Unknown => Err(Error::InvalidPointerType {
            actual: format!("{value:?}"),
        }),
    }
}

/// Execute tensor.load.
pub(crate) fn execute_tensor_load(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorLoad {
        dest_offset,
        view_offset,
        indices,
        view_layout,
        element,
    } = machine.side::<TensorLoad>(instruction);

    // resolve compiled tensor descriptor
    let layout = tensor_layout(machine, *view_layout);

    // resolve indices
    let index_values = frame_offsets(machine, *indices);
    let index = match tensor_index_values(machine, index_values) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let offset = match tensor_linear_index(&index, &layout.shape, &layout.strides) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };

    // offset the view pointer
    let view_value = machine.get_word_at(*view_offset);
    let element = machine.element_access(*element);
    let pointer = match offset_view_pointer(view_value, element, offset, layout.element_span_len) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };

    // load element
    let value = match load_view_tensor_element(machine, pointer, element) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    machine.set_word_at(*dest_offset, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.store.
pub(crate) fn execute_tensor_store(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorStore {
        view_offset,
        indices,
        value_offset,
        view_layout,
        element,
    } = machine.side::<TensorStore>(instruction);

    // resolve compiled tensor descriptor
    let layout = tensor_layout(machine, *view_layout);

    // resolve indices
    let index_values = frame_offsets(machine, *indices);
    let index = match tensor_index_values(machine, index_values) {
        Ok(index) => index,
        Err(error) => return Transfer::Error(error),
    };
    let offset = match tensor_linear_index(&index, &layout.shape, &layout.strides) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };

    // offset the view pointer
    let view_value = machine.get_word_at(*view_offset);
    let element = machine.element_access(*element);
    let pointer = match offset_view_pointer(view_value, element, offset, layout.element_span_len) {
        Ok(pointer) => pointer,
        Err(error) => return Transfer::Error(error),
    };

    // store element
    let value = machine.get_word_at(*value_offset);
    if let Err(error) = store_view_tensor_element(machine, pointer, element, value) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.fill.
pub(crate) fn execute_tensor_fill(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorFill {
        view_offset,
        value_offset,
        view_layout,
        element,
    } = machine.side::<TensorFill>(instruction);

    // resolve compiled tensor descriptor
    let layout = tensor_layout(machine, *view_layout);
    let fill_value = machine.get_word_at(*value_offset);
    let base_pointer = machine.get_word_at(*view_offset);

    // fill each element
    let element = machine.element_access(*element);
    for offset in 0..layout.element_span_len {
        let pointer =
            match offset_view_pointer(base_pointer, element, offset, layout.element_span_len) {
                Ok(pointer) => pointer,
                Err(error) => return Transfer::Error(error),
            };
        if let Err(error) = store_view_tensor_element(machine, pointer, element, fill_value) {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.copy.
pub(crate) fn execute_tensor_copy(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorCopy {
        target_offset,
        source_offset,
        target_layout,
        source_layout,
        target_element,
        source_element,
    } = machine.side::<TensorCopy>(instruction);

    // resolve compiled tensor descriptors
    let target_layout = tensor_layout(machine, *target_layout);
    let source_layout = tensor_layout(machine, *source_layout);

    // validate element counts
    if target_layout.element_span_len != source_layout.element_span_len {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching tensor sizes".to_string(),
            actual: format!(
                "{} vs {}",
                target_layout.element_span_len, source_layout.element_span_len
            ),
        });
    }

    // copy elements
    let target_pointer = machine.get_word_at(*target_offset);
    let source_pointer = machine.get_word_at(*source_offset);
    let source_element = machine.element_access(*source_element);
    let target_element = machine.element_access(*target_element);
    for offset in 0..target_layout.element_span_len {
        let source_pointer = match offset_view_pointer(
            source_pointer,
            source_element,
            offset,
            source_layout.element_span_len,
        ) {
            Ok(pointer) => pointer,
            Err(error) => return Transfer::Error(error),
        };
        let target_pointer = match offset_view_pointer(
            target_pointer,
            target_element,
            offset,
            target_layout.element_span_len,
        ) {
            Ok(pointer) => pointer,
            Err(error) => return Transfer::Error(error),
        };
        let value = match load_view_tensor_element(machine, source_pointer, source_element) {
            Ok(value) => value,
            Err(error) => return Transfer::Error(error),
        };
        if let Err(error) =
            store_view_tensor_element(machine, target_pointer, target_element, value)
        {
            return Transfer::Error(error);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.reshape.
pub(crate) fn execute_tensor_reshape(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorReshape {
        dest_offset,
        tensor_offset,
        shape,
        source_layout,
        dest_layout,
    } = machine.side::<TensorReshape>(instruction);

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // compute expected element count from shape values when provided
    let shape_values = frame_offsets(machine, *shape);
    let mut shape_len = 1u64;
    for offset in shape_values {
        let value = machine.get_word_at(*offset);
        let size = match word_to_u64(value) {
            Ok(size) => size,
            Err(error) => return Transfer::Error(error),
        };
        shape_len = match shape_len.checked_mul(size) {
            Some(shape_len) => shape_len,
            None => {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "reshape element count".to_string(),
                    actual: format!("{shape_len} * {size}"),
                });
            }
        };
    }
    if shape_values.is_empty() {
        shape_len = dest_layout.element_span_len as u64;
    }

    // validate element counts
    if dest_layout.element_span_len as u64 != shape_len {
        return Transfer::Error(Error::TypeMismatch {
            expected: "reshape element count".to_string(),
            actual: format!("{} vs {}", dest_layout.element_span_len, shape_len),
        });
    }

    // copy the source elements into the reshaped result
    if let Err(error) = store_tensor_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, element_index| {
            load_frame_tensor_element_at(machine, tensor_value, source_layout, element_index)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.broadcast.
pub(crate) fn execute_tensor_broadcast(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorBroadcast {
        dest_offset,
        tensor_offset,
        dimensions,
        source_layout,
        dest_layout,
    } = machine.side::<TensorBroadcast>(instruction);
    let table = machine.side_table_ptr();
    let dimensions = unsafe { (*table).u32_range(*dimensions) };

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // validate dimension mapping
    if dimensions.len() != source_layout.shape.len() {
        return Transfer::Error(Error::TypeMismatch {
            expected: "broadcast dimension mapping".to_string(),
            actual: format!("{} vs {}", dimensions.len(), source_layout.shape.len()),
        });
    }

    // resolve source elements
    let tensor_value = frame_value(machine, *tensor_offset);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // store result
    if let Err(error) = store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            for (i, dim) in dimensions.iter().enumerate() {
                let output_value = output_index[*dim as usize];
                let source_dim = source_layout.shape[i];
                input_index[i] = if source_dim == 1 { 0 } else { output_value };
            }

            let source_offset =
                tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)?;

            load_frame_tensor_element_at(machine, tensor_value, source_layout, source_offset)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.transpose.
pub(crate) fn execute_tensor_transpose(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorTranspose {
        dest_offset,
        tensor_offset,
        permutation,
        source_layout,
        dest_layout,
    } = machine.side::<TensorTranspose>(instruction);
    let table = machine.side_table_ptr();
    let permutation = unsafe { (*table).u32_range(*permutation) };

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // validate permutation
    if permutation.len() != source_layout.shape.len() {
        return Transfer::Error(Error::TypeMismatch {
            expected: "transpose permutation".to_string(),
            actual: format!("{} vs {}", permutation.len(), source_layout.shape.len()),
        });
    }

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // store result
    if let Err(error) = store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            for (out_dim, in_dim) in permutation.iter().enumerate() {
                input_index[*in_dim as usize] = output_index[out_dim];
            }

            let source_offset =
                tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)?;

            load_frame_tensor_element_at(machine, tensor_value, source_layout, source_offset)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.slice.
pub(crate) fn execute_tensor_slice(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorSlice {
        dest_offset,
        tensor_offset,
        arguments,
        offsets_count,
        sizes_count,
        strides_count,
        source_layout,
        dest_layout,
    } = machine.side::<TensorSlice>(instruction);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve arguments
    let args = frame_offsets(machine, *arguments);
    let (offset_values, rest) = args.split_at((*offsets_count).into());
    let (size_values, stride_values) = rest.split_at((*sizes_count).into());
    if stride_values.len() != usize::from(*strides_count) {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let mut offsets = Vec::with_capacity(offset_values.len());
    let mut sizes = Vec::with_capacity(size_values.len());
    let mut strides = Vec::with_capacity(stride_values.len());
    for offset in offset_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => offsets.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for offset in size_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => sizes.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for offset in stride_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => strides.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }

    // require full-rank slice arguments
    let rank = source_layout.shape.len();
    if offsets.len() != rank || sizes.len() != rank || strides.len() != rank {
        return Transfer::Error(Error::InvalidInstruction);
    }

    // require runtime sizes to match the destination tensor
    if sizes.len() != dest_layout.shape.len() {
        return Transfer::Error(Error::InvalidInstruction);
    }
    for (expected, actual) in dest_layout.shape.iter().zip(sizes.iter()) {
        if expected != actual {
            return Transfer::Error(Error::TypeMismatch {
                expected: "tensor slice size".to_string(),
                actual: format!("{actual} vs {expected}"),
            });
        }
    }

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // store result
    if let Err(error) = store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            for i in 0..input_index.len() {
                let offset = offsets[i];
                let stride = strides[i];
                input_index[i] = offset + output_index[i] * stride;
            }

            let source_offset =
                tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)?;

            load_frame_tensor_element_at(machine, tensor_value, source_layout, source_offset)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.pad.
pub(crate) fn execute_tensor_pad(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorPad {
        dest_offset,
        tensor_offset,
        arguments,
        low_count,
        high_count,
        interior_count,
        value_offset,
        source_layout,
        dest_layout,
    } = machine.side::<TensorPad>(instruction);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve arguments
    let args = frame_offsets(machine, *arguments);
    let (low_values, rest) = args.split_at((*low_count).into());
    let (high_values, interior_values) = rest.split_at((*high_count).into());
    if interior_values.len() != usize::from(*interior_count) {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let mut low = Vec::with_capacity(low_values.len());
    let mut high = Vec::with_capacity(high_values.len());
    let mut interior = Vec::with_capacity(interior_values.len());
    for offset in low_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => low.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for offset in high_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => high.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for offset in interior_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => interior.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }

    // require full-rank padding arguments
    let rank = source_layout.shape.len();
    if low.len() != rank || high.len() != rank || interior.len() != rank {
        return Transfer::Error(Error::InvalidInstruction);
    }

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);
    let pad_value = machine.get_word_at(*value_offset);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // store result
    if let Err(error) = store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            for i in 0..input_index.len() {
                let low_pad = low[i];
                let interior_pad = interior[i];
                let mut input_position = output_index[i] as i64 - low_pad as i64;
                if input_position < 0 {
                    return Ok(pad_value);
                }

                let stride = interior_pad + 1;
                if !(input_position as u64).is_multiple_of(stride) {
                    return Ok(pad_value);
                }

                input_position /= stride as i64;
                if input_position < 0 || input_position as u64 >= source_layout.shape[i] {
                    return Ok(pad_value);
                }

                input_index[i] = input_position as u64;
            }

            let source_offset =
                tensor_linear_index(&input_index, &source_layout.shape, &source_layout.strides)?;

            load_frame_tensor_element_at(machine, tensor_value, source_layout, source_offset)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.concat.
pub(crate) fn execute_tensor_concat(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorConcat {
        dest_offset,
        tensors,
        tensor_layouts,
        axis,
        dest_layout,
    } = machine.side::<TensorConcat>(instruction);
    let table = machine.side_table_ptr();
    let tensor_layouts = unsafe { (*table).u32_range(*tensor_layouts) };

    // resolve compiled tensor descriptor
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve input tensors
    let tensor_offsets = frame_offsets(machine, *tensors).to_vec();
    // validate input metadata
    if tensor_offsets.len() != tensor_layouts.len() {
        return Transfer::Error(Error::InvalidInstruction);
    }
    let mut inputs = Vec::with_capacity(tensor_offsets.len());
    let mut axis_sizes = Vec::with_capacity(tensor_offsets.len());
    for (offset, layout) in tensor_offsets.iter().zip(tensor_layouts.iter()) {
        let value = frame_value(machine, *offset);
        let layout = tensor_layout(machine, TensorLayoutId(*layout));
        let axis_index = *axis as usize;
        if axis_index >= layout.shape.len() {
            return Transfer::Error(Error::InvalidInstruction);
        }
        axis_sizes.push(layout.shape[axis_index]);
        inputs.push((value, layout));
    }

    // compute axis offsets
    let mut axis_offsets = Vec::with_capacity(axis_sizes.len());
    let mut running = 0u64;
    for size in &axis_sizes {
        axis_offsets.push(running);
        running = match running.checked_add(*size) {
            Some(running) => running,
            None => {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "concat axis size".to_string(),
                    actual: format!("{running} + {size}"),
                });
            }
        };
    }

    let axis_index = *axis as usize;
    let mut input_index = vec![0u64; dest_layout.shape.len()];

    // store result
    if let Err(error) = store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            let axis_value = output_index[axis_index];
            let mut selected = None;
            for (i, offset) in axis_offsets.iter().enumerate() {
                let size = axis_sizes[i];
                if axis_value >= *offset && axis_value < *offset + size {
                    selected = Some((i, axis_value - *offset));
                    break;
                }
            }
            let Some((input_idx, local_axis)) = selected else {
                return Err(Error::InvalidInstruction);
            };

            input_index.clone_from_slice(output_index);
            input_index[axis_index] = local_axis;
            let (tensor_value, layout) = &inputs[input_idx];

            let source_offset = tensor_linear_index(&input_index, &layout.shape, &layout.strides)?;

            load_frame_tensor_element_at(machine, *tensor_value, layout, source_offset)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor reduction.
fn execute_tensor_reduce(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
) -> Transfer {
    // decode side records
    let TensorReduce {
        dest_offset,
        tensor_offset,
        initial_offset,
        axes,
        source_layout,
        dest_layout,
    } = machine.side::<TensorReduce>(instruction);
    let table = machine.side_table_ptr();
    let axes = unsafe { (*table).u32_range(*axes) };

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);
    let element_layout = source_layout.element_layout;

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);
    let init_value = machine.get_word_at(*initial_offset);
    let reduce_axes: HashSet<u32> = axes.iter().copied().collect();
    let mut reduced_shape = Vec::with_capacity(axes.len());
    for axis in axes {
        let Some(size) = source_layout.shape.get(*axis as usize) else {
            return Transfer::Error(Error::InvalidInstruction);
        };
        reduced_shape.push(*size);
    }
    let mut source_index = vec![0u64; source_layout.shape.len()];

    // store result
    if let Err(error) = store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            let mut output_cursor = 0usize;
            for (dim, element) in source_index.iter_mut().enumerate() {
                if reduce_axes.contains(&(dim as u32)) {
                    *element = 0;
                    continue;
                }

                *element = output_index
                    .get(output_cursor)
                    .copied()
                    .ok_or(Error::InvalidInstruction)?;
                output_cursor += 1;
            }

            let mut accum = init_value;
            let mut reduce_error = None;

            // accumulate the reduced axes for this destination index
            for_each_index(&reduced_shape, |reduced_index| {
                if reduce_error.is_some() {
                    return;
                }

                for (reduce_position, axis) in axes.iter().enumerate() {
                    source_index[*axis as usize] = reduced_index[reduce_position];
                }

                let source_offset = match tensor_linear_index(
                    &source_index,
                    &source_layout.shape,
                    &source_layout.strides,
                ) {
                    Ok(offset) => offset,
                    Err(error) => {
                        reduce_error = Some(error);
                        return;
                    }
                };
                let source_value = match load_frame_tensor_element_at(
                    machine,
                    tensor_value,
                    source_layout,
                    source_offset,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        reduce_error = Some(error);
                        return;
                    }
                };

                accum = match operation(element_layout, accum, source_value) {
                    Ok(value) => value,
                    Err(error) => {
                        reduce_error = Some(error);
                        return;
                    }
                };
            });

            if let Some(error) = reduce_error {
                return Err(error);
            }

            Ok(accum)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

tensor_reduce_executor!(
    execute_tensor_reduce_add,
    reduce_add,
    "Execute tensor add reduction."
);
tensor_reduce_executor!(
    execute_tensor_reduce_multiply,
    reduce_multiply,
    "Execute tensor multiply reduction."
);
tensor_reduce_executor!(
    execute_tensor_reduce_min,
    reduce_min,
    "Execute tensor minimum reduction."
);
tensor_reduce_executor!(
    execute_tensor_reduce_max,
    reduce_max,
    "Execute tensor maximum reduction."
);
tensor_reduce_executor!(
    execute_tensor_reduce_and,
    reduce_and,
    "Execute tensor bitwise and reduction."
);
tensor_reduce_executor!(
    execute_tensor_reduce_or,
    reduce_or,
    "Execute tensor bitwise or reduction."
);
tensor_reduce_executor!(
    execute_tensor_reduce_xor,
    reduce_xor,
    "Execute tensor bitwise xor reduction."
);

/// Execute tensor.dot.
pub(crate) fn execute_tensor_dot(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorDot {
        dest_offset,
        left_offset,
        right_offset,
        dimensions,
        left_layout,
        right_layout,
        dest_layout,
        element_layout,
    } = machine.side::<TensorDot>(instruction);
    let table = machine.side_table_ptr();
    let dimensions = unsafe { (*table).tensor_dot(*dimensions) };

    // resolve compiled tensor descriptors
    let left_layout = tensor_layout(machine, *left_layout);
    let right_layout = tensor_layout(machine, *right_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve source tensors
    let left_value = frame_value(machine, *left_offset);
    let right_value = frame_value(machine, *right_offset);

    // validate addressable element spans
    if left_layout.element_span_len != right_layout.element_span_len
        || left_layout.element_span_len != dest_layout.element_span_len
    {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching tensor element spans".to_string(),
            actual: format!(
                "{} vs {} vs {}",
                left_layout.element_span_len,
                right_layout.element_span_len,
                dest_layout.element_span_len
            ),
        });
    }

    // compute axis sets
    let lhs_batch = &dimensions.lhs_batch;
    let rhs_batch = &dimensions.rhs_batch;
    let lhs_contract = &dimensions.lhs_contracting;
    let rhs_contract = &dimensions.rhs_contracting;
    if lhs_batch.len() != rhs_batch.len() || lhs_contract.len() != rhs_contract.len() {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let lhs_rank = left_layout.shape.len();
    let rhs_rank = right_layout.shape.len();
    let mut lhs_free = Vec::new();
    let mut rhs_free = Vec::new();
    for i in 0..lhs_rank {
        if !lhs_batch.contains(&(i as u32)) && !lhs_contract.contains(&(i as u32)) {
            lhs_free.push(i as u32);
        }
    }
    for i in 0..rhs_rank {
        if !rhs_batch.contains(&(i as u32)) && !rhs_contract.contains(&(i as u32)) {
            rhs_free.push(i as u32);
        }
    }

    let contract_shape: Vec<u64> = lhs_contract
        .iter()
        .map(|dim| left_layout.shape[*dim as usize])
        .collect();
    let mut lhs_index = vec![0u64; lhs_rank];
    let mut rhs_index = vec![0u64; rhs_rank];

    // store result
    if let Err(error) =
        store_tensor_indexed_elements(machine, *dest_offset, dest_layout, |machine, out_index| {
            for (i, dim) in lhs_batch.iter().enumerate() {
                let value = out_index[i];
                lhs_index[*dim as usize] = value;
                rhs_index[rhs_batch[i] as usize] = value;
            }
            for (i, dim) in lhs_free.iter().enumerate() {
                lhs_index[*dim as usize] = out_index[lhs_batch.len() + i];
            }
            for (i, dim) in rhs_free.iter().enumerate() {
                let offset = lhs_batch.len() + lhs_free.len() + i;
                rhs_index[*dim as usize] = out_index[offset];
            }

            let mut accum = None;
            let mut contract_error = None;

            // iterate the contracting dimensions for this destination element
            for_each_index(&contract_shape, |contract_index| {
                if contract_error.is_some() {
                    return;
                }

                for (i, dim) in lhs_contract.iter().enumerate() {
                    lhs_index[*dim as usize] = contract_index[i];
                }
                for (i, dim) in rhs_contract.iter().enumerate() {
                    rhs_index[*dim as usize] = contract_index[i];
                }

                let lhs_offset =
                    match tensor_linear_index(&lhs_index, &left_layout.shape, &left_layout.strides)
                    {
                        Ok(offset) => offset,
                        Err(error) => {
                            contract_error = Some(error);
                            return;
                        }
                    };
                let rhs_offset = match tensor_linear_index(
                    &rhs_index,
                    &right_layout.shape,
                    &right_layout.strides,
                ) {
                    Ok(offset) => offset,
                    Err(error) => {
                        contract_error = Some(error);
                        return;
                    }
                };
                let left_element = match load_frame_tensor_element_at(
                    machine,
                    left_value,
                    left_layout,
                    lhs_offset,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        contract_error = Some(error);
                        return;
                    }
                };
                let right_element = match load_frame_tensor_element_at(
                    machine,
                    right_value,
                    right_layout,
                    rhs_offset,
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        contract_error = Some(error);
                        return;
                    }
                };
                let product = match reduce_multiply(*element_layout, left_element, right_element) {
                    Ok(value) => value,
                    Err(error) => {
                        contract_error = Some(error);
                        return;
                    }
                };

                accum = match accum {
                    None => Some(product),
                    Some(current) => match reduce_add(*element_layout, current, product) {
                        Ok(value) => Some(value),
                        Err(error) => {
                            contract_error = Some(error);
                            return;
                        }
                    },
                };
            });

            if let Some(error) = contract_error {
                return Err(error);
            }

            accum.ok_or(Error::InvalidInstruction)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.convolution.
pub(crate) fn execute_tensor_convolution(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorConvolution {
        dest_offset,
        input_offset,
        kernel_offset,
        dimensions,
        window,
        feature_group_count,
        batch_group_count,
        input_layout,
        kernel_layout,
        dest_layout,
        element_layout,
    } = machine.side::<TensorConvolution>(instruction);
    let table = machine.side_table_ptr();
    let dimensions = unsafe { (*table).tensor_convolution(*dimensions) };
    let window = unsafe { (*table).tensor_window(*window) };

    // resolve compiled tensor descriptors
    let input_layout = tensor_layout(machine, *input_layout);
    let kernel_layout = tensor_layout(machine, *kernel_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve source tensors
    let input_value = frame_value(machine, *input_offset);
    let kernel_value = frame_value(machine, *kernel_offset);

    // derive dimension mappings
    let spatial_rank = dimensions.input_spatial.len();
    let output_spatial = &dimensions.output_spatial;
    let kernel_spatial = &dimensions.kernel_spatial;
    if output_spatial.len() != spatial_rank || kernel_spatial.len() != spatial_rank {
        return Transfer::Error(Error::InvalidInstruction);
    }

    // validate window shapes
    if window.strides.len() != spatial_rank
        || window.padding_low.len() != spatial_rank
        || window.padding_high.len() != spatial_rank
        || window.lhs_dilation.len() != spatial_rank
        || window.rhs_dilation.len() != spatial_rank
        || window.window_reversal.len() != spatial_rank
    {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let output_batch_dim = dimensions.output_batch as usize;
    let output_feature_dim = dimensions.output_feature as usize;
    let input_batch_dim = dimensions.input_batch as usize;
    let input_feature_dim = dimensions.input_feature as usize;
    let kernel_input_feature_dim = dimensions.kernel_input_feature as usize;
    let kernel_output_feature_dim = dimensions.kernel_output_feature as usize;

    // validate dimension indices
    if input_batch_dim >= input_layout.shape.len()
        || input_feature_dim >= input_layout.shape.len()
        || output_batch_dim >= dest_layout.shape.len()
        || output_feature_dim >= dest_layout.shape.len()
        || kernel_input_feature_dim >= kernel_layout.shape.len()
        || kernel_output_feature_dim >= kernel_layout.shape.len()
    {
        return Transfer::Error(Error::InvalidInstruction);
    }

    for dim in dimensions.input_spatial.iter() {
        if *dim as usize >= input_layout.shape.len() {
            return Transfer::Error(Error::InvalidInstruction);
        }
    }
    for dim in output_spatial.iter() {
        if *dim as usize >= dest_layout.shape.len() {
            return Transfer::Error(Error::InvalidInstruction);
        }
    }

    let input_batch_size = input_layout.shape[input_batch_dim];
    let input_feature_size = input_layout.shape[input_feature_dim];
    let output_batch_size = dest_layout.shape[output_batch_dim];
    let output_feature_size = dest_layout.shape[output_feature_dim];

    let mut kernel_spatial_shape = Vec::with_capacity(kernel_spatial.len());
    for dim in kernel_spatial {
        let Some(size) = kernel_layout.shape.get(*dim as usize) else {
            return Transfer::Error(Error::InvalidInstruction);
        };
        kernel_spatial_shape.push(*size);
    }

    // validate group counts
    if *feature_group_count == 0 || *batch_group_count == 0 {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let out_features_per_group = output_feature_size / (*feature_group_count as u64);
    let in_features_per_group = input_feature_size / (*feature_group_count as u64);
    let out_batches_per_group = output_batch_size / (*batch_group_count as u64);
    let in_batches_per_group = input_batch_size / (*batch_group_count as u64);
    let mut input_index = vec![0u64; input_layout.shape.len()];
    let mut kernel_index = vec![0u64; kernel_layout.shape.len()];

    // store result
    if let Err(error) =
        store_tensor_indexed_elements(machine, *dest_offset, dest_layout, |machine, out_index| {
            let out_batch = out_index[output_batch_dim];
            let out_feature = out_index[output_feature_dim];

            let batch_group = out_batch / out_batches_per_group;
            let feature_group = out_feature / out_features_per_group;

            let input_batch =
                batch_group * in_batches_per_group + (out_batch % out_batches_per_group);
            let input_feature_base = feature_group * in_features_per_group;
            let kernel_out_feature = out_feature % out_features_per_group;

            input_index[input_batch_dim] = input_batch;
            kernel_index[kernel_output_feature_dim] = kernel_out_feature;

            let mut accum = None;
            let mut convolution_error = None;

            for in_feature in 0..in_features_per_group {
                input_index[input_feature_dim] = input_feature_base + in_feature;
                kernel_index[kernel_input_feature_dim] = in_feature;

                for_each_index(&kernel_spatial_shape, |kernel_spatial_index| {
                    if convolution_error.is_some() {
                        return;
                    }

                    let mut is_valid = true;
                    for (i, &dim) in dimensions.input_spatial.iter().enumerate() {
                        let out_spatial_dim = output_spatial[i] as usize;
                        let kernel_dim = kernel_spatial[i] as usize;
                        let stride = window.strides[i];
                        let padding_low = window.padding_low[i];
                        let lhs_dilation = window.lhs_dilation[i];
                        let rhs_dilation = window.rhs_dilation[i];
                        let kernel_size = kernel_layout.shape[kernel_dim];
                        let mut kernel_pos = kernel_spatial_index[i];
                        if window.window_reversal[i] {
                            kernel_pos = kernel_size - 1 - kernel_pos;
                        }

                        let out_pos = out_index[out_spatial_dim];
                        let mut input_pos = match out_pos.checked_mul(stride).and_then(|position| {
                            kernel_pos
                                .checked_mul(rhs_dilation)
                                .and_then(|kernel| position.checked_add(kernel))
                        }) {
                            Some(position) => position,
                            None => {
                                convolution_error = Some(Error::TypeMismatch {
                                    expected: "convolution input position".to_string(),
                                    actual: format!("{out_pos}, {kernel_pos}"),
                                });
                                return;
                            }
                        };
                        if input_pos < padding_low {
                            is_valid = false;
                            break;
                        }
                        input_pos -= padding_low;
                        if lhs_dilation > 1 {
                            if input_pos % lhs_dilation != 0 {
                                is_valid = false;
                                break;
                            }
                            input_pos /= lhs_dilation;
                        }
                        if input_pos >= input_layout.shape[dim as usize] {
                            is_valid = false;
                            break;
                        }

                        input_index[dim as usize] = input_pos;
                        kernel_index[kernel_dim] = kernel_pos;
                    }

                    if !is_valid {
                        return;
                    }

                    let input_offset = match tensor_linear_index(
                        &input_index,
                        &input_layout.shape,
                        &input_layout.strides,
                    ) {
                        Ok(offset) => offset,
                        Err(error) => {
                            convolution_error = Some(error);
                            return;
                        }
                    };
                    let kernel_offset = match tensor_linear_index(
                        &kernel_index,
                        &kernel_layout.shape,
                        &kernel_layout.strides,
                    ) {
                        Ok(offset) => offset,
                        Err(error) => {
                            convolution_error = Some(error);
                            return;
                        }
                    };
                    let input_element = match load_frame_tensor_element_at(
                        machine,
                        input_value,
                        input_layout,
                        input_offset,
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            convolution_error = Some(error);
                            return;
                        }
                    };
                    let kernel_element = match load_frame_tensor_element_at(
                        machine,
                        kernel_value,
                        kernel_layout,
                        kernel_offset,
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            convolution_error = Some(error);
                            return;
                        }
                    };
                    let product =
                        match reduce_multiply(*element_layout, input_element, kernel_element) {
                            Ok(value) => value,
                            Err(error) => {
                                convolution_error = Some(error);
                                return;
                            }
                        };

                    accum = match accum {
                        None => Some(product),
                        Some(current) => match reduce_add(*element_layout, current, product) {
                            Ok(value) => Some(value),
                            Err(error) => {
                                convolution_error = Some(error);
                                return;
                            }
                        },
                    };
                });
            }

            if let Some(error) = convolution_error {
                return Err(error);
            }

            accum.ok_or(Error::InvalidInstruction)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.gather.
pub(crate) fn execute_tensor_gather(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorGather {
        dest_offset,
        source_offset,
        indices_offset,
        dimensions,
        slice_sizes,
        source_layout,
        indices_layout,
        dest_layout,
    } = machine.side::<TensorGather>(instruction);
    let table = machine.side_table_ptr();
    let dimensions = unsafe { (*table).tensor_gather(*dimensions) };
    let slice_sizes = unsafe { (*table).u32_range(*slice_sizes) };

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let indices_layout = tensor_layout(machine, *indices_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);
    // resolve tensors
    let source_value = frame_value(machine, *source_offset);
    let indices_value = frame_value(machine, *indices_offset);
    let offset_dims: HashSet<u32> = dimensions.offset_dims.iter().copied().collect();
    let collapsed_dims: HashSet<u32> = dimensions.collapsed_slice_dims.iter().copied().collect();

    // store result
    if let Err(error) =
        store_tensor_indexed_elements(machine, *dest_offset, dest_layout, |machine, out_index| {
            let mut index_coords = Vec::new();
            for (dim, value) in out_index.iter().enumerate() {
                if !offset_dims.contains(&(dim as u32)) {
                    index_coords.push(*value);
                }
            }

            let mut index_vec = vec![0u64; dimensions.start_index_map.len()];
            let mut indices_index = vec![0u64; indices_layout.shape.len()];
            let mut coord_iter = index_coords.iter();
            for (dim, element) in indices_index.iter_mut().enumerate() {
                if dim as u32 == dimensions.index_vector_dim {
                    continue;
                }

                let Some(coord) = coord_iter.next() else {
                    return Err(Error::InvalidInstruction);
                };
                *element = *coord;
            }

            let index_offset = tensor_linear_index(
                &indices_index,
                &indices_layout.shape,
                &indices_layout.strides,
            )?;
            let index_base = index_offset;
            for (i, &_map_dim) in dimensions.start_index_map.iter().enumerate() {
                let element_index = index_base + i;
                let value = load_frame_tensor_element_at(
                    machine,
                    indices_value,
                    indices_layout,
                    element_index,
                )?;
                index_vec[i] = word_to_u64(value)?;
                indices_index[dimensions.index_vector_dim as usize] = index_vec[i];
            }

            let mut source_index = vec![0u64; source_layout.shape.len()];
            for (i, &map_dim) in dimensions.start_index_map.iter().enumerate() {
                source_index[map_dim as usize] = index_vec[i];
            }

            let mut offset_iter = out_index.iter();
            for (dim, element) in source_index.iter_mut().enumerate() {
                if collapsed_dims.contains(&(dim as u32)) {
                    continue;
                }

                let offset = if offset_dims.contains(&(dim as u32)) {
                    let Some(offset) = offset_iter.next() else {
                        return Err(Error::InvalidInstruction);
                    };
                    *offset
                } else {
                    0
                };
                let Some(size) = slice_sizes.get(dim) else {
                    return Err(Error::InvalidInstruction);
                };
                let size = *size as u64;
                let start = *element;
                *element = start + offset.min(size.saturating_sub(1));
            }

            let source_offset =
                tensor_linear_index(&source_index, &source_layout.shape, &source_layout.strides)?;

            load_frame_tensor_element_at(machine, source_value, source_layout, source_offset)
        })
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor scatter.
fn execute_tensor_scatter(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    combine: impl Fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
) -> Transfer {
    // decode side records
    let TensorScatter {
        dest_offset,
        source_offset,
        indices_offset,
        updates_offset,
        dimensions,
        source_layout,
        indices_layout,
        updates_layout,
        dest_layout,
        element_layout,
    } = machine.side::<TensorScatter>(instruction);
    let table = machine.side_table_ptr();
    let dimensions = unsafe { (*table).tensor_scatter(*dimensions) };

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let indices_layout = tensor_layout(machine, *indices_layout);
    let updates_layout = tensor_layout(machine, *updates_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve tensors
    let source_value = frame_value(machine, *source_offset);
    let indices_value = frame_value(machine, *indices_offset);
    let updates_value = frame_value(machine, *updates_offset);

    if let Err(error) = store_tensor_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, element_index| {
            load_frame_tensor_element_at(machine, source_value, source_layout, element_index)
        },
    ) {
        return Transfer::Error(error);
    }
    let result = frame_value(machine, *dest_offset);
    let update_window_dims: HashSet<u32> = dimensions.update_window_dims.iter().copied().collect();
    let inserted_window_dims: HashSet<u32> =
        dimensions.inserted_window_dims.iter().copied().collect();
    let mut scatter_error = None;

    // scatter updates
    for_each_index(&updates_layout.shape, |update_index| {
        if scatter_error.is_some() {
            return;
        }

        let mut index_coords = Vec::new();
        for (dim, value) in update_index.iter().enumerate() {
            if !update_window_dims.contains(&(dim as u32)) {
                index_coords.push(*value);
            }
        }

        let mut indices_index = vec![0u64; indices_layout.shape.len()];
        let mut coord_iter = index_coords.iter();
        for (dim, element) in indices_index.iter_mut().enumerate() {
            if dim as u32 == dimensions.index_vector_dim {
                continue;
            }

            let Some(coord) = coord_iter.next() else {
                scatter_error = Some(Error::InvalidInstruction);
                return;
            };
            *element = *coord;
        }

        let index_offset = tensor_linear_index(
            &indices_index,
            &indices_layout.shape,
            &indices_layout.strides,
        );
        let Ok(index_offset) = index_offset else {
            scatter_error = Some(Error::InvalidInstruction);
            return;
        };

        let mut scatter_indices = Vec::with_capacity(dimensions.scatter_dims_to_operand_dims.len());
        for i in 0..dimensions.scatter_dims_to_operand_dims.len() {
            let element_index = index_offset + i;
            let Ok(value) =
                load_frame_tensor_element_at(machine, indices_value, indices_layout, element_index)
            else {
                scatter_error = Some(Error::InvalidInstruction);
                return;
            };
            if let Ok(coord) = word_to_u64(value) {
                scatter_indices.push(coord);
            } else {
                scatter_error = Some(Error::InvalidInstruction);
                return;
            }
        }

        let mut source_index = vec![0u64; source_layout.shape.len()];
        for (i, &dim) in dimensions.scatter_dims_to_operand_dims.iter().enumerate() {
            source_index[dim as usize] = scatter_indices[i];
        }

        let mut update_iter = update_index.iter();
        for (dim, element) in source_index.iter_mut().enumerate() {
            if inserted_window_dims.contains(&(dim as u32)) {
                continue;
            }
            if update_window_dims.contains(&(dim as u32)) {
                let Some(update) = update_iter.next() else {
                    scatter_error = Some(Error::InvalidInstruction);
                    return;
                };
                *element = *update;
            }
        }

        let destination_offset =
            tensor_linear_index(&source_index, &dest_layout.shape, &dest_layout.strides);
        let Ok(destination_offset) = destination_offset else {
            scatter_error = Some(Error::InvalidInstruction);
            return;
        };
        let update_offset =
            tensor_linear_index(update_index, &updates_layout.shape, &updates_layout.strides);
        let Ok(update_offset) = update_offset else {
            scatter_error = Some(Error::InvalidInstruction);
            return;
        };
        let Ok(update_value) =
            load_frame_tensor_element_at(machine, updates_value, updates_layout, update_offset)
        else {
            scatter_error = Some(Error::InvalidInstruction);
            return;
        };
        let current_value =
            match load_frame_tensor_element_at(machine, result, dest_layout, destination_offset) {
                Ok(value) => value,
                Err(error) => {
                    scatter_error = Some(error);
                    return;
                }
            };
        let new_value = match combine(*element_layout, current_value, update_value) {
            Ok(value) => value,
            Err(error) => {
                scatter_error = Some(error);
                return;
            }
        };

        if let Err(error) = store_frame_tensor_element_at(
            machine,
            result,
            dest_layout,
            destination_offset,
            new_value,
        ) {
            scatter_error = Some(error);
        }
    });

    if let Some(error) = scatter_error {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor replacement scatter.
pub(crate) fn execute_tensor_scatter_replace(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_tensor_scatter(machine, instruction, |_layout, _current, update| Ok(update))
}

tensor_scatter_executor!(
    execute_tensor_scatter_add,
    reduce_add,
    "Execute tensor add scatter."
);
tensor_scatter_executor!(
    execute_tensor_scatter_multiply,
    reduce_multiply,
    "Execute tensor multiply scatter."
);
tensor_scatter_executor!(
    execute_tensor_scatter_min,
    reduce_min,
    "Execute tensor minimum scatter."
);
tensor_scatter_executor!(
    execute_tensor_scatter_max,
    reduce_max,
    "Execute tensor maximum scatter."
);
tensor_scatter_executor!(
    execute_tensor_scatter_and,
    reduce_and,
    "Execute tensor bitwise and scatter."
);
tensor_scatter_executor!(
    execute_tensor_scatter_or,
    reduce_or,
    "Execute tensor bitwise or scatter."
);
tensor_scatter_executor!(
    execute_tensor_scatter_xor,
    reduce_xor,
    "Execute tensor bitwise xor scatter."
);

/// Execute tensor conversion.
fn execute_tensor_convert(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    convert: fn(Word, ScalarLayout, ScalarLayout) -> Result<Word, Error>,
) -> Transfer {
    // decode side records
    let TensorConvert {
        dest_offset,
        tensor_offset,
        source_layout,
        dest_layout,
        source_scalar,
        dest_scalar,
    } = machine.side::<TensorConvert>(instruction);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    let tensor_value = frame_value(machine, *tensor_offset);
    if source_layout.element_span_len != dest_layout.element_span_len {
        return Transfer::Error(Error::TypeMismatch {
            expected: "matching tensor element spans".to_string(),
            actual: format!(
                "{} vs {}",
                source_layout.element_span_len, dest_layout.element_span_len
            ),
        });
    }

    // store result
    if let Err(error) = store_tensor_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, element_index| {
            let source =
                load_frame_tensor_element_at(machine, tensor_value, source_layout, element_index)?;

            convert(source, *source_scalar, *dest_scalar)
        },
    ) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

tensor_convert_executor!(
    execute_tensor_convert_exact,
    convert_scalar_exact,
    "Execute exact tensor conversion."
);
tensor_convert_executor!(
    execute_tensor_convert_round_ties_even,
    convert_scalar_round_ties_even,
    "Execute tensor conversion with round to nearest even."
);
tensor_convert_executor!(
    execute_tensor_convert_round_toward_zero,
    convert_scalar_round_toward_zero,
    "Execute tensor conversion with round toward zero."
);
tensor_convert_executor!(
    execute_tensor_convert_round_floor,
    convert_scalar_round_floor,
    "Execute tensor conversion with floor rounding."
);
tensor_convert_executor!(
    execute_tensor_convert_round_ceil,
    convert_scalar_round_ceil,
    "Execute tensor conversion with ceiling rounding."
);
tensor_convert_executor!(
    execute_tensor_convert_saturate,
    convert_scalar_saturate,
    "Execute saturating tensor conversion."
);

/// Execute tensor.select.
pub(crate) fn execute_tensor_select(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let TensorSelect {
        dest_offset,
        mask_offset,
        then_offset,
        else_offset,
        mask_layout,
        then_layout,
        else_layout,
        dest_layout,
    } = machine.side::<TensorSelect>(instruction);

    // resolve compiled tensor descriptors
    let mask_layout = tensor_layout(machine, *mask_layout);
    let then_layout = tensor_layout(machine, *then_layout);
    let else_layout = tensor_layout(machine, *else_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    let mask_value = frame_value(machine, *mask_offset);
    let then_value = frame_value(machine, *then_offset);
    let else_value = frame_value(machine, *else_offset);
    if let Err(error) = store_tensor_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, element_index| {
            let mask_value =
                load_frame_tensor_element_at(machine, mask_value, mask_layout, element_index)?;
            let then_element =
                load_frame_tensor_element_at(machine, then_value, then_layout, element_index)?;
            let else_element =
                load_frame_tensor_element_at(machine, else_value, else_layout, element_index)?;

            let select = mask_value.as_bool();
            Ok(if select { then_element } else { else_element })
        },
    ) {
        return Transfer::Error(error);
    }
    Transfer::Continue
}

/// Execute tensor.cast.
pub(crate) fn execute_tensor_cast(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest_offset = instruction.a;
    let tensor_offset = instruction.b;
    let byte_len = instruction.c as u64 | ((instruction.d as u64) << 32);

    // forward the tensor bytes
    machine.copy_frame_bytes(tensor_offset, dest_offset, byte_len as usize);

    // continue to next instruction
    Transfer::Continue
}

/// Execute tensor.view.
pub(crate) fn execute_tensor_view(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let TensorView {
        dest_offset,
        view_offset,
        arguments,
        offsets_count,
        sizes_count,
        strides_count,
        source_layout,
        dest_layout,
        element,
    } = machine.side::<TensorView>(instruction);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve view arguments
    let args = frame_offsets(machine, *arguments);
    let (offset_values, rest) = args.split_at((*offsets_count).into());
    let (size_values, stride_values) = rest.split_at((*sizes_count).into());
    if stride_values.len() != usize::from(*strides_count) {
        return Transfer::Error(Error::InvalidInstruction);
    }

    let mut offsets = Vec::with_capacity(offset_values.len());
    let mut sizes = Vec::with_capacity(size_values.len());
    let mut strides = Vec::with_capacity(stride_values.len());
    for offset in offset_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => offsets.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for offset in size_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => sizes.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }
    for offset in stride_values {
        let value = machine.get_word_at(*offset);
        match word_to_u64(value) {
            Ok(v) => strides.push(v),
            Err(error) => return Transfer::Error(error),
        }
    }

    // validate sizes and strides against the destination layout
    for (expected, actual) in dest_layout.shape.iter().zip(sizes.iter()) {
        if *expected != *actual {
            return Transfer::Error(Error::TypeMismatch {
                expected: "tensor.view size".to_string(),
                actual: format!("{actual} vs {expected}"),
            });
        }
    }
    for (expected, actual) in dest_layout.strides.iter().zip(strides.iter()) {
        if *expected != *actual {
            return Transfer::Error(Error::TypeMismatch {
                expected: "tensor.view stride".to_string(),
                actual: format!("{actual} vs {expected}"),
            });
        }
    }

    // compute offset into the source view
    let offset = match tensor_linear_index(&offsets, &source_layout.shape, &source_layout.strides) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };

    // offset the view pointer
    let view_value = machine.get_word_at(*view_offset);
    let element = machine.element_access(*element);
    let pointer =
        match offset_view_pointer(view_value, element, offset, source_layout.element_span_len) {
            Ok(pointer) => pointer,
            Err(error) => return Transfer::Error(error),
        };

    // set the view result
    machine.set_word_at(*dest_offset, pointer);

    // continue to next instruction
    Transfer::Continue
}
