mod no_nested_ternary;

use crate::{BoxedLintRule, boxed};

pub use no_nested_ternary::*;

/// Get all style rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoNestedTernary),
    ]
}
