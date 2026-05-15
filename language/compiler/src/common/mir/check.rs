use destack_mir as mir;

use crate::common::mir::analysis::{RangeMap, ValueRange};

/// Snapshot of an integer range for constraint evaluation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IntegerRangeSnapshot {
    /// Minimum value in the range.
    pub(crate) min: i128,
    /// Maximum value in the range.
    pub(crate) max: i128,
    /// Bit width of the integer.
    pub(crate) width: u16,
    /// Signedness of the integer.
    pub(crate) is_signed: bool,
}

/// Evaluate a check constraint to a constant truth value when possible.
pub(crate) fn constraint_truth_value(
    constraint: &mir::CheckConstraint,
    ranges: &RangeMap,
) -> Option<bool> {
    match constraint {
        mir::CheckConstraint::Bounds {
            index,
            length,
            is_signed,
            ..
        } => bounds_constraint_truth(*index, *length, *is_signed, ranges),
        mir::CheckConstraint::Null { .. } => None,
        mir::CheckConstraint::DivZero { divisor } => div_zero_constraint_truth(*divisor, ranges),
        mir::CheckConstraint::ShiftRange {
            value,
            bit_width,
            is_signed,
        } => shift_constraint_truth(*value, u16::from(*bit_width), *is_signed, ranges),
        mir::CheckConstraint::Narrow {
            value,
            to_width,
            is_signed,
        } => narrow_constraint_truth(*value, u16::from(*to_width), *is_signed, ranges),
        mir::CheckConstraint::Overflow {
            operator,
            left,
            right,
            is_signed,
        } => overflow_constraint_truth(*operator, *left, *right, *is_signed, ranges),
        mir::CheckConstraint::Type { .. }
        | mir::CheckConstraint::Variant { .. }
        | mir::CheckConstraint::ReceiverType { .. }
        | mir::CheckConstraint::Implements { .. } => None,
    }
}

/// Evaluate a bounds check constraint using range information.
fn bounds_constraint_truth(
    index: mir::ValueReference,
    length: mir::ValueReference,
    is_signed: bool,
    ranges: &RangeMap,
) -> Option<bool> {
    // require concrete values for range reasoning
    let index = index.value()?;
    let length = length.value()?;

    // read index and length ranges
    let index_range = integer_range_snapshot(index, ranges)?;
    let length_range = integer_range_snapshot(length, ranges)?;

    // require signedness alignment
    if index_range.is_signed != is_signed || length_range.is_signed != is_signed {
        return None;
    }

    // check for always failing ranges
    if length_range.max <= 0 {
        return Some(false);
    }
    if is_signed && index_range.max < 0 {
        return Some(false);
    }
    if index_range.min >= length_range.max {
        return Some(false);
    }

    // check for always successful ranges
    if index_range.min >= 0 && length_range.min >= 0 && index_range.max < length_range.min {
        return Some(true);
    }

    None
}

/// Evaluate a division by zero constraint using range information.
fn div_zero_constraint_truth(divisor: mir::ValueReference, ranges: &RangeMap) -> Option<bool> {
    // require a concrete divisor value
    let divisor = divisor.value()?;

    // read the divisor range
    let divisor_range = integer_range_snapshot(divisor, ranges)?;

    // return when the divisor is always zero or always non zero
    if divisor_range.min == 0 && divisor_range.max == 0 {
        return Some(false);
    }

    if divisor_range.max < 0 || divisor_range.min > 0 {
        return Some(true);
    }

    None
}

/// Evaluate a shift range constraint using range information.
fn shift_constraint_truth(
    value: mir::ValueReference,
    bit_width: u16,
    is_signed: bool,
    ranges: &RangeMap,
) -> Option<bool> {
    // require a concrete shift amount
    let value = value.value()?;

    // reject invalid widths
    let max_shift = i128::from(bit_width).checked_sub(1)?;

    // read the shift amount range
    let shift_range = integer_range_snapshot(value, ranges)?;
    if shift_range.is_signed != is_signed {
        return None;
    }

    // check for always failing ranges
    if shift_range.max < 0 || shift_range.min > max_shift {
        return Some(false);
    }

    // check for always successful ranges
    if shift_range.min >= 0 && shift_range.max <= max_shift {
        return Some(true);
    }

    None
}

