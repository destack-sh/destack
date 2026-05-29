use std::cmp::Ordering;
use std::collections::HashSet;
use std::ptr;

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::{vaddq_u32, vld1q_u32, vst1q_u32};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m128i, _mm_add_epi32, _mm_loadu_si128, _mm_storeu_si128};

use destack_mir as mir;

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
    ElementBinaryKernel, ElementUnaryKernel, Instruction, Projection, ScalarLayout, TensorAddress,
    TensorBinary, TensorBroadcast, TensorConcat, TensorContiguousBinary, TensorContiguousUnary,
    TensorConvert, TensorConvolution, TensorCopy, TensorDot, TensorExtract, TensorFill,
    TensorGather, TensorIndexReduce, TensorLayout, TensorLayoutId, TensorLoad, TensorPad,
    TensorReduce, TensorReshape, TensorScatter, TensorSelect, TensorSlice, TensorStore,
    TensorTranspose, TensorUnary, TensorView, TensorViewCast, U32RangeId, WordLayout,
    tensor_element_span_len,
};

const POINTER_BYTE_LEN: usize = usize::BITS as usize / 8;

/// Borrow one compiled tensor layout.
#[inline(always)]
fn tensor_layout<'iso>(machine: &Machine<'_, 'iso>, id: TensorLayoutId) -> &'iso TensorLayout {
    machine.tensor_layout(id)
}

/// Return a frame value address by byte offset.
#[inline(always)]
fn frame_value(machine: &Machine<'_, '_>, offset: u32) -> Word {
    Word::frame_pointer(machine.frame_pointer_at(offset))
}

/// Return one frame offset range.
#[inline(always)]
fn frame_offsets<'iso>(machine: &Machine<'_, 'iso>, range: U32RangeId) -> &'iso [u32] {
    machine.u32_range(range)
}

/// Return the byte offset for one tensor view stride slot.
fn tensor_view_stride_offset(view_offset: u32, axis: usize) -> Result<u32, Error> {
    let slot = axis.checked_add(1).ok_or(Error::invalid_instruction())?;
    let byte_offset = slot
        .checked_mul(Word::BYTE_LEN)
        .ok_or(Error::invalid_instruction())?;
    let byte_offset = u32::try_from(byte_offset).map_err(|_| Error::invalid_instruction())?;

    view_offset
        .checked_add(byte_offset)
        .ok_or(Error::invalid_instruction())
}

/// Load one tensor view base pointer.
#[inline(always)]
fn load_tensor_view_pointer(machine: &Machine<'_, '_>, view_offset: u32) -> Word {
    machine.load_word_at(view_offset)
}

/// Load one tensor view runtime stride list.
fn load_tensor_view_strides(
    machine: &Machine<'_, '_>,
    view_offset: u32,
    layout: &TensorLayout,
) -> Result<Vec<u64>, Error> {
    let mut strides = Vec::with_capacity(layout.shape.len());

    for axis in 0..layout.shape.len() {
        let offset = tensor_view_stride_offset(view_offset, axis)?;
        let stride = machine.load_word_at(offset);
        strides.push(word_to_u64(stride)?);
    }

    Ok(strides)
}

/// Store one tensor view descriptor.
fn store_tensor_view_descriptor(
    machine: &mut Machine<'_, '_>,
    view_offset: u32,
    pointer: Word,
    strides: &[u64],
) -> Result<(), Error> {
    machine.store_word_at(view_offset, pointer);

    for (axis, stride) in strides.iter().copied().enumerate() {
        let offset = tensor_view_stride_offset(view_offset, axis)?;
        machine.store_word_at(offset, Word::uint(stride, Word::BIT_LEN));
    }

    Ok(())
}

/// Read one tensor index vector from word frame offsets.
fn tensor_index_values(machine: &Machine<'_, '_>, offsets: &[u32]) -> Result<Vec<u64>, Error> {
    let mut values = Vec::with_capacity(offsets.len());
    for offset in offsets {
        let value = machine.load_word_at(*offset);
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

    // SAFETY: pointer addresses layout.byte_len writable bytes in the current frame
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

/// Load one tensor-view element through local heap memory.
#[inline(always)]
fn load_heap_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
) -> Result<Word, Error> {
    let access = element;

    Ok(access::load_heap_scalar_by_layout(machine, pointer, access))
}

/// Load one tensor-view element through shared heap memory.
#[inline(always)]
fn load_shared_heap_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
) -> Result<Word, Error> {
    let access = element;

    Ok(access::load_shared_heap_scalar_by_layout(
        machine, pointer, access,
    ))
}

/// Load one tensor-view element through raw memory.
#[inline(always)]
fn load_raw_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
) -> Result<Word, Error> {
    let access = element;

    Ok(access::load_raw_scalar_by_layout(machine, pointer, access))
}

/// Load one tensor-view element through stack memory.
#[inline(always)]
fn load_stack_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
) -> Result<Word, Error> {
    let access = element;

    Ok(access::load_stack_scalar_by_layout(
        machine,
        pointer.as_stack_pointer(),
        access,
    ))
}

/// Load one tensor-view element through frame memory.
#[inline(always)]
fn load_frame_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
) -> Result<Word, Error> {
    let access = element;

    Ok(access::load_frame_scalar_by_layout(
        machine,
        pointer.as_frame_pointer(),
        access,
    ))
}

/// Load one tensor-view element through static memory.
#[inline(always)]
fn load_static_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
) -> Result<Word, Error> {
    let access = element;

    Ok(access::load_static_scalar_by_layout(
        machine,
        pointer.as_static_pointer(),
        access,
    ))
}

/// Store one tensor-view element through local heap memory.
#[inline(always)]
fn store_heap_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
    value: Word,
) -> Result<(), Error> {
    let access = element;

    access::store_heap_scalar_by_layout(machine, pointer, access, value);

    Ok(())
}

/// Store one tensor-view element through shared heap memory.
#[inline(always)]
fn store_shared_heap_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
    value: Word,
) -> Result<(), Error> {
    let access = element;

    access::store_shared_heap_scalar_by_layout(machine, pointer, access, value);

    Ok(())
}

/// Store one tensor-view element through raw memory.
#[inline(always)]
fn store_raw_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
    value: Word,
) -> Result<(), Error> {
    let access = element;

    access::store_raw_scalar_by_layout(machine, pointer, access, value);

    Ok(())
}

/// Store one tensor-view element through stack memory.
#[inline(always)]
fn store_stack_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
    value: Word,
) -> Result<(), Error> {
    let access = element;

    access::store_stack_scalar_by_layout(machine, pointer.as_stack_pointer(), access, value);

    Ok(())
}

/// Store one tensor-view element through frame memory.
#[inline(always)]
fn store_frame_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
    value: Word,
) -> Result<(), Error> {
    let access = element;

    access::store_frame_scalar_by_layout(machine, pointer.as_frame_pointer(), access, value);

    Ok(())
}

/// Store one tensor-view element through static memory.
#[inline(always)]
fn store_static_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
    value: Word,
) -> Result<(), Error> {
    let access = element;

    access::store_static_scalar_by_layout(machine, pointer.as_static_pointer(), access, value);

    Ok(())
}

/// Load one tensor-view element through selected memory.
#[inline(always)]
fn load_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
    address: TensorAddress,
) -> Result<Word, Error> {
    match address {
        TensorAddress::Heap => load_heap_tensor_element(machine, pointer, element),
        TensorAddress::SharedHeap => load_shared_heap_tensor_element(machine, pointer, element),
        TensorAddress::Address => load_raw_tensor_element(machine, pointer, element),
        TensorAddress::Stack => load_stack_tensor_element(machine, pointer, element),
        TensorAddress::Frame => load_frame_tensor_element(machine, pointer, element),
        TensorAddress::Static => load_static_tensor_element(machine, pointer, element),
    }
}

/// Store one tensor-view element through selected memory.
#[inline(always)]
fn store_tensor_element(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    element: Projection,
    value: Word,
    address: TensorAddress,
) -> Result<(), Error> {
    match address {
        TensorAddress::Heap => store_heap_tensor_element(machine, pointer, element, value),
        TensorAddress::SharedHeap => {
            store_shared_heap_tensor_element(machine, pointer, element, value)
        }
        TensorAddress::Address => store_raw_tensor_element(machine, pointer, element, value),
        TensorAddress::Stack => store_stack_tensor_element(machine, pointer, element, value),
        TensorAddress::Frame => store_frame_tensor_element(machine, pointer, element, value),
        TensorAddress::Static => store_static_tensor_element(machine, pointer, element, value),
    }
}

/// Return one frame tensor element pointer.
fn frame_tensor_element_pointer(
    tensor: Word,
    element: Projection,
    element_index: usize,
    element_count: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(element, element_index, element_count)?;
    let pointer = tensor.as_frame_pointer().add_bytes(byte_offset);

    Ok(Word::frame_pointer(pointer))
}

/// Return one tensor element byte offset.
fn element_byte_offset(element: Projection, offset: usize, length: usize) -> Result<usize, Error> {
    // validate bounds
    if offset >= length {
        return Err(Error::index_out_of_bounds(offset as u64, length as u64));
    }

    Ok(offset * element.byte_stride)
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
        layout.element,
        value,
    );

    Ok(())
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
        return Err(Error::type_mismatch(
            "tensor index rank",
            format!(
                "indices={}, shape={}, strides={}",
                indices.len(),
                shape.len(),
                strides.len()
            ),
        ));
    }

    // compute linear index
    let mut offset = 0usize;
    for ((index, dim), stride) in indices.iter().zip(shape.iter()).zip(strides.iter()) {
        // reject out of bounds indices before accumulating the stride
        if *index >= *dim {
            return Err(Error::index_out_of_bounds(*index, *dim));
        }

        // accumulate the linear offset in element units
        let product = (*index as usize) * (*stride as usize);
        offset += product;
    }

    Ok(offset)
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

    Ok(access::load_frame_scalar_by_layout(
        machine,
        pointer.as_frame_pointer(),
        layout.element,
    ))
}

/// Execute one tensor binary element loop.
fn execute_tensor_binary_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
) -> Result<(), Error> {
    // decode side record
    let TensorBinary {
        dest_offset,
        left_offset,
        right_offset,
        left_layout,
        right_layout,
        dest_layout,
        kernel: _,
        element_layout,
    } = machine.side::<TensorBinary>(instruction);

    // resolve tensor addresses
    let left_value = frame_value(machine, *left_offset);
    let right_value = frame_value(machine, *right_offset);

    // resolve compiled tensor descriptors
    let left_layout = tensor_layout(machine, *left_layout);
    let right_layout = tensor_layout(machine, *right_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // execute the scalar operation on each tensor element
    store_tensor_indexed_elements(
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
    )?;

    Ok(())
}

/// Execute a tensor binary operation.
pub(crate) fn execute_tensor_binary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let kernel = machine.side::<TensorBinary>(instruction).kernel;
    let operation = tensor_binary_operation(kernel);

    execute_tensor_binary_elements(machine, instruction, operation)
}

