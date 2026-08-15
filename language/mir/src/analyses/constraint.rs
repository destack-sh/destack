use crate as mir;

use crate::{RangeState, ValueRange};

/// Integer bounds used by constraint evaluation.
#[derive(Debug, Clone, Copy)]
struct IntegerRange {
    /// Minimum value in the range.
    pub(crate) min: i128,
    /// Maximum value in the range.
    pub(crate) max: i128,
    /// Bit width of the integer.
    pub(crate) width: u16,
    /// Signedness of the integer.
    pub(crate) is_signed: bool,
}

impl RangeState {
    /// Evaluate one check constraint to a constant truth value when possible.
    pub fn truth_value(&self, constraint: &mir::CheckConstraint) -> Option<bool> {
        match constraint {
            mir::CheckConstraint::Bounds {
                index,
                length,
                is_signed,
                ..
            } => self.bounds_truth(*index, *length, *is_signed),
            mir::CheckConstraint::Null { .. } => None,
            mir::CheckConstraint::DivZero { divisor } => self.divisor_truth(*divisor),
            mir::CheckConstraint::ShiftRange {
                value,
                bit_width,
                is_signed,
            } => self.shift_truth(*value, u16::from(*bit_width), *is_signed),
            mir::CheckConstraint::Narrow {
                value,
                to_width,
                is_signed,
            } => self.narrow_truth(*value, u16::from(*to_width), *is_signed),
            mir::CheckConstraint::Overflow {
                operator,
                left,
                right,
                is_signed,
            } => self.overflow_truth(*operator, *left, *right, *is_signed),
            mir::CheckConstraint::IsType { .. } | mir::CheckConstraint::IsSubtype { .. } => None,
        }
    }

    /// Evaluate a bounds check constraint using range information.
    fn bounds_truth(&self, index: mir::Value, length: mir::Value, is_signed: bool) -> Option<bool> {
        // read index and length ranges
        let index_range = self.integer(index)?;
        let length_range = self.integer(length)?;

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
    fn divisor_truth(&self, divisor: mir::Value) -> Option<bool> {
        // read the divisor range
        let divisor_range = self.integer(divisor)?;

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
    fn shift_truth(&self, value: mir::Value, bit_width: u16, is_signed: bool) -> Option<bool> {
        // reject invalid widths
        let max_shift = i128::from(bit_width).checked_sub(1)?;

        // read the shift amount range
        let shift_range = self.integer(value)?;
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
    fn narrow_truth(&self, value: mir::Value, to_width: u16, is_signed: bool) -> Option<bool> {
        // compute target bounds
        let (target_min, target_max) = IntegerRange::bounds(to_width, is_signed)?;

        // read the value range
        let value_range = self.integer(value)?;
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
    fn overflow_truth(
        &self,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
        is_signed: bool,
    ) -> Option<bool> {
        // read operand ranges
        let left_range = self.integer(left)?;
        let right_range = self.integer(right)?;

        // require matching signedness and width
        if left_range.is_signed != is_signed
            || right_range.is_signed != is_signed
            || left_range.width != right_range.width
        {
            return None;
        }

        // compute the target bounds
        let (min_bound, max_bound) = IntegerRange::bounds(left_range.width, is_signed)?;

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
            mir::BinaryOperator::Divide | mir::BinaryOperator::Remainder => {
                // require signed overflow semantics
                if !is_signed {
                    return None;
                }

                // detect the min divided by negative one overflow case
                let min_value = min_bound;
                let neg_one = -1;
                let left_has_min = left_range.contains(min_value);
                let right_has_neg_one = right_range.contains(neg_one);

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

    /// Return the integer range for one value.
    fn integer(&self, value: mir::Value) -> Option<IntegerRange> {
        // fetch the range and require integer bounds
        let range = self.get(value)?;
        let ValueRange::Integer {
            min,
            max,
            width,
            is_signed,
        } = range
        else {
            return None;
        };

        Some(IntegerRange {
            min: *min,
            max: *max,
            width: *width,
            is_signed: *is_signed,
        })
    }
}

impl IntegerRange {
    /// Compute the full bounds for an integer width and signedness.
    fn bounds(width: u16, is_signed: bool) -> Option<(i128, i128)> {
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

    /// Return whether this range includes one value.
    fn contains(&self, value: i128) -> bool {
        self.min <= value && value <= self.max
    }
}
