mod no_cond_assign;
mod no_debugger;
mod no_empty;

use crate::{BoxedLintRule, boxed};

pub use no_cond_assign::*;
pub use no_debugger::*;
pub use no_empty::*;

/// Get all suspicious rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![boxed(NoCondAssign), boxed(NoDebugger), boxed(NoEmpty)]
}
