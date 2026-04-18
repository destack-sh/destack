use crate::core::{overlapping_repeated_index_range, ranges_overlap};
use crate::{EdgeMap, HeapError, HeapResult, Value};

/// Visit each reference encoded by one edge map through one random-access reader.
pub(crate) fn visit_edge_map_in_reader<R>(
    map: &EdgeMap,
    reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut decode_reference: impl FnMut(&[u8]) -> HeapResult<R>,
    mut decode_value_reference: impl FnMut(&[u8], usize) -> HeapResult<Option<R>>,
    mut visit: impl FnMut(R),
) -> HeapResult<()> {
    match map {
        EdgeMap::None => {}
        EdgeMap::ReferenceOffsets { offsets } => visit_reference_offsets_in_reader(
            offsets,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_reference,
            &mut visit,
        )?,
        EdgeMap::ValueOffsets { offsets } => visit_value_offsets_in_reader(
            offsets,
            &mut fill_bytes,
            &mut decode_value_reference,
            &mut visit,
        )?,
        EdgeMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets,
        } => visit_repeated_reference_offsets_in_reader(
            *count,
            *element_size,
            offsets,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_reference,
            &mut visit,
        )?,
    }

    Ok(())
}

/// Visit each overlapping reference encoded by one edge map through one reader.
pub(crate) fn visit_edge_map_in_reader_range<R>(
    map: &EdgeMap,
    start: usize,
    len: usize,
    reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut decode_reference: impl FnMut(&[u8]) -> HeapResult<R>,
    mut decode_value_reference: impl FnMut(&[u8], usize) -> HeapResult<Option<R>>,
    mut visit: impl FnMut(R),
) -> HeapResult<()> {
    // empty writes cannot overlap anything
    if len == 0 {
        return Ok(());
    }

    // reject invalid dirty windows
    let Some(end) = start.checked_add(len) else {
        return Err(HeapError::EdgeMapOffsetOverflow { start, width: len });
    };

    match map {
        EdgeMap::None => {}
        EdgeMap::ReferenceOffsets { offsets } => visit_reference_offsets_in_reader_range(
            offsets,
            start,
            end,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_reference,
            &mut visit,
        )?,
        EdgeMap::ValueOffsets { offsets } => visit_value_offsets_in_reader_range(
            offsets,
            start,
            end,
            &mut fill_bytes,
            &mut decode_value_reference,
            &mut visit,
        )?,
        EdgeMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets,
        } => visit_repeated_reference_offsets_in_reader_range(
            *count,
            *element_size,
            offsets,
            start,
            end,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_reference,
            &mut visit,
        )?,
    }

    Ok(())
}

/// Visit each direct reference through one random-access reader.
fn visit_reference_offsets_in_reader<R>(
    offsets: &[u32],
    reference_bytes: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode_reference: &mut impl FnMut(&[u8]) -> HeapResult<R>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut raw = [0u8; 8];

    // read and decode each direct reference field
    for &offset in offsets {
        let start = offset as usize;
        let buffer = &mut raw[..reference_bytes];

        // fail loudly on truncated reader windows
        if !fill_bytes(start, buffer) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start,
                width: reference_bytes,
            });
        }

        let reference = decode_reference(buffer)?;
        visit(reference);
    }

    Ok(())
}

/// Visit each value reference through one random-access reader.
fn visit_value_offsets_in_reader<R>(
    offsets: &[u32],
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode_value_reference: &mut impl FnMut(&[u8], usize) -> HeapResult<Option<R>>,
    visit: &mut impl FnMut(R),
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

        let reference = decode_value_reference(buffer, start)?;

        // visit only matching reference values
        if let Some(reference) = reference {
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each repeated direct reference through one random-access reader.
fn visit_repeated_reference_offsets_in_reader<R>(
    count: u32,
    element_size: u32,
    offsets: &[u32],
    reference_bytes: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode_reference: &mut impl FnMut(&[u8]) -> HeapResult<R>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let element_size = element_size as usize;
    let mut raw = [0u8; 8];

    // walk each repeated element
    for index in 0..(count as usize) {
        let base = index
            .checked_mul(element_size)
            .ok_or(HeapError::InvariantOverflow {
                context: "repeated reference trace base offset",
            })?;

        // read and decode each reference field within that element
        for &offset in offsets {
            let start = base
                .checked_add(offset as usize)
                .ok_or(HeapError::InvariantOverflow {
                    context: "repeated reference trace field offset",
                })?;
            let buffer = &mut raw[..reference_bytes];

            // fail loudly on truncated reader windows
            if !fill_bytes(start, buffer) {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start,
                    width: reference_bytes,
                });
            }

            let reference = decode_reference(buffer)?;
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each overlapping direct reference through one reader.
fn visit_reference_offsets_in_reader_range<R>(
    offsets: &[u32],
    start: usize,
    end: usize,
    reference_bytes: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode_reference: &mut impl FnMut(&[u8]) -> HeapResult<R>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut raw = [0u8; 8];

    // walk each direct reference field
    for &offset in offsets {
        let field_start = offset as usize;

        // skip non-overlapping fields
        if !ranges_overlap(start, end, field_start, reference_bytes) {
            continue;
        }

        let buffer = &mut raw[..reference_bytes];

        // fail loudly on truncated reader windows
        if !fill_bytes(field_start, buffer) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start: field_start,
                width: reference_bytes,
            });
        }

        let reference = decode_reference(buffer)?;
        visit(reference);
    }

    Ok(())
}

/// Visit each overlapping value reference through one reader.
fn visit_value_offsets_in_reader_range<R>(
    offsets: &[u32],
    start: usize,
    end: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode_value_reference: &mut impl FnMut(&[u8], usize) -> HeapResult<Option<R>>,
    visit: &mut impl FnMut(R),
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

        let reference = decode_value_reference(buffer, field_start)?;

        // visit only matching reference values
        if let Some(reference) = reference {
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each overlapping repeated direct reference through one reader.
fn visit_repeated_reference_offsets_in_reader_range<R>(
    count: u32,
    element_size: u32,
    offsets: &[u32],
    start: usize,
    end: usize,
    reference_bytes: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode_reference: &mut impl FnMut(&[u8]) -> HeapResult<R>,
    visit: &mut impl FnMut(R),
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
            reference_bytes,
        ) else {
            continue;
        };

        // walk the overlapping repeated elements for that field
        for index in first_index..=last_index {
            let base = index
                .checked_mul(element_size)
                .ok_or(HeapError::InvariantOverflow {
                    context: "repeated reference dirty trace base offset",
                })?;
            let field_start =
                base.checked_add(offset as usize)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "repeated reference dirty trace field offset",
                    })?;
            let buffer = &mut raw[..reference_bytes];

            // fail loudly on truncated reader windows
            if !fill_bytes(field_start, buffer) {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start: field_start,
                    width: reference_bytes,
                });
            }

            let reference = decode_reference(buffer)?;
            visit(reference);
        }
    }

    Ok(())
}