/// Return the scalar operation for one tensor binary kernel.
fn tensor_binary_operation(
    kernel: ElementBinaryKernel,
) -> fn(ScalarLayout, Word, Word) -> Result<Word, Error> {
    match kernel {
        ElementBinaryKernel::AndBool => super::scalar::and_bool,
        ElementBinaryKernel::OrBool => super::scalar::or_bool,
        ElementBinaryKernel::XorBool => super::scalar::xor_bool,
        ElementBinaryKernel::EqBool => super::scalar::eq_bool,
        ElementBinaryKernel::NeBool => super::scalar::ne_bool,
        ElementBinaryKernel::AddInt => super::scalar::add_int,
        ElementBinaryKernel::SubInt => super::scalar::sub_int,
        ElementBinaryKernel::MulInt => super::scalar::mul_int,
        ElementBinaryKernel::DivInt => super::scalar::div_int,
        ElementBinaryKernel::DivUint => super::scalar::div_uint,
        ElementBinaryKernel::RemInt => super::scalar::rem_int,
        ElementBinaryKernel::RemUint => super::scalar::rem_uint,
        ElementBinaryKernel::AndInt => super::scalar::and_int,
        ElementBinaryKernel::OrInt => super::scalar::or_int,
        ElementBinaryKernel::XorInt => super::scalar::xor_int,
        ElementBinaryKernel::ShlInt => super::scalar::shl_int,
        ElementBinaryKernel::ShrInt => super::scalar::shr_int,
        ElementBinaryKernel::ShrUint => super::scalar::shr_uint,
        ElementBinaryKernel::EqInt => super::scalar::eq_int,
        ElementBinaryKernel::NeInt => super::scalar::ne_int,
        ElementBinaryKernel::LtInt => super::scalar::lt_int,
        ElementBinaryKernel::LtUint => super::scalar::lt_uint,
        ElementBinaryKernel::LeInt => super::scalar::le_int,
        ElementBinaryKernel::LeUint => super::scalar::le_uint,
        ElementBinaryKernel::GtInt => super::scalar::gt_int,
        ElementBinaryKernel::GtUint => super::scalar::gt_uint,
        ElementBinaryKernel::GeInt => super::scalar::ge_int,
        ElementBinaryKernel::GeUint => super::scalar::ge_uint,
        ElementBinaryKernel::AddF32 => super::scalar::add_f32,
        ElementBinaryKernel::AddF64 => super::scalar::add_f64,
        ElementBinaryKernel::SubF32 => super::scalar::sub_f32,
        ElementBinaryKernel::SubF64 => super::scalar::sub_f64,
        ElementBinaryKernel::MulF32 => super::scalar::mul_f32,
        ElementBinaryKernel::MulF64 => super::scalar::mul_f64,
        ElementBinaryKernel::DivF32 => super::scalar::div_f32,
        ElementBinaryKernel::DivF64 => super::scalar::div_f64,
        ElementBinaryKernel::EqF32 => super::scalar::eq_f32,
        ElementBinaryKernel::EqF64 => super::scalar::eq_f64,
        ElementBinaryKernel::NeF32 => super::scalar::ne_f32,
        ElementBinaryKernel::NeF64 => super::scalar::ne_f64,
        ElementBinaryKernel::LtF32 => super::scalar::lt_f32,
        ElementBinaryKernel::LtF64 => super::scalar::lt_f64,
        ElementBinaryKernel::LeF32 => super::scalar::le_f32,
        ElementBinaryKernel::LeF64 => super::scalar::le_f64,
        ElementBinaryKernel::GtF32 => super::scalar::gt_f32,
        ElementBinaryKernel::GtF64 => super::scalar::gt_f64,
        ElementBinaryKernel::GeF32 => super::scalar::ge_f32,
        ElementBinaryKernel::GeF64 => super::scalar::ge_f64,
    }
}

/// Execute a contiguous tensor binary operation.
pub(crate) fn execute_tensor_contiguous_binary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let TensorContiguousBinary {
        dest_offset,
        left_offset,
        right_offset,
        dest_layout,
        element_layout,
        kernel,
    } = machine.side::<TensorContiguousBinary>(instruction);
    let layout = tensor_layout(machine, *dest_layout);
    execute_contiguous_tensor_binary_elements(
        machine,
        (*dest_offset, *left_offset, *right_offset),
        layout,
        *element_layout,
        *kernel,
    )?;

    Ok(())
}

/// Execute one tensor unary element loop.
fn execute_tensor_unary_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, Word) -> Result<Word, Error>,
) -> Result<(), Error> {
    // decode fixed fields
    let TensorUnary {
        dest_offset,
        argument_offset,
        argument_layout,
        element_layout,
        dest_layout,
        kernel: _,
    } = machine.side::<TensorUnary>(instruction);

    // resolve tensor address
    let argument_value = frame_value(machine, *argument_offset);

    // resolve compiled tensor descriptors
    let argument_layout = tensor_layout(machine, *argument_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // execute the scalar operation on each logical tensor element
    store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            let argument_index = tensor_linear_index(
                output_index,
                &argument_layout.shape,
                &argument_layout.strides,
            )?;
            let value = load_frame_tensor_element_at(
                machine,
                argument_value,
                argument_layout,
                argument_index,
            )?;

            operation(*element_layout, value)
        },
    )?;

    Ok(())
}

/// Execute a tensor unary operation.
pub(crate) fn execute_tensor_unary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let kernel = machine.side::<TensorUnary>(instruction).kernel;
    let operation = tensor_unary_operation(kernel);

    execute_tensor_unary_elements(machine, instruction, operation)
}

/// Return the scalar operation for one tensor unary kernel.
fn tensor_unary_operation(
    kernel: ElementUnaryKernel,
) -> fn(ScalarLayout, Word) -> Result<Word, Error> {
    match kernel {
        ElementUnaryKernel::NotBool => super::scalar::not_bool,
        ElementUnaryKernel::NegInt => super::scalar::neg_int,
        ElementUnaryKernel::NotInt => super::scalar::not_int,
        ElementUnaryKernel::NegF32 => super::scalar::neg_f32,
        ElementUnaryKernel::NegF64 => super::scalar::neg_f64,
    }
}

/// Return the word layout for one tensor scalar layout.
fn tensor_word_layout(layout: ScalarLayout) -> Result<WordLayout, Error> {
    let layout = match layout {
        ScalarLayout::Int {
            width,
            is_signed: true,
        } if width <= u64::BITS as u16 => WordLayout::Int { width: width as u8 },
        ScalarLayout::Int {
            width,
            is_signed: false,
        } if width <= u64::BITS as u16 => WordLayout::Uint { width: width as u8 },
        ScalarLayout::Float { width: 32 } => WordLayout::Float32,
        ScalarLayout::Float { width: 64 } => WordLayout::Float64,
        ScalarLayout::Bool => WordLayout::Bool,
        _ => return Err(Error::invalid_instruction()),
    };

    Ok(layout)
}

/// Return contiguous binary frame addresses.
#[inline(always)]
fn contiguous_binary_addresses(
    machine: &Machine<'_, '_>,
    offsets: (u32, u32, u32),
) -> (*mut u8, *const u8, *const u8) {
    let (dest_offset, left_offset, right_offset) = offsets;
    let dest = machine.frame_pointer_at(dest_offset).address() as *mut u8;
    let left = machine.frame_pointer_at(left_offset).address() as *const u8;
    let right = machine.frame_pointer_at(right_offset).address() as *const u8;

    (dest, left, right)
}

/// Return contiguous unary frame addresses.
#[inline(always)]
fn contiguous_unary_addresses(
    machine: &Machine<'_, '_>,
    offsets: (u32, u32),
) -> (*mut u8, *const u8) {
    let (dest_offset, argument_offset) = offsets;
    let dest = machine.frame_pointer_at(dest_offset).address() as *mut u8;
    let argument = machine.frame_pointer_at(argument_offset).address() as *const u8;

    (dest, argument)
}

/// Read one contiguous element.
#[inline(always)]
fn read_contiguous<T: Copy>(base: *const T, index: usize) -> T {
    // SAFETY: callers pass contiguous tensor storage with index in bounds
    unsafe { base.add(index).read() }
}

/// Write one contiguous element.
#[inline(always)]
fn write_contiguous<T>(base: *mut T, index: usize, value: T) {
    // SAFETY: callers pass contiguous tensor storage with index in bounds
    unsafe {
        base.add(index).write(value);
    }
}

/// Execute one contiguous binary value kernel.
#[inline(always)]
fn execute_contiguous_binary_value<T, F>(
    dest: *mut T,
    left: *const T,
    right: *const T,
    element_count: usize,
    mut operation: F,
) -> Result<(), Error>
where
    T: Copy,
    F: FnMut(T, T) -> Result<T, Error>,
{
    // walk contiguous elements directly
    for index in 0..element_count {
        let left_value = read_contiguous(left, index);
        let right_value = read_contiguous(right, index);
        let value = operation(left_value, right_value)?;
        write_contiguous(dest, index, value);
    }

    Ok(())
}

/// Execute one contiguous binary comparison kernel.
#[inline(always)]
fn execute_contiguous_binary_compare<T, F>(
    dest: *mut u8,
    left: *const T,
    right: *const T,
    element_count: usize,
    mut operation: F,
) -> Result<(), Error>
where
    T: Copy,
    F: FnMut(T, T) -> bool,
{
    // write boolean result bytes directly
    for index in 0..element_count {
        let left_value = read_contiguous(left, index);
        let right_value = read_contiguous(right, index);
        let value = u8::from(operation(left_value, right_value));
        write_contiguous(dest, index, value);
    }

    Ok(())
}

/// Execute one contiguous unary value kernel.
#[inline(always)]
fn execute_contiguous_unary_value<T, F>(
    dest: *mut T,
    argument: *const T,
    element_count: usize,
    mut operation: F,
) -> Result<(), Error>
where
    T: Copy,
    F: FnMut(T) -> T,
{
    // walk contiguous elements directly
    for index in 0..element_count {
        let argument = read_contiguous(argument, index);
        let value = operation(argument);
        write_contiguous(dest, index, value);
    }

    Ok(())
}

/// Execute contiguous 32-bit integer addition.
#[inline(always)]
fn execute_contiguous_add_u32(
    dest: *mut u32,
    left: *const u32,
    right: *const u32,
    element_count: usize,
) -> Result<(), Error> {
    execute_contiguous_add_u32_unchecked(dest, left, right, element_count);

    Ok(())
}

