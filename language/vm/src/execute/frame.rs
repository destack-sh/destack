use std::{ptr, slice};

use smallvec::SmallVec;
use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::Error;
use crate::interpreter::{DispatchState, Frame};
use crate::program::{
    AddressFrame, AddressFrameElement, ArgumentRange, FrameAccess, Instruction, LoadFrame,
    LoadFrameElement, MovePair, MoveRange, MoveSource, PointeeAccess, PointerClass, Program,
    StoreFrame, StoreFrameElement, Transfer, ValueLayout, encode_word_bytes,
    pointer_class_from_reference, repr_type, value_layout_from_type,
};
use crate::{FramePointer, SharedHeap, Word};
use destack_heap::{Heap, SharedAllocator, SharedGcWorker};

use super::access;
use super::reference::{check_reference_address_space, check_reference_mutability};

/// Return the byte offset for one frame element access.
#[inline(always)]
pub(super) fn frame_element_offset(
    state: &DispatchState<'_, '_>,
    access: FrameAccess,
    index: mir::Value,
) -> Result<usize, Error> {
    let index = state.get_word(index).as_u64();
    if state.bounds_checks && index >= access.length {
        return Err(Error::InvalidArrayAccess {
            index,
            length: access.length,
        });
    }

    Ok(access.byte_offset + access.byte_stride * index as usize)
}

