use std::{ptr, slice};

use crate::diagnostic::Error;
use crate::program::{ElementAccess, FieldAccess, PointeeAccess, PointerClass};
use crate::{
    FramePointer, HeapReference, RawPointer, SharedHeapReference, SharedRawPointer, StackPointer,
    StaticPointer, Word,
};
use destack_heap::HeapError;

use crate::interpreter::DispatchState;

/// Build one invalid-pointer-type error for the given value.
#[inline(always)]
pub(super) fn invalid_pointer_type(value: Word) -> Error {
    Error::InvalidPointerType {
        actual: format!("{value:?}"),
    }
}

/// Ensure one shared heap reference is visible to global heap metadata.
#[inline(always)]
fn ensure_shared_heap_live(
    state: &mut DispatchState<'_, '_>,
    reference: SharedHeapReference,
) -> Result<(), Error> {
    if state.shared().is_heap_live(reference) {
        return Ok(());
    }

    state.flush_shared_allocator();
    if state.shared().is_heap_live(reference) {
        return Ok(());
    }

    Err(Error::InvalidHeapReference)
}

/// Debug assert one lowered word access.
#[inline(always)]
fn debug_assert_word_access(access: PointeeAccess) {
    debug_assert!(access.is_word());
    debug_assert!(access.byte_len <= Word::BYTE_LEN);
}

/// Decode raw memory bits through one lowered word layout.
#[inline(always)]
fn decode_word(access: PointeeAccess, raw: u64) -> Word {
    debug_assert!(access.word_layout.is_some());
    let layout = unsafe { access.word_layout.unwrap_unchecked() };

    layout.decode(raw)
}

/// Validate a field index against a known field count.
#[inline(always)]
fn check_field_index(
    state: &DispatchState<'_, '_>,
    index: u32,
    field_count: u32,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // reject out of bounds indices
    if index >= field_count {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: field_count as usize,
        });
    }

    Ok(())
}

/// Validate an array index against a known length.
#[inline(always)]
fn check_array_index(
    state: &DispatchState<'_, '_>,
    index: u64,
    array_length: u64,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // reject out of bounds indices
    if index >= array_length {
        return Err(Error::InvalidArrayAccess {
            index,
            length: array_length,
        });
    }

    Ok(())
}

/// Return one element byte offset.
#[inline(always)]
fn element_byte_offset(index: u64, stride: usize) -> usize {
    index as usize * stride
}

/// Return one local heap native address.
#[inline(always)]
fn local_heap_address(
    state: &DispatchState<'_, '_>,
    reference: HeapReference,
    byte_offset: usize,
) -> Result<usize, HeapError> {
    if state.null_checks && reference.is_null() {
        return Err(HeapError::InvalidHeapReference { reference });
    }

    Ok(state.heap().heap_base_address() + reference.offset() + byte_offset)
}

/// Return one shared heap native address.
#[inline(always)]
fn shared_heap_address(
    state: &mut DispatchState<'_, '_>,
    reference: SharedHeapReference,
    byte_offset: usize,
) -> Result<usize, HeapError> {
    if state.null_checks && reference.is_null() {
        return Err(HeapError::InvalidSharedHeapReference { reference });
    }

    Ok(state.shared().heap_base_address() + reference.offset() + byte_offset)
}

/// Check that one destination byte range matches one access.
#[inline(always)]
fn check_destination_len(access: PointeeAccess, destination_len: usize) -> Result<(), Error> {
    if destination_len != access.byte_len {
        return Err(Error::InvalidInstruction);
    }

    Ok(())
}

/// Load bytes from one native address.
#[inline(always)]
fn load_native_bytes(address: usize, destination: *mut u8, destination_len: usize) {
    unsafe {
        ptr::copy_nonoverlapping(address as *const u8, destination, destination_len);
    }
}

