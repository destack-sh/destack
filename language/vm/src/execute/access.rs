use crate::diagnostic::Error;
use crate::program::{ElementAccess, FieldAccess, PointeeAccess, PointerClass};
use crate::{
    FramePointer, FunctionPointer, HeapReference, RawPointer, SharedHeapReference,
    SharedRawPointer, StackPointer, StaticPointer, Word,
};
use destack_mir as mir;

use super::bytes::*;
use super::{frame_pointer_value, stack_pointer_value, static_pointer_value};
use crate::interpreter::DispatchState;

pub(crate) use super::bytes::{
    allocate_callable, decode_callable, decode_raw_bits, decode_raw_value, load_raw_pointer,
    load_shared_raw_pointer, store_raw_pointer, store_raw_pointer_bytes, store_shared_raw_pointer,
    store_shared_raw_pointer_bytes, word_type_byte_len,
};

/// Build one invalid-pointer-type error for the given value.
#[inline(always)]
pub(super) fn invalid_pointer_type(value: Word) -> Error {
    Error::InvalidPointerType {
        actual: format!("{value:?}"),
    }
}

/// Build one non-word load error.
#[inline(always)]
fn non_word_load_error(ty: mir::LocalNodeId<mir::Type>) -> Error {
    Error::TypeMismatch {
        expected: "word-sized type".to_string(),
        actual: format!("{ty:?}"),
    }
}

/// Validate one word access.
#[inline(always)]
fn check_word_access(access: PointeeAccess) -> Result<(), Error> {
    if !access.is_scalar {
        return Err(non_word_load_error(access.value_type));
    }
    if access.byte_len > Word::BYTE_LEN {
        return Err(Error::InvalidInstruction);
    }

    Ok(())
}

/// Return one heap reference value.
#[inline(always)]
fn heap_reference_from_value(value: Word) -> HeapReference {
    value.as_heap_reference()
}

/// Return one shared heap reference value.
#[inline(always)]
fn shared_heap_reference_from_value(value: Word) -> SharedHeapReference {
    value.as_shared_heap_reference()
}

/// Return one raw pointer value.
#[inline(always)]
fn raw_pointer_from_value(value: Word) -> RawPointer {
    value.as_raw_pointer()
}

/// Return one shared raw pointer value.
#[inline(always)]
fn shared_raw_pointer_from_value(value: Word) -> SharedRawPointer {
    value.as_shared_raw_pointer()
}

/// Return one frame pointer value.
#[inline(always)]
fn frame_pointer_from_value(value: Word) -> FramePointer {
    value.as_frame_pointer()
}

/// Return one static pointer value.
#[inline(always)]
fn static_pointer_from_value(value: Word) -> StaticPointer {
    value.as_static_pointer()
}

/// Return the effective field count for one error path.
#[inline(always)]
fn field_count_for_error(field_count: Option<u32>, actual_count: usize) -> usize {
    field_count.map_or(actual_count, |field_count| field_count as usize)
}

/// Return the effective array length for one error path.
#[inline(always)]
fn array_length_for_error(array_length: Option<u64>, actual_length: u64) -> u64 {
    array_length.unwrap_or(actual_length)
}

/// Return one lowered element access for an already known indexed type.
#[inline(always)]
pub(crate) fn element_access_for_type(
    state: &DispatchState<'_, '_>,
    indexed_type: mir::LocalNodeId<mir::Type>,
    index: u64,
) -> Result<(ElementAccess, u64), Error> {
    let layout = state.layout(indexed_type)?;
    let element_count = layout.element_count().ok_or(Error::InvalidInstruction)? as u64;
    let element = layout.element().ok_or(Error::InvalidArrayAccess {
        index,
        length: element_count,
    })?;

    if index >= element_count {
        return Err(Error::InvalidArrayAccess {
            index,
            length: element_count,
        });
    }

    let access = ElementAccess {
        pointer_class: PointerClass::Frame,
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        is_scalar: state.layout(element.ty)?.is_scalar(),
    };

    Ok((access, element_count))
}

/// Record one shared heap write barrier from one exact stored value.
#[inline(always)]
fn publish_shared_store(
    state: &mut DispatchState<'_, '_>,
    destination: SharedHeapReference,
    start: usize,
    bytes: &[u8],
) -> Result<(), Error> {
    // scan the stored bytes through the shared heap edge map
    state
        .shared()
        .write_barrier_bytes(destination, start, bytes)
        .map_err(Error::from)
}

/// Remap one invalid heap reference into one field access error.
#[inline(always)]
fn map_invalid_field_reference(error: Error, index: u32, field_count: usize) -> Error {
    match error {
        Error::InvalidHeapReference => Error::InvalidFieldAccess { index, field_count },
        other => other,
    }
}

