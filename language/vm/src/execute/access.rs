use std::{ptr, slice};

use crate::diagnostic::Error;
use crate::program::{PointeeAccess, WordLayout};
use crate::{FramePointer, HeapReference, SharedHeapReference, StackPointer, StaticPointer, Word};
use destack_heap::HeapError;

use crate::interpreter::Machine;

const POINTER_BYTE_LEN: usize = usize::BITS as usize / 8;

/// Build one invalid-pointer-type error for the given value.
#[inline(always)]
pub(super) fn invalid_pointer_type(value: Word) -> Error {
    Error::InvalidPointerType {
        actual: format!("{value:?}"),
    }
}

/// Debug assert one lowered scalar access.
#[inline(always)]
fn debug_assert_word_access(access: PointeeAccess) {
    debug_assert!(access.is_word());
    debug_assert!(access.byte_len <= Word::BYTE_LEN);
}

/// Return one lowered scalar layout.
#[inline(always)]
fn scalar_layout(access: PointeeAccess) -> WordLayout {
    debug_assert!(access.word_layout.is_some());
    unsafe { access.word_layout.unwrap_unchecked() }
}

/// Return one local heap native address.
#[inline(always)]
fn local_heap_address(
    machine: &Machine<'_, '_>,
    reference: HeapReference,
    byte_offset: usize,
) -> Result<usize, HeapError> {
    if machine.null_checks && reference.is_null() {
        return Err(HeapError::InvalidHeapReference { reference });
    }

    Ok(machine.heap().heap_base_address() + reference.offset() + byte_offset)
}

/// Return one shared heap native address.
#[inline(always)]
fn shared_heap_address(
    machine: &mut Machine<'_, '_>,
    reference: SharedHeapReference,
    byte_offset: usize,
) -> Result<usize, HeapError> {
    if machine.null_checks && reference.is_null() {
        return Err(HeapError::InvalidSharedHeapReference { reference });
    }

    Ok(machine.shared().heap_base_address() + reference.offset() + byte_offset)
}

/// Load bytes from one native address.
#[inline(always)]
fn load_native_bytes(address: usize, destination: *mut u8, destination_len: usize) {
    unsafe {
        ptr::copy_nonoverlapping(address as *const u8, destination, destination_len);
    }
}

/// Sign extend one loaded scalar.
#[inline(always)]
fn sign_extend_scalar(raw: u64, byte_len: usize) -> u64 {
    let shift = u64::BITS as usize - byte_len * 8;

    ((raw << shift) as i64 >> shift) as u64
}

/// Load unsigned scalar bits from one address.
#[inline(always)]
fn load_unsigned_bits(address: usize, byte_len: usize) -> u64 {
    let source = address as *const u8;

    unsafe {
        match byte_len {
            0 => 0,
            1 => source.read() as u64,
            2 => u16::from_le(source.cast::<u16>().read_unaligned()) as u64,
            4 => u32::from_le(source.cast::<u32>().read_unaligned()) as u64,
            8 => u64::from_le(source.cast::<u64>().read_unaligned()),
            _ => load_bytewise_unsigned_bits(source, byte_len),
        }
    }
}

/// Load one lowered scalar from one native address.
#[inline(always)]
pub(super) fn load_scalar_at_address<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    address: usize,
) -> Word {
    let raw = load_unsigned_bits(address, BYTE_LEN);
    let raw = if IS_SIGNED && BYTE_LEN < Word::BYTE_LEN {
        sign_extend_scalar(raw, BYTE_LEN)
    } else {
        raw
    };

    Word::from_bits(raw)
}

/// Store one lowered scalar to one native address.
#[inline(always)]
pub(super) fn store_scalar_at_address<const BYTE_LEN: usize>(address: usize, value: Word) {
    store_unsigned_bits(address, value.bits(), BYTE_LEN);
}

/// Load non-native-width unsigned bits from one address.
#[cold]
#[inline(never)]
unsafe fn load_bytewise_unsigned_bits(source: *const u8, byte_len: usize) -> u64 {
    let mut raw = 0u64;

    for index in 0..byte_len {
        raw |= unsafe { source.add(index).read() as u64 } << (index * 8);
    }

    raw
}

