use std::sync::atomic::{
    AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicU8, AtomicU16, AtomicU32, AtomicU64,
};

use destack_mir as mir;

use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    AtomicAddress, AtomicCompareExchange, AtomicOrder, AtomicReadModifyWriteOperator,
    AtomicReadModifyWriteShape, AtomicShape, AtomicWidth, Instruction,
};

macro_rules! atomic_ref {
    ($address:expr, $atomic:ty, $value:ty) => {{
        // lowered layouts guarantee atomic width and alignment
        unsafe { <$atomic>::from_ptr($address as *mut $value) }
    }};
}

/// Execute one atomic load.
pub(crate) fn execute_atomic_load(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode precomputed addressing and ordering
    let shape = AtomicShape::decode(instruction.d)?;
    let pointer = machine.load_word_at(instruction.b);
    let address = atomic_address(machine, pointer, shape.address)?;

    // load and publish the scalar result
    let raw = atomic_load(address, shape)?;
    let value = atomic_word(raw, shape);
    machine.store_word_at(instruction.a, value);

    Ok(())
}

/// Execute one atomic store.
pub(crate) fn execute_atomic_store(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode precomputed addressing and ordering
    let shape = AtomicShape::decode(instruction.d)?;
    let pointer = machine.load_word_at(instruction.a);
    let value = machine.load_word_at(instruction.b);
    let address = atomic_address(machine, pointer, shape.address)?;

    // store the scalar payload
    atomic_store(address, value.bits(), shape)?;

    Ok(())
}

/// Execute one atomic exchange.
pub(crate) fn execute_atomic_exchange(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode precomputed addressing and ordering
    let shape = AtomicShape::decode(instruction.d)?;
    let pointer = machine.load_word_at(instruction.b);
    let value = machine.load_word_at(instruction.c);
    let address = atomic_address(machine, pointer, shape.address)?;

    // exchange and publish the old scalar value
    let raw = atomic_exchange(address, value.bits(), shape)?;
    let value = atomic_word(raw, shape);
    machine.store_word_at(instruction.a, value);

    Ok(())
}

/// Execute one atomic compare exchange.
pub(crate) fn execute_atomic_compare_exchange(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode aggregate result metadata
    let compare_exchange = *machine.side::<AtomicCompareExchange>(instruction);
    let pointer = machine.load_word_at(compare_exchange.pointer_offset);
    let expected = machine.load_word_at(compare_exchange.expected_offset);
    let new_value = machine.load_word_at(compare_exchange.new_value_offset);
    let address = atomic_address(machine, pointer, compare_exchange.shape.address)?;

    // execute compare exchange and build the pair result
    let old = atomic_compare_exchange(
        address,
        expected.bits(),
        new_value.bits(),
        compare_exchange.shape,
        compare_exchange.failure_order,
        compare_exchange.is_weak,
    )?;
    let success = old == truncate(expected.bits(), compare_exchange.shape.width);
    let old = atomic_word(old, compare_exchange.shape);

    store_compare_exchange_result(machine, compare_exchange.destination, old, success)
}

/// Execute one atomic read-modify-write.
pub(crate) fn execute_atomic_read_modify_write(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // decode precomputed addressing, ordering, and update operation
    let shape = AtomicReadModifyWriteShape::decode(instruction.d)?;
    let pointer = machine.load_word_at(instruction.b);
    let value = machine.load_word_at(instruction.c);
    let address = atomic_address(machine, pointer, shape.shape.address)?;

    // update and publish the old scalar value
    let raw = atomic_read_modify_write(address, value, shape)?;
    let value = atomic_word(raw, shape.shape);
    machine.store_word_at(instruction.a, value);

    Ok(())
}

/// Execute one atomic fence.
pub(crate) fn execute_atomic_fence(
    _machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    // fences only carry ordering
    let order = AtomicOrder::decode(instruction.d)?;

    std::sync::atomic::fence(order.to_std_fence()?);

    Ok(())
}

/// Resolve an atomic pointer to a native address.
fn atomic_address(
    machine: &Machine<'_, '_>,
    pointer: Word,
    address: AtomicAddress,
) -> Result<usize, Error> {
    // offset zero is the reserved null reference
    if pointer.bits() == 0 {
        return Err(Error::NullPointerDereference);
    }

    // references carry heap offsets, raw pointers carry raw offsets
    let address = match address {
        AtomicAddress::Heap => machine.heap_address(pointer.as_heap_reference(), 0),
        AtomicAddress::SharedHeap => {
            machine.shared_heap_address(pointer.as_shared_heap_reference(), 0)
        }
        AtomicAddress::Raw => machine.raw_address(pointer.as_raw_pointer(), 0),
        AtomicAddress::SharedRaw => machine.shared_raw_address(pointer.as_shared_raw_pointer(), 0),
        AtomicAddress::Stack => pointer.as_stack_pointer().address(),
        AtomicAddress::Frame => pointer.as_frame_pointer().address(),
        AtomicAddress::Static => pointer.as_static_pointer().address(),
    };

    Ok(address)
}

