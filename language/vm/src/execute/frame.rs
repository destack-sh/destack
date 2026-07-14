use std::ptr;

use destack_heap::{AllocationCache, Heap, SharedHeap, SharedMarkWorker};
use destack_mir as mir;
use destack_mir::Space;
use destack_program as program;
use smallvec::SmallVec;

use crate::diagnostic::{Error, ReferenceKind};
use crate::machine::{Activation, Frame};
use destack_program::vm::{
    ArgumentRange, Cell, Instruction, MovePair, MoveRange, MoveSlot, Projection, ProjectionId,
    encode_cell_bytes,
};
use destack_program::{CellLayout, FrameSlot, Program, ScalarFormat, TypeId};

use super::access;

/// Return the byte offset for one frame element projection.
#[inline(always)]
pub(super) fn frame_element_offset(
    activation: &Activation<'_>,
    access: Projection,
    index: u32,
) -> usize {
    let index = activation.load_cell_at(index).as_u64();

    access.byte_offset() + access.byte_stride() * index as usize
}

/// Execute fixed-offset frame value address calculation.
#[inline(always)]
pub(crate) fn execute_address_frame_value_offset(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = instruction.b;
    let byte_offset = instruction.d as usize;

    let pointer = activation.frame_pointer_at(base).add_bytes(byte_offset);
    let value = Cell::frame_pointer(pointer);

    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute frame value element address calculation.
#[inline(always)]
pub(crate) fn execute_address_frame_value_element(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = instruction.b;
    let index = instruction.c;
    let access = ProjectionId(instruction.d);

    let access = activation.projection(access);
    let offset = frame_element_offset(activation, access, index);
    let pointer = activation.frame_pointer_at(base).add_bytes(offset);
    let value = Cell::frame_pointer(pointer);

    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute fixed frame scalar load.
#[inline(always)]
pub(crate) fn execute_load_frame_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = activation.load_cell_at(instruction.b);
    let byte_offset = instruction.c as usize;
    let pointer = base.as_frame_pointer().add_bytes(byte_offset);

    let address = activation.memory_address(pointer.offset());
    let value = access::load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(address);
    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute fixed frame value scalar load.
#[inline(always)]
pub(crate) fn execute_load_frame_value_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let base = instruction.b;
    let byte_offset = instruction.c as usize;
    let pointer = activation.frame_pointer_at(base).add_bytes(byte_offset);

    let address = activation.memory_address(pointer.offset());
    let value = access::load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(address);
    activation.store_cell_at(dest, value);

    Ok(())
}

/// Execute fixed frame scalar store.
#[inline(always)]
pub(crate) fn execute_store_frame_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let base = activation.load_cell_at(instruction.a);
    let value = instruction.b;
    let byte_offset = instruction.c as usize;

    let pointer = base.as_frame_pointer().add_bytes(byte_offset);
    let value = activation.load_cell_at(value);

    let address = activation.memory_address(pointer.offset());
    access::store_scalar_at_address::<BYTE_LEN>(address, value);

    Ok(())
}

/// Execute fixed frame value scalar store.
#[inline(always)]
pub(crate) fn execute_store_frame_value_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let base = instruction.a;
    let value = instruction.b;
    let byte_offset = instruction.c as usize;

    let pointer = activation.frame_pointer_at(base).add_bytes(byte_offset);
    let value = activation.load_cell_at(value);

    let address = activation.memory_address(pointer.offset());
    access::store_scalar_at_address::<BYTE_LEN>(address, value);

    Ok(())
}

/// One owned frame value body.
#[derive(Clone, Debug)]
pub(crate) enum FrameValueBody {
    /// One scalar or pointer cell.
    Cell(Cell),
    /// One non-cell frame byte range.
    Bytes(Box<[u8]>),
}

/// One owned value copied out of a frame with its program type.
#[derive(Clone, Debug)]
pub(crate) struct FrameValue {
    /// The program type carried with the raw value bits.
    ty: TypeId,
    /// The owned value body.
    body: FrameValueBody,
}

impl FrameValue {
    /// Create one cell value.
    #[inline]
    pub(crate) fn cell(ty: TypeId, value: Cell) -> Self {
        Self {
            ty,
            body: FrameValueBody::Cell(value),
        }
    }