/// Execute contiguous 32-bit integer addition on AArch64.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn execute_contiguous_add_u32_unchecked(
    dest: *mut u32,
    left: *const u32,
    right: *const u32,
    element_count: usize,
) {
    let mut index = 0usize;
    let vector_count = element_count / 4;

    // add full vector chunks
    for _ in 0..vector_count {
        // SAFETY: vector_count only covers full four-lane chunks inside the input buffers
        let left_value = unsafe { vld1q_u32(left.add(index)) };
        // SAFETY: vector_count only covers full four-lane chunks inside the input buffers
        let right_value = unsafe { vld1q_u32(right.add(index)) };
        // SAFETY: NEON integer addition is valid for two loaded u32 vectors
        let result = unsafe { vaddq_u32(left_value, right_value) };
        // SAFETY: vector_count only covers full four-lane chunks inside the output buffer
        unsafe {
            vst1q_u32(dest.add(index), result);
        }
        index += 4;
    }

    // finish scalar tail
    while index < element_count {
        let left_value = read_contiguous(left, index);
        let right_value = read_contiguous(right, index);
        let value = left_value.wrapping_add(right_value);
        write_contiguous(dest, index, value);
        index += 1;
    }
}

/// Execute contiguous 32-bit integer addition on x86-64.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn execute_contiguous_add_u32_unchecked(
    dest: *mut u32,
    left: *const u32,
    right: *const u32,
    element_count: usize,
) {
    let mut index = 0usize;
    let vector_count = element_count / 4;

    // add full vector chunks
    for _ in 0..vector_count {
        // SAFETY: vector_count only covers full four-lane chunks inside the input buffers
        let left_value = unsafe { _mm_loadu_si128(left.add(index).cast::<__m128i>()) };
        // SAFETY: vector_count only covers full four-lane chunks inside the input buffers
        let right_value = unsafe { _mm_loadu_si128(right.add(index).cast::<__m128i>()) };
        // SAFETY: SSE2 integer addition is valid for two loaded u32 vectors
        let result = unsafe { _mm_add_epi32(left_value, right_value) };
        // SAFETY: vector_count only covers full four-lane chunks inside the output buffer
        unsafe {
            _mm_storeu_si128(dest.add(index).cast::<__m128i>(), result);
        }
        index += 4;
    }

    // finish scalar tail
    while index < element_count {
        let left_value = read_contiguous(left, index);
        let right_value = read_contiguous(right, index);
        let value = left_value.wrapping_add(right_value);
        write_contiguous(dest, index, value);
        index += 1;
    }
}

/// Execute contiguous 32-bit integer addition on scalar targets.
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
#[inline(always)]
fn execute_contiguous_add_u32_unchecked(
    dest: *mut u32,
    left: *const u32,
    right: *const u32,
    element_count: usize,
) {
    // walk contiguous elements directly
    for index in 0..element_count {
        let left_value = read_contiguous(left, index);
        let right_value = read_contiguous(right, index);
        let value = left_value.wrapping_add(right_value);
        write_contiguous(dest, index, value);
    }
}

/// Divide one signed 32-bit integer.
#[inline(always)]
fn divide_i32(left: i32, right: i32) -> Result<i32, Error> {
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(left.wrapping_div(right))
}

/// Divide one signed 64-bit integer.
#[inline(always)]
fn divide_i64(left: i64, right: i64) -> Result<i64, Error> {
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(left.wrapping_div(right))
}

/// Divide one unsigned 32-bit integer.
#[inline(always)]
fn divide_u32(left: u32, right: u32) -> Result<u32, Error> {
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(left.wrapping_div(right))
}

/// Divide one unsigned 64-bit integer.
#[inline(always)]
fn divide_u64(left: u64, right: u64) -> Result<u64, Error> {
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(left.wrapping_div(right))
}

/// Remainder one signed 32-bit integer.
#[inline(always)]
fn remainder_i32(left: i32, right: i32) -> Result<i32, Error> {
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(left.wrapping_rem(right))
}

/// Remainder one signed 64-bit integer.
#[inline(always)]
fn remainder_i64(left: i64, right: i64) -> Result<i64, Error> {
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(left.wrapping_rem(right))
}

/// Remainder one unsigned 32-bit integer.
#[inline(always)]
fn remainder_u32(left: u32, right: u32) -> Result<u32, Error> {
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(left.wrapping_rem(right))
}

/// Remainder one unsigned 64-bit integer.
#[inline(always)]
fn remainder_u64(left: u64, right: u64) -> Result<u64, Error> {
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(left.wrapping_rem(right))
}

macro_rules! execute_contiguous_binary {
    ($dest:expr, $left:expr, $right:expr, $count:expr, $ty:ty, $operation:expr) => {
        execute_contiguous_binary_value(
            $dest.cast::<$ty>(),
            $left.cast::<$ty>(),
            $right.cast::<$ty>(),
            $count,
            $operation,
        )
    };
}

macro_rules! execute_contiguous_compare {
    ($dest:expr, $left:expr, $right:expr, $count:expr, $ty:ty, $operation:expr) => {
        execute_contiguous_binary_compare(
            $dest,
            $left.cast::<$ty>(),
            $right.cast::<$ty>(),
            $count,
            $operation,
        )
    };
}

macro_rules! execute_contiguous_unary {
    ($dest:expr, $argument:expr, $count:expr, $ty:ty, $operation:expr) => {
        execute_contiguous_unary_value(
            $dest.cast::<$ty>(),
            $argument.cast::<$ty>(),
            $count,
            $operation,
        )
    };
}

