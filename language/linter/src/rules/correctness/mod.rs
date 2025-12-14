mod no_constant_condition;
mod no_debugger;
mod no_empty;
mod no_self_compare;

use crate::{BoxedLintRule, boxed};

pub use no_constant_condition::*;
pub use no_debugger::*;
pub use no_empty::*;
pub use no_self_compare::*;

/// Get all correctness rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoConstantCondition),
        boxed(NoDebugger),
        boxed(NoEmpty),
        boxed(NoSelfCompare),
    ]
}
