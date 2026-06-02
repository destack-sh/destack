use std::ptr;

use destack_engine::StaticAddress;
use destack_heap::{HeapReference, SharedHeapReference};

use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::program::{CellLayout, Projection, SlotProjection};
use crate::{Cell, FramePointer, StackPointer};

const POINTER_BYTE_LEN: usize = usize::BITS as usize / 8;

/// Debug assert one lowered scalar access.
#[inline(always)]
fn debug_assert_cell_access(access: Projection) {
    debug_assert!(access.is_cell());
    debug_assert!(access.byte_len <= Cell::BYTE_LEN);
}

/// Return one lowered scalar layout.
#[inline(always)]
fn scalar_layout(access: Projection) -> CellLayout {
    debug_assert!(access.cell_layout.is_some());

    // SAFETY: scalar projections are lowered with a cell layout and asserted in debug builds
    unsafe { access.cell_layout.unwrap_unchecked() }
}

/// Return one lowered slot layout.
#[inline(always)]
fn slot_layout(access: SlotProjection) -> CellLayout {
    access.cell_layout
}

/// Return one local heap native address.
#[inline(always)]
fn local_heap_address(
    activation: &Activation<'_>,
    reference: HeapReference,
    byte_offset: usize,
) -> usize {
    activation.heap_address(reference, byte_offset)
}

/// Return one shared heap native address.
#[inline(always)]
fn shared_heap_address(
    activation: &Activation<'_>,
    reference: SharedHeapReference,
    byte_offset: usize,
) -> usize {
    activation.shared_heap_address(reference, byte_offset)
}

/// Return one raw native address.
#[inline(always)]
fn raw_address(address: usize, byte_offset: usize) -> usize {
    address + byte_offset
}

/// Load bytes from one native address.
#[inline(always)]
fn load_native_bytes(address: usize, destination: *mut u8, destination_len: usize) {
    // SAFETY: callers pass lowered addresses and destination storage for destination_len bytes
    unsafe {
        ptr::copy_nonoverlapping(address as *const u8, destination, destination_len);
    }
}

/// Store bytes to one native address.
#[inline(always)]
fn store_native_bytes(address: usize, bytes: &[u8]) {
    // SAFETY: callers pass lowered writable addresses for the full byte slice
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }
}

/// Sign extend one loaded scalar.
#[inline(always)]
fn sign_extend_scalar(raw: u64, byte_len: usize) -> u64 {
    let shift = u64::BITS as usize - byte_len * 8;

    ((raw << shift) as i64 >> shift) as u64
}

/// Load one unsigned scalar payload from one address.
#[inline(always)]
fn load_unsigned_raw(address: usize, byte_len: usize) -> u64 {
    let source = address as *const u8;

    // SAFETY: callers pass lowered readable addresses for at least byte_len bytes
    unsafe {
        match byte_len {
            1 => source.read() as u64,
            2 => u16::from_le(source.cast::<u16>().read_unaligned()) as u64,
            4 => u32::from_le(source.cast::<u32>().read_unaligned()) as u64,
            8 => u64::from_le(source.cast::<u64>().read_unaligned()),
            _ => load_bytewise_unsigned_raw(source, byte_len),
        }
    }
}

/// Load one lowered scalar from one native address.
#[inline(always)]
pub(super) fn load_scalar_at_address<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    address: usize,
) -> Cell {
    let raw = load_unsigned_raw(address, BYTE_LEN);
    let raw = if IS_SIGNED && BYTE_LEN < Cell::BYTE_LEN {
        sign_extend_scalar(raw, BYTE_LEN)
    } else {
        raw
    };

    Cell::from_bits(raw)
}

/// Store one lowered scalar to one native address.
#[inline(always)]
pub(super) fn store_scalar_at_address<const BYTE_LEN: usize>(address: usize, value: Cell) {
    store_unsigned_raw(address, value.bits(), BYTE_LEN);
}