/// Execute one contiguous typed tensor binary operation.
fn execute_contiguous_tensor_binary_typed(
    addresses: (*mut u8, *const u8, *const u8),
    element_count: usize,
    element_layout: ScalarLayout,
    kernel: ElementBinaryKernel,
) -> Option<Result<(), Error>> {
    let (dest, left, right) = addresses;

    // select one typed loop before walking elements
    let result = match (kernel, element_layout) {
        (ElementBinaryKernel::AndBool, ScalarLayout::Bool) => {
            execute_contiguous_binary!(dest, left, right, element_count, u8, |a, b| Ok(a & b))
        }
        (ElementBinaryKernel::OrBool, ScalarLayout::Bool) => {
            execute_contiguous_binary!(dest, left, right, element_count, u8, |a, b| Ok(a | b))
        }
        (ElementBinaryKernel::XorBool, ScalarLayout::Bool) => {
            execute_contiguous_binary!(dest, left, right, element_count, u8, |a, b| Ok(a ^ b))
        }
        (ElementBinaryKernel::EqBool, ScalarLayout::Bool) => {
            execute_contiguous_compare!(dest, left, right, element_count, u8, |a, b| a == b)
        }
        (ElementBinaryKernel::NeBool, ScalarLayout::Bool) => {
            execute_contiguous_compare!(dest, left, right, element_count, u8, |a, b| a != b)
        }
        (ElementBinaryKernel::AddInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_add_u32(
                dest.cast::<u32>(),
                left.cast::<u32>(),
                right.cast::<u32>(),
                element_count,
            )
        }
        (ElementBinaryKernel::SubInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, |a, b| {
                Ok(a.wrapping_sub(b))
            })
        }
        (ElementBinaryKernel::MulInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, |a, b| {
                Ok(a.wrapping_mul(b))
            })
        }
        (
            ElementBinaryKernel::DivInt,
            ScalarLayout::Int {
                width: 32,
                is_signed: true,
            },
        ) => {
            execute_contiguous_binary!(dest, left, right, element_count, i32, divide_i32)
        }
        (ElementBinaryKernel::DivUint, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, divide_u32)
        }
        (
            ElementBinaryKernel::RemInt,
            ScalarLayout::Int {
                width: 32,
                is_signed: true,
            },
        ) => {
            execute_contiguous_binary!(dest, left, right, element_count, i32, remainder_i32)
        }
        (ElementBinaryKernel::RemUint, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, remainder_u32)
        }
        (ElementBinaryKernel::AndInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, |a, b| Ok(a & b))
        }
        (ElementBinaryKernel::OrInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, |a, b| Ok(a | b))
        }
        (ElementBinaryKernel::XorInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, |a, b| Ok(a ^ b))
        }
        (ElementBinaryKernel::ShlInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, |a, b| {
                Ok(a.wrapping_shl(b))
            })
        }
        (
            ElementBinaryKernel::ShrInt,
            ScalarLayout::Int {
                width: 32,
                is_signed: true,
            },
        ) => {
            execute_contiguous_binary!(dest, left, right, element_count, i32, |a, b| {
                Ok(a.wrapping_shr(b as u32))
            })
        }
        (ElementBinaryKernel::ShrUint, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u32, |a, b| {
                Ok(a.wrapping_shr(b))
            })
        }
        (ElementBinaryKernel::EqInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u32, |a, b| a == b)
        }
        (ElementBinaryKernel::NeInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u32, |a, b| a != b)
        }
        (
            ElementBinaryKernel::LtInt,
            ScalarLayout::Int {
                width: 32,
                is_signed: true,
            },
        ) => {
            execute_contiguous_compare!(dest, left, right, element_count, i32, |a, b| a < b)
        }
        (ElementBinaryKernel::LtUint, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u32, |a, b| a < b)
        }
        (
            ElementBinaryKernel::LeInt,
            ScalarLayout::Int {
                width: 32,
                is_signed: true,
            },
        ) => {
            execute_contiguous_compare!(dest, left, right, element_count, i32, |a, b| a <= b)
        }
        (ElementBinaryKernel::LeUint, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u32, |a, b| a <= b)
        }
        (
            ElementBinaryKernel::GtInt,
            ScalarLayout::Int {
                width: 32,
                is_signed: true,
            },
        ) => {
            execute_contiguous_compare!(dest, left, right, element_count, i32, |a, b| a > b)
        }
        (ElementBinaryKernel::GtUint, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u32, |a, b| a > b)
        }
        (
            ElementBinaryKernel::GeInt,
            ScalarLayout::Int {
                width: 32,
                is_signed: true,
            },
        ) => {
            execute_contiguous_compare!(dest, left, right, element_count, i32, |a, b| a >= b)
        }
        (ElementBinaryKernel::GeUint, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u32, |a, b| a >= b)
        }
        (ElementBinaryKernel::AddInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, |a, b| {
                Ok(a.wrapping_add(b))
            })
        }
        (ElementBinaryKernel::SubInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, |a, b| {
                Ok(a.wrapping_sub(b))
            })
        }
        (ElementBinaryKernel::MulInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, |a, b| {
                Ok(a.wrapping_mul(b))
            })
        }
        (
            ElementBinaryKernel::DivInt,
            ScalarLayout::Int {
                width: 64,
                is_signed: true,
            },
        ) => {
            execute_contiguous_binary!(dest, left, right, element_count, i64, divide_i64)
        }
        (ElementBinaryKernel::DivUint, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, divide_u64)
        }
        (
            ElementBinaryKernel::RemInt,
            ScalarLayout::Int {
                width: 64,
                is_signed: true,
            },
        ) => {
            execute_contiguous_binary!(dest, left, right, element_count, i64, remainder_i64)
        }
        (ElementBinaryKernel::RemUint, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, remainder_u64)
        }
        (ElementBinaryKernel::AndInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, |a, b| Ok(a & b))
        }
        (ElementBinaryKernel::OrInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, |a, b| Ok(a | b))
        }
        (ElementBinaryKernel::XorInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, |a, b| Ok(a ^ b))
        }
        (ElementBinaryKernel::ShlInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, |a, b| {
                Ok(a.wrapping_shl(b as u32))
            })
        }
        (
            ElementBinaryKernel::ShrInt,
            ScalarLayout::Int {
                width: 64,
                is_signed: true,
            },
        ) => {
            execute_contiguous_binary!(dest, left, right, element_count, i64, |a, b| {
                Ok(a.wrapping_shr(b as u32))
            })
        }
        (ElementBinaryKernel::ShrUint, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_binary!(dest, left, right, element_count, u64, |a, b| {
                Ok(a.wrapping_shr(b as u32))
            })
        }
        (ElementBinaryKernel::EqInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u64, |a, b| a == b)
        }
        (ElementBinaryKernel::NeInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u64, |a, b| a != b)
        }
        (
            ElementBinaryKernel::LtInt,
            ScalarLayout::Int {
                width: 64,
                is_signed: true,
            },
        ) => {
            execute_contiguous_compare!(dest, left, right, element_count, i64, |a, b| a < b)
        }
        (ElementBinaryKernel::LtUint, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u64, |a, b| a < b)
        }
        (
            ElementBinaryKernel::LeInt,
            ScalarLayout::Int {
                width: 64,
                is_signed: true,
            },
        ) => {
            execute_contiguous_compare!(dest, left, right, element_count, i64, |a, b| a <= b)
        }
        (ElementBinaryKernel::LeUint, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u64, |a, b| a <= b)
        }
        (
            ElementBinaryKernel::GtInt,
            ScalarLayout::Int {
                width: 64,
                is_signed: true,
            },
        ) => {
            execute_contiguous_compare!(dest, left, right, element_count, i64, |a, b| a > b)
        }
        (ElementBinaryKernel::GtUint, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u64, |a, b| a > b)
        }
        (
            ElementBinaryKernel::GeInt,
            ScalarLayout::Int {
                width: 64,
                is_signed: true,
            },
        ) => {
            execute_contiguous_compare!(dest, left, right, element_count, i64, |a, b| a >= b)
        }
        (ElementBinaryKernel::GeUint, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_compare!(dest, left, right, element_count, u64, |a, b| a >= b)
        }
        (ElementBinaryKernel::AddF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_binary!(dest, left, right, element_count, f32, |a, b| Ok(a + b))
        }
        (ElementBinaryKernel::SubF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_binary!(dest, left, right, element_count, f32, |a, b| Ok(a - b))
        }
        (ElementBinaryKernel::MulF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_binary!(dest, left, right, element_count, f32, |a, b| Ok(a * b))
        }
        (ElementBinaryKernel::DivF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_binary!(dest, left, right, element_count, f32, |a, b| Ok(a / b))
        }
        (ElementBinaryKernel::EqF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f32, |a, b| a == b)
        }
        (ElementBinaryKernel::NeF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f32, |a, b| a != b)
        }
        (ElementBinaryKernel::LtF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f32, |a, b| a < b)
        }
        (ElementBinaryKernel::LeF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f32, |a, b| a <= b)
        }
        (ElementBinaryKernel::GtF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f32, |a, b| a > b)
        }
        (ElementBinaryKernel::GeF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f32, |a, b| a >= b)
        }
        (ElementBinaryKernel::AddF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_binary!(dest, left, right, element_count, f64, |a, b| Ok(a + b))
        }
        (ElementBinaryKernel::SubF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_binary!(dest, left, right, element_count, f64, |a, b| Ok(a - b))
        }
        (ElementBinaryKernel::MulF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_binary!(dest, left, right, element_count, f64, |a, b| Ok(a * b))
        }
        (ElementBinaryKernel::DivF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_binary!(dest, left, right, element_count, f64, |a, b| Ok(a / b))
        }
        (ElementBinaryKernel::EqF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f64, |a, b| a == b)
        }
        (ElementBinaryKernel::NeF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f64, |a, b| a != b)
        }
        (ElementBinaryKernel::LtF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f64, |a, b| a < b)
        }
        (ElementBinaryKernel::LeF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f64, |a, b| a <= b)
        }
        (ElementBinaryKernel::GtF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f64, |a, b| a > b)
        }
        (ElementBinaryKernel::GeF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_compare!(dest, left, right, element_count, f64, |a, b| a >= b)
        }
        _ => return None,
    };

    Some(result)
}

/// Execute one contiguous tensor binary operation.
fn execute_contiguous_tensor_binary_elements(
    machine: &mut Machine<'_, '_>,
    offsets: (u32, u32, u32),
    layout: &TensorLayout,
    element_layout: ScalarLayout,
    kernel: ElementBinaryKernel,
) -> Result<(), Error> {
    let dest_layout = layout
        .element
        .word_layout
        .ok_or(Error::invalid_instruction())?;
    let element_word_layout = tensor_word_layout(element_layout)?;
    let addresses = contiguous_binary_addresses(machine, offsets);

    // use direct typed loops for machine-natural element layouts
    if let Some(result) = execute_contiguous_tensor_binary_typed(
        addresses,
        layout.element_span_len,
        element_layout,
        kernel,
    ) {
        return result;
    }

    let operation = tensor_binary_operation(kernel);
    let (dest, left, right) = addresses;

    // resolve generic element strides
    let dest_stride = dest_layout.byte_len(POINTER_BYTE_LEN);
    let element_stride = element_word_layout.byte_len(POINTER_BYTE_LEN);

    // walk contiguous elements without recomputing logical indices
    for index in 0..layout.element_span_len {
        let left = access::load_scalar_by_layout_at_address(
            left as usize + index * element_stride,
            element_word_layout,
        );
        let right = access::load_scalar_by_layout_at_address(
            right as usize + index * element_stride,
            element_word_layout,
        );
        let value = operation(element_layout, left, right)?;
        access::store_scalar_by_layout_at_address(
            dest as usize + index * dest_stride,
            dest_layout,
            value,
        );
    }

    Ok(())
}

/// Execute one contiguous typed tensor unary operation.
fn execute_contiguous_tensor_unary_typed(
    addresses: (*mut u8, *const u8),
    element_count: usize,
    element_layout: ScalarLayout,
    kernel: ElementUnaryKernel,
) -> Option<Result<(), Error>> {
    let (dest, argument) = addresses;

    // select one typed loop before walking elements
    let result = match (kernel, element_layout) {
        (ElementUnaryKernel::NotBool, ScalarLayout::Bool) => {
            execute_contiguous_unary!(dest, argument, element_count, u8, |a| u8::from(a == 0))
        }
        (
            ElementUnaryKernel::NegInt,
            ScalarLayout::Int {
                width: 32,
                is_signed: true,
            },
        ) => {
            execute_contiguous_unary!(dest, argument, element_count, i32, |a| a.wrapping_neg())
        }
        (ElementUnaryKernel::NotInt, ScalarLayout::Int { width: 32, .. }) => {
            execute_contiguous_unary!(dest, argument, element_count, u32, |a| !a)
        }
        (
            ElementUnaryKernel::NegInt,
            ScalarLayout::Int {
                width: 64,
                is_signed: true,
            },
        ) => {
            execute_contiguous_unary!(dest, argument, element_count, i64, |a| a.wrapping_neg())
        }
        (ElementUnaryKernel::NotInt, ScalarLayout::Int { width: 64, .. }) => {
            execute_contiguous_unary!(dest, argument, element_count, u64, |a| !a)
        }
        (ElementUnaryKernel::NegF32, ScalarLayout::Float { width: 32 }) => {
            execute_contiguous_unary!(dest, argument, element_count, f32, |a| -a)
        }
        (ElementUnaryKernel::NegF64, ScalarLayout::Float { width: 64 }) => {
            execute_contiguous_unary!(dest, argument, element_count, f64, |a| -a)
        }
        _ => return None,
    };

    Some(result)
}

/// Execute one contiguous tensor unary operation.
fn execute_contiguous_tensor_unary_elements(
    machine: &mut Machine<'_, '_>,
    offsets: (u32, u32),
    layout: &TensorLayout,
    element_layout: ScalarLayout,
    kernel: ElementUnaryKernel,
) -> Result<(), Error> {
    let dest_layout = layout
        .element
        .word_layout
        .ok_or(Error::invalid_instruction())?;
    let element_word_layout = tensor_word_layout(element_layout)?;
    let addresses = contiguous_unary_addresses(machine, offsets);

    // use direct typed loops for machine-natural element layouts
    if let Some(result) = execute_contiguous_tensor_unary_typed(
        addresses,
        layout.element_span_len,
        element_layout,
        kernel,
    ) {
        return result;
    }

    let operation = tensor_unary_operation(kernel);
    let (dest, argument) = addresses;

    // resolve generic element strides
    let dest_stride = dest_layout.byte_len(POINTER_BYTE_LEN);
    let element_stride = element_word_layout.byte_len(POINTER_BYTE_LEN);

    // walk contiguous elements without recomputing logical indices
    for index in 0..layout.element_span_len {
        let value = access::load_scalar_by_layout_at_address(
            argument as usize + index * element_stride,
            element_word_layout,
        );
        let value = operation(element_layout, value)?;
        access::store_scalar_by_layout_at_address(
            dest as usize + index * dest_stride,
            dest_layout,
            value,
        );
    }

    Ok(())
}

/// Execute a contiguous tensor unary operation.
pub(crate) fn execute_tensor_contiguous_unary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let TensorContiguousUnary {
        dest_offset,
        argument_offset,
        dest_layout,
        element_layout,
        kernel,
    } = machine.side::<TensorContiguousUnary>(instruction);
    let layout = tensor_layout(machine, *dest_layout);
    execute_contiguous_tensor_unary_elements(
        machine,
        (*dest_offset, *argument_offset),
        layout,
        *element_layout,
        *kernel,
    )?;

    Ok(())
}