/// Remap one invalid heap reference into one array access error.
#[inline(always)]
fn map_invalid_array_reference(error: Error, index: u64, length: u64) -> Error {
    match error {
        Error::InvalidHeapReference => Error::InvalidArrayAccess { index, length },
        other => other,
    }
}

/// Decode one raw bit pattern into a pointer-shaped VM value.
pub(crate) fn decode_pointer_bits(raw: u64, target_type: &mir::Type) -> Result<Word, Error> {
    match target_type {
        mir::Type::FunctionPointer { .. } => Ok(Word::function_pointer(
            FunctionPointer::from_bits(raw as usize),
        )),
        mir::Type::Reference {
            kind,
            address_space,
            ..
        } => {
            let value = match (*kind, address_space.clone()) {
                (
                    mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                    mir::AddressSpace::Shared,
                ) => Word::shared_heap_reference(SharedHeapReference::from_bits(raw as usize)),
                (mir::ReferenceKind::Managed | mir::ReferenceKind::Owned, _) => {
                    Word::heap_reference(HeapReference::from_bits(raw as usize))
                }
                (mir::ReferenceKind::Borrowed, mir::AddressSpace::Shared) => {
                    Word::shared_heap_reference(SharedHeapReference::from_bits(raw as usize))
                }
                (_, mir::AddressSpace::Stack) => {
                    stack_pointer_value(StackPointer::from_address(raw as usize))?
                }
                (_, mir::AddressSpace::Frame) => {
                    frame_pointer_value(FramePointer::from_address(raw as usize))?
                }
                (_, mir::AddressSpace::Static) => {
                    static_pointer_value(StaticPointer::from_address(raw as usize), 0)?
                }
                (mir::ReferenceKind::Borrowed, _) => {
                    Word::heap_reference(HeapReference::from_bits(raw as usize))
                }
                (_, mir::AddressSpace::Shared) => {
                    Word::shared_raw_pointer(SharedRawPointer::from_bits(raw as usize))
                }
                _ => Word::raw_pointer(RawPointer::from_bits(raw as usize)),
            };

            Ok(value)
        }
        _ => Ok(Word::raw_pointer(RawPointer::from_bits(raw as usize))),
    }
}

/// Check that one destination byte range matches one access.
#[inline(always)]
fn check_target_len(access: PointeeAccess, target_len: usize) -> Result<(), Error> {
    if target_len != access.byte_len {
        return Err(Error::InvalidInstruction);
    }

    Ok(())
}

/// Read one native byte range into a destination.
#[inline(always)]
fn read_address_bytes_into(address: usize, target: *mut u8, target_len: usize) {
    unsafe {
        std::ptr::copy_nonoverlapping(address as *const u8, target, target_len);
    }
}

/// Read one word prefix from one address.
#[inline(always)]
pub(super) fn read_word_bits(address: usize, byte_len: usize) -> u64 {
    unsafe {
        let source = address as *const u8;

        match byte_len {
            0 => 0,
            1 => source.read() as u64,
            2 => u16::from_le(source.cast::<u16>().read_unaligned()) as u64,
            4 => u32::from_le(source.cast::<u32>().read_unaligned()) as u64,
            8 => u64::from_le(source.cast::<u64>().read_unaligned()),
            _ => {
                let mut raw = 0u64;
                for index in 0..byte_len {
                    raw |= (source.add(index).read() as u64) << (index * 8);
                }

                raw
            }
        }
    }
}

/// Write one word prefix to one address.
#[inline(always)]
pub(super) fn write_word_bits(address: usize, raw: u64, byte_len: usize) {
    unsafe {
        let target = address as *mut u8;

        match byte_len {
            0 => {}
            1 => target.write(raw as u8),
            2 => target.cast::<u16>().write_unaligned((raw as u16).to_le()),
            4 => target.cast::<u32>().write_unaligned((raw as u32).to_le()),
            8 => target.cast::<u64>().write_unaligned(raw.to_le()),
            _ => {
                for index in 0..byte_len {
                    target.add(index).write(((raw >> (index * 8)) & 0xFF) as u8);
                }
            }
        }
    }
}

/// Check that one source byte range matches one access.
#[inline(always)]
fn check_source_len(access: PointeeAccess, source_len: usize) -> Result<(), Error> {
    if source_len != access.byte_len {
        return Err(Error::InvalidInstruction);
    }

    Ok(())
}

/// Load bytes from a raw pointer.
#[inline(always)]
pub(crate) fn load_raw_pointer_bytes_into(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    target: *mut u8,
    target_len: usize,
) -> Result<(), Error> {
    check_target_len(access, target_len)?;

    let pointer = raw_pointer_from_value(ptr);
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let target = unsafe { std::slice::from_raw_parts_mut(target, target_len) };
    state
        .heap()
        .read_raw_bytes_into(pointer, access.byte_offset, target)
        .map_err(Error::from)
}

