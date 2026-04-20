use std::mem::size_of;

use super::{overlapping_repeated_index_range, ranges_overlap, HeapScan};
use crate::{HeapError, HeapResult, ManagedReference, SharedManagedReference};

/// The packed byte width for one runtime value lane in heap storage.
pub(crate) const PACKED_VALUE_BYTES: usize = size_of::<u64>() * 2;

/// The byte offset of the packed tag byte within one value lane.
const VALUE_TAG_OFFSET: usize = size_of::<u64>();

/// The highest currently valid packed value tag.
const MAX_VALUE_TAG: u8 = 14;

/// The packed tag for one local managed reference value.
const MANAGED_REFERENCE_TAG: u8 = 7;

/// The packed tag for one shared managed reference value.
const SHARED_MANAGED_REFERENCE_TAG: u8 = 8;

/// Visit each local managed reference encoded in the given payload bytes.
pub fn trace_managed_references(
    trace: &HeapScan,
    bytes: &[u8],
    reference_bytes: usize,
    visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    validate_reference_bytes(reference_bytes)?;

    visit_managed_references_in_reader(
        trace,
        reference_bytes,
        |start, buffer| {
            let Some(end) = start.checked_add(buffer.len()) else {
                return false;
            };
            let Some(window) = bytes.get(start..end) else {
                return false;
            };

            buffer.copy_from_slice(window);
            true
        },
        visit,
    )
}

/// Visit each local managed reference encoded by one scan through one reader.
pub(crate) fn visit_managed_references_in_reader(
    trace: &HeapScan,
    reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    validate_reference_bytes(reference_bytes)?;

    match trace {
        HeapScan::None => {}
        HeapScan::Reference { local_offsets, .. } => visit_reference_offsets_in_reader(
            local_offsets,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_managed_reference_window,
            &mut visit,
        )?,
        HeapScan::PackedValue { offsets } => visit_value_offsets_in_reader(
            offsets,
            &mut fill_bytes,
            &mut decode_managed_reference_value_slot,
            &mut visit,
        )?,
        HeapScan::RepeatedReference {
            count,
            stride,
            local_offsets,
            ..
        } => visit_repeated_reference_offsets_in_reader(
            *count,
            *stride,
            local_offsets,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_managed_reference_window,
            &mut visit,
        )?,
    }

    Ok(())
}

/// Visit each shared managed reference encoded by one scan through one reader.
pub(crate) fn visit_shared_references_in_reader(
    trace: &HeapScan,
    reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(SharedManagedReference),
) -> HeapResult<()> {
    validate_reference_bytes(reference_bytes)?;

    match trace {
        HeapScan::None => {}
        HeapScan::Reference { shared_offsets, .. } => visit_reference_offsets_in_reader(
            shared_offsets,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_shared_reference_window,
            &mut visit,
        )?,
        HeapScan::PackedValue { offsets } => visit_value_offsets_in_reader(
            offsets,
            &mut fill_bytes,
            &mut decode_shared_managed_reference_value_slot,
            &mut visit,
        )?,
        HeapScan::RepeatedReference {
            count,
            stride,
            shared_offsets,
            ..
        } => visit_repeated_reference_offsets_in_reader(
            *count,
            *stride,
            shared_offsets,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_shared_reference_window,
            &mut visit,
        )?,
    }

    Ok(())
}

/// Visit each overlapping local managed reference encoded by one scan through one reader.
pub(crate) fn visit_managed_references_in_reader_range(
    trace: &HeapScan,
    start: usize,
    len: usize,
    reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(ManagedReference),
) -> HeapResult<()> {
    validate_reference_bytes(reference_bytes)?;

    // empty writes cannot overlap anything
    if len == 0 {
        return Ok(());
    }

    // reject invalid dirty windows
    let Some(end) = start.checked_add(len) else {
        return Err(HeapError::TraceOffsetOverflow { start, width: len });
    };

    match trace {
        HeapScan::None => {}
        HeapScan::Reference { local_offsets, .. } => visit_reference_offsets_in_reader_range(
            local_offsets,
            start,
            end,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_managed_reference_window,
            &mut visit,
        )?,
        HeapScan::PackedValue { offsets } => visit_value_offsets_in_reader_range(
            offsets,
            start,
            end,
            &mut fill_bytes,
            &mut decode_managed_reference_value_slot,
            &mut visit,
        )?,
        HeapScan::RepeatedReference {
            count,
            stride,
            local_offsets,
            ..
        } => visit_repeated_reference_offsets_in_reader_range(
            *count,
            *stride,
            local_offsets,
            start,
            end,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_managed_reference_window,
            &mut visit,
        )?,
    }

    Ok(())
}

