use crate::BoxedLintRule;

mod explicit_function_return_type;

pub use explicit_function_return_type::*;

/// Get all pedantic rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![Box::new(ExplicitFunctionReturnType)]
}