/// Store the compare exchange pair result.
fn store_compare_exchange_result(
    machine: &mut Machine<'_, '_>,
    destination: mir::Value,
    value: Word,
    success: bool,
) -> Result<(), Error> {
    // write old value and success flag into the destination tuple
    super::frame::store_frame_fields(machine, destination, |_machine, index, _ty| match index {
        0 => Ok(value),
        1 => Ok(Word::bool(success)),
        _ => Err(Error::InvalidInstruction),
    })?;

    Ok(())
}

/// Convert raw atomic bits into one VM word.
#[inline(always)]
fn atomic_word(raw: u64, shape: AtomicShape) -> Word {
    if shape.is_signed && shape.width != AtomicWidth::Width64 {
        return Word::from_bits(sign_extend(raw, shape.width));
    }

    Word::from_bits(raw)
}

/// Sign-extend one atomic integer payload.
#[inline(always)]
fn sign_extend(raw: u64, width: AtomicWidth) -> u64 {
    let shift = u64::BITS as usize - width.byte_len() * 8;

    ((raw << shift) as i64 >> shift) as u64
}

/// Truncate one raw payload to an atomic width.
#[inline(always)]
fn truncate(raw: u64, width: AtomicWidth) -> u64 {
    match width {
        AtomicWidth::Width8 => raw as u8 as u64,
        AtomicWidth::Width16 => raw as u16 as u64,
        AtomicWidth::Width32 => raw as u32 as u64,
        AtomicWidth::Width64 => raw,
    }
}

/// Load one atomic payload.
#[inline(always)]
fn atomic_load(address: usize, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std_load()?;

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => atomic_ref!(address, AtomicU8, u8).load(order) as u64,
        AtomicWidth::Width16 => atomic_ref!(address, AtomicU16, u16).load(order) as u64,
        AtomicWidth::Width32 => atomic_ref!(address, AtomicU32, u32).load(order) as u64,
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).load(order),
    };

    Ok(value)
}

/// Store one atomic payload.
#[inline(always)]
fn atomic_store(address: usize, raw: u64, shape: AtomicShape) -> Result<(), Error> {
    let order = shape.order.to_std_store()?;

    // compiled layouts guarantee atomic width and alignment
    match shape.width {
        AtomicWidth::Width8 => atomic_ref!(address, AtomicU8, u8).store(raw as u8, order),
        AtomicWidth::Width16 => atomic_ref!(address, AtomicU16, u16).store(raw as u16, order),
        AtomicWidth::Width32 => atomic_ref!(address, AtomicU32, u32).store(raw as u32, order),
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).store(raw, order),
    }

    Ok(())
}

/// Exchange one atomic payload.
#[inline(always)]
fn atomic_exchange(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => atomic_ref!(address, AtomicU8, u8).swap(raw as u8, order) as u64,
        AtomicWidth::Width16 => atomic_ref!(address, AtomicU16, u16).swap(raw as u16, order) as u64,
        AtomicWidth::Width32 => atomic_ref!(address, AtomicU32, u32).swap(raw as u32, order) as u64,
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).swap(raw, order),
    };

    Ok(value)
}

