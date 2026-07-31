use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow local annotations identical to the inferred type.
    pub NO_INFERRABLE_TYPE_ANNOTATION {
        id: "no-inferrable-type-annotation",
        summary: "Disallow local annotations identical to the inferred type",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-inferrable-type-annotation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