/// Visit each overlapping shared managed reference encoded by one scan through one reader.
pub(crate) fn visit_shared_references_in_reader_range(
    trace: &HeapScan,
    start: usize,
    len: usize,
    reference_bytes: usize,
    mut fill_bytes: impl FnMut(usize, &mut [u8]) -> bool,
    mut visit: impl FnMut(SharedManagedReference),
) -> HeapResult<()> {
    validate_reference_bytes(reference_bytes)?;

    // empty writes cannot overlap anything
    if len == 0 {
        return Ok(());
    }

    // reject invalid dirty windows
    let Some(end) = start.checked_add(len) else {
        return Err(HeapError::TraceOffsetOverflow { start, width: len });
    };

    match trace {
        HeapScan::None => {}
        HeapScan::Reference { shared_offsets, .. } => visit_reference_offsets_in_reader_range(
            shared_offsets,
            start,
            end,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_shared_reference_window,
            &mut visit,
        )?,
        HeapScan::PackedValue { offsets } => visit_value_offsets_in_reader_range(
            offsets,
            start,
            end,
            &mut fill_bytes,
            &mut decode_shared_managed_reference_value_slot,
            &mut visit,
        )?,
        HeapScan::RepeatedReference {
            count,
            stride,
            shared_offsets,
            ..
        } => visit_repeated_reference_offsets_in_reader_range(
            *count,
            *stride,
            shared_offsets,
            start,
            end,
            reference_bytes,
            &mut fill_bytes,
            &mut decode_shared_reference_window,
            &mut visit,
        )?,
    }

    Ok(())
}

/// Validate one configured direct-reference width.
fn validate_reference_bytes(reference_bytes: usize) -> HeapResult<()> {
    match reference_bytes {
        4 | 8 => Ok(()),
        bytes => {
            let bytes = u8::try_from(bytes).map_err(|_| HeapError::InvariantOverflow {
                context: "reference width",
            })?;

            Err(HeapError::UnsupportedManagedReferenceWidth { bytes })
        }
    }
}

/// Decode one local managed reference stored inside one packed value lane.
pub(crate) fn decode_managed_reference_value_slot(
    window: &[u8],
    start: usize,
) -> HeapResult<Option<ManagedReference>> {
    let tag = decode_value_tag(window, start)?;
    if tag != MANAGED_REFERENCE_TAG {
        return Ok(None);
    }

    let bits = decode_value_data(window);

    Ok(Some(ManagedReference::from_bits(bits)))
}

/// Decode one shared managed reference stored inside one packed value lane.
pub(crate) fn decode_shared_managed_reference_value_slot(
    window: &[u8],
    start: usize,
) -> HeapResult<Option<SharedManagedReference>> {
    let tag = decode_value_tag(window, start)?;
    if tag != SHARED_MANAGED_REFERENCE_TAG {
        return Ok(None);
    }

    let bits = decode_value_data(window);

    Ok(Some(SharedManagedReference::from_bits(bits)))
}

/// Decode one direct local managed reference from one traced window.
fn decode_managed_reference_window(window: &[u8]) -> HeapResult<ManagedReference> {
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

/// Decode one direct shared managed reference from one traced window.
fn decode_shared_reference_window(window: &[u8]) -> HeapResult<SharedManagedReference> {
    match window.len() {
        4 => {
            let mut raw = [0u8; 4];
            raw.copy_from_slice(window);

            Ok(SharedManagedReference::from_bits(u64::from(
                u32::from_le_bytes(raw),
            )))
        }
        8 => {
            let mut raw = [0u8; 8];
            raw.copy_from_slice(window);

            Ok(SharedManagedReference::from_bits(u64::from_le_bytes(raw)))
        }
        bytes => Err(HeapError::InvalidReferenceWindowWidth { bytes }),
    }
}

/// Decode one packed value tag from one value lane.
fn decode_value_tag(window: &[u8], start: usize) -> HeapResult<u8> {
    if window.len() != PACKED_VALUE_BYTES {
        return Err(HeapError::InvalidReferenceWindowWidth {
            bytes: window.len(),
        });
    }

    let Some(tag) = window.get(VALUE_TAG_OFFSET).copied() else {
        return Err(HeapError::InvalidValuePayload { start });
    };
    if tag > MAX_VALUE_TAG {
        return Err(HeapError::InvalidValuePayload { start });
    }

    Ok(tag)
}

/// Decode the payload bits from one packed value lane.
fn decode_value_data(window: &[u8]) -> u64 {
    let mut raw = [0u8; 8];
    raw.copy_from_slice(&window[..8]);

    u64::from_le_bytes(raw)
}

/// Visit each direct reference offset through one random-access reader.
fn visit_reference_offsets_in_reader<R>(
    offsets: &[u32],
    reference_bytes: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode: &mut impl FnMut(&[u8]) -> HeapResult<R>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = vec![0u8; reference_bytes];

    for offset in offsets.iter().copied() {
        let start = offset as usize;

        if !fill_bytes(start, &mut window) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start,
                width: reference_bytes,
            });
        }

        visit(decode(&window)?);
    }

    Ok(())
}