    /// Create one byte value.
    #[inline]
    pub(crate) fn bytes(ty: TypeId, bytes: Box<[u8]>) -> Self {
        Self {
            ty,
            body: FrameValueBody::Bytes(bytes),
        }
    }
}

/// Return the cell layout for a type that must fit in one VM cell.
fn require_cell_layout(program: &Program, ty: TypeId) -> Result<CellLayout, Error> {
    program
        .cell_layout(ty)
        .ok_or_else(|| Error::type_mismatch("scalar or reference cell", format!("{ty:?}")))
}

/// Encode one function entry argument into frame bytes.
pub(crate) fn encode_argument_bytes(
    activation: &mut Activation<'_>,
    ty: TypeId,
    value: Cell,
) -> Result<Vec<u8>, Error> {
    let layout = *activation.require_layout(ty)?;
    if activation.program.is_cell_type(ty) {
        let cell_layout = require_cell_layout(activation.program, ty)?;
        let bytes = encode_cell_bytes(cell_layout, value, activation.program.pointer_bytes());

        return Ok(bytes.as_slice().to_vec());
    }

    let mut bytes = vec![0u8; layout.byte_len()];
    store_argument_bytes(activation, ty, value, &mut bytes)?;

    Ok(bytes)
}

/// Store one function entry argument into one byte range.
fn store_argument_bytes(
    activation: &mut Activation<'_>,
    ty: TypeId,
    value: Cell,
    destination: &mut [u8],
) -> Result<(), Error> {
    if activation.program.is_cell_type(ty) {
        let cell_layout = require_cell_layout(activation.program, ty)?;
        let bytes = encode_cell_bytes(cell_layout, value, activation.program.pointer_bytes());
        if bytes.len() != destination.len() {
            return Err(Error::invalid_reference(ReferenceKind::Heap));
        }

        destination.copy_from_slice(bytes.as_slice());
        return Ok(());
    }

    let reference = value.as_heap_reference();
    if activation.is_heap_live(reference) {
        let address = activation.heap_address(reference, 0);

        copy_address_to_slice(address, destination);

        return Ok(());
    }

    let reference = value.as_shared_heap_reference();
    if activation.is_shared_heap_live(reference) {
        let address = activation.shared_heap_address(reference, 0);

        copy_address_to_slice(address, destination);

        return Ok(());
    }

    Err(Error::type_mismatch(
        "scalar or heap-backed argument",
        format!("{value:?}"),
    ))
}

/// Load one cell or frame byte range into an owned frame value.
pub(crate) fn frame_value_from_cell(
    program: &Program,
    frames: &[Frame],
    memory_base: usize,
    ty: TypeId,
    value: Cell,
) -> Result<FrameValue, Error> {
    if program.is_cell_type(ty) {
        return Ok(FrameValue::cell(ty, value));
    }

    let byte_len = program
        .type_byte_len(ty)
        .ok_or_else(|| Error::internal(format!("missing frame storage layout: type={ty:?}")))?;
    let pointer = value.as_frame_pointer();
    let frame = frames
        .iter()
        .find(|frame| frame.owns_stack_range(memory_base, pointer.offset(), byte_len))
        .ok_or(Error::invalid_space("frame", format!("{value:?}")))?;
    let start = pointer
        .offset()
        .checked_sub(frame.memory_offset(memory_base))
        .ok_or(Error::invalid_space("frame", format!("{value:?}")))?;
    let end = start + byte_len;
    let bytes = frame
        .bytes()
        .get(start..end)
        .ok_or(Error::invalid_space("frame", format!("{value:?}")))?
        .to_vec()
        .into_boxed_slice();

    Ok(FrameValue::bytes(ty, bytes))
}

/// Load one lowered frame slot into an owned value.
fn load_frame_slot_value(frame: &Frame, slot: MoveSlot) -> FrameValue {
    if slot.is_cell() {
        return FrameValue::cell(slot.ty, frame.read_cell_at(slot.offset));
    }

    let bytes = move_slot_bytes(frame, slot).to_vec().into_boxed_slice();

    FrameValue::bytes(slot.ty, bytes)
}

