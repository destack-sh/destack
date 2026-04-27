use crate::diagnostic::Error;
use crate::program::{PointeeAccess, repr_type};
use crate::{FunctionPointer, RawPointer, Word};
use destack_heap::{HeapReference, Payload};
use destack_mir as mir;

use super::access::decode_pointer_bits;
use crate::interpreter::DispatchState;

/// Encoded bytes for one VM word.
pub(crate) struct WordBytes {
    /// The byte buffer.
    bytes: [u8; Word::BYTE_LEN],
    /// The number of initialized bytes.
    len: usize,
}

impl WordBytes {
    /// Return the initialized bytes.
    #[inline(always)]
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Return the initialized byte count.
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.len
    }
}

/// Align one byte offset up to the requested alignment.
#[inline(always)]
fn align_offset(offset: usize, alignment: usize) -> usize {
    if alignment <= 1 {
        return offset;
    }

    let remainder = offset % alignment;
    if remainder == 0 {
        offset
    } else {
        offset + (alignment - remainder)
    }
}

/// Return the callable box byte layout.
fn callable_box_layout(tree: &mir::Tree) -> (usize, usize, usize) {
    let pointer_bytes = tree.pointer_bytes() as usize;
    let function_offset = 0usize;
    let environment_offset = align_offset(pointer_bytes, pointer_bytes);
    let byte_len = environment_offset + pointer_bytes;

    (function_offset, environment_offset, byte_len)
}

/// Return the environment type for one bound function.
fn callable_environment_type(
    tree: &mir::Tree,
    function_id: mir::LocalNodeId<mir::Function>,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let function = tree.get(function_id);
    let environment = function.environment.ok_or(Error::InvalidInstruction)?;

    environment.ty().ok_or_else(|| Error::ConcreteMirRequired {
        context: "callable environment type".to_string(),
    })
}

/// Decode one callable box into function and environment values.
fn decode_callable_box(
    state: &mut DispatchState<'_, '_>,
    handle: HeapReference,
) -> Result<(Word, Word), Error> {
    let (function_offset, environment_offset, byte_len) = callable_box_layout(state.tree());
    let bytes = state
        .read_heap_bytes(handle, 0, byte_len)
        .map_err(Error::from)?;

    if bytes.len() != byte_len {
        return Err(Error::InvalidHeapReference);
    }

    let function_byte_len = state.tree().pointer_bytes() as usize;
    let function_end = function_offset
        .checked_add(function_byte_len)
        .ok_or(Error::InvalidHeapReference)?;
    let environment_end = environment_offset
        .checked_add(function_byte_len)
        .ok_or(Error::InvalidHeapReference)?;
    let function_bytes = bytes
        .get(function_offset..function_end)
        .ok_or(Error::InvalidHeapReference)?;
    let environment_bytes = bytes
        .get(environment_offset..environment_end)
        .ok_or(Error::InvalidHeapReference)?;

    let mut function_raw = [0u8; 8];
    function_raw[..function_bytes.len()].copy_from_slice(function_bytes);
    let function = FunctionPointer::from_bits(u64::from_le_bytes(function_raw) as usize);
    let function_id = mir::LocalNodeId::new(function.function_index());
    let function = Word::function_pointer(function);

    let mut environment_raw = [0u8; 8];
    environment_raw[..environment_bytes.len()].copy_from_slice(environment_bytes);
    let environment_type = callable_environment_type(state.tree(), function_id)?;
    let environment_value = if state.layout(environment_type)?.is_scalar() {
        decode_raw_value(state.tree(), environment_type, environment_bytes)?
    } else {
        let environment_handle =
            HeapReference::from_bits(u64::from_le_bytes(environment_raw) as usize);

        Word::heap_reference(environment_handle)
    };

    Ok((function, environment_value))
}