/// Load bytes from a shared raw pointer.
#[inline(always)]
pub(crate) fn load_shared_raw_pointer_bytes_into(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    target: *mut u8,
    target_len: usize,
) -> Result<(), Error> {
    check_target_len(access, target_len)?;

    let pointer = shared_raw_pointer_from_value(ptr);
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let target = unsafe { std::slice::from_raw_parts_mut(target, target_len) };
    state
        .shared()
        .read_raw_bytes_into(pointer, access.byte_offset, target)
        .map_err(Error::from)
}

/// Load bytes from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_pointer_bytes_into(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    target: *mut u8,
    target_len: usize,
) -> Result<(), Error> {
    check_target_len(access, target_len)?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "stack".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_stack_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    read_address_bytes_into(pointer.address(), target, target_len);
    Ok(())
}

/// Load bytes from a frame pointer.
#[inline(always)]
pub(crate) fn load_frame_pointer_bytes_into(
    state: &mut DispatchState<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    target: *mut u8,
    target_len: usize,
) -> Result<(), Error> {
    check_target_len(access, target_len)?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "frame".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_frame_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: "foreign".to_string(),
        });
    }

    read_address_bytes_into(pointer.address(), target, target_len);
    Ok(())
}

/// Load bytes from a static pointer.
#[inline(always)]
pub(crate) fn load_static_pointer_bytes_into(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    target: *mut u8,
    target_len: usize,
) -> Result<(), Error> {
    check_target_len(access, target_len)?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "static".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_static_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    read_address_bytes_into(pointer.address(), target, target_len);
    Ok(())
}

/// Load a value from a heap reference.
#[inline(always)]
pub(crate) fn load_heap_reference(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    // reject null pointers before reading the allocation
    let handle = heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_word_access(access)?;
    let address = state
        .heap()
        .heap_address(handle, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;
    let raw = read_word_bits(address as usize, access.byte_len);

    decode_raw_bits(state.tree(), access.value_type, raw, access.byte_len)
}

/// Load a value from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_reference(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    // reject null pointers before reading the allocation
    let handle = shared_heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // require one live shared heap allocation
    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_word_access(access)?;
    let address = state
        .shared()
        .heap_address(handle, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;
    let raw = read_word_bits(address as usize, access.byte_len);

    decode_raw_bits(state.tree(), access.value_type, raw, access.byte_len)
}

/// Load a value from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_pointer(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "stack".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_stack_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    check_word_access(access)?;
    let raw = read_word_bits(pointer.address(), access.byte_len);

    decode_raw_bits(state.tree(), access.value_type, raw, access.byte_len)
}

/// Load a value from a frame value address.
#[inline(always)]
pub(crate) fn load_frame_pointer(
    state: &mut DispatchState<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "frame".to_string(),
                actual: format!("0x{:x}", pointer.address()),
            })?;
    if !state.owns_frame_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    check_word_access(access)?;
    let raw = read_word_bits(pointer.address(), access.byte_len);

    decode_raw_bits(state.tree(), access.value_type, raw, access.byte_len)
}

/// Load a value from a static value address.
#[inline(always)]
pub(crate) fn load_static_pointer(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "static".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_static_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    check_word_access(access)?;
    let raw = read_word_bits(pointer.address(), access.byte_len);

    decode_raw_bits(state.tree(), access.value_type, raw, access.byte_len)
}

/// Store a value through a heap reference.
#[inline(always)]
pub(crate) fn store_heap_reference(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    val: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), access.value_type, val)?;
    check_source_len(access, byte_len)?;

    let handle = heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        let heap = state.heap();
        if !heap.is_heap_live(handle) {
            return Err(Error::InvalidHeapReference);
        }
    }

    let allocation_len = state.heap().heap_byte_len(handle)?;
    let start = access.byte_offset;
    let end = start.saturating_add(byte_len);

    if end > allocation_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: allocation_len,
        });
    }

    let address = state
        .heap_mut()
        .heap_address_mut(handle, start, byte_len)
        .map_err(Error::from)?;
    write_word_bits(address as usize, raw, byte_len);
    state
        .heap_mut()
        .write_barrier(handle, start, byte_len)
        .map_err(Error::from)?;

    Ok(())
}

/// Store bytes through a heap reference.
#[inline(always)]
pub(crate) fn store_heap_reference_bytes(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(access, bytes.len())?;

    let handle = heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }
    if !state.heap().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    let allocation_len = state.heap().heap_byte_len(handle)?;
    let start = access.byte_offset;
    let end = start.saturating_add(bytes.len());

    if end > allocation_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: allocation_len,
        });
    }

    state
        .heap_mut()
        .write_heap_bytes(handle, start, bytes)
        .map_err(Error::from)?;
    state
        .heap_mut()
        .write_barrier(handle, start, bytes.len())
        .map_err(Error::from)?;

    Ok(())
}