/// Load layout-width scalar bits from one address.
#[inline(always)]
pub(super) fn load_scalar_bits(address: usize, byte_len: usize) -> u64 {
    unsafe {
        let source = address as *const u8;

        match byte_len {
            0 => 0,
            1 => source.read() as u64,
            2 => u16::from_le(source.cast::<u16>().read_unaligned()) as u64,
            4 => u32::from_le(source.cast::<u32>().read_unaligned()) as u64,
            8 => u64::from_le(source.cast::<u64>().read_unaligned()),
            _ => load_bytewise_scalar_bits(source, byte_len),
        }
    }
}

/// Load non-native-width scalar bits from one address.
#[cold]
#[inline(never)]
unsafe fn load_bytewise_scalar_bits(source: *const u8, byte_len: usize) -> u64 {
    let mut raw = 0u64;

    for index in 0..byte_len {
        raw |= unsafe { source.add(index).read() as u64 } << (index * 8);
    }

    raw
}

/// Store layout-width scalar bits to one address.
#[inline(always)]
pub(super) fn store_scalar_bits(address: usize, raw: u64, byte_len: usize) {
    unsafe {
        let destination = address as *mut u8;

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
            _ => store_bytewise_scalar_bits(destination, raw, byte_len),
        }
    }
}

/// Store non-native-width scalar bits to one address.
#[cold]
#[inline(never)]
unsafe fn store_bytewise_scalar_bits(destination: *mut u8, raw: u64, byte_len: usize) {
    for index in 0..byte_len {
        unsafe {
            destination
                .add(index)
                .write(((raw >> (index * 8)) & 0xFF) as u8);
        }
    }
}

/// Load one value from raw heap bytes.
pub(crate) fn load_raw_word(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    debug_assert_word_access(access);

    let pointer = pointer.as_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = state
        .heap()
        .raw_address(pointer, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;
    let raw = load_scalar_bits(address as usize, access.byte_len);

    Ok(decode_word(access, raw))
}

/// Load one value from shared raw heap bytes.
pub(crate) fn load_shared_raw_word(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    debug_assert_word_access(access);

    let pointer = pointer.as_shared_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = state
        .shared()
        .raw_address(pointer, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;
    let raw = load_scalar_bits(address as usize, access.byte_len);

    Ok(decode_word(access, raw))
}

/// Store one value into raw heap bytes.
pub(crate) fn store_raw_word(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let raw = value.bits();
    let byte_len = access.byte_len;
    let pointer = pointer.as_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = state
        .heap_mut()
        .raw_address_mut(pointer, access.byte_offset, byte_len)
        .map_err(Error::from)?;
    store_scalar_bits(address as usize, raw, byte_len);

    Ok(())
}

/// Store bytes into a local raw pointer.
pub(crate) fn store_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.as_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    state
        .heap_mut()
        .write_raw_bytes(pointer, access.byte_offset, bytes)
        .map_err(Error::from)
}

/// Store one value into shared raw heap bytes.
pub(crate) fn store_shared_raw_word(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let raw = value.bits();
    let byte_len = access.byte_len;
    let pointer = pointer.as_shared_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = state
        .shared()
        .raw_address_mut(pointer, access.byte_offset, byte_len)
        .map_err(Error::from)?;
    store_scalar_bits(address as usize, raw, byte_len);

    Ok(())
}

/// Store bytes into a shared raw pointer.
pub(crate) fn store_shared_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.as_shared_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    state
        .shared()
        .write_raw_bytes(pointer, access.byte_offset, bytes)
        .map_err(Error::from)
}

/// Load bytes from a local raw pointer.
#[inline(always)]
pub(crate) fn load_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    check_destination_len(access, destination_len)?;

    let pointer = pointer.as_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let destination = unsafe { slice::from_raw_parts_mut(destination, destination_len) };
    state
        .heap()
        .read_raw_bytes_into(pointer, access.byte_offset, destination)
        .map_err(Error::from)
}