/// Compare and exchange one atomic payload.
#[inline(always)]
fn atomic_compare_exchange(
    address: usize,
    expected: u64,
    new_value: u64,
    shape: AtomicShape,
    failure_order: AtomicOrder,
    is_weak: bool,
) -> Result<u64, Error> {
    let success = shape.order.to_std();
    let failure = failure_order.to_std_compare_exchange_failure()?;

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 if is_weak => match atomic_ref!(address, AtomicU8, u8)
            .compare_exchange_weak(expected as u8, new_value as u8, success, failure)
        {
            Ok(old) | Err(old) => old as u64,
        },
        AtomicWidth::Width8 => match atomic_ref!(address, AtomicU8, u8).compare_exchange(
            expected as u8,
            new_value as u8,
            success,
            failure,
        ) {
            Ok(old) | Err(old) => old as u64,
        },
        AtomicWidth::Width16 if is_weak => match atomic_ref!(address, AtomicU16, u16)
            .compare_exchange_weak(expected as u16, new_value as u16, success, failure)
        {
            Ok(old) | Err(old) => old as u64,
        },
        AtomicWidth::Width16 => match atomic_ref!(address, AtomicU16, u16).compare_exchange(
            expected as u16,
            new_value as u16,
            success,
            failure,
        ) {
            Ok(old) | Err(old) => old as u64,
        },
        AtomicWidth::Width32 if is_weak => match atomic_ref!(address, AtomicU32, u32)
            .compare_exchange_weak(expected as u32, new_value as u32, success, failure)
        {
            Ok(old) | Err(old) => old as u64,
        },
        AtomicWidth::Width32 => match atomic_ref!(address, AtomicU32, u32).compare_exchange(
            expected as u32,
            new_value as u32,
            success,
            failure,
        ) {
            Ok(old) | Err(old) => old as u64,
        },
        AtomicWidth::Width64 if is_weak => match atomic_ref!(address, AtomicU64, u64)
            .compare_exchange_weak(expected, new_value, success, failure)
        {
            Ok(old) | Err(old) => old,
        },
        AtomicWidth::Width64 => match atomic_ref!(address, AtomicU64, u64)
            .compare_exchange(expected, new_value, success, failure)
        {
            Ok(old) | Err(old) => old,
        },
    };

    Ok(value)
}

/// Execute one atomic read-modify-write payload.
#[inline(always)]
fn atomic_read_modify_write(
    address: usize,
    value: Word,
    shape: AtomicReadModifyWriteShape,
) -> Result<u64, Error> {
    use AtomicReadModifyWriteOperator as Operator;

    match shape.operator {
        Operator::Add => atomic_fetch_add(address, value.bits(), shape.shape),
        Operator::Sub => atomic_fetch_sub(address, value.bits(), shape.shape),
        Operator::And => atomic_fetch_and(address, value.bits(), shape.shape),
        Operator::Or => atomic_fetch_or(address, value.bits(), shape.shape),
        Operator::Xor => atomic_fetch_xor(address, value.bits(), shape.shape),
        Operator::Min => atomic_fetch_min_signed(address, value.bits(), shape.shape),
        Operator::Max => atomic_fetch_max_signed(address, value.bits(), shape.shape),
        Operator::Umin => atomic_fetch_min(address, value.bits(), shape.shape),
        Operator::Umax => atomic_fetch_max(address, value.bits(), shape.shape),
        Operator::Fadd => {
            atomic_float_update(address, value, shape.shape, |left, right| left + right)
        }
        Operator::Fmin => atomic_float_update(address, value, shape.shape, f64::min),
        Operator::Fmax => atomic_float_update(address, value, shape.shape, f64::max),
    }
}

/// Add one atomic integer payload.
#[inline(always)]
fn atomic_fetch_add(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => {
            atomic_ref!(address, AtomicU8, u8).fetch_add(raw as u8, order) as u64
        }
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicU16, u16).fetch_add(raw as u16, order) as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicU32, u32).fetch_add(raw as u32, order) as u64
        }
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).fetch_add(raw, order),
    };

    Ok(value)
}

/// Subtract one atomic integer payload.
#[inline(always)]
fn atomic_fetch_sub(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => {
            atomic_ref!(address, AtomicU8, u8).fetch_sub(raw as u8, order) as u64
        }
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicU16, u16).fetch_sub(raw as u16, order) as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicU32, u32).fetch_sub(raw as u32, order) as u64
        }
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).fetch_sub(raw, order),
    };

    Ok(value)
}

/// And one atomic integer payload.
#[inline(always)]
fn atomic_fetch_and(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => {
            atomic_ref!(address, AtomicU8, u8).fetch_and(raw as u8, order) as u64
        }
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicU16, u16).fetch_and(raw as u16, order) as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicU32, u32).fetch_and(raw as u32, order) as u64
        }
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).fetch_and(raw, order),
    };

    Ok(value)
}

/// Or one atomic integer payload.
#[inline(always)]
fn atomic_fetch_or(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => atomic_ref!(address, AtomicU8, u8).fetch_or(raw as u8, order) as u64,
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicU16, u16).fetch_or(raw as u16, order) as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicU32, u32).fetch_or(raw as u32, order) as u64
        }
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).fetch_or(raw, order),
    };

    Ok(value)
}

/// Xor one atomic integer payload.
#[inline(always)]
fn atomic_fetch_xor(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => {
            atomic_ref!(address, AtomicU8, u8).fetch_xor(raw as u8, order) as u64
        }
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicU16, u16).fetch_xor(raw as u16, order) as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicU32, u32).fetch_xor(raw as u32, order) as u64
        }
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).fetch_xor(raw, order),
    };

    Ok(value)
}

