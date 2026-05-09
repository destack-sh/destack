use destack_mir as mir;

use crate::common::mir::analysis::ValueRange;

/// Return true when an operator yields a boolean value.
pub(crate) fn is_comparison_operator(operator: mir::BinaryOperator) -> bool {
    matches!(
        operator,
        mir::BinaryOperator::Equal
            | mir::BinaryOperator::NotEqual
            | mir::BinaryOperator::SignedLessThan
            | mir::BinaryOperator::SignedLessEqual
            | mir::BinaryOperator::SignedGreaterThan
            | mir::BinaryOperator::SignedGreaterEqual
            | mir::BinaryOperator::UnsignedLessThan
            | mir::BinaryOperator::UnsignedLessEqual
            | mir::BinaryOperator::UnsignedGreaterThan
            | mir::BinaryOperator::UnsignedGreaterEqual
            | mir::BinaryOperator::FloatEqual
            | mir::BinaryOperator::FloatNotEqual
            | mir::BinaryOperator::FloatLessThan
            | mir::BinaryOperator::FloatLessEqual
            | mir::BinaryOperator::FloatGreaterThan
            | mir::BinaryOperator::FloatGreaterEqual
    )
}

/// Swap a comparison operator when the operands are reversed.
pub(crate) fn swap_comparison_operator(
    operator: mir::BinaryOperator,
) -> Option<mir::BinaryOperator> {
    match operator {
        mir::BinaryOperator::Equal | mir::BinaryOperator::NotEqual => Some(operator),
        mir::BinaryOperator::SignedLessThan => Some(mir::BinaryOperator::SignedGreaterThan),
        mir::BinaryOperator::SignedLessEqual => Some(mir::BinaryOperator::SignedGreaterEqual),
        mir::BinaryOperator::SignedGreaterThan => Some(mir::BinaryOperator::SignedLessThan),
        mir::BinaryOperator::SignedGreaterEqual => Some(mir::BinaryOperator::SignedLessEqual),
        mir::BinaryOperator::UnsignedLessThan => Some(mir::BinaryOperator::UnsignedGreaterThan),
        mir::BinaryOperator::UnsignedLessEqual => Some(mir::BinaryOperator::UnsignedGreaterEqual),
        mir::BinaryOperator::UnsignedGreaterThan => Some(mir::BinaryOperator::UnsignedLessThan),
        mir::BinaryOperator::UnsignedGreaterEqual => Some(mir::BinaryOperator::UnsignedLessEqual),
        _ => None,
    }
}

/// Convert a boolean range into a constant when possible.
pub(crate) fn bool_from_range(range: Option<&ValueRange>) -> Option<bool> {
    let ValueRange::Boolean {
        can_be_true,
        can_be_false,
    } = range?
    else {
        return None;
    };

    match (*can_be_true, *can_be_false) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}

/// Evaluate an integer comparison using range information.
pub(crate) fn evaluate_integer_range_comparison(
    operator: mir::BinaryOperator,
    left: &ValueRange,
    right: &ValueRange,
) -> Option<bool> {
    // extract integer ranges for both operands
    let ValueRange::Integer {
        min: left_min,
        max: left_max,
        width: left_width,
        is_signed: left_signed,
    } = left
    else {
        return None;
    };
    let ValueRange::Integer {
        min: right_min,
        max: right_max,
        width: right_width,
        is_signed: right_signed,
    } = right
    else {
        return None;
    };

    // reject mismatched integer widths or signedness
    if left_width != right_width || left_signed != right_signed {
        return None;
    }

    // reject comparisons that do not match operand signedness
    let expects_signed = matches!(
        operator,
        mir::BinaryOperator::SignedLessThan
            | mir::BinaryOperator::SignedLessEqual
            | mir::BinaryOperator::SignedGreaterThan
            | mir::BinaryOperator::SignedGreaterEqual
    );
    let expects_unsigned = matches!(
        operator,
        mir::BinaryOperator::UnsignedLessThan
            | mir::BinaryOperator::UnsignedLessEqual
            | mir::BinaryOperator::UnsignedGreaterThan
            | mir::BinaryOperator::UnsignedGreaterEqual
    );
    if expects_signed && !*left_signed {
        return None;
    }
    if expects_unsigned && *left_signed {
        return None;
    }

    // evaluate comparison from range relationships
    match operator {
        mir::BinaryOperator::Equal => {
            let is_single = left_min == left_max && right_min == right_max;
            if is_single && left_min == right_min {
                Some(true)
            } else if left_max < right_min || left_min > right_max {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::NotEqual => {
            let is_single = left_min == left_max && right_min == right_max;
            if left_max < right_min || left_min > right_max {
                Some(true)
            } else if is_single && left_min == right_min {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedLessThan | mir::BinaryOperator::UnsignedLessThan => {
            if left_max < right_min {
                Some(true)
            } else if left_min >= right_max {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedLessEqual | mir::BinaryOperator::UnsignedLessEqual => {
            if left_max <= right_min {
                Some(true)
            } else if left_min > right_max {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedGreaterThan | mir::BinaryOperator::UnsignedGreaterThan => {
            if left_min > right_max {
                Some(true)
            } else if left_max <= right_min {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedGreaterEqual | mir::BinaryOperator::UnsignedGreaterEqual => {
            if left_min >= right_max {
                Some(true)
            } else if left_max < right_min {
                Some(false)
            } else {
                None
            }
        }
        _ => None,
    }
}