/// Load bytes from a shared raw pointer.
#[inline(always)]
pub(crate) fn load_shared_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    check_destination_len(access, destination_len)?;

    let pointer = pointer.as_shared_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let destination = unsafe { slice::from_raw_parts_mut(destination, destination_len) };
    state
        .shared()
        .read_raw_bytes_into(pointer, access.byte_offset, destination)
        .map_err(Error::from)
}

/// Load bytes from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    check_destination_len(access, destination_len)?;

    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_stack_range(pointer, destination_len) {
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
    state: &mut DispatchState<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    check_destination_len(access, destination_len)?;

    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_frame_range(pointer, destination_len) {
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
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    check_destination_len(access, destination_len)?;

    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_static_range(pointer, destination_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    load_native_bytes(pointer.address(), destination, destination_len);
    Ok(())
}

/// Load bytes from one pointer value.
#[inline(always)]
pub(crate) fn load_pointer_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    match access.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => {
            let reference = pointer.as_heap_reference();
            let address =
                local_heap_address(state, reference, access.byte_offset).map_err(Error::from)?;
            check_destination_len(access, destination_len)?;
            load_native_bytes(address, destination, destination_len);
            Ok(())
        }
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            let reference = pointer.as_shared_heap_reference();
            let address =
                shared_heap_address(state, reference, access.byte_offset).map_err(Error::from)?;
            check_destination_len(access, destination_len)?;
            load_native_bytes(address, destination, destination_len);
            Ok(())
        }
        PointerClass::Raw => load_raw_bytes(state, pointer, access, destination, destination_len),
        PointerClass::SharedRaw => {
            load_shared_raw_bytes(state, pointer, access, destination, destination_len)
        }
        PointerClass::Stack => load_stack_bytes(
            state,
            pointer.as_stack_pointer(),
            access,
            destination,
            destination_len,
        ),
        PointerClass::Frame => load_frame_bytes(
            state,
            pointer.as_frame_pointer(),
            access,
            destination,
            destination_len,
        ),
        PointerClass::Static => load_static_bytes(
            state,
            pointer.as_static_pointer(),
            access,
            destination,
            destination_len,
        ),
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Store bytes into one pointer value.
#[inline(always)]
pub(crate) fn store_pointer_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    match access.pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => {
            store_heap_bytes(state, pointer, access, bytes)
        }
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => {
            store_shared_heap_bytes(state, pointer, access, bytes)
        }
        PointerClass::Raw => store_raw_bytes(state, pointer, access, bytes),
        PointerClass::SharedRaw => store_shared_raw_bytes(state, pointer, access, bytes),
        PointerClass::Stack => store_stack_bytes(state, pointer.as_stack_pointer(), access, bytes),
        PointerClass::Frame => store_frame_bytes(state, pointer.as_frame_pointer(), access, bytes),
        PointerClass::Static => {
            store_static_bytes(state, pointer.as_static_pointer(), access, bytes)
        }
        PointerClass::Unknown => Err(Error::InvalidInstruction),
    }
}

/// Load one word from a heap reference.
#[inline(always)]
pub(crate) fn load_heap_word(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let reference = pointer.as_heap_reference();
    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    debug_assert_word_access(access);
    let address = local_heap_address(state, reference, access.byte_offset).map_err(Error::from)?;
    let raw = load_scalar_bits(address, access.byte_len);

    Ok(decode_word(access, raw))
}

/// Load one word from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_word(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let reference = pointer.as_shared_heap_reference();
    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    debug_assert_word_access(access);
    let address = shared_heap_address(state, reference, access.byte_offset).map_err(Error::from)?;
    let raw = load_scalar_bits(address, access.byte_len);

    Ok(decode_word(access, raw))
}

/// Load one word from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_word(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_stack_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    debug_assert_word_access(access);
    let raw = load_scalar_bits(pointer.address(), access.byte_len);

    Ok(decode_word(access, raw))
}

