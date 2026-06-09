use std::ptr;

use destack_engine as engine;
use destack_heap::{AllocationCache, GcWorker, Heap, SharedHeap};
use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::{Error, ReferenceKind};
use crate::machine::{Activation, Frame};
use crate::program::{
    AddressSpace, ArgumentRange, Instruction, MovePair, MoveRange, MoveSlot, MoveSource, Program,
    Projection, ProjectionId, ValueShape, address_space_from_reference, encode_cell_bytes,
    repr_type, value_shape_from_type,
};
use crate::{Cell, FramePointer};

use super::access;

/// Return the byte offset for one frame element projection.
#[inline(always)]
pub(super) fn frame_element_offset(
    activation: &Activation<'_>,
    access: Projection,
    index: u32,
) -> usize {
    let index = activation.load_cell_at(index).as_u64();

    access.byte_offset + access.byte_stride * index as usize
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

    let value = access::load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(pointer.address());
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

    let value = access::load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(pointer.address());
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

    access::store_scalar_at_address::<BYTE_LEN>(pointer.address(), value);

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

    access::store_scalar_at_address::<BYTE_LEN>(pointer.address(), value);

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

/// One owned value copied out of a frame with its MIR type.
#[derive(Clone, Debug)]
pub(crate) struct FrameValue {
    /// The MIR type carried with the raw value bits.
    ty: mir::LocalNodeId<mir::Type>,
    /// The owned value body.
    body: FrameValueBody,
}

impl FrameValue {
    /// Create one cell value.
    #[inline]
    pub(crate) fn cell(ty: mir::LocalNodeId<mir::Type>, value: Cell) -> Self {
        Self {
            ty,
            body: FrameValueBody::Cell(value),
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
}

/// Encode one function entry argument into frame bytes.
pub(crate) fn encode_argument_bytes(
    activation: &mut Activation<'_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Cell,
) -> Result<Vec<u8>, Error> {
    let layout = activation.require_layout(ty)?.clone();
    if layout.is_cell() {
        return Ok(encode_cell_bytes(activation.machine.tree(), ty, value)?
            .as_slice()
            .to_vec());
    }

    let mut bytes = vec![0u8; layout.byte_len];
    store_argument_bytes(activation, ty, value, &mut bytes)?;

    Ok(bytes)
}

/// Store one field value into destination frame bytes.
pub(crate) fn store_frame_fields<F>(
    activation: &mut Activation<'_>,
    destination: mir::Value,
    mut field_value: F,
) -> Result<(), Error>
where
    F: FnMut(&mut Activation<'_>, u32, mir::LocalNodeId<mir::Type>) -> Result<Cell, Error>,
{
    let ty = activation.value_type(destination)?;
    let layout = activation.require_layout(ty)?.clone();
    let field_count = layout
        .field_count()
        .ok_or(Error::type_mismatch("field frame value", format!("{ty:?}")))?;

    // validate the destination once before incremental writes
    if activation.value_bytes(destination)?.len() != layout.byte_len {
        return Err(Error::invalid_instruction());
    }

    // encode each field into its physical byte range
    for index in 0..field_count {
        let index = index as u32;
        let field = layout
            .field(index)
            .ok_or(Error::invalid_field_access(index, field_count))?;
        let value = field_value(activation, index, field.ty)?;
        let value_end = field.offset + field.byte_len;
        let value_bytes = encode_cell_bytes(activation.machine.tree(), field.ty, value)?;
        if value_bytes.len() != field.byte_len {
            return Err(Error::invalid_instruction());
        }

        let destination_bytes = activation.value_bytes_mut(destination)?;
        let value_window = destination_bytes
            .get_mut(field.offset..value_end)
            .ok_or(Error::invalid_field_access(index, layout.byte_len))?;

        value_window.copy_from_slice(value_bytes.as_slice());
    }

    Ok(())
}

/// Store one function entry argument into one byte range.
fn store_argument_bytes(
    activation: &mut Activation<'_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Cell,
    destination: &mut [u8],
) -> Result<(), Error> {
    if activation.require_layout(ty)?.is_cell() {
        let bytes = encode_cell_bytes(activation.machine.tree(), ty, value)?;
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

/// Return the MIR type stored in one frame value.
pub(crate) fn frame_value_type(
    program: &Program,
    frame: &Frame,
    value: mir::Value,
) -> Result<mir::LocalNodeId<mir::Type>, Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout())
        .ok_or(Error::invalid_instruction())?;
    let slot = frame_layout
        .value(value.0)
        .ok_or(Error::invalid_instruction())?;

    Ok(program.type_for_storage_id(slot.layout))
}

/// Return the addressable cell for one frame value.
fn frame_value_cell(program: &Program, frame: &Frame, value: mir::Value) -> Result<Cell, Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout())
        .ok_or(Error::invalid_instruction())?;
    let slot = frame_layout
        .value(value.0)
        .ok_or(Error::invalid_instruction())?;
    let layout = program.layout_for_storage_id(slot.layout).ok_or_else(|| {
        Error::internal(format!(
            "missing frame storage layout: layout={:?}",
            slot.layout
        ))
    })?;

    if layout.is_cell() {
        return Ok(frame.read_cell(slot));
    }

    Ok(Cell::frame_pointer(FramePointer::from_address(
        frame.slot_address(slot),
    )))
}

/// Load one cell or frame byte range into an owned frame value.
pub(crate) fn frame_value_from_cell(
    program: &Program,
    frames: &[Frame],
    ty: mir::LocalNodeId<mir::Type>,
    value: Cell,
) -> Result<FrameValue, Error> {
    let layout = program
        .layout(ty)
        .ok_or_else(|| Error::internal(format!("missing frame storage layout: type={ty:?}")))?;
    if layout.is_cell() {
        return Ok(FrameValue::cell(ty, value));
    }

    let pointer = value.as_frame_pointer();
    let frame = frames
        .iter()
        .find(|frame| frame.owns_stack_range(pointer.address(), layout.byte_len))
        .ok_or(Error::invalid_space("frame", format!("{value:?}")))?;
    let start = pointer
        .address()
        .checked_sub(frame.base_address())
        .ok_or(Error::invalid_space("frame", format!("{value:?}")))?;
    let end = start + layout.byte_len;
    let bytes = frame
        .bytes()
        .get(start..end)
        .ok_or(Error::invalid_space("frame", format!("{value:?}")))?
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
    let value = frame_value_cell(program, frame, value)?;

    frame_value_from_cell(program, frames, ty, value)
}

/// Load one lowered frame slot into an owned value.
fn load_frame_slot_value(program: &Program, frame: &Frame, slot: MoveSlot) -> FrameValue {
    let ty = program.type_for_storage_id(slot.layout);
    if slot.is_cell {
        return FrameValue::cell(ty, frame.read_cell_at(slot.offset));
    }

    let bytes = move_slot_bytes(frame, slot).to_vec().into_boxed_slice();

    FrameValue::bytes(ty, bytes)
}

/// Store one owned frame value into a destination frame slot.
pub(crate) fn store_frame_value(
    program: &Program,
    dest_frame: &mut Frame,
    destination: mir::Value,
    value: FrameValue,
) -> Result<(), Error> {
    let frame_layout = program
        .frame_layout_by_id(dest_frame.frame_layout())
        .ok_or(Error::invalid_instruction())?;
    let slot = frame_layout
        .value(destination.0)
        .ok_or(Error::invalid_instruction())?;
    let layout = program.layout_for_storage_id(slot.layout).ok_or_else(|| {
        Error::internal(format!(
            "missing destination storage layout: layout={:?}",
            slot.layout
        ))
    })?;

    match (layout.is_cell(), value.body) {
        (true, FrameValueBody::Cell(value)) => dest_frame.write_cell(slot, value),
        (false, FrameValueBody::Bytes(bytes)) if bytes.len() == slot.byte_len as usize => {
            dest_frame.slot_bytes_mut(slot).copy_from_slice(&bytes);
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
                format!("{} bytes", slot.byte_len),
                format!("{} bytes", bytes.len()),
            ));
        }
    }

    Ok(())
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
    shared_gc: &GcWorker,
    value: FrameValue,
) -> Result<engine::Value, Error> {
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

            match boundary_address_space(program, value.ty) {
                AddressSpace::Local => {
                    let site = heap.options().allocation_site_for_shape(shape);
                    let reference = heap.allocate_bytes(site, shape.trace_map, &bytes)?;

                    Ok(engine::Value::HeapReference(reference))
                }
                AddressSpace::Shared => {
                    let site = shared.options().allocation_site_for_shape(shape);
                    let reference = shared.allocate_bytes(
                        shared_gc,
                        shared_cache,
                        site,
                        shape.trace_map,
                        &bytes,
                        program.trace_table(),
                    )?;

                    Ok(engine::Value::SharedHeapReference(reference))
                }
                address_space => Err(Error::invalid_pointer_type(format!("{address_space:?}"))),
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
        return Err(Error::type_mismatch(
            "engine boundary integer up to 128 bits",
            format!("{width}-bit integer"),
        ));
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

/// Return the address space used to package one non-cell boundary value.
fn boundary_address_space(program: &Program, ty: mir::LocalNodeId<mir::Type>) -> AddressSpace {
    let ty = repr_type(&program.tree, ty);

    match program.tree.get(ty) {
        mir::Type::Slice { kind, space, .. } => address_space_from_reference(space.clone(), *kind),
        _ => AddressSpace::Local,
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
    let layout = program.layout(ty).ok_or(Error::invalid_instruction())?;
    if !layout.is_cell() {
        return dematerialize_bytes(program, heap, shared, ty, value);
    }

    let cell = match value {
        engine::Value::Void => Cell::ZERO,
        engine::Value::Bool(value) => Cell::bool(*value),
        engine::Value::Int { value, width } => Cell::int(*value as i64, *width as u8),
        engine::Value::UInt { value, width } => Cell::uint(*value as u64, *width as u8),
        engine::Value::Float16 { bits } => Cell::from_bits(u64::from(*bits)),
        engine::Value::Bfloat16 { bits } => Cell::from_bits(u64::from(*bits)),
        engine::Value::Float32 { bits } => Cell::float32(f32::from_bits(*bits)),
        engine::Value::Float64 { bits } => Cell::float64(f64::from_bits(*bits)),
        engine::Value::Char(value) => Cell::char(*value),
        engine::Value::HeapReference(reference) => Cell::heap_reference(*reference),
        engine::Value::SharedHeapReference(reference) => Cell::shared_heap_reference(*reference),
        engine::Value::Address(address) => Cell::address(*address),
    };

    Ok(FrameValue::cell(ty, cell))
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

    let layout = program.layout(ty).ok_or(Error::invalid_instruction())?;
    let mut bytes = vec![0u8; layout.byte_len];

    match (boundary_address_space(program, ty), value) {
        (AddressSpace::Local, engine::Value::HeapReference(reference)) => {
            let address = heap.heap_base_address() + reference.offset();

            copy_address_to_slice(address, &mut bytes);
        }
        (AddressSpace::Shared, engine::Value::SharedHeapReference(reference)) => {
            let address = shared.heap_base_address() + reference.offset();

            copy_address_to_slice(address, &mut bytes);
        }
        (address_space, value) => {
            return Err(Error::type_mismatch(
                format!("{address_space:?} frame-backed value"),
                format!("{value:?}"),
            ));
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
    ty: mir::LocalNodeId<mir::Type>,
    value: Cell,
) -> Result<engine::Value, Error> {
    match value_shape_from_type(&program.tree, ty) {
        Some(ValueShape::Void) => Ok(engine::Value::Void),
        Some(ValueShape::Bool) => Ok(engine::Value::Bool(value.as_bool())),
        Some(ValueShape::Int {
            width,
            signed: true,
        }) => Ok(engine::Value::Int {
            value: value.as_i64() as i128,
            width,
        }),
        Some(ValueShape::Int {
            width,
            signed: false,
        }) => Ok(engine::Value::UInt {
            value: value.as_u64() as u128,
            width,
        }),
        Some(ValueShape::Float {
            format: mir::FloatType::Float16,
        }) => Ok(engine::Value::float16_bits(value.bits() as u16)),
        Some(ValueShape::Float {
            format: mir::FloatType::Bfloat16,
        }) => Ok(engine::Value::bfloat16_bits(value.bits() as u16)),
        Some(ValueShape::Float {
            format: mir::FloatType::Float32,
        }) => Ok(engine::Value::Float32 {
            bits: value.as_f32().to_bits(),
        }),
        Some(ValueShape::Float {
            format: mir::FloatType::Float64,
        }) => Ok(engine::Value::Float64 {
            bits: value.as_f64().to_bits(),
        }),
        Some(ValueShape::Char) => {
            let value = value.as_char().ok_or(Error::invalid_instruction())?;

            Ok(engine::Value::Char(value))
        }
        Some(ValueShape::Pointer {
            address_space: AddressSpace::Local,
            ..
        }) => Ok(engine::Value::HeapReference(value.as_heap_reference())),
        Some(ValueShape::Pointer {
            address_space: AddressSpace::Shared,
            ..
        }) => Ok(engine::Value::SharedHeapReference(
            value.as_shared_heap_reference(),
        )),
        Some(ValueShape::Pointer {
            address_space: AddressSpace::Raw,
            ..
        }) => Ok(engine::Value::Address(value.as_address())),
        _ => Err(Error::type_mismatch(
            "cell value",
            format!("{:?}", value_shape_from_type(&program.tree, ty)),
        )),
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
        .frame_layout_by_id(source_frame.frame_layout())
        .ok_or(Error::invalid_instruction())?;
    let dest_layout = program
        .frame_layout_by_id(dest_frame.frame_layout())
        .ok_or(Error::invalid_instruction())?;

    let source_slot = source_layout
        .value(source.0)
        .ok_or(Error::invalid_instruction())?;
    let dest_slot = dest_layout
        .value(destination.0)
        .ok_or(Error::invalid_instruction())?;

    if source_slot.byte_len != dest_slot.byte_len || source_slot.is_cell != dest_slot.is_cell {
        return Err(Error::type_mismatch(
            format!("{} bytes, cell={}", dest_slot.byte_len, dest_slot.is_cell),
            format!(
                "{} bytes, cell={}",
                source_slot.byte_len, source_slot.is_cell
            ),
        ));
    }

    if dest_slot.is_cell {
        let value = source_frame.read_cell(source_slot);
        dest_frame.write_cell(dest_slot, value);

        return Ok(());
    }

    let bytes = source_frame.slot_bytes(source_slot);
    dest_frame.slot_bytes_mut(dest_slot).copy_from_slice(bytes);

    Ok(())
}

/// Move one lowered frame slot into another frame.
fn move_frame_slot(
    source_frame: &Frame,
    source: MoveSlot,
    dest_frame: &mut Frame,
    destination: MoveSlot,
) -> Result<(), Error> {
    if source.byte_len != destination.byte_len || source.is_cell != destination.is_cell {
        return Err(Error::type_mismatch(
            format!(
                "{} bytes, cell={}",
                destination.byte_len, destination.is_cell
            ),
            format!("{} bytes, cell={}", source.byte_len, source.is_cell),
        ));
    }

    if destination.is_cell {
        let value = source_frame.read_cell_at(source.offset);
        dest_frame.write_cell_at(destination.offset, value);

        return Ok(());
    }

    let bytes = move_slot_bytes(source_frame, source);
    move_slot_bytes_mut(dest_frame, destination).copy_from_slice(bytes);

    Ok(())
}

/// Write void to one frame value.
fn store_void_value(
    program: &Program,
    frame: &mut Frame,
    destination: mir::Value,
) -> Result<(), Error> {
    let frame_layout = program
        .frame_layout_by_id(frame.frame_layout())
        .ok_or(Error::invalid_instruction())?;
    let slot = frame_layout
        .value(destination.0)
        .ok_or(Error::invalid_instruction())?;

    if slot.is_cell {
        frame.write_cell(slot, Cell::ZERO);

        return Ok(());
    }

    frame.slot_bytes_mut(slot).fill(0);

    Ok(())
}

/// Write void into one lowered frame slot.
fn store_void_slot(frame: &mut Frame, destination: MoveSlot) {
    if destination.is_cell {
        frame.write_cell_at(destination.offset, Cell::ZERO);

        return;
    }

    move_slot_bytes_mut(frame, destination).fill(0);
}

/// Borrow one lowered frame slot.
fn move_slot_bytes(frame: &Frame, slot: MoveSlot) -> &[u8] {
    let start = slot.offset as usize;
    let end = start + slot.byte_len as usize;

    &frame.bytes()[start..end]
}

/// Borrow one lowered frame slot mutably.
fn move_slot_bytes_mut(frame: &mut Frame, slot: MoveSlot) -> &mut [u8] {
    let start = slot.offset as usize;
    let end = start + slot.byte_len as usize;

    &mut frame.bytes_mut()[start..end]
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
    source_frame: &Frame,
    dest_frame: &mut Frame,
    moves: MoveRange,
    move_pool: &[MovePair],
) -> Result<(), Error> {
    let pairs = moves.slice(move_pool);
    for pair in pairs {
        match pair.source {
            MoveSource::Slot(source) => {
                move_frame_slot(source_frame, source, dest_frame, pair.dest)?;
            }
            MoveSource::Void => {
                store_void_slot(dest_frame, pair.dest);
            }
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
        if !pair.dest.is_cell {
            is_cell_move = false;
            break;
        }

        if let MoveSource::Slot(source) = pair.source
            && !source.is_cell
        {
            is_cell_move = false;
            break;
        }
    }

    if is_cell_move {
        let mut values = SmallVec::<[Cell; 16]>::with_capacity(pairs.len());

        // collect sources before writing destinations
        for pair in pairs {
            let value = match pair.source {
                MoveSource::Slot(source) => frame.read_cell_at(source.offset),
                MoveSource::Void => Cell::ZERO,
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
        let value = match pair.source {
            MoveSource::Slot(source) => {
                if source.byte_len != pair.dest.byte_len || source.is_cell != pair.dest.is_cell {
                    return Err(Error::type_mismatch(
                        format!("{} bytes, cell={}", pair.dest.byte_len, pair.dest.is_cell),
                        format!("{} bytes, cell={}", source.byte_len, source.is_cell),
                    ));
                }

                if source.is_cell {
                    BufferedSlotValue::Cell(frame.read_cell_at(source.offset))
                } else {
                    let mut bytes = SmallVec::<[u8; 32]>::with_capacity(source.byte_len as usize);
                    bytes.extend_from_slice(move_slot_bytes(frame, source));

                    BufferedSlotValue::Bytes(bytes)
                }
            }
            MoveSource::Void => {
                if pair.dest.is_cell {
                    BufferedSlotValue::Cell(Cell::ZERO)
                } else {
                    let mut bytes =
                        SmallVec::<[u8; 32]>::with_capacity(pair.dest.byte_len as usize);
                    bytes.resize(pair.dest.byte_len as usize, 0);

                    BufferedSlotValue::Bytes(bytes)
                }
            }
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
    frame: &Frame,
    move_pool: &[MovePair],
    moves: MoveRange,
) -> Result<SmallVec<[FrameValue; 16]>, Error> {
    let pairs = moves.slice(move_pool);
    let mut arguments = SmallVec::with_capacity(pairs.len());
    for pair in pairs {
        let value = match pair.source {
            MoveSource::Slot(source) => load_frame_slot_value(program, frame, source),
            MoveSource::Void => {
                let destination_type = program.type_for_storage_id(pair.dest.layout);

                FrameValue::cell(destination_type, Cell::ZERO)
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
                FrameValue::cell(ty, Cell::ZERO)
            }
        };

        store_frame_value(program, frame, *param, value)?;
    }

    Ok(())
}