/// Load one non-native-width unsigned scalar payload from one address.
#[cold]
#[inline(never)]
fn load_bytewise_unsigned_raw(source: *const u8, byte_len: usize) -> u64 {
    // SAFETY: callers pass a readable source range of byte_len bytes
    let source = unsafe { std::slice::from_raw_parts(source, byte_len) };
    let mut raw = 0u64;

    for (index, byte) in source.iter().copied().enumerate() {
        raw |= (byte as u64) << (index * 8);
    }

    raw
}

/// Store one unsigned scalar payload to one address.
#[inline(always)]
fn store_unsigned_raw(address: usize, raw: u64, byte_len: usize) {
    let destination = address as *mut u8;

    // SAFETY: callers pass lowered writable addresses for at least byte_len bytes
    unsafe {
        match byte_len {
            1 => destination.write(raw as u8),
            2 => destination
                .cast::<u16>()
                .write_unaligned((raw as u16).to_le()),
            4 => destination
                .cast::<u32>()
                .write_unaligned((raw as u32).to_le()),
            8 => destination.cast::<u64>().write_unaligned(raw.to_le()),
            _ => store_bytewise_unsigned_raw(destination, raw, byte_len),
        }
    }
}

/// Store one non-native-width unsigned scalar payload to one address.
#[cold]
#[inline(never)]
fn store_bytewise_unsigned_raw(destination: *mut u8, raw: u64, byte_len: usize) {
    // SAFETY: callers pass a writable destination range of byte_len bytes
    let destination = unsafe { std::slice::from_raw_parts_mut(destination, byte_len) };

    for (index, byte) in destination.iter_mut().enumerate() {
        *byte = ((raw >> (index * 8)) & 0xFF) as u8;
    }
}

/// Load one typed scalar from one native address.
#[inline(always)]
pub(super) fn load_scalar_by_layout_at_address(address: usize, layout: CellLayout) -> Cell {
    let raw = load_unsigned_raw(address, layout.byte_len(POINTER_BYTE_LEN));

    layout.decode(raw)
}

/// Store one typed scalar to one native address.
#[inline(always)]
pub(super) fn store_scalar_by_layout_at_address(address: usize, layout: CellLayout, value: Cell) {
    let raw = layout.encode(value);

    store_unsigned_raw(address, raw, layout.byte_len(POINTER_BYTE_LEN));
}

/// Load one scalar from raw memory.
pub(crate) fn load_raw_scalar_by_layout(
    _machine: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
) -> Cell {
    debug_assert_cell_access(access);

    let address = raw_address(pointer.as_address(), access.byte_offset);

    load_scalar_by_layout_at_address(address, scalar_layout(access))
}

/// Store one scalar into raw memory.
pub(crate) fn store_raw_scalar_by_layout(
    _machine: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    value: Cell,
) {
    debug_assert_cell_access(access);
    let address = raw_address(pointer.as_address(), access.byte_offset);

    store_scalar_by_layout_at_address(address, scalar_layout(access), value);
}

/// Store bytes into a raw pointer.
pub(crate) fn store_raw_bytes(
    _machine: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    bytes: &[u8],
) -> Result<(), Error> {
    let address = raw_address(pointer.as_address(), access.byte_offset);
    store_native_bytes(address, bytes);

    Ok(())
}

/// Load bytes from a raw pointer.
#[inline(always)]
pub(crate) fn load_raw_bytes(
    _machine: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let address = raw_address(pointer.as_address(), access.byte_offset);
    load_native_bytes(address, destination, destination_len);

    Ok(())
}

/// Load bytes from a local heap reference.
#[inline(always)]
pub(crate) fn load_heap_bytes(
    activation: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let reference = pointer.as_heap_reference();
    let address = local_heap_address(activation, reference, access.byte_offset);
    load_native_bytes(address, destination, destination_len);

    Ok(())
}

