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
    allocate_callable, decode_callable, decode_raw_value, encode_raw_value, encode_value_bytes,
    load_raw_pointer, load_shared_raw_pointer, raw_type_size, store_raw_pointer,
    store_shared_raw_pointer,
};

/// Build one invalid-pointer-type error for the given value.
#[inline(always)]
pub(super) fn invalid_pointer_type(value: Word) -> Error {
    Error::InvalidPointerType {
        actual: format!("{value:?}"),
    }
}

/// Build one non-scalar value load error.
#[inline(always)]
fn non_scalar_load_error(ty: mir::LocalNodeId<mir::Type>) -> Error {
    Error::TypeMismatch {
        expected: "value-representable type".to_string(),
        actual: format!("{ty:?}"),
    }
}

/// Return a one-word scratch slice for one scalar access.
#[inline(always)]
fn scalar_bytes(access: PointeeAccess) -> Result<[u8; Word::BYTE_LEN], Error> {
    if !access.is_scalar {
        return Err(non_scalar_load_error(access.value_type));
    }
    if access.byte_len > Word::BYTE_LEN {
        return Err(Error::InvalidInstruction);
    }

    Ok([0u8; Word::BYTE_LEN])
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

/// Copy one native byte range into a destination.
#[inline(always)]
fn copy_address_bytes_into(address: usize, target: *mut u8, target_len: usize) {
    unsafe {
        std::ptr::copy_nonoverlapping(address as *const u8, target, target_len);
    }
}

/// Load bytes from a heap reference.
#[inline(always)]
pub(crate) fn load_heap_reference_bytes_into(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    target: *mut u8,
    target_len: usize,
) -> Result<(), Error> {
    check_target_len(access, target_len)?;

    let handle = heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }
    if !state.heap().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    let target = unsafe { std::slice::from_raw_parts_mut(target, target_len) };
    state
        .heap()
        .read_heap_bytes_into(handle, access.byte_offset, target)
        .map_err(Error::from)
}

/// Load bytes from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_reference_bytes_into(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    target: *mut u8,
    target_len: usize,
) -> Result<(), Error> {
    check_target_len(access, target_len)?;

    let handle = shared_heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }
    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    let target = unsafe { std::slice::from_raw_parts_mut(target, target_len) };
    state
        .shared()
        .read_heap_bytes_into(handle, access.byte_offset, target)
        .map_err(Error::from)
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

    copy_address_bytes_into(pointer.address(), target, target_len);
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

    copy_address_bytes_into(pointer.address(), target, target_len);
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

    copy_address_bytes_into(pointer.address(), target, target_len);
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

    // decode through one word scratch buffer
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..access.byte_len];
    state
        .heap()
        .read_heap_bytes_into(handle, access.byte_offset, bytes)
        .map_err(Error::from)?;

    decode_raw_value(state.tree(), access.value_type, bytes)
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

    // decode through one word scratch buffer
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..access.byte_len];
    state
        .shared()
        .read_heap_bytes_into(handle, access.byte_offset, bytes)
        .map_err(Error::from)?;

    decode_raw_value(state.tree(), access.value_type, bytes)
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

    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..access.byte_len];
    copy_address_bytes_into(pointer.address(), bytes.as_mut_ptr(), bytes.len());

    decode_raw_value(state.tree(), access.value_type, bytes)
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

    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..access.byte_len];
    copy_address_bytes_into(pointer.address(), bytes.as_mut_ptr(), bytes.len());

    decode_raw_value(state.tree(), access.value_type, bytes)
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

    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..access.byte_len];
    copy_address_bytes_into(pointer.address(), bytes.as_mut_ptr(), bytes.len());

    decode_raw_value(state.tree(), access.value_type, bytes)
}