/// Execute tensor.splat.
pub(crate) fn execute_tensor_splat(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest_offset = instruction.a;
    let value = instruction.b;
    let layout = TensorLayoutId(instruction.c);

    // resolve compiled tensor descriptor
    let layout = tensor_layout(machine, layout);
    let value = machine.load_word_at(value);

    // store the same value into each active index
    store_tensor_indexed_elements(machine, dest_offset, layout, |_machine, _index| Ok(value))?;

    Ok(())
}

/// Execute tensor.extract.
pub(crate) fn execute_tensor_extract(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
    let index = tensor_index_values(machine, index_values)?;
    let element_index = tensor_linear_index(&index, &layout.shape, &layout.strides)?;

    // load the tensor value
    let tensor_value = frame_value(machine, *tensor_offset);
    let value = load_frame_tensor_element_at(machine, tensor_value, layout, element_index)?;
    machine.store_word_at(*dest_offset, value);

    Ok(())
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

/// Offset a local heap tensor view pointer by one element index.
pub(crate) fn offset_heap_view_pointer(
    value: Word,
    element: Projection,
    offset: usize,
    length: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(element, offset, length)?;
    let reference = value.as_heap_reference();
    let reference = reference.add_bytes(byte_offset);

    Ok(Word::heap_reference(reference))
}

/// Offset a shared heap tensor view pointer by one element index.
pub(crate) fn offset_shared_heap_view_pointer(
    value: Word,
    element: Projection,
    offset: usize,
    length: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(element, offset, length)?;
    let reference = value.as_shared_heap_reference();
    let reference = reference.add_bytes(byte_offset);

    Ok(Word::shared_heap_reference(reference))
}

/// Offset a raw tensor view pointer by one element index.
pub(crate) fn offset_raw_view_pointer(
    value: Word,
    element: Projection,
    offset: usize,
    length: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(element, offset, length)?;
    let address = value.as_address() + byte_offset;

    Ok(Word::address(address))
}

/// Offset a stack tensor view pointer by one element index.
pub(crate) fn offset_stack_view_pointer(
    value: Word,
    element: Projection,
    offset: usize,
    length: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(element, offset, length)?;
    let pointer = value.as_stack_pointer();
    let pointer = pointer.add_bytes(byte_offset);

    Ok(Word::stack_pointer(pointer))
}

/// Offset a frame tensor view pointer by one element index.
pub(crate) fn offset_frame_view_pointer(
    value: Word,
    element: Projection,
    offset: usize,
    length: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(element, offset, length)?;
    let pointer = value.as_frame_pointer();
    let pointer = pointer.add_bytes(byte_offset);

    Ok(Word::frame_pointer(pointer))
}

/// Offset a static tensor view pointer by one element index.
pub(crate) fn offset_static_view_pointer(
    value: Word,
    element: Projection,
    offset: usize,
    length: usize,
) -> Result<Word, Error> {
    let byte_offset = element_byte_offset(element, offset, length)?;
    let pointer = value.as_static_pointer();
    let pointer = pointer.add_bytes(byte_offset);

    Ok(Word::static_pointer(pointer))
}

/// Offset a tensor view pointer through selected memory.
#[inline(always)]
fn offset_tensor_view_pointer(
    value: Word,
    element: Projection,
    offset: usize,
    length: usize,
    address: TensorAddress,
) -> Result<Word, Error> {
    match address {
        TensorAddress::Heap => offset_heap_view_pointer(value, element, offset, length),
        TensorAddress::SharedHeap => {
            offset_shared_heap_view_pointer(value, element, offset, length)
        }
        TensorAddress::Address => offset_raw_view_pointer(value, element, offset, length),
        TensorAddress::Stack => offset_stack_view_pointer(value, element, offset, length),
        TensorAddress::Frame => offset_frame_view_pointer(value, element, offset, length),
        TensorAddress::Static => offset_static_view_pointer(value, element, offset, length),
    }
}

/// Fill a tensor view through one concrete pointer space.
fn fill_tensor_view<O, S>(
    machine: &mut Machine<'_, '_>,
    base_pointer: Word,
    layout: &TensorLayout,
    strides: &[u64],
    element: Projection,
    fill_value: Word,
    mut offset_pointer: O,
    mut store_element: S,
) -> Result<(), Error>
where
    O: FnMut(Word, Projection, usize, usize) -> Result<Word, Error>,
    S: FnMut(&mut Machine<'_, '_>, Word, Projection, Word) -> Result<(), Error>,
{
    let span_len = tensor_element_span_len(&layout.shape, strides)?;
    let mut error = None;

    // write each logical element through the selected concrete accessor
    for_each_index(&layout.shape, |index| {
        if error.is_some() {
            return;
        }

        let offset = match tensor_linear_index(index, &layout.shape, strides) {
            Ok(offset) => offset,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };
        let pointer = match offset_pointer(base_pointer, element, offset, span_len) {
            Ok(pointer) => pointer,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };

        if let Err(current_error) = store_element(machine, pointer, element, fill_value) {
            error = Some(current_error);
        }
    });

    if let Some(error) = error {
        return Err(error);
    }

    Ok(())
}

/// Copy tensor elements through one concrete address pair.
fn copy_tensor_view<TO, SO, L, S>(
    machine: &mut Machine<'_, '_>,
    target_pointer: Word,
    mir_pointer: Word,
    target_layout: &TensorLayout,
    source_layout: &TensorLayout,
    target_strides: &[u64],
    source_strides: &[u64],
    target_element: Projection,
    source_element: Projection,
    mut target_offset: TO,
    mut source_offset: SO,
    mut load_element: L,
    mut store_element: S,
) -> Result<(), Error>
where
    TO: FnMut(Word, Projection, usize, usize) -> Result<Word, Error>,
    SO: FnMut(Word, Projection, usize, usize) -> Result<Word, Error>,
    L: FnMut(&mut Machine<'_, '_>, Word, Projection) -> Result<Word, Error>,
    S: FnMut(&mut Machine<'_, '_>, Word, Projection, Word) -> Result<(), Error>,
{
    let target_span_len = tensor_element_span_len(&target_layout.shape, target_strides)?;
    let source_span_len = tensor_element_span_len(&source_layout.shape, source_strides)?;
    let mut error = None;

    // copy each logical element through the selected concrete accessors
    for_each_index(&target_layout.shape, |index| {
        if error.is_some() {
            return;
        }

        let target_index = match tensor_linear_index(index, &target_layout.shape, target_strides) {
            Ok(offset) => offset,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };
        let source_index = match tensor_linear_index(index, &source_layout.shape, source_strides) {
            Ok(offset) => offset,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };

        let mir_pointer =
            match source_offset(mir_pointer, source_element, source_index, source_span_len) {
                Ok(pointer) => pointer,
                Err(current_error) => {
                    error = Some(current_error);
                    return;
                }
            };
        let target_pointer = match target_offset(
            target_pointer,
            target_element,
            target_index,
            target_span_len,
        ) {
            Ok(pointer) => pointer,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };
        let value = match load_element(machine, mir_pointer, source_element) {
            Ok(value) => value,
            Err(current_error) => {
                error = Some(current_error);
                return;
            }
        };

        if let Err(current_error) = store_element(machine, target_pointer, target_element, value) {
            error = Some(current_error);
        }
    });

    if let Some(error) = error {
        return Err(error);
    }

    Ok(())
}

/// Execute a dense pointer to tensor view cast.
pub(crate) fn execute_tensor_view_cast(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let TensorViewCast {
        dest_offset,
        pointer_offset,
        view_layout,
    } = machine.side::<TensorViewCast>(instruction);

    let layout = tensor_layout(machine, *view_layout);
    let pointer = machine.load_word_at(*pointer_offset);

    store_tensor_view_descriptor(machine, *dest_offset, pointer, &layout.strides)
}

/// Execute tensor.load.
pub(crate) fn execute_tensor_load(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed tensor view descriptor
    let TensorLoad {
        dest_offset,
        view_offset,
        indices,
        view_layout,
        element,
        address,
    } = machine.side::<TensorLoad>(instruction);

    // compute the logical element offset
    let layout = tensor_layout(machine, *view_layout);
    let index_values = frame_offsets(machine, *indices);
    let index = tensor_index_values(machine, index_values)?;
    let strides = load_tensor_view_strides(machine, *view_offset, layout)?;
    let span_len = tensor_element_span_len(&layout.shape, &strides)?;
    let offset = tensor_linear_index(&index, &layout.shape, &strides)?;

    // load through the concrete memory accessors selected by lower
    let view_value = load_tensor_view_pointer(machine, *view_offset);
    let element = machine.projection(*element);
    let pointer = offset_tensor_view_pointer(view_value, element, offset, span_len, *address)?;

    let value = load_tensor_element(machine, pointer, element, *address)?;
    machine.store_word_at(*dest_offset, value);

    Ok(())
}

/// Execute tensor.store.
pub(crate) fn execute_tensor_store(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed tensor view descriptor
    let TensorStore {
        view_offset,
        indices,
        value_offset,
        view_layout,
        element,
        address,
    } = machine.side::<TensorStore>(instruction);

    // compute the logical element offset
    let layout = tensor_layout(machine, *view_layout);
    let index_values = frame_offsets(machine, *indices);
    let index = tensor_index_values(machine, index_values)?;
    let strides = load_tensor_view_strides(machine, *view_offset, layout)?;
    let span_len = tensor_element_span_len(&layout.shape, &strides)?;
    let offset = tensor_linear_index(&index, &layout.shape, &strides)?;

    // store through the concrete memory accessors selected by lower
    let view_value = load_tensor_view_pointer(machine, *view_offset);
    let element = machine.projection(*element);
    let pointer = offset_tensor_view_pointer(view_value, element, offset, span_len, *address)?;

    let value = machine.load_word_at(*value_offset);
    store_tensor_element(machine, pointer, element, value, *address)?;

    Ok(())
}

/// Execute tensor.fill.
pub(crate) fn execute_tensor_fill(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed tensor view descriptor
    let TensorFill {
        view_offset,
        value_offset,
        view_layout,
        element,
        address,
    } = machine.side::<TensorFill>(instruction);

    // resolve the repeated value and base view once
    let layout = tensor_layout(machine, *view_layout);
    let fill_value = machine.load_word_at(*value_offset);
    let strides = load_tensor_view_strides(machine, *view_offset, layout)?;
    let base_pointer = load_tensor_view_pointer(machine, *view_offset);

    // write each addressable element through one concrete memory accessor
    let element = machine.projection(*element);
    match address {
        TensorAddress::Heap => fill_tensor_view(
            machine,
            base_pointer,
            layout,
            &strides,
            element,
            fill_value,
            offset_heap_view_pointer,
            store_heap_tensor_element,
        )?,
        TensorAddress::SharedHeap => fill_tensor_view(
            machine,
            base_pointer,
            layout,
            &strides,
            element,
            fill_value,
            offset_shared_heap_view_pointer,
            store_shared_heap_tensor_element,
        )?,
        TensorAddress::Address => fill_tensor_view(
            machine,
            base_pointer,
            layout,
            &strides,
            element,
            fill_value,
            offset_raw_view_pointer,
            store_raw_tensor_element,
        )?,
        TensorAddress::Stack => fill_tensor_view(
            machine,
            base_pointer,
            layout,
            &strides,
            element,
            fill_value,
            offset_stack_view_pointer,
            store_stack_tensor_element,
        )?,
        TensorAddress::Frame => fill_tensor_view(
            machine,
            base_pointer,
            layout,
            &strides,
            element,
            fill_value,
            offset_frame_view_pointer,
            store_frame_tensor_element,
        )?,
        TensorAddress::Static => fill_tensor_view(
            machine,
            base_pointer,
            layout,
            &strides,
            element,
            fill_value,
            offset_static_view_pointer,
            store_static_tensor_element,
        )?,
    }

    Ok(())
}

/// Execute tensor.copy.
pub(crate) fn execute_tensor_copy(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed tensor copy descriptor
    let TensorCopy {
        target_offset: target_frame_offset,
        source_offset: source_frame_offset,
        target_layout,
        source_layout,
        target_element,
        source_element,
        target_address,
        source_address,
    } = machine.side::<TensorCopy>(instruction);

    // resolve compiled tensor descriptors
    let target_layout = tensor_layout(machine, *target_layout);
    let source_layout = tensor_layout(machine, *source_layout);

    // require identical logical shapes
    if target_layout.shape != source_layout.shape {
        return Err(Error::type_mismatch(
            "matching tensor shapes",
            format!("{:?} vs {:?}", target_layout.shape, source_layout.shape),
        ));
    }

    // resolve both view pointers and element descriptors once
    let target_strides = load_tensor_view_strides(machine, *target_frame_offset, target_layout)?;
    let source_strides = load_tensor_view_strides(machine, *source_frame_offset, source_layout)?;
    let target_pointer = load_tensor_view_pointer(machine, *target_frame_offset);
    let mir_pointer = load_tensor_view_pointer(machine, *source_frame_offset);
    let source_element = machine.projection(*source_element);
    let target_element = machine.projection(*target_element);

    // copy through the selected pointer spaces
    copy_tensor_view(
        machine,
        target_pointer,
        mir_pointer,
        target_layout,
        source_layout,
        &target_strides,
        &source_strides,
        target_element,
        source_element,
        |pointer, element, offset, length| {
            offset_tensor_view_pointer(pointer, element, offset, length, *target_address)
        },
        |pointer, element, offset, length| {
            offset_tensor_view_pointer(pointer, element, offset, length, *source_address)
        },
        |machine, pointer, element| load_tensor_element(machine, pointer, element, *source_address),
        |machine, pointer, element, value| {
            store_tensor_element(machine, pointer, element, value, *target_address)
        },
    )?;

    Ok(())
}

/// Execute tensor.reshape.
pub(crate) fn execute_tensor_reshape(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
        let value = machine.load_word_at(*offset);
        let size = word_to_u64(value)?;
        shape_len *= size;
    }
    if shape_values.is_empty() {
        shape_len = dest_layout.element_span_len as u64;
    }

    // validate element counts
    if dest_layout.element_span_len as u64 != shape_len {
        return Err(Error::type_mismatch(
            "reshape element count",
            format!("{} vs {}", dest_layout.element_span_len, shape_len),
        ));
    }

    // copy the source elements into the reshaped result
    store_tensor_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, element_index| {
            load_frame_tensor_element_at(machine, tensor_value, source_layout, element_index)
        },
    )?;

    Ok(())
}