/// Load one word from a frame pointer.
#[inline(always)]
pub(crate) fn load_frame_word(
    state: &mut DispatchState<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_frame_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    debug_assert_word_access(access);
    let raw = load_scalar_bits(pointer.address(), access.byte_len);

    Ok(decode_word(access, raw))
}

/// Load one word from a static pointer.
#[inline(always)]
pub(crate) fn load_static_word(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_static_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    debug_assert_word_access(access);
    let raw = load_scalar_bits(pointer.address(), access.byte_len);

    Ok(decode_word(access, raw))
}

/// Store one word through a heap reference.
#[inline(always)]
pub(crate) fn store_heap_word(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let raw = value.bits();
    let byte_len = access.byte_len;
    let reference = pointer.as_heap_reference();
    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let start = access.byte_offset;
    let address = local_heap_address(state, reference, start).map_err(Error::from)?;
    store_scalar_bits(address, raw, byte_len);

    Ok(())
}

/// Store bytes into a local heap reference.
#[inline(always)]
pub(crate) fn store_heap_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let reference = pointer.as_heap_reference();
    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let start = access.byte_offset;
    let address = local_heap_address(state, reference, start).map_err(Error::from)?;
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }

    Ok(())
}

/// Store one word through a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_word(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let raw = value.bits();
    let byte_len = access.byte_len;
    let reference = pointer.as_shared_heap_reference();
    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let start = access.byte_offset;
    let address = shared_heap_address(state, reference, start).map_err(Error::from)?;
    store_scalar_bits(address, raw, byte_len);

    Ok(())
}

/// Store bytes into a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let reference = pointer.as_shared_heap_reference();
    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let start = access.byte_offset;
    let address = shared_heap_address(state, reference, start).map_err(Error::from)?;
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }

    Ok(())
}

/// Store one word through a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_word(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let raw = value.bits();
    let byte_len = access.byte_len;
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_stack_range(pointer, byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    store_scalar_bits(pointer.address(), raw, byte_len);

    Ok(())
}

/// Store bytes into a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_stack_range(pointer, bytes.len()) {
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

/// Store one word through a frame pointer.
#[inline(always)]
pub(crate) fn store_frame_word(
    state: &mut DispatchState<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let raw = value.bits();
    let byte_len = access.byte_len;
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_frame_range(pointer, byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    store_scalar_bits(pointer.address(), raw, byte_len);

    Ok(())
}

/// Store bytes into a frame pointer.
#[inline(always)]
pub(crate) fn store_frame_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_frame_range(pointer, bytes.len()) {
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

/// Store one word through a static pointer.
#[inline(always)]
pub(crate) fn store_static_word(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    debug_assert_word_access(access);
    let raw = value.bits();
    let byte_len = access.byte_len;
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_mutable_static_range(pointer, byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "mutable static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    store_scalar_bits(pointer.address(), raw, byte_len);

    Ok(())
}

/// Store bytes into a static pointer.
#[inline(always)]
pub(crate) fn store_static_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);
    if !state.owns_mutable_static_range(pointer, bytes.len()) {
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

/// Compute a field address from a heap reference.
#[inline(always)]
pub(crate) fn address_field_heap(
    state: &mut DispatchState<'_, '_>,
    reference: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    if state.bounds_checks && !state.heap().is_heap_live(reference) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let reference = reference.add_bytes(field.byte_offset);

    Ok(Word::heap_reference(reference))
}

/// Compute a field address from a shared heap reference.
#[inline(always)]
pub(crate) fn address_field_shared_heap(
    state: &mut DispatchState<'_, '_>,
    reference: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    if state.bounds_checks {
        ensure_shared_heap_live(state, reference)?;
    }

    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let reference = reference.add_bytes(field.byte_offset);

    Ok(Word::shared_heap_reference(reference))
}

/// Compute a field address from a raw pointer.
#[inline(always)]
pub(crate) fn address_field_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = pointer.add_bytes(field.byte_offset);

    Ok(Word::raw_pointer(pointer))
}

/// Compute a field address from a shared raw pointer.
#[inline(always)]
pub(crate) fn address_field_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = pointer.add_bytes(field.byte_offset);

    Ok(Word::shared_raw_pointer(pointer))
}

/// Compute a field address from a stack pointer.
#[inline(always)]
pub(crate) fn address_field_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    let pointer = pointer.add_bytes(field.byte_offset);

    Ok(Word::stack_pointer(pointer))
}

