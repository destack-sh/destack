use crate::core::{visit_edge_map_in_reader, visit_edge_map_in_reader_range};
use crate::local::managed::{EdgeMap, managed_reference_width};
use crate::value::{ManagedReference, Value};
use crate::{HeapError, HeapResult};

/// Visit each managed reference stored in the given payload bytes.
pub fn trace_managed_references(
    map: &EdgeMap,
    bytes: &[u8],
    managed_reference_bytes: u8,
    visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    // validate the traced reference width once
    let managed_reference_bytes = managed_reference_width(managed_reference_bytes)?;

    // walk the encoded shape directly
    match map {
        EdgeMap::None => {}
        EdgeMap::ReferenceOffsets { offsets } => {
            trace_reference_offsets(offsets, bytes, managed_reference_bytes, visit)?
        }
        EdgeMap::ValueOffsets { offsets } => trace_value_offsets(offsets, bytes, visit)?,
        EdgeMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets,
        } => trace_repeated_reference_offsets(
            *count,
            *element_size,
            offsets,
            bytes,
            managed_reference_bytes,
            visit,
        )?,
    }

    Ok(())
}

/// Visit each managed reference using one random-access byte reader.
pub(crate) fn trace_managed_references_in_reader(
    map: &EdgeMap,
    managed_reference_bytes: u8,
    fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    // validate the traced reference width once
    let managed_reference_bytes = managed_reference_width(managed_reference_bytes)?;

    visit_edge_map_in_reader(
        map,
        managed_reference_bytes,
        fill_bytes,
        decode_managed_reference_window,
        decode_value_reference_window,
        visit,
    )
}

/// Visit each managed reference whose encoded bytes overlap the given byte range.
pub(crate) fn trace_managed_references_in_reader_range(
    map: &EdgeMap,
    start: usize,
    len: usize,
    managed_reference_bytes: u8,
    fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    // validate the traced reference width once
    let managed_reference_bytes = managed_reference_width(managed_reference_bytes)?;

    visit_edge_map_in_reader_range(
        map,
        start,
        len,
        managed_reference_bytes,
        fill_bytes,
        decode_managed_reference_window,
        decode_value_reference_window,
        visit,
    )
}

/// Visit each direct managed reference stored in one payload.
fn trace_reference_offsets(
    offsets: &[u32],
    bytes: &[u8],
    managed_reference_bytes: usize,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    // decode each direct managed-reference field
    for &offset in offsets {
        let start = offset as usize;
        let reference = decode_managed_reference(bytes, start, managed_reference_bytes)?;

        visit(reference);
    }

    Ok(())
}

/// Visit each managed reference stored inside one full value payload.
fn trace_value_offsets(
    offsets: &[u32],
    bytes: &[u8],
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    // decode each full value slot
    for &offset in offsets {
        let start = offset as usize;
        let reference = decode_value_reference(bytes, start)?;

        // visit only managed-reference values
        if let Some(reference) = reference {
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each repeated managed reference stored in one payload.
fn trace_repeated_reference_offsets(
    count: u32,
    element_size: u32,
    offsets: &[u32],
    bytes: &[u8],
    managed_reference_bytes: usize,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    let element_size = element_size as usize;

    // walk each repeated element
    for index in 0..(count as usize) {
        let base = index
            .checked_mul(element_size)
            .ok_or(HeapError::InvariantOverflow {
                context: "repeated managed trace base offset",
            })?;

        // decode each managed-reference field within that element
        for &offset in offsets {
            let start = base
                .checked_add(offset as usize)
                .ok_or(HeapError::InvariantOverflow {
                    context: "repeated managed trace field offset",
                })?;
            let reference = decode_managed_reference(bytes, start, managed_reference_bytes)?;

            visit(reference);
        }
    }

    Ok(())
}

/// Decode one managed reference from one traced byte window.
fn decode_managed_reference(
    bytes: &[u8],
    start: usize,
    managed_reference_bytes: usize,
) -> HeapResult<ManagedReference> {
    // resolve the traced byte window
    let end =
        start
            .checked_add(managed_reference_bytes)
            .ok_or(HeapError::EdgeMapOffsetOverflow {
                start,
                width: managed_reference_bytes,
            })?;
    let window = bytes
        .get(start..end)
        .ok_or(HeapError::TruncatedEdgeMapPayload {
            start,
            width: managed_reference_bytes,
            len: bytes.len(),
        })?;

    // decode the resolved window
    decode_managed_reference_window(window)
}

/// Decode one managed reference from one traced byte window.
fn decode_managed_reference_window(window: &[u8]) -> HeapResult<ManagedReference> {
    // decode according to the encoded width
    match window.len() {
        4 => {
            let mut raw = [0u8; 4];
            raw.copy_from_slice(window);

            Ok(ManagedReference::from_bits(u64::from(u32::from_le_bytes(
                raw,
            ))))
        }
        8 => {
            let mut raw = [0u8; 8];
            raw.copy_from_slice(window);

            Ok(ManagedReference::from_bits(u64::from_le_bytes(raw)))
        }
        bytes => Err(HeapError::InvalidReferenceWindowWidth { bytes }),
    }
}

/// Decode one managed reference stored inside one full value payload.
fn decode_value_reference(bytes: &[u8], start: usize) -> HeapResult<Option<ManagedReference>> {
    // resolve the traced byte window
    let end = start
        .checked_add(Value::BYTE_LEN)
        .ok_or(HeapError::EdgeMapOffsetOverflow {
            start,
            width: Value::BYTE_LEN,
        })?;
    let window = bytes
        .get(start..end)
        .ok_or(HeapError::TruncatedEdgeMapPayload {
            start,
            width: Value::BYTE_LEN,
            len: bytes.len(),
        })?;

    // decode the resolved value window
    decode_value_reference_window(window, start)
}

/// Decode one managed reference stored inside one full value window.
fn decode_value_reference_window(
    window: &[u8],
    start: usize,
) -> HeapResult<Option<ManagedReference>> {
    // decode the value payload first
    let value =
        Value::from_byte_slice(window).ok_or(HeapError::InvalidReferenceValuePayload { start })?;

    // then select only managed references
    Ok(value.as_managed_reference())
}
