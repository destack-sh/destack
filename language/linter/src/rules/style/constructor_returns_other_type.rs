use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow constructor-shaped statics returning a different type.
    pub CONSTRUCTOR_RETURNS_OTHER_TYPE {
        id: "constructor-returns-other-type",
        summary: "Disallow constructor-shaped statics returning a different type",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check constructor-returns-other-type.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