/// Compute a field address from a static pointer.
#[inline(always)]
pub(crate) fn address_field_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    let pointer = pointer.add_bytes(field.byte_offset);

    Ok(Word::static_pointer(pointer))
}

/// Compute an element address from a heap reference.
#[inline(always)]
pub(crate) fn address_element_heap(
    state: &mut DispatchState<'_, '_>,
    reference: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    if state.bounds_checks && !state.heap().is_heap_live(reference) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let reference = reference.add_bytes(element_offset);

    Ok(Word::heap_reference(reference))
}

/// Compute an element address from a shared heap reference.
#[inline(always)]
pub(crate) fn address_element_shared_heap(
    state: &mut DispatchState<'_, '_>,
    reference: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    if state.bounds_checks {
        ensure_shared_heap_live(state, reference)?;
    }

    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let reference = reference.add_bytes(element_offset);

    Ok(Word::shared_heap_reference(reference))
}

/// Compute an element address from a raw pointer.
#[inline(always)]
pub(crate) fn address_element_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Ok(Word::raw_pointer(pointer))
}

/// Compute an element address from a shared raw pointer.
#[inline(always)]
pub(crate) fn address_element_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // compute the element byte offset
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Ok(Word::shared_raw_pointer(pointer))
}

/// Compute an element address from a stack pointer.
#[inline(always)]
pub(crate) fn address_element_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    // compute the element byte offset
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Ok(Word::stack_pointer(pointer))
}

/// Compute an element address from a static pointer.
#[inline(always)]
pub(crate) fn address_element_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    Ok(Word::static_pointer(pointer))
}

/// Load a field from a heap allocation.
#[inline(always)]
pub(crate) fn load_field_heap(
    state: &mut DispatchState<'_, '_>,
    reference: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate the requested field before decoding bytes
    check_field_index(state, index, field_count)?;

    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let access = PointeeAccess::from(field);
    debug_assert_word_access(access);
    let address = local_heap_address(state, reference, field.byte_offset).map_err(Error::from)?;
    let raw = load_scalar_bits(address, field.byte_len);

    Ok(decode_word(field.into(), raw))
}

/// Load a field from a shared heap allocation.
#[inline(always)]
pub(crate) fn load_field_shared_heap(
    state: &mut DispatchState<'_, '_>,
    reference: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    check_field_index(state, index, field_count)?;

    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let access = PointeeAccess::from(field);
    debug_assert_word_access(access);
    let address = shared_heap_address(state, reference, field.byte_offset).map_err(Error::from)?;
    let raw = load_scalar_bits(address, field.byte_len);

    Ok(decode_word(field.into(), raw))
}

/// Store a field into a heap allocation.
#[inline(always)]
pub(crate) fn store_field_heap(
    state: &mut DispatchState<'_, '_>,
    reference: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
    value: Word,
) -> Result<(), Error> {
    debug_assert!(field.is_word());
    let raw = value.bits();
    let byte_len = field.byte_len;
    let null_checks = state.null_checks;

    check_field_index(state, index, field_count)?;

    if null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = local_heap_address(state, reference, field.byte_offset).map_err(Error::from)?;
    store_scalar_bits(address, raw, byte_len);

    Ok(())
}