/// One buffered frame slot value.
enum BufferedSlotValue {
    /// Cell slot value.
    Cell(Cell),
    /// Byte slot value.
    Bytes(SmallVec<[u8; 32]>),
}

/// Materialize one owned frame value into one engine boundary value.
pub(crate) fn materialize_value(
    program: &Program,
    heap: &mut Heap,
    shared: &SharedHeap,
    shared_cache: &mut AllocationCache,
    shared_mark_worker: &SharedMarkWorker,
    value: FrameValue,
) -> Result<program::Value, Error> {
    match value.body {
        FrameValueBody::Cell(cell) => materialize_cell(program, value.ty, cell),
        FrameValueBody::Bytes(bytes) => {
            if let Some(value) = materialize_scalar_bytes(program, value.ty, &bytes)? {
                return Ok(value);
            }

            let layout_id = program
                .layout_id_for_type(value.ty)
                .ok_or(Error::invalid_instruction())?;
            let shape = program.allocation_shape(layout_id)?;

            match boundary_space(program, value.ty)? {
                Space::Local => {
                    let plan = heap.options().allocation_plan(&shape);
                    let reference = heap.allocate_bytes(plan, &shape.trace_map, &bytes)?;

                    Ok(program::Value::HeapReference(reference))
                }
                Space::Shared => {
                    let plan = shared.options().allocation_plan(&shape);
                    let reference = shared.allocate_bytes(
                        shared_mark_worker,
                        shared_cache,
                        plan,
                        &shape.trace_map,
                        &bytes,
                        program.trace_view(),
                    )?;

                    Ok(program::Value::SharedHeapReference(reference))
                }
                space => Err(Error::invalid_pointer_type(format!("{space:?}"))),
            }
        }
    }
}

/// Materialize one frame-backed scalar when the engine boundary can carry it.
fn materialize_scalar_bytes(
    program: &Program,
    ty: TypeId,
    bytes: &[u8],
) -> Result<Option<program::Value>, Error> {
    let Some(ScalarFormat::Int { width, is_signed }) = program.scalar_format(ty) else {
        return Ok(None);
    };

    if width > 128 {
        return Err(Error::type_mismatch(
            "engine boundary integer up to 128 bits",
            format!("{width}-bit integer"),
        ));
    }

    let mut raw = [0u8; 16];
    raw[..bytes.len()].copy_from_slice(bytes);
    let raw = u128::from_le_bytes(raw);

    if is_signed != 0 {
        let value = sign_extend_i128(raw, width);

        return Ok(Some(program::Value::Int { value, width }));
    }

    Ok(Some(program::Value::UInt { value: raw, width }))
}

/// Sign-extend an integer with the given bit width into i128.
fn sign_extend_i128(value: u128, width: u16) -> i128 {
    if width == 0 || width >= 128 {
        return value as i128;
    }

    let shift = 128 - width;

    ((value << shift) as i128) >> shift
}

/// Return the space used to package one non-cell boundary value.
fn boundary_space(program: &Program, ty: TypeId) -> Result<Space, Error> {
    let layout = program
        .layout(ty)
        .ok_or_else(|| Error::invalid_program(format!("missing boundary layout for {ty:?}")))?;

    let space = match &layout.shape {
        program::LayoutShape::Reference(reference) => reference
            .space()
            .ok_or_else(|| Error::invalid_program(format!("missing reference space for {ty:?}")))?,
        program::LayoutShape::Slice(slice) => slice
            .reference
            .space()
            .ok_or_else(|| Error::invalid_program(format!("missing slice space for {ty:?}")))?,
        _ => Space::Local,
    };

    Ok(space)
}