/// Execute tensor.broadcast.
pub(crate) fn execute_tensor_broadcast(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode side records
    let TensorBroadcast {
        dest_offset,
        tensor_offset,
        dimensions,
        source_layout,
        dest_layout,
    } = machine.side::<TensorBroadcast>(instruction);
    let dimensions = machine.u32_range(*dimensions);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // validate dimension mapping
    if dimensions.len() != source_layout.shape.len() {
        return Err(Error::type_mismatch(
            "broadcast dimension mapping",
            format!("{} vs {}", dimensions.len(), source_layout.shape.len()),
        ));
    }

    // resolve source elements
    let tensor_value = frame_value(machine, *tensor_offset);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // store result
    store_tensor_indexed_elements(
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
    )?;

    Ok(())
}

/// Execute tensor.transpose.
pub(crate) fn execute_tensor_transpose(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode side records
    let TensorTranspose {
        dest_offset,
        tensor_offset,
        permutation,
        source_layout,
        dest_layout,
    } = machine.side::<TensorTranspose>(instruction);
    let permutation = machine.u32_range(*permutation);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // validate permutation
    if permutation.len() != source_layout.shape.len() {
        return Err(Error::type_mismatch(
            "transpose permutation",
            format!("{} vs {}", permutation.len(), source_layout.shape.len()),
        ));
    }

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // store result
    store_tensor_indexed_elements(
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
    )?;

    Ok(())
}

/// Execute tensor.slice.
pub(crate) fn execute_tensor_slice(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
        return Err(Error::invalid_instruction());
    }

    let mut offsets = Vec::with_capacity(offset_values.len());
    let mut sizes = Vec::with_capacity(size_values.len());
    let mut strides = Vec::with_capacity(stride_values.len());
    for offset in offset_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        offsets.push(value);
    }
    for offset in size_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        sizes.push(value);
    }
    for offset in stride_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        strides.push(value);
    }

    // require full-rank slice arguments
    let rank = source_layout.shape.len();
    if offsets.len() != rank || sizes.len() != rank || strides.len() != rank {
        return Err(Error::invalid_instruction());
    }

    // require runtime sizes to match the destination tensor
    if sizes.len() != dest_layout.shape.len() {
        return Err(Error::invalid_instruction());
    }
    for (expected, actual) in dest_layout.shape.iter().zip(sizes.iter()) {
        if expected != actual {
            return Err(Error::type_mismatch(
                "tensor slice size",
                format!("{actual} vs {expected}"),
            ));
        }
    }

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // store result
    store_tensor_indexed_elements(
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
    )?;

    Ok(())
}

/// Execute tensor.pad.
pub(crate) fn execute_tensor_pad(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
        return Err(Error::invalid_instruction());
    }

    let mut low = Vec::with_capacity(low_values.len());
    let mut high = Vec::with_capacity(high_values.len());
    let mut interior = Vec::with_capacity(interior_values.len());
    for offset in low_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        low.push(value);
    }
    for offset in high_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        high.push(value);
    }
    for offset in interior_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        interior.push(value);
    }

    // require full-rank padding arguments
    let rank = source_layout.shape.len();
    if low.len() != rank || high.len() != rank || interior.len() != rank {
        return Err(Error::invalid_instruction());
    }

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);
    let pad_value = machine.load_word_at(*value_offset);
    let mut input_index = vec![0u64; source_layout.shape.len()];

    // store result
    store_tensor_indexed_elements(
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
    )?;

    Ok(())
}

/// Execute tensor.concat.
pub(crate) fn execute_tensor_concat(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode side records
    let TensorConcat {
        dest_offset,
        tensors,
        tensor_layouts,
        axis,
        dest_layout,
    } = machine.side::<TensorConcat>(instruction);
    let tensor_layouts = machine.u32_range(*tensor_layouts);

    // resolve compiled tensor descriptor
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve input tensors
    let tensor_offsets = frame_offsets(machine, *tensors).to_vec();
    // validate input metadata
    if tensor_offsets.len() != tensor_layouts.len() {
        return Err(Error::invalid_instruction());
    }
    let mut inputs = Vec::with_capacity(tensor_offsets.len());
    let mut axis_sizes = Vec::with_capacity(tensor_offsets.len());
    for (offset, layout) in tensor_offsets.iter().zip(tensor_layouts.iter()) {
        let value = frame_value(machine, *offset);
        let layout = tensor_layout(machine, TensorLayoutId(*layout));
        let axis_index = *axis as usize;
        if axis_index >= layout.shape.len() {
            return Err(Error::invalid_instruction());
        }
        axis_sizes.push(layout.shape[axis_index]);
        inputs.push((value, layout));
    }

    // compute axis offsets
    let mut axis_offsets = Vec::with_capacity(axis_sizes.len());
    let mut running = 0u64;
    for size in &axis_sizes {
        axis_offsets.push(running);
        running += *size;
    }

    let axis_index = *axis as usize;
    let mut input_index = vec![0u64; dest_layout.shape.len()];

    // store result
    store_tensor_indexed_elements(
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
                return Err(Error::invalid_instruction());
            };

            input_index.clone_from_slice(output_index);
            input_index[axis_index] = local_axis;
            let (tensor_value, layout) = &inputs[input_idx];

            let source_offset = tensor_linear_index(&input_index, &layout.shape, &layout.strides)?;

            load_frame_tensor_element_at(machine, *tensor_value, layout, source_offset)
        },
    )?;

    Ok(())
}

/// Execute tensor.reduce.
pub(crate) fn execute_tensor_reduce(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let TensorReduce { kernel, .. } = machine.side::<TensorReduce>(instruction);
    let operation = tensor_reduce_operation(*kernel);
    execute_tensor_reduce_elements(machine, instruction, operation)
}

/// Execute tensor reduction with one scalar kernel.
fn execute_tensor_reduce_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
) -> Result<(), Error> {
    // decode side records
    let TensorReduce {
        dest_offset,
        tensor_offset,
        initial_offset,
        axes,
        source_layout,
        dest_layout,
        kernel: _,
    } = machine.side::<TensorReduce>(instruction);
    let axes = machine.u32_range(*axes);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);
    let element_layout = source_layout.element_layout;

    // resolve source tensor
    let tensor_value = frame_value(machine, *tensor_offset);
    let init_value = machine.load_word_at(*initial_offset);
    let reduction = tensor_reduction(&source_layout.shape, axes)?;
    if dest_layout.shape.as_ref() != reduction.output_shape.as_slice() {
        return Err(Error::invalid_instruction());
    }
    let mut source_index = vec![0u64; source_layout.shape.len()];

    // store result
    store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            let mut output_cursor = 0usize;
            for (dim, element) in source_index.iter_mut().enumerate() {
                if reduction.axis_mask[dim] {
                    *element = 0;
                    continue;
                }

                *element = output_index
                    .get(output_cursor)
                    .copied()
                    .ok_or(Error::invalid_instruction())?;
                output_cursor += 1;
            }

            let mut accum = init_value;
            let mut reduce_error = None;

            // accumulate the reduced axes for this destination index
            for_each_index(&reduction.reduced_shape, |reduced_index| {
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
    )?;

    Ok(())
}