/// Store a field into a shared heap allocation.
#[inline(always)]
pub(crate) fn store_field_shared_heap(
    state: &mut DispatchState<'_, '_>,
    reference: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: u32,
    value: Word,
) -> Result<(), Error> {
    debug_assert!(field.is_word());
    let raw = value.bits();
    let byte_len = field.byte_len;
    let null_checks = state.null_checks;

    check_field_index(state, index, field_count)?;

    if null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = shared_heap_address(state, reference, field.byte_offset).map_err(Error::from)?;
    store_scalar_bits(address, raw, byte_len);

    Ok(())
}

/// Load a field from a raw pointer.
#[inline(always)]
pub(crate) fn load_field_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = Word::raw_pointer(pointer);
    let access = PointeeAccess::from(field);

    load_raw_word(state, pointer, access)
}

/// Store a field through a raw pointer.
#[inline(always)]
pub(crate) fn store_field_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
    value: Word,
) -> Result<(), Error> {
    check_field_index(state, index, field_count)?;

    store_raw_word(state, Word::raw_pointer(pointer), field.into(), value)
}

/// Load a field through a shared raw pointer.
#[inline(always)]
pub(crate) fn load_field_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    let pointer = Word::shared_raw_pointer(pointer);
    let access = PointeeAccess::from(field);

    load_shared_raw_word(state, pointer, access)
}

/// Store a field through a shared raw pointer.
#[inline(always)]
pub(crate) fn store_field_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
    value: Word,
) -> Result<(), Error> {
    check_field_index(state, index, field_count)?;

    store_shared_raw_word(
        state,
        Word::shared_raw_pointer(pointer),
        field.into(),
        value,
    )
}

/// Load a field from a stack allocation.
#[inline(always)]
pub(crate) fn load_field_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    let pointer = pointer.add_bytes(field.byte_offset);

    let access = PointeeAccess::from(field);
    debug_assert_word_access(access);
    let raw = load_scalar_bits(pointer.address(), field.byte_len);

    Ok(decode_word(field.into(), raw))
}

/// Store a field into a stack allocation.
#[inline(always)]
pub(crate) fn store_field_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
    value: Word,
) -> Result<(), Error> {
    check_field_index(state, index, field_count)?;

    store_stack_word(state, pointer, field.into(), value)
}

/// Load a field through a static pointer.
#[inline(always)]
pub(crate) fn load_field_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
) -> Result<Word, Error> {
    // validate field index
    check_field_index(state, index, field_count)?;

    let pointer = pointer.add_bytes(field.byte_offset);

    let access = PointeeAccess::from(field);
    debug_assert_word_access(access);
    let raw = load_scalar_bits(pointer.address(), field.byte_len);

    Ok(decode_word(field.into(), raw))
}

/// Store a field through a static pointer.
#[inline(always)]
pub(crate) fn store_field_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    field: FieldAccess,
    index: u32,
    field_count: u32,
    value: Word,
) -> Result<(), Error> {
    check_field_index(state, index, field_count)?;

    store_static_word(state, pointer, field.into(), value)
}

/// Load an element from a heap allocation.
#[inline(always)]
pub(crate) fn load_element_heap(
    state: &mut DispatchState<'_, '_>,
    reference: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate the requested element before decoding bytes
    check_array_index(state, index, array_length)?;

    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let access = PointeeAccess::from(element);
    debug_assert_word_access(access);
    let address = local_heap_address(state, reference, element_offset).map_err(Error::from)?;
    let raw = load_scalar_bits(address, element.byte_len);

    Ok(decode_word(element.into(), raw))
}

/// Load an element from a shared heap allocation.
#[inline(always)]
pub(crate) fn load_element_shared_heap(
    state: &mut DispatchState<'_, '_>,
    reference: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    check_array_index(state, index, array_length)?;

    if state.null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let access = PointeeAccess::from(element);
    debug_assert_word_access(access);
    let address = shared_heap_address(state, reference, element_offset).map_err(Error::from)?;
    let raw = load_scalar_bits(address, element.byte_len);

    Ok(decode_word(element.into(), raw))
}