/// Visit each packed value offset through one random-access reader.
fn visit_value_offsets_in_reader<R>(
    offsets: &[u32],
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode: &mut impl FnMut(&[u8], usize) -> HeapResult<Option<R>>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = [0u8; PACKED_VALUE_BYTES];

    for offset in offsets.iter().copied() {
        let start = offset as usize;

        if !fill_bytes(start, &mut window) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start,
                width: PACKED_VALUE_BYTES,
            });
        }

        if let Some(reference) = decode(&window, start)? {
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each repeated reference offset through one random-access reader.
fn visit_repeated_reference_offsets_in_reader<R>(
    count: u32,
    stride: u32,
    offsets: &[u32],
    reference_bytes: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode: &mut impl FnMut(&[u8]) -> HeapResult<R>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = vec![0u8; reference_bytes];

    for index in 0..count as usize {
        let base = index
            .checked_mul(stride as usize)
            .ok_or(HeapError::InvariantOverflow {
                context: "repeated scan base",
            })?;

        for offset in offsets.iter().copied() {
            let start =
                base.checked_add(offset as usize)
                    .ok_or(HeapError::TraceOffsetOverflow {
                        start: base,
                        width: offset as usize,
                    })?;

            if !fill_bytes(start, &mut window) {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start,
                    width: reference_bytes,
                });
            }

            visit(decode(&window)?);
        }
    }

    Ok(())
}

/// Visit each overlapping direct reference offset through one random-access reader.
fn visit_reference_offsets_in_reader_range<R>(
    offsets: &[u32],
    start: usize,
    end: usize,
    reference_bytes: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode: &mut impl FnMut(&[u8]) -> HeapResult<R>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = vec![0u8; reference_bytes];

    for offset in offsets.iter().copied() {
        let offset = offset as usize;
        if !ranges_overlap(start, end, offset, reference_bytes) {
            continue;
        }

        if !fill_bytes(offset, &mut window) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start: offset,
                width: reference_bytes,
            });
        }

        visit(decode(&window)?);
    }

    Ok(())
}

/// Visit each overlapping packed value offset through one random-access reader.
fn visit_value_offsets_in_reader_range<R>(
    offsets: &[u32],
    start: usize,
    end: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode: &mut impl FnMut(&[u8], usize) -> HeapResult<Option<R>>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = [0u8; PACKED_VALUE_BYTES];

    for offset in offsets.iter().copied() {
        let offset = offset as usize;
        if !ranges_overlap(start, end, offset, PACKED_VALUE_BYTES) {
            continue;
        }

        if !fill_bytes(offset, &mut window) {
            return Err(HeapError::TruncatedReferenceReaderWindow {
                start: offset,
                width: PACKED_VALUE_BYTES,
            });
        }

        if let Some(reference) = decode(&window, offset)? {
            visit(reference);
        }
    }

    Ok(())
}

/// Visit each overlapping repeated reference offset through one random-access reader.
fn visit_repeated_reference_offsets_in_reader_range<R>(
    count: u32,
    stride: u32,
    offsets: &[u32],
    start: usize,
    end: usize,
    reference_bytes: usize,
    fill_bytes: &mut impl FnMut(usize, &mut [u8]) -> bool,
    decode: &mut impl FnMut(&[u8]) -> HeapResult<R>,
    visit: &mut impl FnMut(R),
) -> HeapResult<()> {
    let mut window = vec![0u8; reference_bytes];

    for offset in offsets.iter().copied() {
        let Some((first, last)) = overlapping_repeated_index_range(
            start,
            end,
            count as usize,
            stride as usize,
            offset as usize,
            reference_bytes,
        ) else {
            continue;
        };

        for index in first..=last {
            let base = index
                .checked_mul(stride as usize)
                .ok_or(HeapError::InvariantOverflow {
                    context: "repeated scan base",
                })?;
            let offset =
                base.checked_add(offset as usize)
                    .ok_or(HeapError::TraceOffsetOverflow {
                        start: base,
                        width: offset as usize,
                    })?;

            if !fill_bytes(offset, &mut window) {
                return Err(HeapError::TruncatedReferenceReaderWindow {
                    start: offset,
                    width: reference_bytes,
                });
            }

            visit(decode(&window)?);
        }
    }

    Ok(())
}