/// Store unsigned scalar bits to one address.
#[inline(always)]
fn store_unsigned_bits(address: usize, raw: u64, byte_len: usize) {
    let destination = address as *mut u8;

    unsafe {
        match byte_len {
            0 => {}
            1 => destination.write(raw as u8),
            2 => destination
                .cast::<u16>()
                .write_unaligned((raw as u16).to_le()),
            4 => destination
                .cast::<u32>()
                .write_unaligned((raw as u32).to_le()),
            8 => destination.cast::<u64>().write_unaligned(raw.to_le()),
            _ => store_bytewise_unsigned_bits(destination, raw, byte_len),
        }
    }
}

/// Store non-native-width unsigned bits to one address.
#[cold]
#[inline(never)]
unsafe fn store_bytewise_unsigned_bits(destination: *mut u8, raw: u64, byte_len: usize) {
    for index in 0..byte_len {
        unsafe {
            destination
                .add(index)
                .write(((raw >> (index * 8)) & 0xFF) as u8);
        }
    }
}

/// Load one typed scalar from one native address.
#[inline(always)]
pub(super) fn load_scalar_by_layout_at_address(address: usize, layout: WordLayout) -> Word {
    let raw = load_unsigned_bits(address, layout.byte_len(POINTER_BYTE_LEN));

    layout.decode(raw)
}

/// Store one typed scalar to one native address.
#[inline(always)]
pub(super) fn store_scalar_by_layout_at_address(address: usize, layout: WordLayout, value: Word) {
    let raw = layout.encode(value);

    store_unsigned_bits(address, raw, layout.byte_len(POINTER_BYTE_LEN));
}

/// Load one scalar from local raw heap bytes.
pub(crate) fn load_raw_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    debug_assert_word_access(access);

    let pointer = pointer.as_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = machine
        .heap()
        .raw_address(pointer, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;

    Ok(load_scalar_by_layout_at_address(
        address as usize,
        scalar_layout(access),
    ))
}

/// Load one scalar from shared raw heap bytes.
pub(crate) fn load_shared_raw_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    debug_assert_word_access(access);

    let pointer = pointer.as_shared_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = machine
        .shared()
        .raw_address(pointer, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;

    Ok(load_scalar_by_layout_at_address(
        address as usize,
        scalar_layout(access),
    ))
}

/// Store one scalar into local raw heap bytes.
pub(crate) fn store_raw_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let pointer = pointer.as_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = machine
        .heap_mut()
        .raw_address_mut(pointer, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;
    store_scalar_by_layout_at_address(address as usize, scalar_layout(access), value);

    Ok(())
}

/// Store bytes into a local raw pointer.
pub(crate) fn store_raw_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.as_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    machine
        .heap_mut()
        .write_raw_bytes(pointer, access.byte_offset, bytes)
        .map_err(Error::from)
}

/// Store one scalar into shared raw heap bytes.
pub(crate) fn store_shared_raw_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let pointer = pointer.as_shared_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = machine
        .shared()
        .raw_address_mut(pointer, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;
    store_scalar_by_layout_at_address(address as usize, scalar_layout(access), value);

    Ok(())
}

/// Store bytes into a shared raw pointer.
pub(crate) fn store_shared_raw_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.as_shared_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    machine
        .shared()
        .write_raw_bytes(pointer, access.byte_offset, bytes)
        .map_err(Error::from)
}

/// Load bytes from a local raw pointer.
#[inline(always)]
pub(crate) fn load_raw_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let pointer = pointer.as_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let destination = unsafe { slice::from_raw_parts_mut(destination, destination_len) };
    machine
        .heap()
        .read_raw_bytes_into(pointer, access.byte_offset, destination)
        .map_err(Error::from)
}

/// Load bytes from a local heap reference.
#[inline(always)]
pub(crate) fn load_heap_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let reference = pointer.as_heap_reference();
    let address =
        local_heap_address(machine, reference, access.byte_offset).map_err(Error::from)?;
    load_native_bytes(address, destination, destination_len);

    Ok(())
}

/// Load bytes from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let reference = pointer.as_shared_heap_reference();
    let address =
        shared_heap_address(machine, reference, access.byte_offset).map_err(Error::from)?;
    load_native_bytes(address, destination, destination_len);

    Ok(())
}

/// Load bytes from a shared raw pointer.
#[inline(always)]
pub(crate) fn load_shared_raw_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let pointer = pointer.as_shared_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let destination = unsafe { slice::from_raw_parts_mut(destination, destination_len) };
    machine
        .shared()
        .read_raw_bytes_into(pointer, access.byte_offset, destination)
        .map_err(Error::from)
}