/// Dematerialize one engine boundary value into frame representation.
pub(crate) fn dematerialize_value(
    program: &Program,
    heap: &Heap,
    shared: &SharedHeap,
    ty: TypeId,
    value: &program::Value,
) -> Result<FrameValue, Error> {
    if !program.is_cell_type(ty) {
        return dematerialize_bytes(program, heap, shared, ty, value);
    }

    let cell = match value {
        program::Value::Void => Cell::ZERO,
        program::Value::Bool(value) => Cell::bool(*value),
        program::Value::Int { value, width } => Cell::int(*value as i64, *width as u8),
        program::Value::UInt { value, width } => Cell::uint(*value as u64, *width as u8),
        program::Value::Float16 { bits } => Cell::from_bits(u64::from(*bits)),
        program::Value::Bfloat16 { bits } => Cell::from_bits(u64::from(*bits)),
        program::Value::Float32 { bits } => Cell::float32(f32::from_bits(*bits)),
        program::Value::Float64 { bits } => Cell::float64(f64::from_bits(*bits)),
        program::Value::Char(value) => Cell::char(*value),
        program::Value::HeapReference(reference) => Cell::heap_reference(*reference),
        program::Value::SharedHeapReference(reference) => Cell::shared_heap_reference(*reference),
        program::Value::Address(address) => Cell::address(*address),
    };

    Ok(FrameValue::cell(ty, cell))
}

/// Return the lowered move slot for one program frame slot.
pub(crate) fn move_slot_from_frame_slot(program: &Program, slot: &FrameSlot) -> MoveSlot {
    MoveSlot::new(
        slot.ty,
        slot.offset,
        slot.byte_len(),
        program.frame_slot_is_cell(slot),
    )
}

/// Store one two-field frame result in field order.
pub(crate) fn store_frame_pair(
    activation: &mut Activation<'_>,
    destination: MoveSlot,
    first: Cell,
    second: Cell,
) -> Result<(), Error> {
    let layout = *activation.require_layout(destination.ty)?;
    let first_field = activation
        .program
        .layout_field_at(&layout, 0)
        .ok_or_else(|| Error::type_mismatch("2-field result", format!("{layout:?}")))?;
    let second_field = activation
        .program
        .layout_field_at(&layout, 1)
        .ok_or_else(|| Error::type_mismatch("2-field result", format!("{layout:?}")))?;

    let first_layout = require_cell_layout(activation.program, first_field.ty)?;
    let second_layout = require_cell_layout(activation.program, second_field.ty)?;
    let first_bytes = encode_cell_bytes(first_layout, first, activation.program.pointer_bytes());
    let second_bytes = encode_cell_bytes(second_layout, second, activation.program.pointer_bytes());

    let first_offset = destination.offset + first_field.offset;
    let second_offset = destination.offset + second_field.offset;
    activation.store_frame_bytes_at(first_offset, first_bytes.as_slice());
    activation.store_frame_bytes_at(second_offset, second_bytes.as_slice());

    Ok(())
}

/// Dematerialize one engine boundary value into frame bytes.
fn dematerialize_bytes(
    program: &Program,
    heap: &Heap,
    shared: &SharedHeap,
    ty: TypeId,
    value: &program::Value,
) -> Result<FrameValue, Error> {
    if let Some(bytes) = dematerialize_scalar_bytes(program, ty, value)? {
        return Ok(FrameValue::bytes(ty, bytes));
    }

    let byte_len = program
        .type_byte_len(ty)
        .ok_or(Error::invalid_instruction())?;
    let mut bytes = vec![0u8; byte_len];

    match (boundary_space(program, ty)?, value) {
        (Space::Local, program::Value::HeapReference(reference)) => {
            let address = heap.heap_base_address() + reference.offset();

            copy_address_to_slice(address, &mut bytes);
        }
        (Space::Shared, program::Value::SharedHeapReference(reference)) => {
            let address = shared.heap_base_address() + reference.offset();

            copy_address_to_slice(address, &mut bytes);
        }
        (space, value) => {
            return Err(Error::type_mismatch(
                format!("{space:?} frame-backed value"),
                format!("{value:?}"),
            ));
        }
    }

    Ok(FrameValue::bytes(ty, bytes.into_boxed_slice()))
}

