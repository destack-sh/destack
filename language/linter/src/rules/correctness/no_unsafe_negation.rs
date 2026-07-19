use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow negation of left operand in relational operators.
    pub NO_UNSAFE_NEGATION {
        id: "no-unsafe-negation",
        description: "Disallow negation of left operand in relational operators",
        category: Correctness,
        level: Error,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-unsafe-negation.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