/// Return the scalar kernel for one tensor reduction.
fn tensor_reduce_operation(
    kernel: mir::TensorReduceOperator,
) -> fn(ScalarLayout, Word, Word) -> Result<Word, Error> {
    match kernel {
        mir::TensorReduceOperator::Add => reduce_add,
        mir::TensorReduceOperator::Multiply => reduce_multiply,
        mir::TensorReduceOperator::Min => reduce_min,
        mir::TensorReduceOperator::Max => reduce_max,
        mir::TensorReduceOperator::And => reduce_and,
        mir::TensorReduceOperator::Or => reduce_or,
        mir::TensorReduceOperator::Xor => reduce_xor,
    }
}

/// Shape information for one tensor reduction.
struct TensorReduction {
    /// The source axes reduced by this operation.
    axis_mask: Vec<bool>,
    /// The shape traversed inside the reduction loop.
    reduced_shape: Vec<u64>,
    /// The result shape after removing reduced axes.
    output_shape: Vec<u64>,
    /// The number of elements read for one output element.
    element_count: u64,
}

/// Compute shape information for one tensor reduction.
fn tensor_reduction(source_shape: &[u64], axes: &[u32]) -> Result<TensorReduction, Error> {
    let mut axis_mask = vec![false; source_shape.len()];
    let mut reduced_shape = Vec::with_capacity(axes.len());
    let mut element_count = 1u64;

    for axis in axes {
        let axis = *axis as usize;
        if axis >= source_shape.len() || axis_mask[axis] {
            return Err(Error::invalid_instruction());
        }

        let dim = source_shape[axis];
        axis_mask[axis] = true;
        reduced_shape.push(dim);
        element_count = element_count
            .checked_mul(dim)
            .ok_or(Error::invalid_instruction())?;
    }

    let output_shape = source_shape
        .iter()
        .enumerate()
        .filter_map(|(index, dim)| (!axis_mask[index]).then_some(*dim))
        .collect();

    Ok(TensorReduction {
        axis_mask,
        reduced_shape,
        output_shape,
        element_count,
    })
}

/// Execute tensor index reduction.
pub(crate) fn execute_tensor_index_reduce(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode side records
    let TensorIndexReduce {
        dest_offset,
        tensor_offset,
        axis,
        source_layout,
        dest_layout,
        kernel,
        tie_break,
    } = machine.side::<TensorIndexReduce>(instruction);
    let axes = [*axis];

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);
    let element_layout = source_layout.element_layout;

    // resolve reduced shape
    let tensor_value = frame_value(machine, *tensor_offset);
    let reduction = tensor_reduction(&source_layout.shape, &axes)?;
    if reduction.element_count == 0 {
        return Err(Error::invalid_instruction());
    }
    if dest_layout.shape.as_ref() != reduction.output_shape.as_slice() {
        return Err(Error::invalid_instruction());
    }
    if !tensor_index_layout_can_store(dest_layout.element_layout, reduction.element_count) {
        return Err(Error::invalid_instruction());
    }
    let mut source_index = vec![0u64; source_layout.shape.len()];

    // store result
    store_tensor_indexed_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, output_index| {
            let mut output_cursor = 0usize;
            for (dim, element) in source_index.iter_mut().enumerate() {
                if reduction.axis_mask[dim] {
                    *element = 0;
                } else {
                    *element = output_index
                        .get(output_cursor)
                        .copied()
                        .ok_or(Error::invalid_instruction())?;
                    output_cursor += 1;
                }
            }

            let mut best_value = None;
            let mut best_index = 0u64;
            let mut linear_index = 0u64;
            let mut reduce_error = None;

            // scan the reduced axis for this destination index
            for_each_index(&reduction.reduced_shape, |reduced_index| {
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

                let is_selected = match best_value {
                    Some(value) => tensor_index_reduce_select(
                        element_layout,
                        *kernel,
                        *tie_break,
                        value,
                        source_value,
                    ),
                    None => Ok(true),
                };
                match is_selected {
                    Ok(true) => {
                        best_value = Some(source_value);
                        best_index = linear_index;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        reduce_error = Some(error);
                        return;
                    }
                }

                linear_index += 1;
            });

            if let Some(error) = reduce_error {
                return Err(error);
            }

            Ok(Word::uint(best_index, Word::BIT_LEN))
        },
    )?;

    Ok(())
}

/// Return whether an index result layout can store every reduced position.
fn tensor_index_layout_can_store(layout: ScalarLayout, element_count: u64) -> bool {
    let ScalarLayout::Int {
        width,
        is_signed: false,
    } = layout
    else {
        return false;
    };

    let max_index = element_count - 1;
    let needed_bits = (u64::BITS - max_index.leading_zeros()).max(1) as u16;

    width >= needed_bits
}

/// Return whether `candidate` should replace `current`.
fn tensor_index_reduce_select(
    layout: ScalarLayout,
    operator: mir::TensorIndexReduceOperator,
    tie_break: mir::TensorIndexTieBreak,
    current: Word,
    candidate: Word,
) -> Result<bool, Error> {
    let comparison = compare_tensor_words(layout, current, candidate)?;
    let is_equal_last =
        comparison == Ordering::Equal && tie_break == mir::TensorIndexTieBreak::Last;
    let is_better = match operator {
        mir::TensorIndexReduceOperator::Min => comparison == Ordering::Greater,
        mir::TensorIndexReduceOperator::Max => comparison == Ordering::Less,
    };

    Ok(is_better || is_equal_last)
}

/// Compare two tensor scalar words.
fn compare_tensor_words(layout: ScalarLayout, left: Word, right: Word) -> Result<Ordering, Error> {
    let ordering = match layout {
        ScalarLayout::Int {
            is_signed: true, ..
        } => left.as_i64().cmp(&right.as_i64()),
        ScalarLayout::Int { .. } => left.as_u64().cmp(&right.as_u64()),
        ScalarLayout::Float { width: 32 } => left
            .as_f32()
            .partial_cmp(&right.as_f32())
            .ok_or(Error::invalid_instruction())?,
        ScalarLayout::Float { width: 64 } => left
            .as_f64()
            .partial_cmp(&right.as_f64())
            .ok_or(Error::invalid_instruction())?,
        _ => return Err(Error::invalid_instruction()),
    };

    Ok(ordering)
}

/// Execute tensor.dot.
pub(crate) fn execute_tensor_dot(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
    let dimensions = machine.tensor_dot(*dimensions);

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
        return Err(Error::type_mismatch(
            "matching tensor element spans",
            format!(
                "{} vs {} vs {}",
                left_layout.element_span_len,
                right_layout.element_span_len,
                dest_layout.element_span_len
            ),
        ));
    }

    // compute axis sets
    let lhs_batch = &dimensions.lhs_batch;
    let rhs_batch = &dimensions.rhs_batch;
    let lhs_contract = &dimensions.lhs_contracting;
    let rhs_contract = &dimensions.rhs_contracting;
    if lhs_batch.len() != rhs_batch.len() || lhs_contract.len() != rhs_contract.len() {
        return Err(Error::invalid_instruction());
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
                match tensor_linear_index(&lhs_index, &left_layout.shape, &left_layout.strides) {
                    Ok(offset) => offset,
                    Err(error) => {
                        contract_error = Some(error);
                        return;
                    }
                };
            let rhs_offset =
                match tensor_linear_index(&rhs_index, &right_layout.shape, &right_layout.strides) {
                    Ok(offset) => offset,
                    Err(error) => {
                        contract_error = Some(error);
                        return;
                    }
                };
            let left_element =
                match load_frame_tensor_element_at(machine, left_value, left_layout, lhs_offset) {
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

        accum.ok_or(Error::invalid_instruction())
    })?;

    Ok(())
}

/// Execute tensor.convolution.
pub(crate) fn execute_tensor_convolution(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
    let dimensions = machine.tensor_convolution(*dimensions);
    let window = machine.tensor_window(*window);

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
        return Err(Error::invalid_instruction());
    }

    // validate window shapes
    if window.strides.len() != spatial_rank
        || window.padding_low.len() != spatial_rank
        || window.padding_high.len() != spatial_rank
        || window.lhs_dilation.len() != spatial_rank
        || window.rhs_dilation.len() != spatial_rank
        || window.window_reversal.len() != spatial_rank
    {
        return Err(Error::invalid_instruction());
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
        return Err(Error::invalid_instruction());
    }

    for dim in dimensions.input_spatial.iter() {
        if *dim as usize >= input_layout.shape.len() {
            return Err(Error::invalid_instruction());
        }
    }
    for dim in output_spatial.iter() {
        if *dim as usize >= dest_layout.shape.len() {
            return Err(Error::invalid_instruction());
        }
    }

    let input_batch_size = input_layout.shape[input_batch_dim];
    let input_feature_size = input_layout.shape[input_feature_dim];
    let output_batch_size = dest_layout.shape[output_batch_dim];
    let output_feature_size = dest_layout.shape[output_feature_dim];

    let mut kernel_spatial_shape = Vec::with_capacity(kernel_spatial.len());
    for dim in kernel_spatial {
        let Some(size) = kernel_layout.shape.get(*dim as usize) else {
            return Err(Error::invalid_instruction());
        };
        kernel_spatial_shape.push(*size);
    }

    // validate group counts
    if *feature_group_count == 0 || *batch_group_count == 0 {
        return Err(Error::invalid_instruction());
    }

    let out_features_per_group = output_feature_size / (*feature_group_count as u64);
    let in_features_per_group = input_feature_size / (*feature_group_count as u64);
    let out_batches_per_group = output_batch_size / (*batch_group_count as u64);
    let in_batches_per_group = input_batch_size / (*batch_group_count as u64);
    let mut input_index = vec![0u64; input_layout.shape.len()];
    let mut kernel_index = vec![0u64; kernel_layout.shape.len()];

    // store result
    store_tensor_indexed_elements(machine, *dest_offset, dest_layout, |machine, out_index| {
        let out_batch = out_index[output_batch_dim];
        let out_feature = out_index[output_feature_dim];

        let batch_group = out_batch / out_batches_per_group;
        let feature_group = out_feature / out_features_per_group;

        let input_batch = batch_group * in_batches_per_group + (out_batch % out_batches_per_group);
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
                    let position = out_pos * stride;
                    let kernel = kernel_pos * rhs_dilation;
                    let mut input_pos = position + kernel;
                    if input_pos < padding_low {
                        is_valid = false;
                        break;
                    }
                    input_pos -= padding_low;
                    if lhs_dilation > 1 {
                        if !input_pos.is_multiple_of(lhs_dilation) {
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
                let product = match reduce_multiply(*element_layout, input_element, kernel_element)
                {
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

        accum.ok_or(Error::invalid_instruction())
    })?;

    Ok(())
}

/// Execute tensor.gather.
pub(crate) fn execute_tensor_gather(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
    let dimensions = machine.tensor_gather(*dimensions);
    let slice_sizes = machine.u32_range(*slice_sizes);

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
                return Err(Error::invalid_instruction());
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
                    return Err(Error::invalid_instruction());
                };
                *offset
            } else {
                0
            };
            let Some(size) = slice_sizes.get(dim) else {
                return Err(Error::invalid_instruction());
            };
            let size = *size as u64;
            if size == 0 {
                return Err(Error::invalid_instruction());
            }

            let start = *element;
            *element = start + offset.min(size - 1);
        }

        let source_offset =
            tensor_linear_index(&source_index, &source_layout.shape, &source_layout.strides)?;

        load_frame_tensor_element_at(machine, source_value, source_layout, source_offset)
    })?;

    Ok(())
}