/// Dematerialize one engine boundary scalar into frame bytes.
fn dematerialize_scalar_bytes(
    program: &Program,
    ty: TypeId,
    value: &program::Value,
) -> Result<Option<Box<[u8]>>, Error> {
    let Some(ScalarFormat::Int { width, is_signed }) = program.scalar_format(ty) else {
        return Ok(None);
    };

    let byte_len = (width as usize).div_ceil(8);
    let raw = match (is_signed, value) {
        (
            1,
            program::Value::Int {
                value,
                width: value_width,
            },
        ) if *value_width == width => *value as u128,
        (
            0,
            program::Value::UInt {
                value,
                width: value_width,
            },
        ) if *value_width == width => *value,
        _ => {
            return Err(Error::type_mismatch(
                format!("{width}-bit integer"),
                format!("{value:?}"),
            ));
        }
    };

    Ok(Some(
        raw.to_le_bytes()[..byte_len].to_vec().into_boxed_slice(),
    ))
}

/// Copy bytes from one native address into one mutable slice.
#[inline(always)]
fn copy_address_to_slice(address: usize, destination: &mut [u8]) {
    unsafe {
        ptr::copy_nonoverlapping(
            address as *const u8,
            destination.as_mut_ptr(),
            destination.len(),
        );
    }
}

/// Materialize one cell into one engine boundary value.
pub(crate) fn materialize_cell(
    program: &Program,
    ty: TypeId,
    value: Cell,
) -> Result<program::Value, Error> {
    let Some(layout) = program.layout(ty) else {
        return Err(Error::type_mismatch(
            "cell value layout",
            format!("missing layout for {ty:?}"),
        ));
    };

    match &layout.shape {
        program::LayoutShape::None => Ok(program::Value::Void),
        program::LayoutShape::Scalar(ScalarFormat::Boolean) => {
            Ok(program::Value::Bool(value.as_bool()))
        }
        program::LayoutShape::Scalar(ScalarFormat::Int {
            width,
            is_signed: 1,
        }) => Ok(program::Value::Int {
            value: value.as_i64() as i128,
            width: *width,
        }),
        program::LayoutShape::Scalar(ScalarFormat::Int {
            width,
            is_signed: 0,
        }) => Ok(program::Value::UInt {
            value: value.as_u64() as u128,
            width: *width,
        }),
        program::LayoutShape::Scalar(ScalarFormat::Float {
            format: mir::FloatType::Float16,
        }) => Ok(program::Value::float16_bits(value.bits() as u16)),
        program::LayoutShape::Scalar(ScalarFormat::Float {
            format: mir::FloatType::Bfloat16,
        }) => Ok(program::Value::bfloat16_bits(value.bits() as u16)),
        program::LayoutShape::Scalar(ScalarFormat::Float {
            format: mir::FloatType::Float32,
        }) => Ok(program::Value::Float32 {
            bits: value.as_f32().to_bits(),
        }),
        program::LayoutShape::Scalar(ScalarFormat::Float {
            format: mir::FloatType::Float64,
        }) => Ok(program::Value::Float64 {
            bits: value.as_f64().to_bits(),
        }),
        program::LayoutShape::Reference(reference)
            if reference.cell_layout() == Some(CellLayout::HeapReference) =>
        {
            Ok(program::Value::HeapReference(value.as_heap_reference()))
        }
        program::LayoutShape::Reference(reference)
            if reference.cell_layout() == Some(CellLayout::SharedHeapReference) =>
        {
            Ok(program::Value::SharedHeapReference(
                value.as_shared_heap_reference(),
            ))
        }
        program::LayoutShape::Reference(reference)
            if reference.cell_layout() == Some(CellLayout::Address) =>
        {
            Ok(program::Value::Address(value.as_address()))
        }
        _ => Err(Error::type_mismatch(
            "cell value",
            format!("{:?}", layout.shape),
        )),
    }
}

/// Move one lowered frame slot into another frame.
fn move_frame_slot(
    source_frame: &Frame,
    source: MoveSlot,
    dest_frame: &mut Frame,
    destination: MoveSlot,
) -> Result<(), Error> {
    if source.byte_len() != destination.byte_len() || source.is_cell() != destination.is_cell() {
        return Err(Error::type_mismatch(
            format!(
                "{} bytes, cell={}",
                destination.byte_len(),
                destination.is_cell()
            ),
            format!("{} bytes, cell={}", source.byte_len(), source.is_cell()),
        ));
    }

    if destination.is_cell() {
        let value = source_frame.read_cell_at(source.offset);
        dest_frame.write_cell_at(destination.offset, value);

        return Ok(());
    }

    let bytes = move_slot_bytes(source_frame, source);
    move_slot_bytes_mut(dest_frame, destination).copy_from_slice(bytes);

    Ok(())
}

