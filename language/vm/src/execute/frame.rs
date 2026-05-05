use std::{ptr, slice};

use smallvec::SmallVec;
use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::Error;
use crate::interpreter::{Frame, Machine};
use crate::program::{
    ArgumentRange, FrameAccess, FrameAccessId, Instruction, MovePair, MoveRange, MoveSource,
    PointerClass, Program, Transfer, ValueLayout, encode_word_bytes, pointer_class_from_reference,
    repr_type, value_layout_from_type,
};
use crate::{FramePointer, ReferenceMeta, SharedHeap, Word};
use destack_heap::{Heap, SharedAllocator, SharedGcWorker};

use super::access;
use super::reference::{check_reference_address_space, check_reference_mutability};

/// Return the byte offset for one frame element access.
#[inline(always)]
pub(super) fn frame_element_offset(
    machine: &Machine<'_, '_>,
    access: FrameAccess,
    index: u32,
) -> Result<usize, Error> {
    let index = machine.get_word_at(index).as_u64();
    if machine.bounds_checks && index >= access.length {
        return Err(Error::InvalidArrayAccess {
            index,
            length: access.length,
        });
    }

    Ok(access.byte_offset + access.byte_stride * index as usize)
}

/// Execute fixed-offset frame address calculation.
#[inline(always)]
pub(crate) fn execute_address_frame_offset(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = instruction.b;
    let reference = ReferenceMeta::from_bits(instruction.c as u8);
    let byte_offset = instruction.d as usize;

    let pointer = machine.frame_pointer_at(base).add_bytes(byte_offset);
    let value = Word::frame_pointer(pointer);

    if let Err(error) = check_reference_address_space(machine, reference) {
        return Transfer::Error(error);
    }

    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute frame element address calculation.
#[inline(always)]
pub(crate) fn execute_address_frame_element(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = instruction.b;
    let index = instruction.c;
    let access = FrameAccessId(instruction.d);

    let access = machine.frame_access(access);
    let offset = match frame_element_offset(machine, access, index) {
        Ok(offset) => offset,
        Err(error) => return Transfer::Error(error),
    };
    let pointer = machine.frame_pointer_at(base).add_bytes(offset);
    let value = Word::frame_pointer(pointer);

    if let Err(error) = check_reference_address_space(machine, access.reference) {
        return Transfer::Error(error);
    }

    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute fixed frame scalar load.
#[inline(always)]
pub(crate) fn execute_load_frame_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = machine.get_word_at(instruction.b);
    let byte_offset = instruction.c as usize;
    let pointer = base.as_frame_pointer().add_bytes(byte_offset);

    if !machine.owns_frame_range(pointer, BYTE_LEN) {
        return Transfer::Error(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    let value = access::load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(pointer.address());
    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute fixed frame value scalar load.
#[inline(always)]
pub(crate) fn execute_load_frame_value_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let base = instruction.b;
    let byte_offset = instruction.c as usize;
    let pointer = machine.frame_pointer_at(base).add_bytes(byte_offset);

    if !machine.owns_frame_range(pointer, BYTE_LEN) {
        return Transfer::Error(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    let value = access::load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(pointer.address());
    machine.set_word_at(dest, value);

    Transfer::Continue
}

/// Execute fixed frame scalar store.
#[inline(always)]
pub(crate) fn execute_store_frame_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let base = machine.get_word_at(instruction.a);
    let value = instruction.b;
    let byte_offset = instruction.c as usize;
    let reference = ReferenceMeta::from_bits(instruction.d as u8);

    if let Err(error) = check_reference_address_space(machine, reference) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(machine, reference) {
        return Transfer::Error(error);
    }

    let pointer = base.as_frame_pointer().add_bytes(byte_offset);
    let value = machine.get_word_at(value);

    if !machine.owns_frame_range(pointer, BYTE_LEN) {
        return Transfer::Error(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    access::store_scalar_at_address::<BYTE_LEN>(pointer.address(), value);

    Transfer::Continue
}

/// Execute fixed frame value scalar store.
#[inline(always)]
pub(crate) fn execute_store_frame_value_scalar<const BYTE_LEN: usize>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let base = instruction.a;
    let value = instruction.b;
    let byte_offset = instruction.c as usize;
    let reference = ReferenceMeta::from_bits(instruction.d as u8);

    if let Err(error) = check_reference_address_space(machine, reference) {
        return Transfer::Error(error);
    }
    if let Err(error) = check_reference_mutability(machine, reference) {
        return Transfer::Error(error);
    }

    let pointer = machine.frame_pointer_at(base).add_bytes(byte_offset);
    let value = machine.get_word_at(value);

    if !machine.owns_frame_range(pointer, BYTE_LEN) {
        return Transfer::Error(Error::InvalidAddressSpace {
            expected: "frame".to_string(),
            actual: format!("0x{:x}", pointer.address()),
        });
    }

    access::store_scalar_at_address::<BYTE_LEN>(pointer.address(), value);

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
    machine: &mut Machine<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
) -> Result<Vec<u8>, Error> {
    let layout = machine.layout(ty)?.clone();
    if layout.is_word() {
        return Ok(encode_word_bytes(machine.tree(), ty, value)?
            .as_slice()
            .to_vec());
    }

    let mut bytes = vec![0u8; layout.byte_len];
    store_argument_bytes(machine, ty, value, &mut bytes)?;

    Ok(bytes)
}

/// Store one field-shaped value into destination frame bytes.
pub(crate) fn store_frame_fields<F>(
    machine: &mut Machine<'_, '_>,
    destination: mir::Value,
    mut field_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut Machine<'_, '_>, u32, mir::LocalNodeId<mir::Type>) -> Result<Word, Error>,
{
    let ty = machine.value_type(destination)?;
    let layout = machine.layout(ty)?.clone();
    let field_count = layout.field_count().ok_or(Error::TypeMismatch {
        expected: "field-shaped frame value".to_string(),
        actual: format!("{ty:?}"),
    })?;

    // validate the destination once before incremental writes
    if machine.value_bytes(destination)?.len() != layout.byte_len {
        return Err(Error::InvalidInstruction);
    }

    // encode each field into its physical byte range
    for index in 0..field_count {
        let index = index as u32;
        let field = layout
            .field(index)
            .ok_or(Error::InvalidFieldAccess { index, field_count })?;
        let value = field_value(machine, index, field.ty)?;
        let value_end = field.offset + field.byte_len;
        let value_bytes = encode_word_bytes(machine.tree(), field.ty, value)?;
        if value_bytes.len() != field.byte_len {
            return Err(Error::InvalidInstruction);
        }

        let destination_bytes = machine.value_bytes_mut(destination)?;
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

/// Store one function entry argument into one byte range.
fn store_argument_bytes(
    machine: &mut Machine<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
    destination: &mut [u8],
) -> Result<(), Error> {
    if machine.layout(ty)?.is_word() {
        let bytes = encode_word_bytes(machine.tree(), ty, value)?;
        if bytes.len() != destination.len() {
            return Err(Error::InvalidHeapReference);
        }

        destination.copy_from_slice(bytes.as_slice());
        return Ok(());
    }

    let reference = value.as_heap_reference();
    if machine.heap().is_heap_live(reference) {
        let address = machine.heap().heap_base_address() + reference.offset();

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
    if !machine.shared().is_heap_live(reference) {
        machine.flush_shared_allocator();
    }

    if machine.shared().is_heap_live(reference) {
        let address = machine.shared().heap_base_address() + reference.offset();

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
    let slot = frame_layout
        .value(value.0)
        .ok_or(Error::InvalidInstruction)?;

    Ok(program.type_for_layout(slot.layout))
}

/// Return the addressable word for one frame value.
fn frame_value_word(program: &Program, frame: &Frame, value: mir::Value) -> Result<Word, Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let slot = frame_layout
        .value(value.0)
        .ok_or(Error::InvalidInstruction)?;
    let layout =
        program
            .layout_for_layout(slot.layout)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing frame value layout: layout={:?}", slot.layout),
            })?;

    if layout.is_word() {
        return Ok(frame.read_word(slot));
    }

    Ok(Word::frame_pointer(FramePointer::from_address(
        frame.slot_address(slot),
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

/// Store one owned frame value into a destination frame slot.
pub(crate) fn store_frame_value(
    program: &Program,
    dest_frame: &mut Frame,
    destination: mir::Value,
    value: FrameValue,
) -> Result<(), Error> {
    let frame_layout = program
        .frame_layout_by_id(dest_frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let slot = frame_layout
        .value(destination.0)
        .ok_or(Error::InvalidInstruction)?;
    let layout =
        program
            .layout_for_layout(slot.layout)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing destination value layout: layout={:?}", slot.layout),
            })?;

    match (layout.is_word(), value.body) {
        (true, FrameValueBody::Word(value)) => dest_frame.write_word(slot, value),
        (false, FrameValueBody::Bytes(bytes)) if bytes.len() == slot.byte_len as usize => {
            dest_frame.slot_bytes_mut(slot).copy_from_slice(&bytes);
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
                expected: format!("{} bytes", slot.byte_len),
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
            let shape = program.allocation_shape(layout_id)?;

            match boundary_pointer_class(program, value.ty) {
                PointerClass::Heap => {
                    let layout = heap.allocation_layout(shape);
                    let reference = heap.allocate_bytes(&layout, &bytes).map_err(Error::from)?;

                    Ok(engine::Value::HeapReference(reference))
                }
                PointerClass::SharedHeap => {
                    let layout = shared.allocation_layout(shape);
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

    let source_slot = source_layout
        .value(source.0)
        .ok_or(Error::InvalidInstruction)?;
    let dest_slot = dest_layout
        .value(destination.0)
        .ok_or(Error::InvalidInstruction)?;

    if source_slot.byte_len != dest_slot.byte_len || source_slot.is_word != dest_slot.is_word {
        return Err(Error::TypeMismatch {
            expected: format!("{} bytes, word={}", dest_slot.byte_len, dest_slot.is_word),
            actual: format!(
                "{} bytes, word={}",
                source_slot.byte_len, source_slot.is_word
            ),
        });
    }

    if dest_slot.is_word {
        let value = source_frame.read_word(source_slot);
        dest_frame.write_word(dest_slot, value);

        return Ok(());
    }

    let bytes = source_frame.slot_bytes(source_slot);
    dest_frame.slot_bytes_mut(dest_slot).copy_from_slice(bytes);

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
    let slot = frame_layout
        .value(destination.0)
        .ok_or(Error::InvalidInstruction)?;

    if slot.is_word {
        frame.write_word(slot, Word::VOID);

        return Ok(());
    }

    frame.slot_bytes_mut(slot).fill(0);

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

/// Move frame values using one lowered move range.
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
    if move_words_within_frame(program, frame, pairs)? {
        return Ok(());
    }

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

/// Move word frame values within one frame.
#[inline(always)]
fn move_words_within_frame(
    program: &Program,
    frame: &mut Frame,
    pairs: &[MovePair],
) -> Result<bool, Error> {
    let layout = program
        .frame_layout_by_id(frame.frame_layout)
        .ok_or(Error::InvalidInstruction)?;
    let mut values = SmallVec::<[Word; 16]>::with_capacity(pairs.len());

    // collect word sources before writing destinations
    for pair in pairs {
        let destination = layout.value(pair.dest.0).ok_or(Error::InvalidInstruction)?;
        if !destination.is_word {
            return Ok(false);
        }

        let value = match pair.source {
            MoveSource::Value(source) => {
                let source = layout.value(source.0).ok_or(Error::InvalidInstruction)?;
                if !source.is_word {
                    return Ok(false);
                }

                frame.read_word(source)
            }
            MoveSource::Void => Word::VOID,
        };

        values.push(value);
    }

    // store destinations after preserving parallel move semantics
    for (pair, value) in pairs.iter().zip(values) {
        let destination = layout.value(pair.dest.0).ok_or(Error::InvalidInstruction)?;

        frame.write_word(destination, value);
    }

    Ok(true)
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

/// Copy one lowered argument move range out of the current frame.
pub(crate) fn load_moved_arguments(
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

/// Write owned frame values into parameter slots.
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