/// Load bytes from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_stack_range(pointer, destination_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    load_native_bytes(pointer.address(), destination, destination_len);
    Ok(())
}

/// Load bytes from a frame pointer.
#[inline(always)]
pub(crate) fn load_frame_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_frame_range(pointer, destination_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: "foreign".to_string(),
        });
    }

    load_native_bytes(pointer.address(), destination, destination_len);
    Ok(())
}

/// Load bytes from a static pointer.
#[inline(always)]
pub(crate) fn load_static_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_static_range(pointer, destination_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    load_native_bytes(pointer.address(), destination, destination_len);
    Ok(())
}

/// Load one scalar from a local heap reference.
#[inline(always)]
pub(crate) fn load_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    byte_offset: usize,
) -> Result<Word, Error> {
    let reference = pointer.as_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = local_heap_address(machine, reference, byte_offset).map_err(Error::from)?;

    Ok(load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(address))
}

/// Load one scalar from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    byte_offset: usize,
) -> Result<Word, Error> {
    let reference = pointer.as_shared_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = shared_heap_address(machine, reference, byte_offset).map_err(Error::from)?;

    Ok(load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(address))
}

/// Load one scalar from a local raw pointer.
#[inline(always)]
pub(crate) fn load_raw_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    byte_offset: usize,
) -> Result<Word, Error> {
    let pointer = pointer.as_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = machine
        .heap()
        .raw_address(pointer, byte_offset, BYTE_LEN)
        .map_err(Error::from)?;

    Ok(load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(
        address as usize,
    ))
}

/// Load one scalar from a shared raw pointer.
#[inline(always)]
pub(crate) fn load_shared_raw_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    byte_offset: usize,
) -> Result<Word, Error> {
    let pointer = pointer.as_shared_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = machine
        .shared()
        .raw_address(pointer, byte_offset, BYTE_LEN)
        .map_err(Error::from)?;

    Ok(load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(
        address as usize,
    ))
}

/// Load one scalar from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    pointer: StackPointer,
    byte_offset: usize,
) -> Result<Word, Error> {
    let pointer = pointer.add_bytes(byte_offset);
    if !machine.owns_stack_range(pointer, BYTE_LEN) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    Ok(load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(
        pointer.address(),
    ))
}

/// Load one scalar from a static pointer.
#[inline(always)]
pub(crate) fn load_static_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    pointer: StaticPointer,
    byte_offset: usize,
) -> Result<Word, Error> {
    let pointer = pointer.add_bytes(byte_offset);
    if !machine.owns_static_range(pointer, BYTE_LEN) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    Ok(load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(
        pointer.address(),
    ))
}

/// Store one scalar through a local heap reference.
#[inline(always)]
pub(crate) fn store_heap_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    byte_offset: usize,
    value: Word,
) -> Result<(), Error> {
    let reference = pointer.as_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = local_heap_address(machine, reference, byte_offset).map_err(Error::from)?;
    store_scalar_at_address::<BYTE_LEN>(address, value);

    Ok(())
}

/// Store one scalar through a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    byte_offset: usize,
    value: Word,
) -> Result<(), Error> {
    let reference = pointer.as_shared_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = shared_heap_address(machine, reference, byte_offset).map_err(Error::from)?;
    store_scalar_at_address::<BYTE_LEN>(address, value);

    Ok(())
}

/// Store one scalar through a local raw pointer.
#[inline(always)]
pub(crate) fn store_raw_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    byte_offset: usize,
    value: Word,
) -> Result<(), Error> {
    let pointer = pointer.as_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = machine
        .heap_mut()
        .raw_address_mut(pointer, byte_offset, BYTE_LEN)
        .map_err(Error::from)?;
    store_scalar_at_address::<BYTE_LEN>(address as usize, value);

    Ok(())
}

/// Store one scalar through a shared raw pointer.
#[inline(always)]
pub(crate) fn store_shared_raw_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    byte_offset: usize,
    value: Word,
) -> Result<(), Error> {
    let pointer = pointer.as_shared_raw_pointer();
    if machine.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = machine
        .shared()
        .raw_address_mut(pointer, byte_offset, BYTE_LEN)
        .map_err(Error::from)?;
    store_scalar_at_address::<BYTE_LEN>(address as usize, value);

    Ok(())
}