/// Load bytes from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_bytes(
    activation: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let reference = pointer.as_shared_heap_reference();
    let address = shared_heap_address(activation, reference, access.byte_offset);
    load_native_bytes(address, destination, destination_len);

    Ok(())
}

/// Load bytes from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_bytes(
    _machine: &mut Activation<'_>,
    pointer: StackPointer,
    access: Projection,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);

    load_native_bytes(pointer.address(), destination, destination_len);
    Ok(())
}

/// Load bytes from a frame pointer.
#[inline(always)]
pub(crate) fn load_frame_bytes(
    _machine: &mut Activation<'_>,
    pointer: FramePointer,
    access: Projection,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);

    load_native_bytes(pointer.address(), destination, destination_len);
    Ok(())
}

/// Load bytes from a static address.
#[inline(always)]
pub(crate) fn load_static_bytes(
    activation: &mut Activation<'_>,
    address: StaticAddress,
    access: Projection,
    destination: *mut u8,
    destination_len: usize,
) -> Result<(), Error> {
    let address = address
        .add_bytes(access.byte_offset)
        .ok_or(Error::invalid_instruction())?;
    let address = activation.static_native_address(address, destination_len)?;

    load_native_bytes(address, destination, destination_len);
    Ok(())
}

/// Load one scalar from a local heap reference.
#[inline(always)]
pub(crate) fn load_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    pointer: Cell,
    byte_offset: usize,
) -> Cell {
    let reference = pointer.as_heap_reference();
    let address = local_heap_address(activation, reference, byte_offset);

    load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(address)
}

/// Load one scalar from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    pointer: Cell,
    byte_offset: usize,
) -> Cell {
    let reference = pointer.as_shared_heap_reference();
    let address = shared_heap_address(activation, reference, byte_offset);

    load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(address)
}

/// Load one scalar from a raw pointer.
#[inline(always)]
pub(crate) fn load_raw_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    _machine: &mut Activation<'_>,
    pointer: Cell,
    byte_offset: usize,
) -> Cell {
    let address = raw_address(pointer.as_address(), byte_offset);

    load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(address)
}

/// Load one scalar from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    _machine: &mut Activation<'_>,
    pointer: StackPointer,
    byte_offset: usize,
) -> Cell {
    let pointer = pointer.add_bytes(byte_offset);

    load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(pointer.address())
}

/// Load one scalar from a static address.
#[inline(always)]
pub(crate) fn load_static_scalar<const BYTE_LEN: usize, const IS_SIGNED: bool>(
    activation: &mut Activation<'_>,
    address: StaticAddress,
    byte_offset: usize,
) -> Result<Cell, Error> {
    let address = address
        .add_bytes(byte_offset)
        .ok_or(Error::invalid_instruction())?;
    let address = activation.static_native_address(address, BYTE_LEN)?;

    Ok(load_scalar_at_address::<BYTE_LEN, IS_SIGNED>(address))
}

/// Store one scalar through a local heap reference.
#[inline(always)]
pub(crate) fn store_heap_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    pointer: Cell,
    byte_offset: usize,
    value: Cell,
) {
    let reference = pointer.as_heap_reference();
    let address = local_heap_address(activation, reference, byte_offset);

    store_scalar_at_address::<BYTE_LEN>(address, value);
}

/// Store one scalar through a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    pointer: Cell,
    byte_offset: usize,
    value: Cell,
) {
    let reference = pointer.as_shared_heap_reference();
    let address = shared_heap_address(activation, reference, byte_offset);

    store_scalar_at_address::<BYTE_LEN>(address, value);
}

/// Store one scalar through a raw pointer.
#[inline(always)]
pub(crate) fn store_raw_scalar<const BYTE_LEN: usize>(
    _machine: &mut Activation<'_>,
    pointer: Cell,
    byte_offset: usize,
    value: Cell,
) {
    let address = raw_address(pointer.as_address(), byte_offset);

    store_scalar_at_address::<BYTE_LEN>(address, value);
}