/// Encode one function entry argument into frame bytes.
pub(crate) fn encode_argument_bytes(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<Vec<u8>, Error> {
    let layout = state.layout(ty)?.clone();

    // scalars
    if layout.is_scalar() {
        return Ok(encode_word_bytes(state.tree(), ty, value)?
            .as_slice()
            .to_vec());
    }

    let mut bytes = vec![0u8; layout.byte_len];
    write_argument_bytes(state, ty, value, &mut bytes)?;

    Ok(bytes)
}

/// Encode one SSA value into its byte representation.
pub(crate) fn encode_frame_value_bytes(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: mir::Value,
) -> Result<Vec<u8>, Error> {
    let layout = state.layout(ty)?.clone();
    if layout.is_scalar() {
        return Ok(encode_word_bytes(state.tree(), ty, state.get(value))?
            .as_slice()
            .to_vec());
    }

    let bytes = state.value_bytes(value)?;
    if bytes.len() != layout.byte_len {
        return Err(Error::TypeMismatch {
            expected: format!("{} value bytes", layout.byte_len),
            actual: format!("{} value bytes", bytes.len()),
        });
    }

    Ok(bytes.to_vec())
}

/// Return one frame byte range with the expected size.
#[inline(always)]
pub(crate) fn checked_frame_value_byte_range(
    state: &DispatchState<'_, '_>,
    value: mir::Value,
    byte_len: usize,
) -> Result<(*const u8, usize), Error> {
    let (bytes, actual_byte_len) = state.frame_value_byte_range(value)?;
    if actual_byte_len != byte_len {
        return Err(Error::InvalidInstruction);
    }

    Ok((bytes, actual_byte_len))
}

/// Encode one callable environment value.
fn encode_callable_environment(
    state: &mut DispatchState<'_, '_>,
    function_id: mir::LocalNodeId<mir::Function>,
    environment_value: mir::Value,
) -> Result<Vec<u8>, Error> {
    let environment_type = callable_environment_type(state.tree(), function_id)?;
    if state.layout(environment_type)?.is_scalar() {
        return Ok(encode_word_bytes(
            state.tree(),
            environment_type,
            state.get(environment_value),
        )?
        .as_slice()
        .to_vec());
    }

    let environment_layout_id = state
        .program
        .layout_id_for_type(environment_type)
        .ok_or(Error::InvalidInstruction)?;
    let environment_bytes = encode_frame_value_bytes(state, environment_type, environment_value)?;
    let environment_handle =
        state.allocate_heap_layout(environment_layout_id, Payload::Bytes(&environment_bytes))?;

    Ok(
        (environment_handle.bits() as u64).to_le_bytes()[..state.tree().pointer_bytes() as usize]
            .to_vec(),
    )
}

/// Allocate one callable box from function and environment values.
pub(crate) fn allocate_callable(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    function: Word,
    environment_value: mir::Value,
) -> Result<Word, Error> {
    let (function_offset, environment_offset, byte_len) = callable_box_layout(state.tree());
    let function = function.as_function_pointer();
    let function_id = mir::LocalNodeId::new(function.function_index());
    let function_bytes = (function.bits() as u64).to_le_bytes();
    let environment_bytes = encode_callable_environment(state, function_id, environment_value)?;
    let mut bytes = vec![0; byte_len];

    let function_end = function_offset
        .checked_add(state.tree().pointer_bytes() as usize)
        .ok_or(Error::InvalidHeapReference)?;
    let environment_end = environment_offset
        .checked_add(state.tree().pointer_bytes() as usize)
        .ok_or(Error::InvalidHeapReference)?;
    if function_end > bytes.len() || environment_end > bytes.len() {
        return Err(Error::InvalidHeapReference);
    }

    bytes[function_offset..function_end]
        .copy_from_slice(&function_bytes[..state.tree().pointer_bytes() as usize]);
    bytes[environment_offset..environment_end].copy_from_slice(&environment_bytes);

    let layout_id = state
        .program
        .layout_id_for_type(ty)
        .ok_or(Error::InvalidInstruction)?;
    let handle = state.allocate_heap_layout(layout_id, Payload::Bytes(&bytes))?;

    Ok(Word::heap_reference(handle))
}

/// Validate a field index against a known field count.
#[inline(always)]
pub(super) fn check_field_index(
    state: &DispatchState<'_, '_>,
    index: u32,
    field_count: Option<u32>,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // skip checks when the field count is unknown
    let Some(field_count) = field_count else {
        return Ok(());
    };

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
pub(super) fn check_array_index(
    state: &DispatchState<'_, '_>,
    index: u64,
    array_length: Option<u64>,
) -> Result<(), Error> {
    // skip checks when bounds are disabled
    if !state.bounds_checks {
        return Ok(());
    }

    // skip checks when the length is unknown
    let Some(array_length) = array_length else {
        return Ok(());
    };

    // reject out of bounds indices
    if index >= array_length {
        return Err(Error::InvalidArrayAccess {
            index,
            length: array_length,
        });
    }

    Ok(())
}

/// Return the byte width of a type that fits in one VM word.
pub(crate) fn word_type_byte_len(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<usize, Error> {
    let ty = repr_type(tree, ty);
    let size = match tree.get(ty) {
        mir::Type::Void => 0,
        mir::Type::Boolean => 1,
        mir::Type::Int { width, .. } => (*width as usize).div_ceil(8),
        mir::Type::Isize
        | mir::Type::Usize
        | mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::Callable { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::FunctionSignature { .. }
        | mir::Type::TensorView { .. } => tree.pointer_bytes() as usize,
        mir::Type::Float { width } => (*width as usize).div_ceil(8),
        mir::Type::Newtype { .. } => {
            return Err(Error::TypeMismatch {
                expected: "runtime representation type".to_string(),
                actual: format!("{ty:?}"),
            });
        }
        mir::Type::Array { .. }
        | mir::Type::Slice { .. }
        | mir::Type::Tuple { .. }
        | mir::Type::Struct { .. }
        | mir::Type::Vector { .. }
        | mir::Type::Tensor { .. } => {
            return Err(Error::TypeMismatch {
                expected: "word-sized type".to_string(),
                actual: format!("{ty:?}"),
            });
        }
    };

    Ok(size)
}

/// Write one raw byte range from the given buffer.
fn write_raw_bytes(
    state: &mut DispatchState<'_, '_>,
    pointer: RawPointer,
    byte_offset: usize,
    bytes: &[u8],
) -> Result<(), Error> {
    let byte_len = state.heap().raw_byte_len(pointer)?;
    let end = byte_offset
        .checked_add(bytes.len())
        .ok_or(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: byte_len,
        })?;

    if end > byte_len {
        return Err(Error::InvalidFieldAccess {
            index: byte_offset as u32,
            field_count: byte_len,
        });
    }

    state
        .heap_mut()
        .write_raw_bytes(pointer, byte_offset, bytes)
        .map_err(Error::from)?;

    Ok(())
}

/// Decode one raw byte range into a VM value.
pub(crate) fn decode_raw_value(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    bytes: &[u8],
) -> Result<Word, Error> {
    let byte_len = word_type_byte_len(tree, ty)?;
    if bytes.len() != byte_len {
        return Err(Error::TypeMismatch {
            expected: format!("{byte_len} raw bytes"),
            actual: format!("{} raw bytes", bytes.len()),
        });
    }

    let mut raw = [0u8; Word::BYTE_LEN];
    raw[..bytes.len()].copy_from_slice(bytes);
    let raw = u64::from_le_bytes(raw);

    decode_raw_bits(tree, ty, raw, byte_len)
}

/// Decode one raw scalar bit pattern into a VM word.
pub(crate) fn decode_raw_bits(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    raw: u64,
    byte_len: usize,
) -> Result<Word, Error> {
    let expected_byte_len = word_type_byte_len(tree, ty)?;
    if byte_len != expected_byte_len {
        return Err(Error::TypeMismatch {
            expected: format!("{expected_byte_len} raw bytes"),
            actual: format!("{byte_len} raw bytes"),
        });
    }

    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Void => Ok(Word::VOID),
        mir::Type::Boolean => Ok(Word::bool(raw != 0)),
        mir::Type::Int { width, is_signed } => {
            if *is_signed {
                Ok(Word::int(raw as i64, *width as u8))
            } else {
                Ok(Word::uint(raw, *width as u8))
            }
        }
        mir::Type::Isize => Ok(Word::int(raw as i64, tree.pointer_bits() as u8)),
        mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
            Ok(Word::uint(raw, tree.pointer_bits() as u8))
        }
        mir::Type::Float { width: 32 } => Ok(Word::float32(f32::from_bits(raw as u32))),
        mir::Type::Float { width: 64 } => Ok(Word::float64(f64::from_bits(raw))),
        mir::Type::Float { width } => Err(Error::TypeMismatch {
            expected: "supported float width".to_string(),
            actual: width.to_string(),
        }),
        mir::Type::Reference { .. } => decode_pointer_bits(raw, tree.get(ty)),
        mir::Type::Callable { .. } => {
            Ok(Word::heap_reference(HeapReference::from_bits(raw as usize)))
        }
        mir::Type::FunctionSignature { .. } => Ok(Word::function_pointer(
            FunctionPointer::from_bits(raw as usize),
        )),
        mir::Type::FunctionPointer { .. } => decode_pointer_bits(raw, tree.get(ty)),
        mir::Type::Newtype { .. } => Err(Error::TypeMismatch {
            expected: "runtime representation type".to_string(),
            actual: format!("{ty:?}"),
        }),
        _ => Err(Error::TypeMismatch {
            expected: "scalar or reference raw load".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Write one field-shaped value into destination frame bytes.
pub(crate) fn write_frame_fields<F>(
    state: &mut DispatchState<'_, '_>,
    destination: mir::Value,
    mut field_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut DispatchState<'_, '_>, u32, mir::LocalNodeId<mir::Type>) -> Result<Word, Error>,
{
    let ty = state.value_type(destination)?;
    let layout = state.layout(ty)?.clone();
    let field_count = layout.field_count().ok_or(Error::TypeMismatch {
        expected: "field-shaped frame value".to_string(),
        actual: format!("{ty:?}"),
    })?;
    let mut bytes = vec![0u8; layout.byte_len];

    // encode each field into its physical byte range
    for index in 0..field_count {
        let index = index as u32;
        let field = layout
            .field(index)
            .ok_or(Error::InvalidFieldAccess { index, field_count })?;
        let value = field_value(state, index, field.ty)?;
        let value_end =
            field
                .offset
                .checked_add(field.byte_len)
                .ok_or(Error::InvalidFieldAccess {
                    index,
                    field_count: layout.byte_len,
                })?;
        let value_window =
            bytes
                .get_mut(field.offset..value_end)
                .ok_or(Error::InvalidFieldAccess {
                    index,
                    field_count: layout.byte_len,
                })?;

        let value_bytes = encode_word_bytes(state.tree(), field.ty, value)?;
        if value_bytes.len() != value_window.len() {
            return Err(Error::InvalidInstruction);
        }
        value_window.copy_from_slice(value_bytes.as_slice());
    }

    let destination_bytes = state.value_bytes_mut(destination)?;
    if destination_bytes.len() != bytes.len() {
        return Err(Error::InvalidInstruction);
    }
    destination_bytes.copy_from_slice(&bytes);

    Ok(())
}

/// Write one indexed value into destination frame bytes.
pub(crate) fn write_frame_elements<F>(
    state: &mut DispatchState<'_, '_>,
    destination: mir::Value,
    mut element_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut DispatchState<'_, '_>, usize, mir::LocalNodeId<mir::Type>) -> Result<Word, Error>,
{
    let ty = state.value_type(destination)?;
    let layout = state.layout(ty)?.clone();
    let element = layout.element().ok_or(Error::TypeMismatch {
        expected: "indexed frame value".to_string(),
        actual: format!("{ty:?}"),
    })?;
    let element_count = layout.element_count().ok_or(Error::InvalidInstruction)?;
    let mut bytes = vec![0u8; layout.byte_len];

    // encode each element into its physical byte range
    for index in 0..element_count {
        let offset = element
            .stride
            .checked_mul(index)
            .ok_or(Error::InvalidArrayAccess {
                index: index as u64,
                length: element_count as u64,
            })?;
        let value_end = offset
            .checked_add(element.byte_len)
            .ok_or(Error::InvalidArrayAccess {
                index: index as u64,
                length: element_count as u64,
            })?;
        let value_window = bytes
            .get_mut(offset..value_end)
            .ok_or(Error::InvalidArrayAccess {
                index: index as u64,
                length: element_count as u64,
            })?;

        let value = element_value(state, index, element.ty)?;
        let value_bytes = encode_word_bytes(state.tree(), element.ty, value)?;
        if value_bytes.len() != value_window.len() {
            return Err(Error::InvalidInstruction);
        }
        value_window.copy_from_slice(value_bytes.as_slice());
    }

    let destination_bytes = state.value_bytes_mut(destination)?;
    if destination_bytes.len() != bytes.len() {
        return Err(Error::InvalidInstruction);
    }
    destination_bytes.copy_from_slice(&bytes);

    Ok(())
}

/// Encode one VM word into raw bytes for the given type.
fn encode_word_raw_bytes(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<WordBytes, Error> {
    let (raw, byte_len) = encode_word_bits(tree, ty, value)?;

    Ok(WordBytes {
        bytes: raw.to_le_bytes(),
        len: byte_len,
    })
}

/// Encode one VM word into raw bits for the given type.
pub(crate) fn encode_word_bits(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<(u64, usize), Error> {
    let byte_len = word_type_byte_len(tree, ty)?;

    let raw = match tree.get(ty) {
        mir::Type::Void => 0u64,
        mir::Type::Boolean => u64::from(value.as_bool()),
        mir::Type::Int { .. } | mir::Type::Isize => value.bits(),
        mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => value.as_uint(),
        mir::Type::Float { width: 32 } => {
            let raw = value.as_float32();
            raw.to_bits() as u64
        }
        mir::Type::Float { width: 64 } => {
            let raw = value.as_float64();
            raw.to_bits()
        }
        mir::Type::Float { width } => {
            return Err(Error::TypeMismatch {
                expected: "supported float width".to_string(),
                actual: width.to_string(),
            });
        }
        mir::Type::Reference { .. } => value.bits(),
        mir::Type::Callable { .. } => value.as_heap_reference().bits() as u64,
        mir::Type::FunctionSignature { .. } | mir::Type::FunctionPointer { .. } => {
            let raw = value.as_function_pointer();
            raw.bits() as u64
        }
        mir::Type::Newtype { inner, .. } => {
            let inner = (*inner).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "newtype inner".to_string(),
            })?;
            return encode_word_bits(tree, inner, value);
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: "scalar or reference raw store".to_string(),
                actual: format!("{ty:?}"),
            });
        }
    };

    Ok((raw, byte_len))
}

/// Encode one VM value into raw bytes for the given type.
pub(crate) fn encode_raw_value(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<Vec<u8>, Error> {
    Ok(encode_word_raw_bytes(tree, ty, value)?.as_slice().to_vec())
}

/// Encode one VM word into scalar bytes.
pub(crate) fn encode_word_bytes(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<WordBytes, Error> {
    encode_word_raw_bytes(tree, ty, value)
}

/// Write one function entry argument into one byte range.
fn write_argument_bytes(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
    destination: &mut [u8],
) -> Result<(), Error> {
    if state.layout(ty)?.is_scalar() {
        let bytes = encode_word_bytes(state.tree(), ty, value)?;
        if bytes.len() != destination.len() {
            return Err(Error::InvalidHeapReference);
        }

        destination.copy_from_slice(bytes.as_slice());
        return Ok(());
    }

    let handle = value.as_heap_reference();
    if state.heap().is_heap_live(handle) {
        state
            .heap()
            .read_heap_bytes_into(handle, 0, destination)
            .map_err(Error::from)?;

        return Ok(());
    }

    let handle = value.as_shared_heap_reference();
    if state.shared().is_heap_live(handle) {
        state
            .shared()
            .read_heap_bytes_into(handle, 0, destination)
            .map_err(Error::from)?;

        return Ok(());
    }

    Err(Error::TypeMismatch {
        expected: "scalar or heap-backed argument".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Decode one callable box into function and environment values.
pub(crate) fn decode_callable(
    state: &mut DispatchState<'_, '_>,
    value: Word,
) -> Result<(Word, Word), Error> {
    let handle = value.as_heap_reference();
    if state.heap().is_heap_live(handle) {
        return decode_callable_box(state, handle);
    }

    Err(Error::TypeMismatch {
        expected: "boxed callable".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Load one value from raw heap bytes.
pub(crate) fn load_raw_pointer(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer = ptr.as_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    if !access.is_scalar {
        return Err(Error::TypeMismatch {
            expected: "word raw load".to_string(),
            actual: format!("{:?}", access.value_type),
        });
    }
    if access.byte_len > Word::BYTE_LEN {
        return Err(Error::InvalidInstruction);
    }

    let address = state
        .heap()
        .raw_address(pointer, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;
    let raw = super::access::read_word_bits(address as usize, access.byte_len);

    decode_raw_bits(state.tree(), access.value_type, raw, access.byte_len)
}

/// Load one value from shared raw heap bytes.
pub(crate) fn load_shared_raw_pointer(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
) -> Result<Word, Error> {
    let pointer = ptr.as_shared_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    if !access.is_scalar {
        return Err(Error::TypeMismatch {
            expected: "word shared raw load".to_string(),
            actual: format!("{:?}", access.value_type),
        });
    }
    if access.byte_len > Word::BYTE_LEN {
        return Err(Error::InvalidInstruction);
    }

    let address = state
        .shared()
        .raw_address(pointer, access.byte_offset, access.byte_len)
        .map_err(Error::from)?;
    let raw = super::access::read_word_bits(address as usize, access.byte_len);

    decode_raw_bits(state.tree(), access.value_type, raw, access.byte_len)
}

/// Store one value into raw heap bytes.
pub(crate) fn store_raw_pointer(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), access.value_type, value)?;
    if byte_len != access.byte_len {
        return Err(Error::InvalidInstruction);
    }

    let pointer = ptr.as_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = state
        .heap_mut()
        .raw_address_mut(pointer, access.byte_offset, byte_len)
        .map_err(Error::from)?;
    super::access::write_word_bits(address as usize, raw, byte_len);

    Ok(())
}

/// Store bytes into raw heap bytes.
pub(crate) fn store_raw_pointer_bytes(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    if bytes.len() != access.byte_len {
        return Err(Error::InvalidInstruction);
    }

    let pointer = ptr.as_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    write_raw_bytes(state, pointer, access.byte_offset, bytes)
}

/// Store one value into shared raw heap bytes.
pub(crate) fn store_shared_raw_pointer(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    value: Word,
) -> Result<(), Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), access.value_type, value)?;
    if byte_len != access.byte_len {
        return Err(Error::InvalidInstruction);
    }

    let pointer = ptr.as_shared_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    let address = state
        .shared()
        .raw_address_mut(pointer, access.byte_offset, byte_len)
        .map_err(Error::from)?;
    super::access::write_word_bits(address as usize, raw, byte_len);

    Ok(())
}

/// Store bytes into shared raw heap bytes.
pub(crate) fn store_shared_raw_pointer_bytes(
    state: &mut DispatchState<'_, '_>,
    ptr: Word,
    access: PointeeAccess,
    bytes: &[u8],
) -> Result<(), Error> {
    if bytes.len() != access.byte_len {
        return Err(Error::InvalidInstruction);
    }

    let pointer = ptr.as_shared_raw_pointer();
    if state.null_checks && pointer.is_null() {
        return Err(Error::NullPointerDereference);
    }

    state
        .shared()
        .write_raw_bytes(pointer, access.byte_offset, bytes)
        .map_err(Error::from)
}