/// Store one scalar through a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    pointer: StackPointer,
    byte_offset: usize,
    value: Word,
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(byte_offset);
    if !machine.owns_stack_range(pointer, BYTE_LEN) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    store_scalar_at_address::<BYTE_LEN>(pointer.address(), value);

    Ok(())
}

/// Store one scalar through a static pointer.
#[inline(always)]
pub(crate) fn store_static_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    pointer: StaticPointer,
    byte_offset: usize,
    value: Word,
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(byte_offset);
    if !machine.owns_mutable_static_range(pointer, BYTE_LEN) {
        return Err(Error::InvalidAddressSpace {
            expected: "mutable static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    store_scalar_at_address::<BYTE_LEN>(pointer.address(), value);

    Ok(())
}

/// Load one scalar from a heap reference.
#[inline(always)]
pub(crate) fn load_heap_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let reference = pointer.as_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    debug_assert_word_access(access);
    let address =
        local_heap_address(machine, reference, access.byte_offset).map_err(Error::from)?;

    Ok(load_scalar_by_layout_at_address(
        address,
        scalar_layout(access),
    ))
}

/// Load one scalar from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let reference = pointer.as_shared_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    debug_assert_word_access(access);
    let address =
        shared_heap_address(machine, reference, access.byte_offset).map_err(Error::from)?;

    Ok(load_scalar_by_layout_at_address(
        address,
        scalar_layout(access),
    ))
}

/// Load one scalar from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_stack_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    debug_assert_word_access(access);

    Ok(load_scalar_by_layout_at_address(
        pointer.address(),
        scalar_layout(access),
    ))
}

/// Load one scalar from a frame pointer.
#[inline(always)]
pub(crate) fn load_frame_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_frame_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    debug_assert_word_access(access);

    Ok(load_scalar_by_layout_at_address(
        pointer.address(),
        scalar_layout(access),
    ))
}

/// Load one scalar from a static pointer.
#[inline(always)]
pub(crate) fn load_static_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_static_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    debug_assert_word_access(access);

    Ok(load_scalar_by_layout_at_address(
        pointer.address(),
        scalar_layout(access),
    ))
}

/// Store one scalar through a heap reference.
#[inline(always)]
pub(crate) fn store_heap_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let reference = pointer.as_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let start = access.byte_offset;
    let address = local_heap_address(machine, reference, start).map_err(Error::from)?;
    store_scalar_by_layout_at_address(address, scalar_layout(access), value);

    Ok(())
}

/// Store bytes into a local heap reference.
#[inline(always)]
pub(crate) fn store_heap_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let reference = pointer.as_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let start = access.byte_offset;
    let address = local_heap_address(machine, reference, start).map_err(Error::from)?;
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }

    Ok(())
}

/// Store one scalar through a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let reference = pointer.as_shared_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let start = access.byte_offset;
    let address = shared_heap_address(machine, reference, start).map_err(Error::from)?;
    store_scalar_by_layout_at_address(address, scalar_layout(access), value);

    Ok(())
}

/// Store bytes into a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let reference = pointer.as_shared_heap_reference();
    if machine.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let start = access.byte_offset;
    let address = shared_heap_address(machine, reference, start).map_err(Error::from)?;
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }

    Ok(())
}

/// Store one scalar through a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_stack_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    store_scalar_by_layout_at_address(pointer.address(), scalar_layout(access), value);

    Ok(())
}

/// Store bytes into a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_stack_range(pointer, bytes.len()) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Store one scalar through a frame pointer.
#[inline(always)]
pub(crate) fn store_frame_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_frame_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    store_scalar_by_layout_at_address(pointer.address(), scalar_layout(access), value);

    Ok(())
}

/// Store bytes into a frame pointer.
#[inline(always)]
pub(crate) fn store_frame_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_frame_range(pointer, bytes.len()) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    unsafe {
        ptr::copy(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Store one scalar through a static pointer.
#[inline(always)]
pub(crate) fn store_static_scalar_by_layout(
    machine: &mut Machine<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_mutable_static_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "mutable static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    store_scalar_by_layout_at_address(pointer.address(), scalar_layout(access), value);

    Ok(())
}

/// Store bytes into a static pointer.
#[inline(always)]
pub(crate) fn store_static_bytes(
    machine: &mut Machine<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !machine.owns_mutable_static_range(pointer, bytes.len()) {
        return Err(Error::InvalidAddressSpace {
            expected: "mutable static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}