/// Store one scalar through a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_scalar<const BYTE_LEN: usize>(
    _machine: &mut Activation<'_>,
    pointer: StackPointer,
    byte_offset: usize,
    value: Cell,
) {
    let pointer = pointer.add_bytes(byte_offset);

    store_scalar_at_address::<BYTE_LEN>(pointer.address(), value);
}

/// Store one scalar through a static address.
#[inline(always)]
pub(crate) fn store_static_scalar<const BYTE_LEN: usize>(
    activation: &mut Activation<'_>,
    address: StaticAddress,
    byte_offset: usize,
    value: Cell,
) -> Result<(), Error> {
    let address = address
        .add_bytes(byte_offset)
        .ok_or(Error::invalid_instruction())?;
    let address = activation.static_native_address_mut(address, BYTE_LEN)?;

    store_scalar_at_address::<BYTE_LEN>(address, value);
    Ok(())
}

/// Load one scalar from a heap reference.
#[inline(always)]
pub(crate) fn load_heap_scalar_by_layout(
    activation: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
) -> Cell {
    let reference = pointer.as_heap_reference();

    debug_assert_cell_access(access);
    let address = local_heap_address(activation, reference, access.byte_offset);

    load_scalar_by_layout_at_address(address, scalar_layout(access))
}

/// Load one scalar from a shared heap reference.
#[inline(always)]
pub(crate) fn load_shared_heap_scalar_by_layout(
    activation: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
) -> Cell {
    let reference = pointer.as_shared_heap_reference();

    debug_assert_cell_access(access);
    let address = shared_heap_address(activation, reference, access.byte_offset);

    load_scalar_by_layout_at_address(address, scalar_layout(access))
}

/// Load one scalar from a stack pointer.
#[inline(always)]
pub(crate) fn load_stack_scalar_by_layout(
    _machine: &mut Activation<'_>,
    pointer: StackPointer,
    access: Projection,
) -> Cell {
    let pointer = pointer.add_bytes(access.byte_offset);

    debug_assert_cell_access(access);

    load_scalar_by_layout_at_address(pointer.address(), scalar_layout(access))
}

/// Load one scalar from a frame pointer.
#[inline(always)]
pub(crate) fn load_frame_scalar_by_layout(
    _machine: &mut Activation<'_>,
    pointer: FramePointer,
    access: Projection,
) -> Cell {
    let pointer = pointer.add_bytes(access.byte_offset);

    debug_assert_cell_access(access);

    load_scalar_by_layout_at_address(pointer.address(), scalar_layout(access))
}

/// Load one physical slot from a frame pointer.
#[inline(always)]
pub(crate) fn load_frame_slot_by_layout(
    _machine: &mut Activation<'_>,
    pointer: FramePointer,
    access: SlotProjection,
) -> Cell {
    let pointer = pointer.add_bytes(access.byte_offset);

    debug_assert!(access.byte_len <= Cell::BYTE_LEN);

    load_scalar_by_layout_at_address(pointer.address(), slot_layout(access))
}

/// Load one scalar from a static address.
#[inline(always)]
pub(crate) fn load_static_scalar_by_layout(
    activation: &mut Activation<'_>,
    address: StaticAddress,
    access: Projection,
) -> Result<Cell, Error> {
    let address = address
        .add_bytes(access.byte_offset)
        .ok_or(Error::invalid_instruction())?;
    let address = activation.static_native_address(address, access.byte_len)?;

    debug_assert_cell_access(access);

    Ok(load_scalar_by_layout_at_address(
        address,
        scalar_layout(access),
    ))
}

/// Store one scalar through a heap reference.
#[inline(always)]
pub(crate) fn store_heap_scalar_by_layout(
    activation: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    value: Cell,
) {
    debug_assert_cell_access(access);
    let reference = pointer.as_heap_reference();

    let start = access.byte_offset;
    let address = local_heap_address(activation, reference, start);

    store_scalar_by_layout_at_address(address, scalar_layout(access), value);
}

