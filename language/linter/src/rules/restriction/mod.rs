use crate::BoxedLintRule;

mod no_sequences;

pub use no_sequences::*;

/// Get all restriction rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![Box::new(NoSequences)]
}
