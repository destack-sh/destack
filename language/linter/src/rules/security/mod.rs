mod no_eval;

use crate::{BoxedLintRule, boxed};

pub use no_eval::*;

/// Get all security rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoEval),
    ]
}