/// Evaluate a narrowing constraint using range information.
fn narrow_constraint_truth(
    value: mir::ValueReference,
    to_width: u16,
    is_signed: bool,
    ranges: &RangeMap,
) -> Option<bool> {
    // require a concrete input value
    let value = value.value()?;

    // compute target bounds
    let (target_min, target_max) = integer_bounds(to_width, is_signed)?;

    // read the value range
    let value_range = integer_range_snapshot(value, ranges)?;
    if value_range.is_signed != is_signed {
        return None;
    }

    // check for always failing ranges
    if value_range.max < target_min || value_range.min > target_max {
        return Some(false);
    }

    // check for always successful ranges
    if value_range.min >= target_min && value_range.max <= target_max {
        return Some(true);
    }

    None
}

/// Evaluate an overflow constraint using range information.
fn overflow_constraint_truth(
    operator: mir::BinaryOperator,
    left: mir::ValueReference,
    right: mir::ValueReference,
    is_signed: bool,
    ranges: &RangeMap,
) -> Option<bool> {
    // require concrete operands
    let left = left.value()?;
    let right = right.value()?;

    // read operand ranges
    let left_range = integer_range_snapshot(left, ranges)?;
    let right_range = integer_range_snapshot(right, ranges)?;

    // require matching signedness and width
    if left_range.is_signed != is_signed
        || right_range.is_signed != is_signed
        || left_range.width != right_range.width
    {
        return None;
    }

    // compute the target bounds
    let (min_bound, max_bound) = integer_bounds(left_range.width, is_signed)?;

    // compute the result bounds
    let (result_min, result_max) = match operator {
        mir::BinaryOperator::Add => (
            left_range.min.checked_add(right_range.min)?,
            left_range.max.checked_add(right_range.max)?,
        ),
        mir::BinaryOperator::Subtract => (
            left_range.min.checked_sub(right_range.max)?,
            left_range.max.checked_sub(right_range.min)?,
        ),
        mir::BinaryOperator::Multiply => {
            let candidates = [
                left_range.min.checked_mul(right_range.min)?,
                left_range.min.checked_mul(right_range.max)?,
                left_range.max.checked_mul(right_range.min)?,
                left_range.max.checked_mul(right_range.max)?,
            ];
            let min = *candidates.iter().min()?;
            let max = *candidates.iter().max()?;
            (min, max)
        }
        mir::BinaryOperator::SignedDivide | mir::BinaryOperator::SignedRemainder => {
            // require signed overflow semantics
            if !is_signed {
                return None;
            }

            // detect the min divided by negative one overflow case
            let min_value = min_bound;
            let neg_one = -1;
            let left_has_min = range_includes_value(&left_range, min_value);
            let right_has_neg_one = range_includes_value(&right_range, neg_one);

            // reject when overflow remains possible
            if left_has_min && right_has_neg_one {
                if left_range.min == min_value
                    && left_range.max == min_value
                    && right_range.min == neg_one
                    && right_range.max == neg_one
                {
                    return Some(false);
                }

                return None;
            }

            // remaining cases cannot overflow
            return Some(true);
        }
        _ => return None,
    };

    // check for always failing ranges
    if result_max < min_bound || result_min > max_bound {
        return Some(false);
    }

    // check for always successful ranges
    if result_min >= min_bound && result_max <= max_bound {
        return Some(true);
    }

    None
}

/// Extract an integer range snapshot for a value.
fn integer_range_snapshot(value: mir::Value, ranges: &RangeMap) -> Option<IntegerRangeSnapshot> {
    // fetch the range and require integer bounds
    let range = ranges.get(value)?;
    let ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    } = range
    else {
        return None;
    };

    Some(IntegerRangeSnapshot {
        min: *min,
        max: *max,
        width: *width,
        is_signed: *is_signed,
    })
}

/// Compute the full bounds for an integer width and signedness.
fn integer_bounds(width: u16, is_signed: bool) -> Option<(i128, i128)> {
    // reject nonsensical widths
    if width == 0 {
        return None;
    }

    // handle full width signed bounds
    if is_signed && width == 128 {
        return Some((i128::MIN, i128::MAX));
    }

    // compute signed or unsigned bounds
    if is_signed {
        let shift = u32::from(width).checked_sub(1)?;
        let magnitude = (1_i128).checked_shl(shift)?;
        let min = -magnitude;
        let max = magnitude.checked_sub(1)?;
        Some((min, max))
    } else {
        let magnitude = (1_i128).checked_shl(u32::from(width))?;
        let max = magnitude.checked_sub(1)?;
        Some((0, max))
    }
}

/// Return true when a range includes a specific value.
fn range_includes_value(range: &IntegerRangeSnapshot, value: i128) -> bool {
    range.min <= value && value <= range.max
}