/// Store a value through a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_reference(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    val: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), access.value_type, val)?;
    check_source_len(access, byte_len)?;

    let handle = shared_heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        if !state.shared().is_heap_live(handle) {
            return Err(Error::InvalidHeapReference);
        }
    }

    let allocation_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let start = access.byte_offset;
    let end = start.saturating_add(byte_len);

    if end > allocation_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: allocation_len,
        });
    }

    let address = state
        .shared()
        .heap_address_mut(handle, start, byte_len)
        .map_err(Error::from)?;
    write_word_bits(address as usize, raw, byte_len);
    let bytes = raw.to_le_bytes();
    publish_shared_store(state, handle, start, &bytes[..byte_len])?;

    Ok(())
}

/// Store bytes through a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_reference_bytes(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(access, bytes.len())?;

    let handle = shared_heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }
    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    let allocation_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let start = access.byte_offset;
    let end = start.saturating_add(bytes.len());

    if end > allocation_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: allocation_len,
        });
    }

    state
        .shared()
        .write_heap_bytes(handle, start, bytes)
        .map_err(Error::from)?;
    publish_shared_store(state, handle, start, bytes)?;

    Ok(())
}

/// Store a value through a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_pointer(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), access.value_type, value)?;
    check_source_len(access, byte_len)?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "stack".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_stack_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    write_word_bits(pointer.address(), raw, byte_len);

    Ok(())
}

/// Store bytes through a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_pointer_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(access, bytes.len())?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "stack".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_stack_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "stack".to_string(),
            actual: "foreign".to_string(),
        });
    }

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Store a value through a frame value address.
#[inline(always)]
pub(crate) fn store_frame_pointer(
    state: &mut DispatchState<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), access.value_type, value)?;
    check_source_len(access, byte_len)?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "frame".to_string(),
                actual: format!("0x{:x}", pointer.address()),
            })?;
    if !state.owns_frame_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    write_word_bits(pointer.address(), raw, byte_len);

    Ok(())
}

/// Store bytes through a frame pointer.
#[inline(always)]
pub(crate) fn store_frame_pointer_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: FramePointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(access, bytes.len())?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "frame".to_string(),
                actual: format!("0x{:x}", pointer.address()),
            })?;
    if !state.owns_frame_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    unsafe {
        std::ptr::copy(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Store a value through a static value address.
#[inline(always)]
pub(crate) fn store_static_pointer(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), access.value_type, value)?;
    check_source_len(access, byte_len)?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "static".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_static_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    write_word_bits(pointer.address(), raw, byte_len);

    Ok(())
}

/// Store bytes through a static pointer.
#[inline(always)]
pub(crate) fn store_static_pointer_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(access, bytes.len())?;

    let pointer =
        pointer
            .add_bytes(access.byte_offset)
            .ok_or_else(|| Error::InvalidAddressSpace {
                expected: "static".to_string(),
                actual: "foreign".to_string(),
            })?;
    if !state.owns_static_range(pointer, access.byte_len) {
        return Err(Error::InvalidAddressSpace {
            expected: "static".to_string(),
            actual: "foreign".to_string(),
        });
    }

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Get the address of a field from a heap reference.
#[inline(always)]
pub(crate) fn field_addr_heap(
    state: &mut DispatchState<'_, '_>,
    handle: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let field_count_for_error = field_count_for_error(field_count, index as usize + 1);
    let handle = handle
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        })?;

    Ok(Word::heap_reference(handle))
}

/// Get the address of a field from a shared heap reference.
#[inline(always)]
pub(crate) fn field_addr_shared_heap(
    state: &mut DispatchState<'_, '_>,
    handle: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let field_count_for_error = field_count_for_error(field_count, index as usize + 1);
    let handle = handle
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        })?;

    Ok(Word::shared_heap_reference(handle))
}

/// Get the address of a field from a raw pointer.
#[inline(always)]
pub(crate) fn field_addr_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let field_count_for_error = field_count_for_error(field_count, index as usize + 1);
    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        })?;

    Ok(Word::raw_pointer(pointer))
}

/// Get the address of a field from a shared raw pointer.
#[inline(always)]
pub(crate) fn field_addr_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let field_count_for_error = field_count_for_error(field_count, index as usize + 1);
    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error,
        })?;

    Ok(Word::shared_raw_pointer(pointer))
}

/// Get the address of a field from a stack pointer.
#[inline(always)]
pub(crate) fn field_addr_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error(field_count, index as usize + 1),
        })?;

    stack_pointer_value(pointer)
}

/// Get the address of a field from a static pointer.
#[inline(always)]
pub(crate) fn field_addr_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error(field_count, index as usize + 1),
        })?;

    Ok(Word::static_pointer(pointer))
}

