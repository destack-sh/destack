pub mod correctness;

use crate::BoxedLintRule;

/// Get all built-in lint rules.
pub fn all_rules() -> Vec<BoxedLintRule> {
    let mut rules = Vec::new();
    rules.extend(correctness::rules());
    rules
}

/// Get all recommended lint rules.
pub fn recommended_rules() -> Vec<BoxedLintRule> {
    all_rules()
        .into_iter()
        .filter(|r| r.meta().category.is_recommended())
        .collect()
}
