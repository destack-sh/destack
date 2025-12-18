use crate::BoxedLintRule;

pub mod common;
pub mod complexity;
pub mod correctness;
pub mod pedantic;
pub mod performance;
pub mod restriction;
pub mod security;
pub mod style;
pub mod suspicious;

pub use common::*;

/// Get all built-in lint rules.
pub fn all_rules() -> Vec<BoxedLintRule> {
    let mut rules = Vec::new();
    rules.extend(complexity::rules());
    rules.extend(correctness::rules());
    rules.extend(pedantic::rules());
    rules.extend(performance::rules());
    rules.extend(restriction::rules());
    rules.extend(security::rules());
    rules.extend(style::rules());
    rules.extend(suspicious::rules());
    rules
}

/// Get all recommended lint rules.
pub fn recommended_rules() -> Vec<BoxedLintRule> {
    all_rules()
        .into_iter()
        .filter(|r| r.meta().category.is_recommended())
        .collect()
}