/// Get the address of an element from a heap reference.
#[inline(always)]
pub(crate) fn element_addr_heap(
    state: &mut DispatchState<'_, '_>,
    handle: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let handle = handle
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    Ok(Word::heap_reference(handle))
}

/// Get the address of an element from a shared heap reference.
#[inline(always)]
pub(crate) fn element_addr_shared_heap(
    state: &mut DispatchState<'_, '_>,
    handle: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let handle = handle
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    Ok(Word::shared_heap_reference(handle))
}

/// Get the address of an element from a raw pointer.
#[inline(always)]
pub(crate) fn element_addr_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // resolve byte offset
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    Ok(Word::raw_pointer(pointer))
}

/// Get the address of an element from a shared raw pointer.
#[inline(always)]
pub(crate) fn element_addr_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // compute the element byte offset
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    Ok(Word::shared_raw_pointer(pointer))
}

/// Get the address of an element from a stack pointer.
#[inline(always)]
pub(crate) fn element_addr_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // compute the element byte offset
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    stack_pointer_value(pointer)
}

/// Get the address of an element from a static pointer.
#[inline(always)]
pub(crate) fn element_addr_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    Ok(Word::static_pointer(pointer))
}

/// Load a field from a heap allocation.
#[inline(always)]
pub(crate) fn load_field_heap(
    state: &mut DispatchState<'_, '_>,
    handle: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    // validate the requested field before decoding bytes
    check_field_index(state, index, field_count)?;

    // reject null handles before reading the allocation bytes
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let access = PointeeAccess::from(field);
    check_word_access(access)?;
    let address = state
        .heap()
        .heap_address(handle, field.byte_offset, field.byte_len)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_field_reference(error, index, field_count_for_error(field_count, 0))
        })?;
    let raw = read_word_bits(address as usize, field.byte_len);

    decode_raw_bits(state.tree(), field.value_type, raw, field.byte_len)
}

/// Load a field from a shared heap allocation.
#[inline(always)]
pub(crate) fn load_field_shared_heap(
    state: &mut DispatchState<'_, '_>,
    handle: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_field_index(state, index, field_count)?;

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let access = PointeeAccess::from(field);
    check_word_access(access)?;
    let address = state
        .shared()
        .heap_address(handle, field.byte_offset, field.byte_len)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_field_reference(error, index, field_count_for_error(field_count, 0))
        })?;
    let raw = read_word_bits(address as usize, field.byte_len);

    decode_raw_bits(state.tree(), field.value_type, raw, field.byte_len)
}

/// Store a field into a heap allocation.
#[inline(always)]
pub(crate) fn store_field_heap(
    state: &mut DispatchState<'_, '_>,
    handle: HeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), field.value_type, value)?;
    check_source_len(field.into(), byte_len)?;

    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_field_index(state, index, field_count)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let allocation_len = state.heap().heap_byte_len(handle)?;
    let start = field.byte_offset;
    let end = start
        .checked_add(byte_len)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: allocation_len,
        })?;

    if bounds_checks && end > allocation_len {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: allocation_len,
        });
    }

    let address = state
        .heap_mut()
        .heap_address_mut(handle, start, byte_len)
        .map_err(Error::from)?;
    write_word_bits(address as usize, raw, byte_len);
    state
        .heap_mut()
        .write_barrier(handle, start, byte_len)
        .map_err(Error::from)?;

    Ok(())
}

/// Store a field into a shared heap allocation.
#[inline(always)]
pub(crate) fn store_field_shared_heap(
    state: &mut DispatchState<'_, '_>,
    handle: SharedHeapReference,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), field.value_type, value)?;
    check_source_len(field.into(), byte_len)?;

    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_field_index(state, index, field_count)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let allocation_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let start = field.byte_offset;
    let end = start
        .checked_add(byte_len)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: allocation_len,
        })?;

    if bounds_checks && end > allocation_len {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: allocation_len,
        });
    }

    let address = state
        .shared()
        .heap_address_mut(handle, start, byte_len)
        .map_err(Error::from)?;
    write_word_bits(address as usize, raw, byte_len);
    let bytes = raw.to_le_bytes();
    publish_shared_store(state, handle, start, &bytes[..byte_len])?;

    Ok(())
}

/// Load a field from a raw pointer.
#[inline(always)]
pub(crate) fn load_field_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = Word::raw_pointer(pointer);
    let access = PointeeAccess::from(field);

    load_raw_pointer(state, pointer, access)
}

/// Store a field through a raw pointer.
#[inline(always)]
pub(crate) fn store_field_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Word,
) -> Result<(), Error> {
    check_field_index(state, index, field_count)?;

    store_raw_pointer(state, Word::raw_pointer(pointer), field.into(), value)
}

