use destack_mir::EdgeMap;

use crate::{HeapError, HeapResult, Value};

/// Return whether one write range may overlap any managed-edge bytes.
pub(crate) fn touches_managed_range(
    map: &EdgeMap,
    start: usize,
    len: usize,
    managed_reference_bytes: u8,
) -> HeapResult<bool> {
    if len == 0 {
        return Ok(false);
    }

    let Some(end) = start.checked_add(len) else {
        return Ok(true);
    };

    let managed_reference_bytes = managed_reference_width(managed_reference_bytes)?;
    let is_overlapping = match map {
        EdgeMap::None => false,
        EdgeMap::ReferenceOffsets { offsets } => {
            touches_reference_offsets(offsets, start, end, managed_reference_bytes)
        }
        EdgeMap::ValueOffsets { offsets } => {
            touches_value_offsets(offsets, start, end, Value::BYTE_LEN)
        }
        EdgeMap::RepeatedReferenceOffsets {
            count,
            element_size,
            offsets,
        } => touches_repeated_reference_offsets(
            *count,
            *element_size,
            offsets,
            start,
            end,
            managed_reference_bytes,
        ),
    };

    Ok(is_overlapping)
}

/// Return the validated byte width for one traced managed reference.
pub(crate) fn managed_reference_width(managed_reference_bytes: u8) -> HeapResult<usize> {
    match managed_reference_bytes {
        4 | 8 => Ok(managed_reference_bytes as usize),
        _ => Err(HeapError::UnsupportedManagedReferenceWidth {
            bytes: managed_reference_bytes,
        }),
    }
}

/// Report whether one direct managed-reference table overlaps the given byte range.
fn touches_reference_offsets(
    offsets: &[u32],
    start: usize,
    end: usize,
    managed_reference_bytes: usize,
) -> bool {
    offsets
        .iter()
        .copied()
        .any(|offset| ranges_overlap(start, end, offset as usize, managed_reference_bytes))
}

/// Report whether one value table overlaps the given byte range.
fn touches_value_offsets(offsets: &[u32], start: usize, end: usize, value_bytes: usize) -> bool {
    offsets
        .iter()
        .copied()
        .any(|offset| ranges_overlap(start, end, offset as usize, value_bytes))
}

/// Report whether one repeated managed-reference table overlaps the given byte range.
fn touches_repeated_reference_offsets(
    count: u32,
    element_size: u32,
    offsets: &[u32],
    start: usize,
    end: usize,
    managed_reference_bytes: usize,
) -> bool {
    offsets.iter().copied().any(|offset| {
        overlapping_repeated_index_range(
            start,
            end,
            count as usize,
            element_size as usize,
            offset as usize,
            managed_reference_bytes,
        )
        .is_some()
    })
}

/// Report whether one byte range overlaps one fixed-width field range.
pub(crate) fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_len: usize,
) -> bool {
    let Some(right_end) = right_start.checked_add(right_len) else {
        return true;
    };

    left_start < right_end && right_start < left_end
}

/// Return the overlapping repeated-element index range for the given byte window.
pub(crate) fn overlapping_repeated_index_range(
    start: usize,
    end: usize,
    count: usize,
    element_size: usize,
    offset: usize,
    width: usize,
) -> Option<(usize, usize)> {
    if count == 0 || width == 0 {
        return None;
    }

    if element_size == 0 {
        return ranges_overlap(start, end, offset, width).then_some((0, count - 1));
    }

    let start = start as i128;
    let end = end as i128;
    let count = count as i128;
    let element_size = element_size as i128;
    let offset = offset as i128;
    let width = width as i128;
    let low_numerator = start - offset - width + 1;
    let high_numerator = end - offset - 1;
    let low = div_ceil_i128(low_numerator, element_size).max(0);
    let high = div_floor_i128(high_numerator, element_size).min(count - 1);

    (low <= high).then_some((low as usize, high as usize))
}

/// Return floor(lhs / rhs) for signed integers with positive rhs.
fn div_floor_i128(lhs: i128, rhs: i128) -> i128 {
    debug_assert!(rhs > 0);

    let quotient = lhs / rhs;
    let remainder = lhs % rhs;

    if remainder < 0 {
        quotient - 1
    } else {
        quotient
    }
}

/// Return ceil(lhs / rhs) for signed integers with positive rhs.
fn div_ceil_i128(lhs: i128, rhs: i128) -> i128 {
    debug_assert!(rhs > 0);

    let quotient = lhs / rhs;
    let remainder = lhs % rhs;

    if remainder > 0 {
        quotient + 1
    } else {
        quotient
    }
}
