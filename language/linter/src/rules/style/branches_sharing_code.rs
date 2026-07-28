use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Move identical branch prefixes or suffixes outside the conditional.
    pub BRANCHES_SHARING_CODE {
        id: "branches-sharing-code",
        summary: "Move identical branch prefixes or suffixes outside the conditional",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check branches-sharing-code.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
