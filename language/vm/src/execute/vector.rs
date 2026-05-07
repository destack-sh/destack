#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::{vaddq_u32, vld1q_u32, vst1q_u32};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m128i, _mm_add_epi32, _mm_loadu_si128, _mm_storeu_si128};

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
    ElementBinaryKernel, ElementUnaryKernel, Instruction, Projection, ScalarLayout, VectorBinary,
    VectorConvert, VectorExtract, VectorInsert, VectorReduce, VectorSelect, VectorShuffle,
    VectorSplat, VectorUnary,
};
use destack_mir as mir;

macro_rules! packed_binary_executor {
    ($function:ident, $ty:ty, $count:literal, $operation:expr, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Result<(), Error> {
            execute_vector_binary_packed::<$ty, $count, _>(machine, instruction, $operation)
        }
    };
}

macro_rules! packed_unary_executor {
    ($function:ident, $ty:ty, $count:literal, $operation:expr, $doc:literal) => {
        #[doc = $doc]
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Result<(), Error> {
            execute_vector_unary_packed::<$ty, $count, _>(machine, instruction, $operation)
        }
    };
}

/// Read one packed frame vector.
#[inline(always)]
fn read_packed<T: Copy, const N: usize>(machine: &Machine<'_, '_>, offset: u32) -> [T; N] {
    let pointer = machine.frame_pointer_at(offset).address() as *const [T; N];

    unsafe { std::ptr::read(pointer) }
}

/// Write one packed frame vector.
#[inline(always)]
fn write_packed<T, const N: usize>(machine: &mut Machine<'_, '_>, offset: u32, value: [T; N]) {
    let pointer = machine.frame_pointer_at(offset).address() as *mut [T; N];

    unsafe {
        std::ptr::write(pointer, value);
    }
}

/// Store one 32-bit vector add result.
#[inline(always)]
fn store_add_u32x4(dest: *mut u32, left: *const u32, right: *const u32) {
    unsafe {
        store_add_u32x4_unchecked(dest, left, right);
    }
}

/// Store one 32-bit vector add result on AArch64.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn store_add_u32x4_unchecked(dest: *mut u32, left: *const u32, right: *const u32) {
    // load both packed operands directly from the frame
    let left = unsafe { vld1q_u32(left) };
    let right = unsafe { vld1q_u32(right) };

    // add and write one 128-bit result
    let result = unsafe { vaddq_u32(left, right) };
    unsafe {
        vst1q_u32(dest, result);
    }
}

/// Store one 32-bit vector add result on x86-64.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn store_add_u32x4_unchecked(dest: *mut u32, left: *const u32, right: *const u32) {
    // load both packed operands directly from the frame
    let left = unsafe { _mm_loadu_si128(left.cast::<__m128i>()) };
    let right = unsafe { _mm_loadu_si128(right.cast::<__m128i>()) };

    // add and write one 128-bit result
    let result = unsafe { _mm_add_epi32(left, right) };
    unsafe {
        _mm_storeu_si128(dest.cast::<__m128i>(), result);
    }
}

/// Store one 32-bit vector add result on scalar targets.
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
#[inline(always)]
unsafe fn store_add_u32x4_unchecked(dest: *mut u32, left: *const u32, right: *const u32) {
    // preserve the same element semantics without target SIMD
    for index in 0..4 {
        let left = unsafe { left.add(index).read() };
        let right = unsafe { right.add(index).read() };
        unsafe {
            dest.add(index).write(left.wrapping_add(right));
        }
    }
}

/// Execute one packed vector binary operation.
#[inline(always)]
fn execute_vector_binary_packed<T: Copy, const N: usize, F>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: F,
) -> Result<(), Error>
where
    F: Fn(T, T) -> T,
{
    // read both packed operands from frame bytes
    let left = read_packed::<T, N>(machine, instruction.b);
    let right = read_packed::<T, N>(machine, instruction.c);

    // execute the element kernel in register storage
    let result: [T; N] = std::array::from_fn(|i| operation(left[i], right[i]));

    write_packed(machine, instruction.a, result);

    Ok(())
}

