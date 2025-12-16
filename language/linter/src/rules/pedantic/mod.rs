mod radix;

use crate::{BoxedLintRule, boxed};

pub use radix::*;

/// Get all pedantic rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(Radix),
    ]
}