/// Execute tensor.scatter.
pub(crate) fn execute_tensor_scatter(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let TensorScatter { mode, .. } = machine.side::<TensorScatter>(instruction);
    let combine = tensor_scatter_operation(*mode);

    execute_tensor_scatter_elements(machine, instruction, combine)
}

/// Execute tensor scatter with one scalar kernel.
fn execute_tensor_scatter_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    combine: fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
) -> Result<(), Error> {
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
        mode: _,
    } = machine.side::<TensorScatter>(instruction);
    let dimensions = machine.tensor_scatter(*dimensions);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let indices_layout = tensor_layout(machine, *indices_layout);
    let updates_layout = tensor_layout(machine, *updates_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    // resolve tensors
    let source_value = frame_value(machine, *source_offset);
    let indices_value = frame_value(machine, *indices_offset);
    let updates_value = frame_value(machine, *updates_offset);

    store_tensor_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, element_index| {
            load_frame_tensor_element_at(machine, source_value, source_layout, element_index)
        },
    )?;
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
                scatter_error = Some(Error::invalid_instruction());
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
            scatter_error = Some(Error::invalid_instruction());
            return;
        };

        let mut scatter_indices = Vec::with_capacity(dimensions.scatter_dims_to_operand_dims.len());
        for i in 0..dimensions.scatter_dims_to_operand_dims.len() {
            let element_index = index_offset + i;
            let Ok(value) =
                load_frame_tensor_element_at(machine, indices_value, indices_layout, element_index)
            else {
                scatter_error = Some(Error::invalid_instruction());
                return;
            };
            if let Ok(coord) = word_to_u64(value) {
                scatter_indices.push(coord);
            } else {
                scatter_error = Some(Error::invalid_instruction());
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
                    scatter_error = Some(Error::invalid_instruction());
                    return;
                };
                *element = *update;
            }
        }

        let destination_offset =
            tensor_linear_index(&source_index, &dest_layout.shape, &dest_layout.strides);
        let Ok(destination_offset) = destination_offset else {
            scatter_error = Some(Error::invalid_instruction());
            return;
        };
        let update_offset =
            tensor_linear_index(update_index, &updates_layout.shape, &updates_layout.strides);
        let Ok(update_offset) = update_offset else {
            scatter_error = Some(Error::invalid_instruction());
            return;
        };
        let Ok(update_value) =
            load_frame_tensor_element_at(machine, updates_value, updates_layout, update_offset)
        else {
            scatter_error = Some(Error::invalid_instruction());
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
        return Err(error);
    }

    Ok(())
}

/// Return the scalar kernel for one tensor scatter.
fn tensor_scatter_operation(
    mode: mir::TensorScatterMode,
) -> fn(ScalarLayout, Word, Word) -> Result<Word, Error> {
    match mode {
        mir::TensorScatterMode::Replace => scatter_replace,
        mir::TensorScatterMode::Add => reduce_add,
        mir::TensorScatterMode::Multiply => reduce_multiply,
        mir::TensorScatterMode::Min => reduce_min,
        mir::TensorScatterMode::Max => reduce_max,
        mir::TensorScatterMode::And => reduce_and,
        mir::TensorScatterMode::Or => reduce_or,
        mir::TensorScatterMode::Xor => reduce_xor,
    }
}

/// Return the replacement scatter value.
fn scatter_replace(_layout: ScalarLayout, _current: Word, update: Word) -> Result<Word, Error> {
    Ok(update)
}

/// Execute tensor.convert.
pub(crate) fn execute_tensor_convert(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let TensorConvert { mode, .. } = machine.side::<TensorConvert>(instruction);
    let convert = tensor_convert_operation(*mode);

    execute_tensor_convert_elements(machine, instruction, convert)
}

/// Execute tensor conversion with one scalar kernel.
fn execute_tensor_convert_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    convert: fn(Word, ScalarLayout, ScalarLayout) -> Result<Word, Error>,
) -> Result<(), Error> {
    // decode side records
    let TensorConvert {
        dest_offset,
        tensor_offset,
        source_layout,
        dest_layout,
        source_scalar,
        dest_scalar,
        mode: _,
    } = machine.side::<TensorConvert>(instruction);

    // resolve compiled tensor descriptors
    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    let tensor_value = frame_value(machine, *tensor_offset);
    if source_layout.element_span_len != dest_layout.element_span_len {
        return Err(Error::type_mismatch(
            "matching tensor element spans",
            format!(
                "{} vs {}",
                source_layout.element_span_len, dest_layout.element_span_len
            ),
        ));
    }

    // store result
    store_tensor_elements(
        machine,
        *dest_offset,
        dest_layout,
        |machine, element_index| {
            let source =
                load_frame_tensor_element_at(machine, tensor_value, source_layout, element_index)?;

            convert(source, *source_scalar, *dest_scalar)
        },
    )?;

    Ok(())
}

/// Return the scalar kernel for one tensor conversion.
fn tensor_convert_operation(
    mode: mir::TensorConvertMode,
) -> fn(Word, ScalarLayout, ScalarLayout) -> Result<Word, Error> {
    match mode {
        mir::TensorConvertMode::Exact => convert_scalar_exact,
        mir::TensorConvertMode::RoundTiesEven => convert_scalar_round_ties_even,
        mir::TensorConvertMode::RoundTowardZero => convert_scalar_round_toward_zero,
        mir::TensorConvertMode::RoundFloor => convert_scalar_round_floor,
        mir::TensorConvertMode::RoundCeil => convert_scalar_round_ceil,
        mir::TensorConvertMode::Saturate => convert_scalar_saturate,
    }
}

/// Execute tensor.select.
pub(crate) fn execute_tensor_select(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
    store_tensor_elements(
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
    )?;
    Ok(())
}

/// Execute tensor.cast.
pub(crate) fn execute_tensor_cast(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest_offset = instruction.a;
    let tensor_offset = instruction.b;
    let byte_len = instruction.c as u64 | ((instruction.d as u64) << 32);

    // forward the tensor bytes
    machine.copy_frame_bytes(tensor_offset, dest_offset, byte_len as usize);

    Ok(())
}

/// Execute tensor.view.
pub(crate) fn execute_tensor_view(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
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
        address,
    } = machine.side::<TensorView>(instruction);

    let source_layout = tensor_layout(machine, *source_layout);
    let dest_layout = tensor_layout(machine, *dest_layout);

    let args = frame_offsets(machine, *arguments);
    let (offset_values, rest) = args.split_at((*offsets_count).into());
    let (size_values, stride_values) = rest.split_at((*sizes_count).into());
    if stride_values.len() != usize::from(*strides_count) {
        return Err(Error::invalid_instruction());
    }

    let mut offsets = Vec::with_capacity(offset_values.len());
    let mut sizes = Vec::with_capacity(size_values.len());
    let mut strides = Vec::with_capacity(stride_values.len());
    for offset in offset_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        offsets.push(value);
    }
    for offset in size_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        sizes.push(value);
    }
    for offset in stride_values {
        let value = machine.load_word_at(*offset);
        let value = word_to_u64(value)?;
        strides.push(value);
    }

    for (expected, actual) in dest_layout.shape.iter().zip(sizes.iter()) {
        if *expected != *actual {
            return Err(Error::type_mismatch(
                "tensor.view size",
                format!("{actual} vs {expected}"),
            ));
        }
    }

    if offsets.len() != source_layout.shape.len()
        || sizes.len() != dest_layout.shape.len()
        || strides.len() != source_layout.shape.len()
        || dest_layout.shape.len() != source_layout.shape.len()
    {
        return Err(Error::type_mismatch(
            "tensor.view rank",
            format!(
                "offsets={}, sizes={}, strides={}, source_rank={}, dest_rank={}",
                offsets.len(),
                sizes.len(),
                strides.len(),
                source_layout.shape.len(),
                dest_layout.shape.len(),
            ),
        ));
    }

    let source_strides = load_tensor_view_strides(machine, *view_offset, source_layout)?;
    let mut dest_strides = Vec::with_capacity(strides.len());
    for (source_stride, stride) in source_strides.iter().copied().zip(strides.iter().copied()) {
        let dest_stride = source_stride
            .checked_mul(stride)
            .ok_or(Error::invalid_instruction())?;
        dest_strides.push(dest_stride);
    }

    if dest_strides.len() != dest_layout.shape.len() {
        return Err(Error::type_mismatch(
            "tensor.view stride rank",
            format!("{} vs {}", dest_strides.len(), dest_layout.shape.len()),
        ));
    }

    for axis in 0..source_layout.shape.len() {
        let offset = offsets[axis];
        let size = sizes[axis];
        let stride = strides[axis];
        let source_size = source_layout.shape[axis];

        if size == 0 {
            continue;
        }

        let last_step = size.checked_sub(1).ok_or(Error::invalid_instruction())?;
        let last_distance = last_step
            .checked_mul(stride)
            .ok_or(Error::invalid_instruction())?;
        let last_index = offset
            .checked_add(last_distance)
            .ok_or(Error::invalid_instruction())?;
        if last_index >= source_size {
            return Err(Error::index_out_of_bounds(last_index, source_size));
        }
    }

    let offset = tensor_linear_index(&offsets, &source_layout.shape, &source_strides)?;

    let view_value = load_tensor_view_pointer(machine, *view_offset);
    let element = machine.projection(*element);
    let source_span_len = tensor_element_span_len(&source_layout.shape, &source_strides)?;
    let pointer =
        offset_tensor_view_pointer(view_value, element, offset, source_span_len, *address)?;

    store_tensor_view_descriptor(machine, *dest_offset, pointer, &dest_strides)?;

    Ok(())
}
