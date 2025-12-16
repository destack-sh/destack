mod max_params;

use crate::{BoxedLintRule, boxed};

pub use max_params::*;

/// Get all complexity rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![boxed(MaxParams)]
}
