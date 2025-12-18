mod for_direction;
mod no_compare_neg_zero;
mod no_constant_binary_expression;
mod no_constant_condition;
mod no_duplicate_case;
mod no_fallthrough;
mod no_self_compare;
mod no_unsafe_finally;
mod no_unsafe_negation;
mod use_isnan;

use crate::{BoxedLintRule, boxed};

pub use for_direction::*;
pub use no_compare_neg_zero::*;
pub use no_constant_binary_expression::*;
pub use no_constant_condition::*;
pub use no_duplicate_case::*;
pub use no_fallthrough::*;
pub use no_self_compare::*;
pub use no_unsafe_finally::*;
pub use no_unsafe_negation::*;
pub use use_isnan::*;

/// Get all correctness rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(ForDirection),
        boxed(NoCompareNegZero),
        boxed(NoConstantBinaryExpression),
        boxed(NoConstantCondition),
        boxed(NoDuplicateCase),
        boxed(NoFallthrough),
        boxed(NoSelfCompare),
        boxed(NoUnsafeFinally),
        boxed(NoUnsafeNegation),
        boxed(UseIsnan),
    ]
}