/// Write void into one lowered frame slot.
fn store_void_slot(frame: &mut Frame, destination: MoveSlot) {
    if destination.is_cell() {
        frame.write_cell_at(destination.offset, Cell::ZERO);

        return;
    }

    move_slot_bytes_mut(frame, destination).fill(0);
}

/// Borrow one lowered frame slot.
fn move_slot_bytes(frame: &Frame, slot: MoveSlot) -> &[u8] {
    let start = slot.offset as usize;
    let end = start + slot.byte_len() as usize;

    &frame.bytes()[start..end]
}

/// Borrow one lowered frame slot mutably.
fn move_slot_bytes_mut(frame: &mut Frame, slot: MoveSlot) -> &mut [u8] {
    let start = slot.offset as usize;
    let end = start + slot.byte_len() as usize;

    &mut frame.bytes_mut()[start..end]
}

/// Move call arguments between two frames.
pub(crate) fn move_arguments_between_frames(
    source_frame: &Frame,
    dest_frame: &mut Frame,
    param_pool: &[MoveSlot],
    params: ArgumentRange,
    argument_pool: &[MoveSlot],
    arguments: ArgumentRange,
) -> Result<(), Error> {
    let argument_slice = arguments.slice(argument_pool);
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let Some(argument) = argument_slice.get(index) else {
            store_void_slot(dest_frame, *param);

            continue;
        };

        move_frame_slot(source_frame, *argument, dest_frame, *param)?;
    }

    Ok(())
}

/// Move frame values using one lowered move range.
pub(crate) fn move_values(
    source_frame: &Frame,
    dest_frame: &mut Frame,
    moves: MoveRange,
    move_pool: &[MovePair],
) -> Result<(), Error> {
    let pairs = moves.slice(move_pool);
    for pair in pairs {
        if let Some(source) = pair.source.as_slot() {
            move_frame_slot(source_frame, source, dest_frame, pair.dest)?;
        } else {
            store_void_slot(dest_frame, pair.dest);
        }
    }

    Ok(())
}

/// Move frame values within one frame.
pub(crate) fn move_values_within_frame(
    frame: &mut Frame,
    moves: MoveRange,
    move_pool: &[MovePair],
) -> Result<(), Error> {
    let pairs = moves.slice(move_pool);
    if pairs.is_empty() {
        return Ok(());
    }

    // keep scalar edge moves on the cell-only path
    let mut is_cell_move = true;
    for pair in pairs {
        if !pair.dest.is_cell() {
            is_cell_move = false;
            break;
        }

        if let Some(source) = pair.source.as_slot()
            && !source.is_cell()
        {
            is_cell_move = false;
            break;
        }
    }

    if is_cell_move {
        let mut values = SmallVec::<[Cell; 16]>::with_capacity(pairs.len());

        // collect sources before writing destinations
        for pair in pairs {
            let value = if let Some(source) = pair.source.as_slot() {
                frame.read_cell_at(source.offset)
            } else {
                Cell::ZERO
            };

            values.push(value);
        }

        // store destinations after preserving parallel move semantics
        for (pair, value) in pairs.iter().zip(values) {
            frame.write_cell_at(pair.dest.offset, value);
        }

        return Ok(());
    }

    let mut values = SmallVec::<[BufferedSlotValue; 16]>::with_capacity(pairs.len());

    // collect sources before writing destinations
    for pair in pairs {
        let value = if let Some(source) = pair.source.as_slot() {
            if source.byte_len() != pair.dest.byte_len() || source.is_cell() != pair.dest.is_cell()
            {
                return Err(Error::type_mismatch(
                    format!(
                        "{} bytes, cell={}",
                        pair.dest.byte_len(),
                        pair.dest.is_cell()
                    ),
                    format!("{} bytes, cell={}", source.byte_len(), source.is_cell()),
                ));
            }

            if source.is_cell() {
                BufferedSlotValue::Cell(frame.read_cell_at(source.offset))
            } else {
                let mut bytes = SmallVec::<[u8; 32]>::with_capacity(source.byte_len() as usize);
                bytes.extend_from_slice(move_slot_bytes(frame, source));

                BufferedSlotValue::Bytes(bytes)
            }
        } else if pair.dest.is_cell() {
            BufferedSlotValue::Cell(Cell::ZERO)
        } else {
            let mut bytes = SmallVec::<[u8; 32]>::with_capacity(pair.dest.byte_len() as usize);
            bytes.resize(pair.dest.byte_len() as usize, 0);

            BufferedSlotValue::Bytes(bytes)
        };

        values.push(value);
    }

    // store destinations after preserving parallel move semantics
    for (pair, value) in pairs.iter().zip(values) {
        match value {
            BufferedSlotValue::Cell(value) => {
                frame.write_cell_at(pair.dest.offset, value);
            }
            BufferedSlotValue::Bytes(bytes) => {
                move_slot_bytes_mut(frame, pair.dest).copy_from_slice(&bytes);
            }
        }
    }

    Ok(())
}