/// Store field bytes through a raw pointer.
#[inline(always)]
pub(crate) fn store_field_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(field.into(), bytes.len())?;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = Word::raw_pointer(pointer);
    let access = PointeeAccess::from(field);

    store_raw_pointer_bytes(state, pointer, access, bytes)
}

/// Load a field through a shared raw pointer.
#[inline(always)]
pub(crate) fn load_field_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let pointer = Word::shared_raw_pointer(pointer);
    let access = PointeeAccess::from(field);

    load_shared_raw_pointer(state, pointer, access)
}

/// Store a field through a shared raw pointer.
#[inline(always)]
pub(crate) fn store_field_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Word,
) -> Result<(), Error> {
    check_field_index(state, index, field_count)?;

    store_shared_raw_pointer(
        state,
        Word::shared_raw_pointer(pointer),
        field.into(),
        value,
    )
}

/// Store field bytes through a shared raw pointer.
#[inline(always)]
pub(crate) fn store_field_shared_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(field.into(), bytes.len())?;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    let pointer = Word::shared_raw_pointer(pointer);
    let access = PointeeAccess::from(field);

    store_shared_raw_pointer_bytes(state, pointer, access, bytes)
}

/// Load a field from a stack allocation.
#[inline(always)]
pub(crate) fn load_field_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error(field_count, index as usize + 1),
        })?;

    let access = PointeeAccess::from(field);
    check_word_access(access)?;
    let raw = read_word_bits(pointer.address(), field.byte_len);

    decode_raw_bits(state.tree(), field.value_type, raw, field.byte_len)
}

/// Store a field into a stack allocation.
#[inline(always)]
pub(crate) fn store_field_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Word,
) -> Result<(), Error> {
    check_field_index(state, index, field_count)?;

    store_stack_pointer(state, pointer, field.into(), value)
}

/// Store field bytes into a stack allocation.
#[inline(always)]
pub(crate) fn store_field_stack_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(field.into(), bytes.len())?;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error(field_count, index as usize + 1),
        })?;
    unsafe {
        std::ptr::copy(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Load a field through a static pointer.
#[inline(always)]
pub(crate) fn load_field_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
) -> Result<Word, Error> {
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error(field_count, index as usize + 1),
        })?;

    let access = PointeeAccess::from(field);
    check_word_access(access)?;
    let raw = read_word_bits(pointer.address(), field.byte_len);

    decode_raw_bits(state.tree(), field.value_type, raw, field.byte_len)
}

/// Store a field through a static pointer.
#[inline(always)]
pub(crate) fn store_field_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    value: Word,
) -> Result<(), Error> {
    check_field_index(state, index, field_count)?;

    store_static_pointer(state, pointer, field.into(), value)
}

/// Store field bytes through a static pointer.
#[inline(always)]
pub(crate) fn store_field_static_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    field: FieldAccess,
    index: u32,
    field_count: Option<u32>,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(field.into(), bytes.len())?;

    // validate field index when known
    check_field_index(state, index, field_count)?;

    let pointer = pointer
        .add_bytes(field.byte_offset)
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: field_count_for_error(field_count, index as usize + 1),
        })?;
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Load an element from a heap allocation.
#[inline(always)]
pub(crate) fn load_element_heap(
    state: &mut DispatchState<'_, '_>,
    handle: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    // validate the requested element before decoding bytes
    check_array_index(state, index, array_length)?;

    // reject null handles before reading the allocation bytes
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // resolve the element bytes inside the allocation
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let access = PointeeAccess::from(element);
    check_word_access(access)?;
    let address = state
        .heap()
        .heap_address(handle, element_offset, element.byte_len)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_array_reference(error, index, array_length_for_error(array_length, 0))
        })?;
    let raw = read_word_bits(address as usize, element.byte_len);

    decode_raw_bits(state.tree(), element.value_type, raw, element.byte_len)
}

/// Load an element from a shared heap allocation.
#[inline(always)]
pub(crate) fn load_element_shared_heap(
    state: &mut DispatchState<'_, '_>,
    handle: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_array_index(state, index, array_length)?;

    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let access = PointeeAccess::from(element);
    check_word_access(access)?;
    let address = state
        .shared()
        .heap_address(handle, element_offset, element.byte_len)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_array_reference(error, index, array_length_for_error(array_length, 0))
        })?;
    let raw = read_word_bits(address as usize, element.byte_len);

    decode_raw_bits(state.tree(), element.value_type, raw, element.byte_len)
}