/// Store a value through a heap reference.
#[inline(always)]
pub(crate) fn store_heap_reference(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    val: Word,
) -> Result<(), Error> {
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

    let bytes = encode_value_bytes(state, access.value_type, val)?;
    let byte_len = state.heap().heap_byte_len(handle)?;
    let start = access.byte_offset;
    let end = start.saturating_add(bytes.len());

    if end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: byte_len,
        });
    }

    state
        .write_heap_bytes(handle, start, &bytes)
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
    let handle = shared_heap_reference_from_value(ptr);
    if state.null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    {
        if !state.shared().is_heap_live(handle) {
            return Err(Error::InvalidHeapReference);
        }
    }

    let bytes = encode_value_bytes(state, access.value_type, val)?;
    let byte_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let start = access.byte_offset;
    let end = start.saturating_add(bytes.len());

    if end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index: start as u32,
            field_count: byte_len,
        });
    }

    state
        .shared()
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;
    publish_shared_store(state, handle, start, &bytes)?;

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

    // encode the typed value
    let bytes = encode_value_bytes(state, access.value_type, value)?;

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

    let bytes = encode_value_bytes(state, access.value_type, value)?;
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.address() as *mut u8, bytes.len());
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

    let bytes = encode_value_bytes(state, access.value_type, value)?;
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

    // decode through one word scratch buffer
    let access = PointeeAccess::from(field);
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..field.byte_len];
    state
        .heap()
        .read_heap_bytes_into(handle, field.byte_offset, bytes)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_field_reference(error, index, field_count_for_error(field_count, 0))
        })?;

    decode_raw_value(state.tree(), field.value_type, bytes)
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
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..field.byte_len];
    state
        .shared()
        .read_heap_bytes_into(handle, field.byte_offset, bytes)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_field_reference(error, index, field_count_for_error(field_count, 0))
        })?;

    decode_raw_value(state.tree(), field.value_type, bytes)
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
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;
    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    // validate the requested field before encoding bytes
    check_field_index(state, index, field_count)?;

    // reject null handles before writing the allocation bytes
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // encode the field value into managed bytes
    let bytes = encode_value_bytes(state, field.value_type, value)?;
    let byte_len = state.heap().heap_byte_len(handle)?;
    let start = field.byte_offset;
    let end = start
        .checked_add(bytes.len())
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: byte_len,
        })?;

    if bounds_checks && end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: byte_len,
        });
    }

    // write the encoded field bytes
    state
        .write_heap_bytes(handle, start, &bytes)
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
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;

    if !state.shared().is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    check_field_index(state, index, field_count)?;

    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let bytes = encode_value_bytes(state, field.value_type, value)?;
    let byte_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let start = field.byte_offset;
    let end = start
        .checked_add(bytes.len())
        .ok_or(Error::InvalidFieldAccess {
            index,
            field_count: byte_len,
        })?;

    if bounds_checks && end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index,
            field_count: byte_len,
        });
    }

    state
        .shared()
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;
    publish_shared_store(state, handle, start, &bytes)?;

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
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // reject null pointers when enabled
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let pointer = Word::raw_pointer(pointer);
    let access = PointeeAccess::from(field);

    store_raw_pointer(state, pointer, access, value)
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
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..field.byte_len];
    copy_address_bytes_into(pointer.address(), bytes.as_mut_ptr(), bytes.len());

    decode_raw_value(state.tree(), field.value_type, bytes)
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
    // validate field index when known
    check_field_index(state, index, field_count)?;

    // encode the typed field value
    let bytes = encode_value_bytes(state, field.value_type, value)?;

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
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..field.byte_len];
    copy_address_bytes_into(pointer.address(), bytes.as_mut_ptr(), bytes.len());

    decode_raw_value(state.tree(), field.value_type, bytes)
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
    // validate field index when known
    check_field_index(state, index, field_count)?;

    let bytes = encode_value_bytes(state, field.value_type, value)?;
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
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..element.byte_len];
    state
        .heap()
        .read_heap_bytes_into(handle, element_offset, bytes)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_array_reference(error, index, array_length_for_error(array_length, 0))
        })?;

    decode_raw_value(state.tree(), element.value_type, bytes)
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
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..element.byte_len];
    state
        .shared()
        .read_heap_bytes_into(handle, element_offset, bytes)
        .map_err(Error::from)
        .map_err(|error| {
            map_invalid_array_reference(error, index, array_length_for_error(array_length, 0))
        })?;

    decode_raw_value(state.tree(), element.value_type, bytes)
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
    let bounds_checks = state.bounds_checks;
    let null_checks = state.null_checks;
    // require one live heap allocation
    let heap = state.heap();
    if !heap.is_heap_live(handle) {
        return Err(Error::InvalidHeapReference);
    }

    // validate the requested element before encoding bytes
    check_array_index(state, index, array_length)?;

    // reject null handles before writing the allocation bytes
    if null_checks && handle.is_null() {
        return Err(Error::NullPointerDereference);
    }

    // encode the element value into managed bytes
    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let bytes = encode_value_bytes(state, element.value_type, value)?;
    let allocation_len = state.heap().heap_byte_len(handle)?;
    let start = element_offset;
    let end = start
        .checked_add(bytes.len())
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

    // write the encoded element bytes
    state
        .write_heap_bytes(handle, start, &bytes)
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
    let bytes = encode_value_bytes(state, element.value_type, value)?;
    let allocation_len = state
        .shared_ref()
        .heap_byte_len(handle)
        .map_err(Error::from)?;
    let start = element_offset;
    let end = start
        .checked_add(bytes.len())
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

    state
        .shared()
        .write_heap_bytes(handle, start, &bytes)
        .map_err(Error::from)?;
    publish_shared_store(state, handle, start, &bytes)?;

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

    store_raw_pointer(state, pointer, access, value)
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

    // resolve the typed element bytes
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
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..element.byte_len];
    copy_address_bytes_into(pointer.address(), bytes.as_mut_ptr(), bytes.len());

    decode_raw_value(state.tree(), element.value_type, bytes)
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

    // encode the typed element value
    let bytes = encode_value_bytes(state, element.value_type, value)?;

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
    let mut bytes = scalar_bytes(access)?;
    let bytes = &mut bytes[..element.byte_len];
    copy_address_bytes_into(pointer.address(), bytes.as_mut_ptr(), bytes.len());

    decode_raw_value(state.tree(), element.value_type, bytes)
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
    // validate array index when known
    check_array_index(state, index, array_length)?;

    let element_offset = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_mul(element.byte_stride))
        .ok_or(Error::InvalidArrayAccess {
            index,
            length: array_length_for_error(array_length, 0),
        })?;
    let bytes = encode_value_bytes(state, element.value_type, value)?;
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
            expected: "typed field access".to_string(),
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
            expected: "typed element access".to_string(),
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