/// Store an element into a heap allocation.
#[inline(always)]
pub(crate) fn store_element_heap(
    state: &mut DispatchState<'_, '_>,
    reference: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
    value: Word,
) -> Result<(), Error> {
    debug_assert!(element.is_word());
    let raw = value.bits();
    let byte_len = element.byte_len;
    let null_checks = state.null_checks;

    check_array_index(state, index, array_length)?;

    if null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let address = local_heap_address(state, reference, element_offset).map_err(Error::from)?;
    store_scalar_bits(address, raw, byte_len);

    Ok(())
}

/// Store an element into a shared heap allocation.
#[inline(always)]
pub(crate) fn store_element_shared_heap(
    state: &mut DispatchState<'_, '_>,
    reference: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: u64,
    value: Word,
) -> Result<(), Error> {
    debug_assert!(element.is_word());
    let raw = value.bits();
    let byte_len = element.byte_len;
    let null_checks = state.null_checks;

    check_array_index(state, index, array_length)?;

    if null_checks && reference.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let address = shared_heap_address(state, reference, element_offset).map_err(Error::from)?;
    store_scalar_bits(address, raw, byte_len);

    Ok(())
}

/// Load an element from a raw pointer.
#[inline(always)]
pub(crate) fn load_element_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);
    let pointer = Word::raw_pointer(pointer);
    let access = PointeeAccess::from(element);

    load_raw_word(state, pointer, access)
}

/// Store an element through a raw pointer.
#[inline(always)]
pub(crate) fn store_element_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
    value: Word,
) -> Result<(), Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    store_raw_word(state, Word::raw_pointer(pointer), element.into(), value)
}

/// Load an element through a shared raw pointer.
#[inline(always)]
pub(crate) fn load_element_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    // compute the element offset before loading
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    load_shared_raw_word(state, Word::shared_raw_pointer(pointer), element.into())
}

/// Store an element through a shared raw pointer.
#[inline(always)]
pub(crate) fn store_element_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
    value: Word,
) -> Result<(), Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    store_shared_raw_word(
        state,
        Word::shared_raw_pointer(pointer),
        element.into(),
        value,
    )
}

/// Load an element from a stack allocation.
#[inline(always)]
pub(crate) fn load_element_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    // compute the element address
    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    let access = PointeeAccess::from(element);
    debug_assert_word_access(access);
    let raw = load_scalar_bits(pointer.address(), element.byte_len);

    Ok(decode_word(element.into(), raw))
}

/// Store an element into a stack allocation.
#[inline(always)]
pub(crate) fn store_element_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
    value: Word,
) -> Result<(), Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    store_stack_word(state, pointer, element.into(), value)
}

/// Load an element through a static pointer.
#[inline(always)]
pub(crate) fn load_element_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
) -> Result<Word, Error> {
    // validate array index
    check_array_index(state, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    let access = PointeeAccess::from(element);
    debug_assert_word_access(access);
    let raw = load_scalar_bits(pointer.address(), element.byte_len);

    Ok(decode_word(element.into(), raw))
}

/// Load one word element from a frame value.
#[inline(always)]
pub(crate) fn load_frame_element(
    state: &mut DispatchState<'_, '_>,
    value: Word,
    index: u64,
    array_length: u64,
    element: ElementAccess,
) -> Result<Word, Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = value.as_frame_pointer().add_bytes(element_offset);

    load_frame_word(state, pointer, element.into())
}

/// Store an element through a static pointer.
#[inline(always)]
pub(crate) fn store_element_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    element: ElementAccess,
    index: u64,
    array_length: u64,
    value: Word,
) -> Result<(), Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = element_byte_offset(index, element.byte_stride);
    let pointer = pointer.add_bytes(element_offset);

    store_static_word(state, pointer, element.into(), value)
}
