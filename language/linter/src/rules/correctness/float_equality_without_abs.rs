use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require absolute difference in epsilon float comparisons.
    pub FLOAT_EQUALITY_WITHOUT_ABS {
        id: "float-equality-without-abs",
        summary: "Require absolute difference in epsilon float comparisons",
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check float-equality-without-abs.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