/// Min one atomic unsigned integer payload.
#[inline(always)]
fn atomic_fetch_min(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => {
            atomic_ref!(address, AtomicU8, u8).fetch_min(raw as u8, order) as u64
        }
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicU16, u16).fetch_min(raw as u16, order) as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicU32, u32).fetch_min(raw as u32, order) as u64
        }
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).fetch_min(raw, order),
    };

    Ok(value)
}

/// Max one atomic unsigned integer payload.
#[inline(always)]
fn atomic_fetch_max(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => {
            atomic_ref!(address, AtomicU8, u8).fetch_max(raw as u8, order) as u64
        }
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicU16, u16).fetch_max(raw as u16, order) as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicU32, u32).fetch_max(raw as u32, order) as u64
        }
        AtomicWidth::Width64 => atomic_ref!(address, AtomicU64, u64).fetch_max(raw, order),
    };

    Ok(value)
}

/// Min one atomic signed integer payload.
#[inline(always)]
fn atomic_fetch_min_signed(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => {
            atomic_ref!(address, AtomicI8, i8).fetch_min(raw as i8, order) as u8 as u64
        }
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicI16, i16).fetch_min(raw as i16, order) as u16 as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicI32, i32).fetch_min(raw as i32, order) as u32 as u64
        }
        AtomicWidth::Width64 => {
            atomic_ref!(address, AtomicI64, i64).fetch_min(raw as i64, order) as u64
        }
    };

    Ok(value)
}

/// Max one atomic signed integer payload.
#[inline(always)]
fn atomic_fetch_max_signed(address: usize, raw: u64, shape: AtomicShape) -> Result<u64, Error> {
    let order = shape.order.to_std();

    // compiled layouts guarantee atomic width and alignment
    let value = match shape.width {
        AtomicWidth::Width8 => {
            atomic_ref!(address, AtomicI8, i8).fetch_max(raw as i8, order) as u8 as u64
        }
        AtomicWidth::Width16 => {
            atomic_ref!(address, AtomicI16, i16).fetch_max(raw as i16, order) as u16 as u64
        }
        AtomicWidth::Width32 => {
            atomic_ref!(address, AtomicI32, i32).fetch_max(raw as i32, order) as u32 as u64
        }
        AtomicWidth::Width64 => {
            atomic_ref!(address, AtomicI64, i64).fetch_max(raw as i64, order) as u64
        }
    };

    Ok(value)
}

/// Execute one floating atomic update with a CAS loop.
fn atomic_float_update<F>(
    address: usize,
    value: Word,
    shape: AtomicShape,
    operation: F,
) -> Result<u64, Error>
where
    F: Fn(f64, f64) -> f64,
{
    match shape.width {
        AtomicWidth::Width32 => {
            let raw = atomic_update_u32(address, shape, |old| {
                let old = f32::from_bits(old);
                let value = value.as_float32();

                (operation(old as f64, value as f64) as f32).to_bits()
            })?;

            Ok(raw as u64)
        }
        AtomicWidth::Width64 => atomic_update_u64(address, shape, |old| {
            let old = f64::from_bits(old);
            let value = value.as_float64();

            operation(old, value).to_bits()
        }),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Update one atomic 32-bit payload with a CAS loop.
fn atomic_update_u32<F>(address: usize, shape: AtomicShape, operation: F) -> Result<u32, Error>
where
    F: Fn(u32) -> u32,
{
    let success = shape.order.to_std();
    let failure = shape.order.to_std_update_failure();

    // compiled layouts guarantee atomic width and alignment
    let atomic = atomic_ref!(address, AtomicU32, u32);
    let mut old = atomic.load(failure);

    // retry until the weak compare exchange accepts the computed update
    loop {
        let new_value = operation(old);

        match atomic.compare_exchange_weak(old, new_value, success, failure) {
            Ok(value) => return Ok(value),
            Err(value) => old = value,
        }
    }
}

/// Update one atomic 64-bit payload with a CAS loop.
fn atomic_update_u64<F>(address: usize, shape: AtomicShape, operation: F) -> Result<u64, Error>
where
    F: Fn(u64) -> u64,
{
    let success = shape.order.to_std();
    let failure = shape.order.to_std_update_failure();

    // compiled layouts guarantee atomic width and alignment
    let atomic = atomic_ref!(address, AtomicU64, u64);
    let mut old = atomic.load(failure);

    // retry until the weak compare exchange accepts the computed update
    loop {
        let new_value = operation(old);

        match atomic.compare_exchange_weak(old, new_value, success, failure) {
            Ok(value) => return Ok(value),
            Err(value) => old = value,
        }
    }
}