/// Execute fixed frame address calculation.
#[inline(always)]
pub(crate) fn execute_address_frame(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let AddressFrame {
        dest,
        base,
        reference,
        access,
    } = instruction.payload_as::<AddressFrame>();

    let base = match state.value_operand(*base) {
        Ok(base) => base,
        Err(error) => return Transfer::Error(error),
    };
    let access = state.frame_access(*access);
    let pointer = base.as_frame_pointer().add_bytes(access.byte_offset);
    let value = Word::frame_pointer(pointer);

    if let Err(error) = check_reference_address_space(state, *reference) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute frame element address calculation.
#[inline(always)]
pub(crate) fn execute_address_frame_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let AddressFrameElement {
        dest,
        base,
        index,
        reference,
        access,
    } = instruction.payload_as::<AddressFrameElement>();

    let base = match state.value_operand(*base) {
        Ok(base) => base,
        Err(error) => return Transfer::Error(error),
    };
    let access = state.frame_access(*access);
    let offset = match frame_element_offset(state, access, *index) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = base.as_frame_pointer().add_bytes(offset);
    let value = Word::frame_pointer(pointer);

    if let Err(error) = check_reference_address_space(state, *reference) {
        return Transfer::Error(error);
    }

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute fixed frame word load.
#[inline(always)]
pub(crate) fn execute_frame_load(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let LoadFrame { dest, base, access } = instruction.payload_as::<LoadFrame>();

    let base = match state.value_operand(*base) {
        Ok(base) => base,
        Err(error) => return Transfer::Error(error),
    };
    let access = state.frame_access(*access);
    let access = PointeeAccess {
        byte_offset: access.byte_offset,
        ..access.into()
    };

    let value = match access::load_frame_word(state, base.as_frame_pointer(), access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute frame element word load.
#[inline(always)]
pub(crate) fn execute_load_frame_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let LoadFrameElement {
        dest,
        base,
        index,
        access,
    } = instruction.payload_as::<LoadFrameElement>();

    let base = match state.value_operand(*base) {
        Ok(base) => base,
        Err(error) => return Transfer::Error(error),
    };
    let access = state.frame_access(*access);
    let offset = match frame_element_offset(state, access, *index) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };
    let access = PointeeAccess {
        byte_offset: offset,
        ..access.into()
    };

    let value = match access::load_frame_word(state, base.as_frame_pointer(), access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    state.set_word(*dest, value);

    Transfer::Continue
}

/// Execute fixed frame word store.
#[inline(always)]
pub(crate) fn execute_frame_store(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let StoreFrame {
        base,
        value,
        reference,
        access,
    } = instruction.payload_as::<StoreFrame>();

    let base = match state.value_operand(*base) {
        Ok(base) => base,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_address_space(state, *reference) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    let access = state.frame_access(*access);
    let access = PointeeAccess {
        byte_offset: access.byte_offset,
        ..access.into()
    };
    let value = state.get_word(*value);

    if let Err(error) = access::store_frame_word(state, base.as_frame_pointer(), access, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// Execute frame element word store.
#[inline(always)]
pub(crate) fn execute_store_frame_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let StoreFrameElement {
        base,
        index,
        value,
        reference,
        access,
    } = instruction.payload_as::<StoreFrameElement>();

    let base = match state.value_operand(*base) {
        Ok(base) => base,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = check_reference_address_space(state, *reference) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return Transfer::Error(error);
    }

    let access = state.frame_access(*access);
    let offset = match frame_element_offset(state, access, *index) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };
    let access = PointeeAccess {
        byte_offset: offset,
        ..access.into()
    };
    let value = state.get_word(*value);

    if let Err(error) = access::store_frame_word(state, base.as_frame_pointer(), access, value) {
        return Transfer::Error(error);
    }

    Transfer::Continue
}

/// One owned frame value body.
#[derive(Clone, Debug)]
pub(crate) enum FrameValueBody {
    /// One scalar or pointer word.
    Word(Word),
    /// One non-word frame byte range.
    Bytes(Box<[u8]>),
}

/// One owned value copied out of a frame with its MIR type.
#[derive(Clone, Debug)]
pub(crate) struct FrameValue {
    /// The MIR type carried with the raw value bits.
    ty: mir::LocalNodeId<mir::Type>,
    /// The owned value body.
    body: FrameValueBody,
}

impl FrameValue {
    /// Create one word value.
    #[inline]
    pub(crate) fn word(ty: mir::LocalNodeId<mir::Type>, value: Word) -> Self {
        Self {
            ty,
            body: FrameValueBody::Word(value),
        }
    }

    /// Create one byte value.
    #[inline]
    pub(crate) fn bytes(ty: mir::LocalNodeId<mir::Type>, bytes: Box<[u8]>) -> Self {
        Self {
            ty,
            body: FrameValueBody::Bytes(bytes),
        }
    }

    /// Return this value as one word.
    #[inline]
    pub(crate) fn into_word(self) -> Result<Word, Error> {
        match self.body {
            FrameValueBody::Word(value) => Ok(value),
            FrameValueBody::Bytes(_) => Err(Error::TypeMismatch {
                expected: "word frame value".to_string(),
                actual: format!("byte frame value: type={:?}", self.ty),
            }),
        }
    }
}

/// Encode one function entry argument into frame bytes.
pub(crate) fn encode_argument_bytes(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<Vec<u8>, Error> {
    let layout = state.layout(ty)?.clone();
    if layout.is_word() {
        return Ok(encode_word_bytes(state.tree(), ty, value)?
            .as_slice()
            .to_vec());
    }

    let mut bytes = vec![0u8; layout.byte_len];
    store_argument_bytes(state, ty, value, &mut bytes)?;

    Ok(bytes)
}

/// Encode one frame value into its byte representation.
pub(crate) fn encode_frame_value_bytes(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: mir::Value,
) -> Result<Vec<u8>, Error> {
    let layout = state.layout(ty)?.clone();
    if layout.is_word() {
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

/// Return one frame byte range for one lowered memory access.
#[inline(always)]
pub(crate) fn frame_value_bytes_for_access(
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

/// Store one field-shaped value into destination frame bytes.
pub(crate) fn store_frame_fields<F>(
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

    // validate the destination once before incremental writes
    if state.value_bytes(destination)?.len() != layout.byte_len {
        return Err(Error::InvalidInstruction);
    }

    // encode each field into its physical byte range
    for index in 0..field_count {
        let index = index as u32;
        let field = layout
            .field(index)
            .ok_or(Error::InvalidFieldAccess { index, field_count })?;
        let value = field_value(state, index, field.ty)?;
        let value_end = field.offset + field.byte_len;
        let value_bytes = encode_word_bytes(state.tree(), field.ty, value)?;
        if value_bytes.len() != field.byte_len {
            return Err(Error::InvalidInstruction);
        }

        let destination_bytes = state.value_bytes_mut(destination)?;
        let value_window = destination_bytes.get_mut(field.offset..value_end).ok_or(
            Error::InvalidFieldAccess {
                index,
                field_count: layout.byte_len,
            },
        )?;

        value_window.copy_from_slice(value_bytes.as_slice());
    }

    Ok(())
}

/// Store one indexed value into destination frame bytes.
pub(crate) fn store_frame_elements<F>(
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

    // validate the destination once before incremental writes
    if state.value_bytes(destination)?.len() != layout.byte_len {
        return Err(Error::InvalidInstruction);
    }

    // encode each element into its physical byte range
    for index in 0..element_count {
        let offset = element.stride * index;
        let value_end = offset + element.byte_len;
        let value = element_value(state, index, element.ty)?;
        let value_bytes = encode_word_bytes(state.tree(), element.ty, value)?;
        if value_bytes.len() != element.byte_len {
            return Err(Error::InvalidInstruction);
        }

        let destination_bytes = state.value_bytes_mut(destination)?;
        let value_window =
            destination_bytes
                .get_mut(offset..value_end)
                .ok_or(Error::InvalidArrayAccess {
                    index: index as u64,
                    length: element_count as u64,
                })?;

        value_window.copy_from_slice(value_bytes.as_slice());
    }

    Ok(())
}

/// Store one function entry argument into one byte range.
fn store_argument_bytes(
    state: &mut DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
    destination: &mut [u8],
) -> Result<(), Error> {
    if state.layout(ty)?.is_word() {
        let bytes = encode_word_bytes(state.tree(), ty, value)?;
        if bytes.len() != destination.len() {
            return Err(Error::InvalidHeapReference);
        }

        destination.copy_from_slice(bytes.as_slice());
        return Ok(());
    }

    let reference = value.as_heap_reference();
    if state.heap().is_heap_live(reference) {
        let address = state.heap().heap_base_address() + reference.offset();

        // copy from the managed payload address
        unsafe {
            ptr::copy_nonoverlapping(
                address as *const u8,
                destination.as_mut_ptr(),
                destination.len(),
            );
        }

        return Ok(());
    }

    let reference = value.as_shared_heap_reference();
    if !state.shared().is_heap_live(reference) {
        state.flush_shared_allocator();
    }

    if state.shared().is_heap_live(reference) {
        let address = state.shared().heap_base_address() + reference.offset();

        // copy from the shared payload address
        unsafe {
            ptr::copy_nonoverlapping(
                address as *const u8,
                destination.as_mut_ptr(),
                destination.len(),
            );
        }

        return Ok(());
    }

    Err(Error::TypeMismatch {
        expected: "scalar or heap-backed argument".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Return the MIR type stored in one frame value.
pub(crate) fn frame_value_type(
    program: &Program,
    frame: &Frame,
    value: mir::Value,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let region = frame_layout
        .value(value.0)
        .ok_or(Error::InvalidInstruction)?;

    Ok(program.type_for_id(region.ty))
}

/// Return the addressable word for one frame value.
fn frame_value_word(program: &Program, frame: &Frame, value: mir::Value) -> Result<Word, Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let region = frame_layout
        .value(value.0)
        .ok_or(Error::InvalidInstruction)?;
    let layout = program
        .layout_for_id(region.ty)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!("missing frame value layout: type={:?}", region.ty),
        })?;

    if layout.is_word() {
        return Ok(frame.read_word(region));
    }

    Ok(Word::frame_pointer(FramePointer::from_address(
        frame.region_address(region),
    )))
}

/// Return one function return type.
pub(crate) fn function_return_type(
    program: &Program,
    function: mir::LocalNodeId<mir::Function>,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let function = program.tree.get(function);

    (function.return_type)
        .ty()
        .ok_or_else(|| Error::MissingRepresentation {
            context: "function return type".to_string(),
        })
}

/// Load one word or frame byte range into an owned frame value.
pub(crate) fn frame_value_from_word(
    program: &Program,
    frames: &[Frame],
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<FrameValue, Error> {
    let layout = program
        .layout(ty)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!("missing frame value layout: type={ty:?}"),
        })?;
    if layout.is_word() {
        return Ok(FrameValue::word(ty, value));
    }

    let pointer = value.as_frame_pointer();
    let frame = frames
        .iter()
        .find(|frame| frame.owns_stack_range(pointer.address(), layout.byte_len))
        .ok_or(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("{value:?}"),
        })?;
    let start =
        pointer
            .address()
            .checked_sub(frame.base_address())
            .ok_or(Error::InvalidAddressSpace {
                expected: "frame".to_string(),
                actual: format!("{value:?}"),
            })?;
    let end = start + layout.byte_len;
    let bytes = frame
        .bytes()
        .get(start..end)
        .ok_or(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("{value:?}"),
        })?
        .to_vec()
        .into_boxed_slice();

    Ok(FrameValue::bytes(ty, bytes))
}

/// Load one frame value into an owned value.
fn load_frame_value(
    program: &Program,
    frames: &[Frame],
    frame: &Frame,
    value: mir::Value,
) -> Result<FrameValue, Error> {
    let ty = frame_value_type(program, frame, value)?;
    let value = frame_value_word(program, frame, value)?;

    frame_value_from_word(program, frames, ty, value)
}

/// Store one owned frame value into a destination frame region.
pub(crate) fn store_frame_value(
    program: &Program,
    dest_frame: &mut Frame,
    destination: mir::Value,
    value: FrameValue,
) -> Result<(), Error> {
    let frame_layout = program
        .frame_layout_by_id(dest_frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let region = frame_layout
        .value(destination.0)
        .ok_or(Error::InvalidInstruction)?;
    let layout = program
        .layout_for_id(region.ty)
        .ok_or_else(|| Error::InvariantViolation {
            context: format!("missing destination value layout: type={:?}", region.ty),
        })?;

    match (layout.is_word(), value.body) {
        (true, FrameValueBody::Word(value)) => dest_frame.write_word(region, value),
        (false, FrameValueBody::Bytes(bytes)) if bytes.len() == region.byte_len as usize => {
            dest_frame.region_bytes_mut(region).copy_from_slice(&bytes);
        }
        (false, FrameValueBody::Word(value)) => {
            return Err(Error::TypeMismatch {
                expected: "byte frame value".to_string(),
                actual: format!("{value:?}"),
            });
        }
        (true, FrameValueBody::Bytes(bytes)) => {
            return Err(Error::TypeMismatch {
                expected: "word frame value".to_string(),
                actual: format!("{} bytes", bytes.len()),
            });
        }
        (false, FrameValueBody::Bytes(bytes)) => {
            return Err(Error::TypeMismatch {
                expected: format!("{} bytes", region.byte_len),
                actual: format!("{} bytes", bytes.len()),
            });
        }
    }

    Ok(())
}

/// Materialize one owned frame value into one engine boundary value.
pub(crate) fn materialize_value(
    program: &Program,
    heap: &mut Heap,
    shared: &SharedHeap,
    shared_allocator: &mut SharedAllocator,
    shared_gc: &SharedGcWorker,
    value: FrameValue,
) -> Result<engine::Value, Error> {
    match value.body {
        FrameValueBody::Word(word) => materialize_word(program, value.ty, word),
        FrameValueBody::Bytes(bytes) => {
            if let Some(value) = materialize_scalar_bytes(program, value.ty, &bytes)? {
                return Ok(value);
            }

            let layout_id = program
                .layout_id_for_type(value.ty)
                .ok_or(Error::InvalidInstruction)?;
            let plan = program.allocation_plan(layout_id)?;

            match boundary_pointer_class(program, value.ty) {
                PointerClass::Heap => {
                    let layout = heap.allocation_layout(plan);
                    let reference = heap.allocate_bytes(&layout, &bytes).map_err(Error::from)?;

                    Ok(engine::Value::HeapReference(reference))
                }
                PointerClass::SharedHeap => {
                    let layout = shared.allocation_layout(plan);
                    let reference = shared
                        .allocate_bytes(shared_gc, shared_allocator, &layout, &bytes)
                        .map_err(Error::from)?;

                    Ok(engine::Value::SharedHeapReference(reference))
                }
                pointer_class => Err(Error::InvalidPointerType {
                    actual: format!("{pointer_class:?}"),
                }),
            }
        }
    }
}

/// Materialize one frame-backed scalar when the engine boundary can carry it.
fn materialize_scalar_bytes(
    program: &Program,
    ty: mir::LocalNodeId<mir::Type>,
    bytes: &[u8],
) -> Result<Option<engine::Value>, Error> {
    let ty = repr_type(&program.tree, ty);
    let mir::Type::Int { width, is_signed } = program.tree.get(ty) else {
        return Ok(None);
    };

    if *width > 128 {
        return Err(Error::TypeMismatch {
            expected: "engine boundary integer up to 128 bits".to_string(),
            actual: format!("{width}-bit integer"),
        });
    }

    let mut raw = [0u8; 16];
    raw[..bytes.len()].copy_from_slice(bytes);
    let raw = u128::from_le_bytes(raw);

    if *is_signed {
        let value = sign_extend_i128(raw, *width);

        return Ok(Some(engine::Value::Int {
            value,
            width: *width,
        }));
    }

    Ok(Some(engine::Value::UInt {
        value: raw,
        width: *width,
    }))
}

/// Sign-extend an integer with the given bit width into i128.
fn sign_extend_i128(value: u128, width: u16) -> i128 {
    if width == 0 || width >= 128 {
        return value as i128;
    }

    let shift = 128 - width;

    ((value << shift) as i128) >> shift
}

/// Return the pointer class used to package one non-word boundary value.
fn boundary_pointer_class(program: &Program, ty: mir::LocalNodeId<mir::Type>) -> PointerClass {
    let ty = repr_type(&program.tree, ty);

    match program.tree.get(ty) {
        mir::Type::Slice {
            kind,
            address_space,
            ..
        } => pointer_class_from_reference(address_space.clone(), *kind),
        _ => PointerClass::Heap,
    }
}

/// Dematerialize one engine boundary value into frame representation.
pub(crate) fn dematerialize_value(
    program: &Program,
    heap: &Heap,
    shared: &SharedHeap,
    ty: mir::LocalNodeId<mir::Type>,
    value: &engine::Value,
) -> Result<FrameValue, Error> {
    let layout = program.layout(ty).ok_or(Error::InvalidInstruction)?;
    if !layout.is_word() {
        return dematerialize_bytes(program, heap, shared, ty, value);
    }

    let word = match value {
        engine::Value::Void => Word::VOID,
        engine::Value::Bool(value) => Word::bool(*value),
        engine::Value::Int { value, width } => Word::int(*value as i64, *width as u8),
        engine::Value::UInt { value, width } => Word::uint(*value as u64, *width as u8),
        engine::Value::Float32 { bits } => Word::float32(f32::from_bits(*bits)),
        engine::Value::Float64 { bits } => Word::float64(f64::from_bits(*bits)),
        engine::Value::Char(value) => Word::char(*value),
        engine::Value::HeapReference(reference) => Word::heap_reference(*reference),
        engine::Value::SharedHeapReference(reference) => Word::shared_heap_reference(*reference),
        engine::Value::RawPointer(pointer) => Word::raw_pointer(*pointer),
        engine::Value::SharedRawPointer(pointer) => Word::shared_raw_pointer(*pointer),
    };

    Ok(FrameValue::word(ty, word))
}

/// Dematerialize one engine boundary value into frame bytes.
fn dematerialize_bytes(
    program: &Program,
    heap: &Heap,
    shared: &SharedHeap,
    ty: mir::LocalNodeId<mir::Type>,
    value: &engine::Value,
) -> Result<FrameValue, Error> {
    if let Some(bytes) = dematerialize_scalar_bytes(program, ty, value)? {
        return Ok(FrameValue::bytes(ty, bytes));
    }

    let layout = program.layout(ty).ok_or(Error::InvalidInstruction)?;
    let mut bytes = vec![0u8; layout.byte_len];

    match (boundary_pointer_class(program, ty), value) {
        (PointerClass::Heap, engine::Value::HeapReference(reference)) => {
            let address = heap.heap_base_address() + reference.offset();

            // copy from the managed payload address
            unsafe {
                ptr::copy_nonoverlapping(address as *const u8, bytes.as_mut_ptr(), bytes.len());
            }
        }
        (PointerClass::SharedHeap, engine::Value::SharedHeapReference(reference)) => {
            let address = shared.heap_base_address() + reference.offset();

            // copy from the shared payload address
            unsafe {
                ptr::copy_nonoverlapping(address as *const u8, bytes.as_mut_ptr(), bytes.len());
            }
        }
        (pointer_class, value) => {
            return Err(Error::TypeMismatch {
                expected: format!("{pointer_class:?} frame-backed value"),
                actual: format!("{value:?}"),
            });
        }
    }

    Ok(FrameValue::bytes(ty, bytes.into_boxed_slice()))
}

/// Dematerialize one engine boundary scalar into frame bytes.
fn dematerialize_scalar_bytes(
    program: &Program,
    ty: mir::LocalNodeId<mir::Type>,
    value: &engine::Value,
) -> Result<Option<Box<[u8]>>, Error> {
    let ty = repr_type(&program.tree, ty);
    let mir::Type::Int { width, is_signed } = program.tree.get(ty) else {
        return Ok(None);
    };

    let byte_len = (*width as usize).div_ceil(8);
    let raw = match (is_signed, value) {
        (
            true,
            engine::Value::Int {
                value,
                width: value_width,
            },
        ) if value_width == width => *value as u128,
        (
            false,
            engine::Value::UInt {
                value,
                width: value_width,
            },
        ) if value_width == width => *value,
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("{width}-bit integer"),
                actual: format!("{value:?}"),
            });
        }
    };

    Ok(Some(
        raw.to_le_bytes()[..byte_len].to_vec().into_boxed_slice(),
    ))
}

/// Materialize one word into one engine boundary value.
pub(crate) fn materialize_word(
    program: &Program,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<engine::Value, Error> {
    match value_layout_from_type(&program.tree, ty) {
        ValueLayout::Void => Ok(engine::Value::Void),
        ValueLayout::Bool => Ok(engine::Value::Bool(value.as_bool())),
        ValueLayout::Int {
            width,
            signed: true,
        } => Ok(engine::Value::Int {
            value: value.as_int() as i128,
            width,
        }),
        ValueLayout::Int {
            width,
            signed: false,
        } => Ok(engine::Value::UInt {
            value: value.as_uint() as u128,
            width,
        }),
        ValueLayout::Float { width: 32 } => Ok(engine::Value::Float32 {
            bits: value.as_float32().to_bits(),
        }),
        ValueLayout::Float { .. } => Ok(engine::Value::Float64 {
            bits: value.as_float64().to_bits(),
        }),
        ValueLayout::Char => {
            let value = value.as_char().ok_or(Error::InvalidInstruction)?;

            Ok(engine::Value::Char(value))
        }
        ValueLayout::Pointer {
            pointer_class: PointerClass::Heap,
            ..
        } => Ok(engine::Value::HeapReference(value.as_heap_reference())),
        ValueLayout::Pointer {
            pointer_class: PointerClass::SharedHeap,
            ..
        } => Ok(engine::Value::SharedHeapReference(
            value.as_shared_heap_reference(),
        )),
        ValueLayout::Pointer {
            pointer_class: PointerClass::Raw,
            ..
        } => Ok(engine::Value::RawPointer(value.as_raw_pointer())),
        ValueLayout::Pointer {
            pointer_class: PointerClass::SharedRaw,
            ..
        } => Ok(engine::Value::SharedRawPointer(
            value.as_shared_raw_pointer(),
        )),
        _ => Err(Error::TypeMismatch {
            expected: "word value".to_string(),
            actual: format!("{:?}", value_layout_from_type(&program.tree, ty)),
        }),
    }
}

/// Move one frame value into another frame.
pub(crate) fn move_frame_value(
    program: &Program,
    source_frame: &Frame,
    source: mir::Value,
    dest_frame: &mut Frame,
    destination: mir::Value,
) -> Result<(), Error> {
    let source_layout = program
        .frame_layout_by_id(source_frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let dest_layout = program
        .frame_layout_by_id(dest_frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;

    let source_region = source_layout
        .value(source.0)
        .ok_or(Error::InvalidInstruction)?;
    let dest_region = dest_layout
        .value(destination.0)
        .ok_or(Error::InvalidInstruction)?;

    if source_region.byte_len != dest_region.byte_len
        || source_region.is_word != dest_region.is_word
    {
        return Err(Error::TypeMismatch {
            expected: format!(
                "{} bytes, word={}",
                dest_region.byte_len, dest_region.is_word
            ),
            actual: format!(
                "{} bytes, word={}",
                source_region.byte_len, source_region.is_word
            ),
        });
    }

    if dest_region.is_word {
        let value = source_frame.read_word(source_region);
        dest_frame.write_word(dest_region, value);

        return Ok(());
    }

    let bytes = source_frame.region_bytes(source_region);
    dest_frame
        .region_bytes_mut(dest_region)
        .copy_from_slice(bytes);

    Ok(())
}

/// Write void to one frame value.
fn store_void_value(
    program: &Program,
    frame: &mut Frame,
    destination: mir::Value,
) -> Result<(), Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let region = frame_layout
        .value(destination.0)
        .ok_or(Error::InvalidInstruction)?;

    if region.is_word {
        frame.write_word(region, Word::VOID);

        return Ok(());
    }

    frame.region_bytes_mut(region).fill(0);

    Ok(())
}

/// Move call arguments between two frames.
pub(crate) fn move_arguments_between_frames(
    program: &Program,
    source_frame: &Frame,
    dest_frame: &mut Frame,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> Result<(), Error> {
    let argument_slice = arguments.slice(argument_pool);
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let Some(argument) = argument_slice.get(index) else {
            store_void_value(program, dest_frame, *param)?;

            continue;
        };

        move_frame_value(program, source_frame, *argument, dest_frame, *param)?;
    }

    Ok(())
}

/// Move frame values using one lowered move plan.
pub(crate) fn move_values(
    program: &Program,
    source_frame: &Frame,
    dest_frame: &mut Frame,
    moves: MoveRange,
    move_pool: &[MovePair],
) -> Result<(), Error> {
    let pairs = moves.slice(move_pool);
    for pair in pairs {
        match pair.source {
            MoveSource::Value(source) => {
                move_frame_value(program, source_frame, source, dest_frame, pair.dest)?;
            }
            MoveSource::Void => {
                store_void_value(program, dest_frame, pair.dest)?;
            }
        }
    }

    Ok(())
}

/// Move frame values within one frame.
pub(crate) fn move_values_within_frame(
    program: &Program,
    frame: &mut Frame,
    moves: MoveRange,
    move_pool: &[MovePair],
) -> Result<(), Error> {
    let pairs = moves.slice(move_pool);
    let mut values = SmallVec::<[FrameValue; 16]>::with_capacity(pairs.len());

    // collect sources before writing destinations
    for pair in pairs {
        let value = match pair.source {
            MoveSource::Value(source) => {
                load_frame_value(program, slice::from_ref(frame), frame, source)?
            }
            MoveSource::Void => {
                let destination_type = frame_value_type(program, frame, pair.dest)?;

                FrameValue::word(destination_type, Word::VOID)
            }
        };

        values.push(value);
    }

    // store destinations after preserving parallel move semantics
    for (pair, value) in pairs.iter().zip(values) {
        store_frame_value(program, frame, pair.dest, value)?;
    }

    Ok(())
}

/// Copy one ordered argument list out of the current frame.
pub(crate) fn load_arguments(
    program: &Program,
    frames: &[Frame],
    frame: &Frame,
    argument_pool: &[mir::Value],
    arguments: ArgumentRange,
) -> Result<SmallVec<[FrameValue; 16]>, Error> {
    let argument_slice = arguments.slice(argument_pool);
    let mut collected_arguments = SmallVec::with_capacity(argument_slice.len());
    for argument in argument_slice {
        let value = load_frame_value(program, frames, frame, *argument)?;

        collected_arguments.push(value);
    }

    Ok(collected_arguments)
}

/// Copy one lowered argument plan out of the current frame.
pub(crate) fn load_planned_arguments(
    program: &Program,
    frames: &[Frame],
    frame: &Frame,
    move_pool: &[MovePair],
    moves: MoveRange,
) -> Result<SmallVec<[FrameValue; 16]>, Error> {
    let pairs = moves.slice(move_pool);
    let mut arguments = SmallVec::with_capacity(pairs.len());
    for pair in pairs {
        let value = match pair.source {
            MoveSource::Value(source) => load_frame_value(program, frames, frame, source)?,
            MoveSource::Void => {
                let destination_type = frame_value_type(program, frame, pair.dest)?;

                FrameValue::word(destination_type, Word::VOID)
            }
        };

        arguments.push(value);
    }

    Ok(arguments)
}

/// Write owned frame values into parameter regions.
pub(crate) fn store_parameters(
    program: &Program,
    frame: &mut Frame,
    param_pool: &[mir::Value],
    params: ArgumentRange,
    arguments: &[FrameValue],
) -> Result<(), Error> {
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let value = match arguments.get(index).cloned() {
            Some(value) => value,
            None => {
                let ty = frame_value_type(program, frame, *param)?;
                FrameValue::word(ty, Word::VOID)
            }
        };

        store_frame_value(program, frame, *param, value)?;
    }

    Ok(())
}
