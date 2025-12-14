mod no_debugger;

use crate::{boxed, BoxedLintRule};

pub use no_debugger::*;

/// Get all correctness rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![boxed(NoDebugger)]
}