/// Store an element into a heap allocation.
#[inline(always)]
pub(crate) fn store_element_heap(
    state: &mut DispatchState<'_, '_>,
    handle: HeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), element.value_type, value)?;
    check_source_len(element.into(), byte_len)?;

    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_array_index(state, index, array_length)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let allocation_len = state.heap().heap_byte_len(handle)?;
    let end = element_offset
        .checked_add(byte_len)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: allocation_len as u64,
        })?;

    if bounds_checks && end > allocation_len {
        return Err(Error::InvalidArrayAccess {
            index,
            length: allocation_len as u64,
        });
    }

    let address = state
        .heap_mut()
        .heap_address_mut(handle, element_offset, byte_len)
        .map_err(Error::from)?;
    write_word_bits(address as usize, raw, byte_len);
    state
        .heap_mut()
        .write_barrier(handle, element_offset, byte_len)
        .map_err(Error::from)?;

    Ok(())
}

/// Store an element into a shared heap allocation.
#[inline(always)]
pub(crate) fn store_element_shared_heap(
    state: &mut DispatchState<'_, '_>,
    handle: SharedHeapReference,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), element.value_type, value)?;
    check_source_len(element.into(), byte_len)?;

    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_array_index(state, index, array_length)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let allocation_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let end = element_offset
        .checked_add(byte_len)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: allocation_len as u64,
        })?;

    if bounds_checks && end > allocation_len {
        return Err(Error::InvalidArrayAccess {
            index,
            length: allocation_len as u64,
        });
    }

    let address = state
        .shared()
        .heap_address_mut(handle, element_offset, byte_len)
        .map_err(Error::from)?;
    write_word_bits(address as usize, raw, byte_len);
    let bytes = raw.to_le_bytes();
    publish_shared_store(state, handle, element_offset, &bytes[..byte_len])?;

    Ok(())
}

/// Load an element from a raw pointer.
#[inline(always)]
pub(crate) fn load_element_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = Word::raw_pointer(pointer);
    let access = PointeeAccess::from(element);

    load_raw_pointer(state, pointer, access)
}

/// Store an element through a raw pointer.
#[inline(always)]
pub(crate) fn store_element_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Word,
) -> Result<(), Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    store_raw_pointer(state, Word::raw_pointer(pointer), element.into(), value)
}

/// Store element bytes through a raw pointer.
#[inline(always)]
pub(crate) fn store_element_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(element.into(), bytes.len())?;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = Word::raw_pointer(pointer);
    let access = PointeeAccess::from(element);

    store_raw_pointer_bytes(state, pointer, access, bytes)
}

/// Load an element through a shared raw pointer.
#[inline(always)]
pub(crate) fn load_element_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // compute the element offset before loading
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    load_shared_raw_pointer(state, Word::shared_raw_pointer(pointer), element.into())
}

/// Store an element through a shared raw pointer.
#[inline(always)]
pub(crate) fn store_element_shared_raw(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Word,
) -> Result<(), Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    store_shared_raw_pointer(
        state,
        Word::shared_raw_pointer(pointer),
        element.into(),
        value,
    )
}

/// Store element bytes through a shared raw pointer.
#[inline(always)]
pub(crate) fn store_element_shared_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: SharedRawPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(element.into(), bytes.len())?;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // compute the element offset before storing
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    store_shared_raw_pointer_bytes(
        state,
        Word::shared_raw_pointer(pointer),
        element.into(),
        bytes,
    )
}

/// Load an element from a stack allocation.
#[inline(always)]
pub(crate) fn load_element_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    // compute the element address
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    let access = PointeeAccess::from(element);
    check_word_access(access)?;
    let raw = read_word_bits(pointer.address(), element.byte_len);

    decode_raw_bits(state.tree(), element.value_type, raw, element.byte_len)
}

/// Store an element into a stack allocation.
#[inline(always)]
pub(crate) fn store_element_stack(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Word,
) -> Result<(), Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    store_stack_pointer(state, pointer, element.into(), value)
}

/// Store element bytes into a stack allocation.
#[inline(always)]
pub(crate) fn store_element_stack_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StackPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(element.into(), bytes.len())?;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    // compute the element offset before taking mutable borrows
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    unsafe {
        std::ptr::copy(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Load an element through a static pointer.
#[inline(always)]
pub(crate) fn load_element_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
) -> Result<Word, Error> {
    // validate array index when known
    check_array_index(state, index, array_length)?;

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    let access = PointeeAccess::from(element);
    check_word_access(access)?;
    let raw = read_word_bits(pointer.address(), element.byte_len);

    decode_raw_bits(state.tree(), element.value_type, raw, element.byte_len)
}

/// Store an element through a static pointer.
#[inline(always)]
pub(crate) fn store_element_static(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    value: Word,
) -> Result<(), Error> {
    check_array_index(state, index, array_length)?;

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;

    store_static_pointer(state, pointer, element.into(), value)
}

/// Store element bytes through a static pointer.
#[inline(always)]
pub(crate) fn store_element_static_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: StaticPointer,
    element: ElementAccess,
    index: u64,
    array_length: Option<u64>,
    bytes: &[u8],
) -> Result<(), Error> {
    check_source_len(element.into(), bytes.len())?;

    // validate array index when known
    check_array_index(state, index, array_length)?;

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let pointer = pointer
        .add_bytes(element_offset)
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
    }

    Ok(())
}