/// Copy one ordered argument list out of the current frame.
pub(crate) fn load_arguments(
    frame: &Frame,
    argument_pool: &[MoveSlot],
    arguments: ArgumentRange,
) -> Result<SmallVec<[FrameValue; 16]>, Error> {
    let argument_slice = arguments.slice(argument_pool);
    let mut collected_arguments = SmallVec::with_capacity(argument_slice.len());
    for argument in argument_slice {
        let value = load_frame_slot_value(frame, *argument);

        collected_arguments.push(value);
    }

    Ok(collected_arguments)
}

/// Copy one lowered argument move range out of the current frame.
pub(crate) fn load_moved_arguments(
    frame: &Frame,
    move_pool: &[MovePair],
    moves: MoveRange,
) -> Result<SmallVec<[FrameValue; 16]>, Error> {
    let pairs = moves.slice(move_pool);
    let mut arguments = SmallVec::with_capacity(pairs.len());
    for pair in pairs {
        let value = if let Some(source) = pair.source.as_slot() {
            load_frame_slot_value(frame, source)
        } else {
            FrameValue::cell(pair.dest.ty, Cell::ZERO)
        };

        arguments.push(value);
    }

    Ok(arguments)
}

/// Write owned frame values into parameter slots.
pub(crate) fn store_parameters(
    frame: &mut Frame,
    param_pool: &[MoveSlot],
    params: ArgumentRange,
    arguments: &[FrameValue],
) -> Result<(), Error> {
    let param_slice = params.slice(param_pool);

    for (index, param) in param_slice.iter().enumerate() {
        let value = match arguments.get(index).cloned() {
            Some(value) => value,
            None => {
                let ty = param.ty;
                FrameValue::cell(ty, Cell::ZERO)
            }
        };

        store_frame_slot_value(frame, *param, value)?;
    }

    Ok(())
}

/// Store one owned frame value into a lowered frame slot.
pub(crate) fn store_frame_slot_value(
    frame: &mut Frame,
    destination: MoveSlot,
    value: FrameValue,
) -> Result<(), Error> {
    match (destination.is_cell(), value.body) {
        (true, FrameValueBody::Cell(value)) => frame.write_cell_at(destination.offset, value),
        (false, FrameValueBody::Bytes(bytes)) if bytes.len() == destination.byte_len() as usize => {
            move_slot_bytes_mut(frame, destination).copy_from_slice(&bytes);
        }
        (false, FrameValueBody::Cell(value)) => {
            return Err(Error::type_mismatch(
                "byte frame value",
                format!("{value:?}"),
            ));
        }
        (true, FrameValueBody::Bytes(bytes)) => {
            return Err(Error::type_mismatch(
                "cell frame value",
                format!("{} bytes", bytes.len()),
            ));
        }
        (false, FrameValueBody::Bytes(bytes)) => {
            return Err(Error::type_mismatch(
                format!("{} frame bytes", destination.byte_len()),
                format!("{} frame bytes", bytes.len()),
            ));
        }
    }

    Ok(())
}
