mod for_direction;
mod no_constant_condition;
mod no_self_compare;

use crate::{BoxedLintRule, boxed};

pub use for_direction::*;
pub use no_constant_condition::*;
pub use no_self_compare::*;

/// Get all correctness rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(ForDirection),
        boxed(NoConstantCondition),
        boxed(NoSelfCompare),
    ]
}
