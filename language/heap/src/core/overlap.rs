use super::{HeapScan, PACKED_VALUE_BYTES};
use crate::HeapResult;

/// Return whether one write range may overlap any heap-edge bytes.
pub(crate) fn overlaps_heap_range(
    map: &HeapScan,
    start: usize,
    len: usize,
    heap_reference_bytes: usize,
) -> HeapResult<bool> {
    if len == 0 {
        return Ok(false);
    }

    let Some(end) = start.checked_add(len) else {
        return Ok(true);
    };

    let is_overlapping = match map {
        HeapScan::None => false,
        HeapScan::Reference { local_offsets, .. } => {
            overlaps_reference_offsets(local_offsets, start, end, heap_reference_bytes)
        }
        HeapScan::PackedValue { offsets } => {
            overlaps_value_offsets(offsets, start, end, PACKED_VALUE_BYTES)
        }
        HeapScan::RepeatedReference {
            count,
            stride,
            local_offsets,
            ..
        } => overlaps_repeated_reference_offsets(
            *count,
            *stride,
            local_offsets,
            start,
            end,
            heap_reference_bytes,
        ),
    };

    Ok(is_overlapping)
}

/// Return whether one write range may overlap any shared heap-edge bytes.
pub(crate) fn overlaps_shared_range(
    map: &HeapScan,
    start: usize,
    len: usize,
    shared_reference_bytes: usize,
) -> HeapResult<bool> {
    if len == 0 {
        return Ok(false);
    }

    let Some(end) = start.checked_add(len) else {
        return Ok(true);
    };

    let is_overlapping = match map {
        HeapScan::None => false,
        HeapScan::Reference { shared_offsets, .. } => {
            overlaps_reference_offsets(shared_offsets, start, end, shared_reference_bytes)
        }
        HeapScan::PackedValue { offsets } => {
            overlaps_value_offsets(offsets, start, end, PACKED_VALUE_BYTES)
        }
        HeapScan::RepeatedReference {
            count,
            stride,
            shared_offsets,
            ..
        } => overlaps_repeated_reference_offsets(
            *count,
            *stride,
            shared_offsets,
            start,
            end,
            shared_reference_bytes,
        ),
    };

    Ok(is_overlapping)
}

/// Report whether one direct heap-reference table overlaps the given byte range.
fn overlaps_reference_offsets(
    offsets: &[u32],
    start: usize,
    end: usize,
    heap_reference_bytes: usize,
) -> bool {
    offsets
        .iter()
        .copied()
        .any(|offset| ranges_overlap(start, end, offset as usize, heap_reference_bytes))
}

/// Report whether one value table overlaps the given byte range.
fn overlaps_value_offsets(offsets: &[u32], start: usize, end: usize, value_bytes: usize) -> bool {
    offsets
        .iter()
        .copied()
        .any(|offset| ranges_overlap(start, end, offset as usize, value_bytes))
}

/// Report whether one repeated heap-reference table overlaps the given byte range.
fn overlaps_repeated_reference_offsets(
    count: u32,
    stride: u32,
    offsets: &[u32],
    start: usize,
    end: usize,
    heap_reference_bytes: usize,
) -> bool {
    offsets.iter().copied().any(|offset| {
        overlapping_repeated_index_range(
            start,
            end,
            count as usize,
            stride as usize,
            offset as usize,
            heap_reference_bytes,
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
    stride: usize,
    offset: usize,
    width: usize,
) -> Option<(usize, usize)> {
    if count == 0 || width == 0 {
        return None;
    }

    if stride == 0 {
        return ranges_overlap(start, end, offset, width).then_some((0, count - 1));
    }

    let start = start as i128;
    let end = end as i128;
    let count = count as i128;
    let stride = stride as i128;
    let offset = offset as i128;
    let width = width as i128;
    let low_numerator = start - offset - width + 1;
    let high_numerator = end - offset - 1;
    let low = div_ceil_i128(low_numerator, stride).max(0);
    let high = div_floor_i128(high_numerator, stride).min(count - 1);

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
