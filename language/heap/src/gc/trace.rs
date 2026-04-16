use crate::managed::{
    ReferenceMap, managed_reference_width, overlapping_repeated_index_range, ranges_overlap,
};
use crate::value::{ManagedReference, Value};
use crate::{HeapError, HeapResult};

/// Visit each managed reference stored in the given payload bytes.
pub fn trace_managed_references(
    map: &ReferenceMap,
    bytes: &[u8],
    managed_reference_bytes: u8,
    visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    // validate the traced reference width once
    let managed_reference_bytes = managed_reference_width(managed_reference_bytes)?;

    // walk the encoded shape directly
    match map {
        ReferenceMap::None => {}
        ReferenceMap::ReferenceOffsets { offsets } => {
            trace_reference_offsets(offsets, bytes, managed_reference_bytes, visit)?
        }
        ReferenceMap::ValueOffsets { offsets } => trace_value_offsets(offsets, bytes, visit)?,
        ReferenceMap::RepeatedReferenceOffsets {
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
    map: &ReferenceMap,
    managed_reference_bytes: u8,
    fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    // validate the traced reference width once
    let managed_reference_bytes = managed_reference_width(managed_reference_bytes)?;

    // walk the encoded shape through the reader
    match map {
        ReferenceMap::None => {}
        ReferenceMap::ReferenceOffsets { offsets } => {
            trace_reference_offsets_in_reader(offsets, managed_reference_bytes, fill_bytes, visit)?
        }
        ReferenceMap::ValueOffsets { offsets } => {
            trace_value_offsets_in_reader(offsets, fill_bytes, visit)?
        }
        ReferenceMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets,
        } => trace_repeated_reference_offsets_in_reader(
            *count,
            *element_size,
            offsets,
            managed_reference_bytes,
            fill_bytes,
            visit,
        )?,
    }

    Ok(())
}

/// Visit each managed reference whose encoded bytes overlap the given byte range.
pub(crate) fn trace_managed_references_in_reader_range(
    map: &ReferenceMap,
    start: usize,
    len: usize,
    managed_reference_bytes: u8,
    fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    // empty writes cannot overlap anything
    if len == 0 {
        return Ok(());
    }

    // reject invalid dirty windows
    let Some(end) = start.checked_add(len) else {
        return Err(HeapError::ReferenceMapOffsetOverflow { start, width: len });
    };

    // validate the traced reference width once
    let managed_reference_bytes = managed_reference_width(managed_reference_bytes)?;

    // walk only overlapping fields in the encoded shape
    match map {
        ReferenceMap::None => {}
        ReferenceMap::ReferenceOffsets { offsets } => trace_reference_offsets_in_reader_range(
            offsets,
            start,
            end,
            managed_reference_bytes,
            fill_bytes,
            visit,
        )?,
        ReferenceMap::ValueOffsets { offsets } => {
            trace_value_offsets_in_reader_range(offsets, start, end, fill_bytes, visit)?
        }
        ReferenceMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets,
        } => trace_repeated_reference_offsets_in_reader_range(
            *count,
            *element_size,
            offsets,
            start,
            end,
            managed_reference_bytes,
            fill_bytes,
            visit,
        )?,
    }

    Ok(())
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
        let base = index.saturating_mul(element_size);

        // decode each managed-reference field within that element
        for &offset in offsets {
            let start = base.saturating_add(offset as usize);
            let reference = decode_managed_reference(bytes, start, managed_reference_bytes)?;

            visit(reference);
        }
    }

    Ok(())
}

/// Visit each direct managed reference through one random-access reader.
fn trace_reference_offsets_in_reader(
    offsets: &[u32],
    managed_reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    let mut raw = [0u8; 8];

    // read and decode each direct managed-reference field
    for &offset in offsets {
        let start = offset as usize;
        let buffer = &mut raw[..managed_reference_bytes];

        // fail loudly on truncated reader windows
        if !fill_bytes(start, buffer) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start,
                width: managed_reference_bytes,
            });
        }

        let reference = decode_managed_reference_window(buffer)?;
        visit(reference);
    }

    Ok(())
}