/// Execute one packed vector unary operation.
#[inline(always)]
fn execute_vector_unary_packed<T: Copy, const N: usize, F>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: F,
) -> Result<(), Error>
where
    F: Fn(T) -> T,
{
    // read the packed operand from frame bytes
    let value = read_packed::<T, N>(machine, instruction.b);

    // execute the element kernel in register storage
    let result: [T; N] = std::array::from_fn(|i| operation(value[i]));

    write_packed(machine, instruction.a, result);

    Ok(())
}

/// Load one vector element through a precomputed element projection.
#[inline(always)]
fn load_vector_element(
    machine: &mut Machine<'_, '_>,
    vector_offset: u32,
    element: Projection,
    element_index: usize,
) -> Result<Word, Error> {
    // compute the exact element address
    let element_offset = element.byte_stride * element_index;
    let pointer = machine
        .frame_pointer_at(vector_offset)
        .add_bytes(element_offset);

    Ok(access::load_frame_scalar_by_layout(
        machine, pointer, element,
    ))
}

/// Store one vector result into frame bytes.
fn store_vector_elements<F>(
    machine: &mut Machine<'_, '_>,
    dest_offset: u32,
    dest_element: Projection,
    element_count: u32,
    mut element_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut Machine<'_, '_>, usize) -> Result<Word, Error>,
{
    // write each result element by lowered frame layout
    for element_index in 0..element_count as usize {
        let value = element_value(machine, element_index)?;
        let element_offset = dest_element.byte_stride * element_index;
        let pointer = machine
            .frame_pointer_at(dest_offset)
            .add_bytes(element_offset);

        access::store_frame_scalar_by_layout(machine, pointer, dest_element, value);
    }

    Ok(())
}

/// Execute one vector binary element loop.
fn execute_vector_binary_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
    let VectorBinary {
        dest_offset,
        left_offset,
        right_offset,
        dest_element,
        left_element,
        right_element,
        kernel: _,
        element_layout,
        element_count,
    } = machine.side::<VectorBinary>(instruction);

    // execute the scalar operation on each vector element
    store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |machine, element_index| {
            let left = load_vector_element(machine, *left_offset, *left_element, element_index)?;
            let right = load_vector_element(machine, *right_offset, *right_element, element_index)?;

            operation(*element_layout, left, right)
        },
    )?;

    Ok(())
}

/// Execute a vector binary operation.
pub(crate) fn execute_vector_binary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let kernel = machine.side::<VectorBinary>(instruction).kernel;
    let operation = vector_binary_operation(kernel);

    execute_vector_binary_elements(machine, instruction, operation)
}

/// Return the scalar operation for one vector binary kernel.
fn vector_binary_operation(
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

/// Execute one vector unary element loop.
fn execute_vector_unary_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, Word) -> Result<Word, Error>,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
    let VectorUnary {
        dest_offset,
        argument_offset,
        dest_element,
        argument_element,
        kernel: _,
        element_layout,
        element_count,
    } = machine.side::<VectorUnary>(instruction);

    // execute the scalar operation on each vector element
    store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |machine, element_index| {
            let value =
                load_vector_element(machine, *argument_offset, *argument_element, element_index)?;

            operation(*element_layout, value)
        },
    )?;

    Ok(())
}

/// Execute a vector unary operation.
pub(crate) fn execute_vector_unary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let kernel = machine.side::<VectorUnary>(instruction).kernel;
    let operation = vector_unary_operation(kernel);

    execute_vector_unary_elements(machine, instruction, operation)
}