/// Store bytes into a local heap reference.
#[inline(always)]
pub(crate) fn store_heap_bytes(
    activation: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    bytes: &[u8],
) -> Result<(), Error> {
    let reference = pointer.as_heap_reference();

    let start = access.byte_offset;
    let address = local_heap_address(activation, reference, start);
    store_native_bytes(address, bytes);

    Ok(())
}

/// Store one scalar through a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_scalar_by_layout(
    activation: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    value: Cell,
) {
    debug_assert_cell_access(access);
    let reference = pointer.as_shared_heap_reference();

    let start = access.byte_offset;
    let address = shared_heap_address(activation, reference, start);

    store_scalar_by_layout_at_address(address, scalar_layout(access), value);
}

/// Store bytes into a shared heap reference.
#[inline(always)]
pub(crate) fn store_shared_heap_bytes(
    activation: &mut Activation<'_>,
    pointer: Cell,
    access: Projection,
    bytes: &[u8],
) -> Result<(), Error> {
    let reference = pointer.as_shared_heap_reference();

    let start = access.byte_offset;
    let address = shared_heap_address(activation, reference, start);
    store_native_bytes(address, bytes);

    Ok(())
}

/// Store one scalar through a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_scalar_by_layout(
    _machine: &mut Activation<'_>,
    pointer: StackPointer,
    access: Projection,
    value: Cell,
) {
    debug_assert_cell_access(access);
    let pointer = pointer.add_bytes(access.byte_offset);

    store_scalar_by_layout_at_address(pointer.address(), scalar_layout(access), value);
}

/// Store bytes into a stack pointer.
#[inline(always)]
pub(crate) fn store_stack_bytes(
    _machine: &mut Activation<'_>,
    pointer: StackPointer,
    access: Projection,
    bytes: &[u8],
) -> Result<(), Error> {
    let pointer = pointer.add_bytes(access.byte_offset);

    store_native_bytes(pointer.address(), bytes);

    Ok(())
}

/// Store one scalar through a frame pointer.
#[inline(always)]
pub(crate) fn store_frame_scalar_by_layout(
    _machine: &mut Activation<'_>,
    pointer: FramePointer,
    access: Projection,
    value: Cell,
) {
    debug_assert_cell_access(access);
    let pointer = pointer.add_bytes(access.byte_offset);

    store_scalar_by_layout_at_address(pointer.address(), scalar_layout(access), value);
}

/// Store one physical slot through a frame pointer.
#[inline(always)]
pub(crate) fn store_frame_slot_by_layout(
    _machine: &mut Activation<'_>,
    pointer: FramePointer,
    access: SlotProjection,
    value: Cell,
) {
    let pointer = pointer.add_bytes(access.byte_offset);

    debug_assert!(access.byte_len <= Cell::BYTE_LEN);

    store_scalar_by_layout_at_address(pointer.address(), slot_layout(access), value);
}

/// Store one scalar through a static address.
#[inline(always)]
pub(crate) fn store_static_scalar_by_layout(
    activation: &mut Activation<'_>,
    address: StaticAddress,
    access: Projection,
    value: Cell,
) -> Result<(), Error> {
    debug_assert_cell_access(access);
    let address = address
        .add_bytes(access.byte_offset)
        .ok_or(Error::invalid_instruction())?;
    let address = activation.static_native_address_mut(address, access.byte_len)?;

    store_scalar_by_layout_at_address(address, scalar_layout(access), value);
    Ok(())
}

/// Store bytes into a static address.
#[inline(always)]
pub(crate) fn store_static_bytes(
    activation: &mut Activation<'_>,
    address: StaticAddress,
    access: Projection,
    bytes: &[u8],
) -> Result<(), Error> {
    let address = address
        .add_bytes(access.byte_offset)
        .ok_or(Error::invalid_instruction())?;
    let address = activation.static_native_address_mut(address, bytes.len())?;

    store_native_bytes(address, bytes);

    Ok(())
}