/// Visit each managed reference stored inside one value slot through one reader.
fn trace_value_offsets_in_reader(
    offsets: &[u32],
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    let mut raw = [0u8; Value::BYTE_LEN];

    // read and decode each full value slot
    for &offset in offsets {
        let start = offset as usize;
        let buffer = &mut raw[..Value::BYTE_LEN];

        // fail loudly on truncated reader windows
        if !fill_bytes(start, buffer) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start,
                width: Value::BYTE_LEN,
            });
        }

        let reference = decode_value_reference_window(buffer, start)?;

        // visit only managed-reference values
        if let Some(reference) = reference {
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each repeated managed reference through one random-access reader.
fn trace_repeated_reference_offsets_in_reader(
    count: u32,
    element_size: u32,
    offsets: &[u32],
    managed_reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    let element_size = element_size as usize;
    let mut raw = [0u8; 8];

    // walk each repeated element
    for index in 0..(count as usize) {
        let base = index.saturating_mul(element_size);

        // read and decode each managed-reference field within that element
        for &offset in offsets {
            let start = base.saturating_add(offset as usize);
            let buffer = &mut raw[..managed_reference_bytes];

            // fail loudly on truncated reader windows
            if !fill_bytes(start, buffer) {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start,
                    width: managed_reference_bytes,
                });
            }

            let reference = decode_managed_reference_window(buffer)?;
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each overlapping direct managed reference through one reader.
fn trace_reference_offsets_in_reader_range(
    offsets: &[u32],
    start: usize,
    end: usize,
    managed_reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    let mut raw = [0u8; 8];

    // walk each direct managed-reference field
    for &offset in offsets {
        let field_start = offset as usize;

        // skip non-overlapping fields
        if !ranges_overlap(start, end, field_start, managed_reference_bytes) {
            continue;
        }

        let buffer = &mut raw[..managed_reference_bytes];

        // fail loudly on truncated reader windows
        if !fill_bytes(field_start, buffer) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start: field_start,
                width: managed_reference_bytes,
            });
        }

        let reference = decode_managed_reference_window(buffer)?;
        visit(reference);
    }

    Ok(())
}

/// Visit each overlapping managed reference stored inside one value slot through one reader.
fn trace_value_offsets_in_reader_range(
    offsets: &[u32],
    start: usize,
    end: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    let mut raw = [0u8; Value::BYTE_LEN];

    // walk each full value slot
    for &offset in offsets {
        let field_start = offset as usize;

        // skip non-overlapping fields
        if !ranges_overlap(start, end, field_start, Value::BYTE_LEN) {
            continue;
        }

        let buffer = &mut raw[..Value::BYTE_LEN];

        // fail loudly on truncated reader windows
        if !fill_bytes(field_start, buffer) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start: field_start,
                width: Value::BYTE_LEN,
            });
        }

        let reference = decode_value_reference_window(buffer, field_start)?;

        // visit only managed-reference values
        if let Some(reference) = reference {
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each overlapping repeated managed reference through one reader.
fn trace_repeated_reference_offsets_in_reader_range(
    count: u32,
    element_size: u32,
    offsets: &[u32],
    start: usize,
    end: usize,
    managed_reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    let element_size = element_size as usize;
    let mut raw = [0u8; 8];

    // walk each repeated field definition
    for &offset in offsets {
        let Some((first_index, last_index)) = overlapping_repeated_index_range(
            start,
            end,
            count as usize,
            element_size,
            offset as usize,
            managed_reference_bytes,
        ) else {
            continue;
        };

        // walk the overlapping repeated elements for that field
        for index in first_index..=last_index {
            let field_start = index
                .saturating_mul(element_size)
                .saturating_add(offset as usize);
            let buffer = &mut raw[..managed_reference_bytes];

            // fail loudly on truncated reader windows
            if !fill_bytes(field_start, buffer) {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start: field_start,
                    width: managed_reference_bytes,
                });
            }

            let reference = decode_managed_reference_window(buffer)?;
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
    let end = start.checked_add(managed_reference_bytes).ok_or(
        HeapError::ReferenceMapOffsetOverflow {
            start,
            width: managed_reference_bytes,
        },
    )?;
    let window = bytes
        .get(start..end)
        .ok_or(HeapError::TruncatedReferenceMapPayload {
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
        .ok_or(HeapError::ReferenceMapOffsetOverflow {
            start,
            width: Value::BYTE_LEN,
        })?;
    let window = bytes
        .get(start..end)
        .ok_or(HeapError::TruncatedReferenceMapPayload {
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