/// Return the scalar operation for one vector unary kernel.
fn vector_unary_operation(
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

/// Execute packed 32-bit element add.
pub(crate) fn execute_packed_add_32x4(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // compute frame addresses for the SIMD kernel
    let dest = machine.frame_pointer_at(instruction.a).address() as *mut u32;
    let left = machine.frame_pointer_at(instruction.b).address() as *const u32;
    let right = machine.frame_pointer_at(instruction.c).address() as *const u32;

    store_add_u32x4(dest, left, right);

    Ok(())
}

packed_binary_executor!(
    execute_packed_sub_32x4,
    u32,
    4,
    u32::wrapping_sub,
    "Execute packed 32-bit element subtract."
);
packed_binary_executor!(
    execute_packed_mul_32x4,
    u32,
    4,
    u32::wrapping_mul,
    "Execute packed 32-bit element multiply."
);
packed_binary_executor!(
    execute_packed_and_32x4,
    u32,
    4,
    |left, right| left & right,
    "Execute packed 32-bit element and."
);
packed_binary_executor!(
    execute_packed_or_32x4,
    u32,
    4,
    |left, right| left | right,
    "Execute packed 32-bit element or."
);
packed_binary_executor!(
    execute_packed_xor_32x4,
    u32,
    4,
    |left, right| left ^ right,
    "Execute packed 32-bit element xor."
);
packed_binary_executor!(
    execute_packed_shl_32x4,
    u32,
    4,
    |left: u32, right: u32| left.wrapping_shl(right),
    "Execute packed 32-bit element shift left."
);
packed_binary_executor!(
    execute_packed_shr_i32x4,
    i32,
    4,
    |left: i32, right: i32| left.wrapping_shr(right as u32),
    "Execute packed signed 32-bit element shift right."
);
packed_binary_executor!(
    execute_packed_shr_u32x4,
    u32,
    4,
    |left: u32, right: u32| left.wrapping_shr(right),
    "Execute packed unsigned 32-bit element shift right."
);
packed_binary_executor!(
    execute_packed_add_64x2,
    u64,
    2,
    u64::wrapping_add,
    "Execute packed 64-bit element add."
);
packed_binary_executor!(
    execute_packed_sub_64x2,
    u64,
    2,
    u64::wrapping_sub,
    "Execute packed 64-bit element subtract."
);
packed_binary_executor!(
    execute_packed_mul_64x2,
    u64,
    2,
    u64::wrapping_mul,
    "Execute packed 64-bit element multiply."
);
packed_binary_executor!(
    execute_packed_and_64x2,
    u64,
    2,
    |left, right| left & right,
    "Execute packed 64-bit element and."
);
packed_binary_executor!(
    execute_packed_or_64x2,
    u64,
    2,
    |left, right| left | right,
    "Execute packed 64-bit element or."
);
packed_binary_executor!(
    execute_packed_xor_64x2,
    u64,
    2,
    |left, right| left ^ right,
    "Execute packed 64-bit element xor."
);
packed_binary_executor!(
    execute_packed_shl_64x2,
    u64,
    2,
    |left: u64, right: u64| left.wrapping_shl(right as u32),
    "Execute packed 64-bit element shift left."
);
packed_binary_executor!(
    execute_packed_shr_i64x2,
    i64,
    2,
    |left: i64, right: i64| left.wrapping_shr(right as u32),
    "Execute packed signed 64-bit element shift right."
);
packed_binary_executor!(
    execute_packed_shr_u64x2,
    u64,
    2,
    |left: u64, right: u64| left.wrapping_shr(right as u32),
    "Execute packed unsigned 64-bit element shift right."
);
packed_binary_executor!(
    execute_packed_add_f32x4,
    f32,
    4,
    |left, right| left + right,
    "Execute packed float32 element add."
);
packed_binary_executor!(
    execute_packed_sub_f32x4,
    f32,
    4,
    |left, right| left - right,
    "Execute packed float32 element subtract."
);
packed_binary_executor!(
    execute_packed_mul_f32x4,
    f32,
    4,
    |left, right| left * right,
    "Execute packed float32 element multiply."
);
packed_binary_executor!(
    execute_packed_div_f32x4,
    f32,
    4,
    |left, right| left / right,
    "Execute packed float32 element divide."
);
packed_binary_executor!(
    execute_packed_add_f64x2,
    f64,
    2,
    |left, right| left + right,
    "Execute packed float64 element add."
);
packed_binary_executor!(
    execute_packed_sub_f64x2,
    f64,
    2,
    |left, right| left - right,
    "Execute packed float64 element subtract."
);
packed_binary_executor!(
    execute_packed_mul_f64x2,
    f64,
    2,
    |left, right| left * right,
    "Execute packed float64 element multiply."
);
packed_binary_executor!(
    execute_packed_div_f64x2,
    f64,
    2,
    |left, right| left / right,
    "Execute packed float64 element divide."
);
packed_unary_executor!(
    execute_packed_neg_i32x4,
    i32,
    4,
    i32::wrapping_neg,
    "Execute packed signed 32-bit element negation."
);
packed_unary_executor!(
    execute_packed_not_32x4,
    u32,
    4,
    |value| !value,
    "Execute packed 32-bit element inversion."
);
packed_unary_executor!(
    execute_packed_neg_i64x2,
    i64,
    2,
    i64::wrapping_neg,
    "Execute packed signed 64-bit element negation."
);
packed_unary_executor!(
    execute_packed_not_64x2,
    u64,
    2,
    |value| !value,
    "Execute packed 64-bit element inversion."
);
packed_unary_executor!(
    execute_packed_neg_f32x4,
    f32,
    4,
    |value| -value,
    "Execute packed float32 element negation."
);
packed_unary_executor!(
    execute_packed_neg_f64x2,
    f64,
    2,
    |value| -value,
    "Execute packed float64 element negation."
);

/// Execute packed 32-bit splat.
pub(crate) fn execute_packed_splat_32x4(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // broadcast one word-sized scalar into packed frame bytes
    let value = machine.load_word_at(instruction.b).bits() as u32;

    write_packed(machine, instruction.a, [value; 4]);

    Ok(())
}

/// Execute packed 64-bit splat.
pub(crate) fn execute_packed_splat_64x2(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // broadcast one word-sized scalar into packed frame bytes
    let value = machine.load_word_at(instruction.b).bits();

    write_packed(machine, instruction.a, [value; 2]);

    Ok(())
}

/// Execute vector.splat.
pub(crate) fn execute_vector_splat(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
    let VectorSplat {
        dest_offset,
        value_offset,
        dest_element,
        element_count,
    } = machine.side::<VectorSplat>(instruction);

    let element_value = machine.load_word_at(*value_offset);

    // store the same value into each element
    store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |_machine, _element_index| Ok(element_value),
    )?;

    Ok(())
}