/// Get a field from an addressable value.
#[inline(always)]
pub(crate) fn get_field(
    state: &mut DispatchState<'_, '_>,
    base: Word,
    index: u32,
    field_count: Option<u32>,
    field: Option<FieldAccess>,
) -> Result<Word, Error> {
    let Some(field) = field else {
        return Err(Error::TypeMismatch {
            expected: "field access".to_string(),
            actual: format!("{base:?}"),
        });
    };

    match field.pointer_class {
        PointerClass::Heap => {
            let handle = heap_reference_from_value(base);
            load_field_heap(state, handle, field, index, field_count)
        }
        PointerClass::SharedHeap => {
            let handle = shared_heap_reference_from_value(base);
            load_field_shared_heap(state, handle, field, index, field_count)
        }
        PointerClass::Raw => {
            let pointer = RawPointer::from_bits(base.bits() as usize);
            load_field_raw(state, pointer, field, index, field_count)
        }
        PointerClass::SharedRaw => {
            check_field_index(state, index, field_count)?;
            let pointer = Word::shared_raw_pointer(base.as_shared_raw_pointer());
            load_shared_raw_pointer(state, pointer, field.into())
        }
        PointerClass::Stack => {
            let pointer = StackPointer::from_address(base.bits() as usize);
            load_field_stack(state, pointer, field, index, field_count)
        }
        PointerClass::Frame => {
            let pointer = frame_pointer_from_value(base);

            load_frame_pointer(state, pointer, field.into())
        }
        PointerClass::Static => {
            let pointer = static_pointer_from_value(base);

            load_static_pointer(state, pointer, field.into())
        }
        _ => Err(Error::TypeMismatch {
            expected: "field source".to_string(),
            actual: format!("{base:?}"),
        }),
    }
}

/// Get an element from a heap array reference.
#[inline(always)]
pub(crate) fn get_element(
    state: &mut DispatchState<'_, '_>,
    arr: Word,
    index: u64,
    array_length: Option<u64>,
    element: Option<ElementAccess>,
) -> Result<Word, Error> {
    let Some(element) = element else {
        return Err(Error::TypeMismatch {
            expected: "element access".to_string(),
            actual: format!("{arr:?}"),
        });
    };

    match element.pointer_class {
        PointerClass::Heap => {
            let handle = heap_reference_from_value(arr);
            load_element_heap(state, handle, element, index, array_length)
        }
        PointerClass::SharedHeap => {
            let handle = shared_heap_reference_from_value(arr);
            load_element_shared_heap(state, handle, element, index, array_length)
        }
        PointerClass::Raw => {
            let pointer = RawPointer::from_bits(arr.bits() as usize);
            load_element_raw(state, pointer, element, index, array_length)
        }
        PointerClass::SharedRaw => {
            check_array_index(state, index, array_length)?;
            let offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element.byte_stride))
                .ok_or(Error::InvalidArrayAccess {
                    index,
                    length: array_length_for_error(array_length, 0),
                })?;
            let pointer =
                arr.as_shared_raw_pointer()
                    .add_bytes(offset)
                    .ok_or(Error::InvalidArrayAccess {
                        index,
                        length: array_length_for_error(array_length, 0),
                    })?;

            load_shared_raw_pointer(state, Word::shared_raw_pointer(pointer), element.into())
        }
        PointerClass::Stack => {
            let pointer = StackPointer::from_address(arr.bits() as usize);
            load_element_stack(state, pointer, element, index, array_length)
        }
        PointerClass::Frame => {
            check_array_index(state, index, array_length)?;
            let offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element.byte_stride))
                .ok_or(Error::InvalidArrayAccess {
                    index,
                    length: array_length_for_error(array_length, 0),
                })?;
            let pointer = frame_pointer_from_value(arr).add_bytes(offset).ok_or(
                Error::InvalidArrayAccess {
                    index,
                    length: array_length_for_error(array_length, 0),
                },
            )?;

            load_frame_pointer(state, pointer, element.into())
        }
        PointerClass::Static => {
            check_array_index(state, index, array_length)?;
            let offset = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(element.byte_stride))
                .ok_or(Error::InvalidArrayAccess {
                    index,
                    length: array_length_for_error(array_length, 0),
                })?;
            let pointer = static_pointer_from_value(arr).add_bytes(offset).ok_or(
                Error::InvalidArrayAccess {
                    index,
                    length: array_length_for_error(array_length, 0),
                },
            )?;

            load_static_pointer(state, pointer, element.into())
        }
        _ => Err(Error::TypeMismatch {
            expected: "element source".to_string(),
            actual: format!("{arr:?}"),
        }),
    }
}