/// Execute vector.extract.
pub(crate) fn execute_vector_extract(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
    let VectorExtract {
        dest_offset,
        vector_offset,
        index_offset,
        vector_element,
        element_count,
    } = machine.side::<VectorExtract>(instruction);

    // resolve and validate the dynamic element index
    let index_value = word_to_usize(machine.load_word_at(*index_offset))?;
    let element_count = *element_count as usize;
    if index_value >= element_count {
        return Err(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: element_count as u64,
        });
    }

    // load the selected element into the destination word
    let result = load_vector_element(machine, *vector_offset, *vector_element, index_value)?;
    machine.store_word_at(*dest_offset, result);

    Ok(())
}

/// Execute vector.insert.
pub(crate) fn execute_vector_insert(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
    let VectorInsert {
        dest_offset,
        vector_offset,
        index_offset,
        value_offset,
        dest_element,
        vector_element,
        element_count,
    } = machine.side::<VectorInsert>(instruction);

    // resolve and validate the dynamic element index
    let index_value = word_to_usize(machine.load_word_at(*index_offset))?;
    let element_count = *element_count;
    let element_count_usize = element_count as usize;

    // reject out of bounds element indices
    if index_value >= element_count_usize {
        return Err(Error::IndexOutOfBounds {
            index: index_value as u64,
            length: element_count as u64,
        });
    }

    // read the inserted scalar once
    let inserted_value = machine.load_word_at(*value_offset);

    // write the updated vector one element at a time
    store_vector_elements(
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
    )?;

    Ok(())
}

/// Execute vector.shuffle.
pub(crate) fn execute_vector_shuffle(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
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

    // resolve source ranges
    let left_count = *left_count as usize;
    let right_count = *right_count as usize;

    // write the shuffled elements directly
    store_vector_elements(
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
    )?;

    Ok(())
}

/// Execute vector.select.
pub(crate) fn execute_vector_select(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
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
    store_vector_elements(
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
    )?;

    Ok(())
}

/// Execute one vector reduction loop.
fn execute_vector_reduce_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, Word, Word) -> Result<Word, Error>,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
    let VectorReduce {
        dest_offset,
        vector_offset,
        kernel: _,
        vector_element,
        element_layout,
        element_count,
    } = machine.side::<VectorReduce>(instruction);

    // reject empty reductions
    let element_count = *element_count as usize;
    if element_count == 0 {
        return Err(Error::InvalidInstruction);
    }

    // fold elements from left to right
    let mut result = load_vector_element(machine, *vector_offset, *vector_element, 0)?;
    for element_index in 1..element_count {
        let value = load_vector_element(machine, *vector_offset, *vector_element, element_index)?;
        match operation(*element_layout, result, value) {
            Ok(value) => result = value,
            Err(error) => return Err(error),
        }
    }

    machine.store_word_at(*dest_offset, result);

    Ok(())
}

/// Execute vector.reduce.
pub(crate) fn execute_vector_reduce(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let kernel = machine.side::<VectorReduce>(instruction).kernel;

    match kernel {
        mir::VectorReduceOperator::Add => {
            execute_vector_reduce_elements(machine, instruction, reduce_add)
        }
        mir::VectorReduceOperator::Multiply => {
            execute_vector_reduce_elements(machine, instruction, reduce_multiply)
        }
        mir::VectorReduceOperator::Min => {
            execute_vector_reduce_elements(machine, instruction, reduce_min)
        }
        mir::VectorReduceOperator::Max => {
            execute_vector_reduce_elements(machine, instruction, reduce_max)
        }
        mir::VectorReduceOperator::And => {
            execute_vector_reduce_elements(machine, instruction, reduce_and)
        }
        mir::VectorReduceOperator::Or => {
            execute_vector_reduce_elements(machine, instruction, reduce_or)
        }
        mir::VectorReduceOperator::Xor => {
            execute_vector_reduce_elements(machine, instruction, reduce_xor)
        }
    }
}

/// Execute one vector conversion loop.
fn execute_vector_convert_elements(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    convert: fn(Word, ScalarLayout, ScalarLayout) -> Result<Word, Error>,
) -> Result<(), Error> {
    // decode the precomputed vector descriptor
    let VectorConvert {
        dest_offset,
        vector_offset,
        mode: _,
        dest_element,
        source_element,
        dest_layout,
        source_layout,
        element_count,
    } = machine.side::<VectorConvert>(instruction);

    // convert the elements one by one
    store_vector_elements(
        machine,
        *dest_offset,
        *dest_element,
        *element_count,
        |machine, element_index| {
            let value =
                load_vector_element(machine, *vector_offset, *source_element, element_index)?;

            convert(value, *source_layout, *dest_layout)
        },
    )?;

    Ok(())
}

/// Execute vector.convert.
pub(crate) fn execute_vector_convert(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let mode = machine.side::<VectorConvert>(instruction).mode;

    match mode {
        mir::VectorConvertMode::Exact => {
            execute_vector_convert_elements(machine, instruction, convert_scalar_exact)
        }
        mir::VectorConvertMode::RoundTiesEven => {
            execute_vector_convert_elements(machine, instruction, convert_scalar_round_ties_even)
        }
        mir::VectorConvertMode::RoundTowardZero => {
            execute_vector_convert_elements(machine, instruction, convert_scalar_round_toward_zero)
        }
        mir::VectorConvertMode::RoundFloor => {
            execute_vector_convert_elements(machine, instruction, convert_scalar_round_floor)
        }
        mir::VectorConvertMode::RoundCeil => {
            execute_vector_convert_elements(machine, instruction, convert_scalar_round_ceil)
        }
        mir::VectorConvertMode::Saturate => {
            execute_vector_convert_elements(machine, instruction, convert_scalar_saturate)
        }
    }
}
