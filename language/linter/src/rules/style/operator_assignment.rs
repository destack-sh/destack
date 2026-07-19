use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require compound assignment where equivalent.
    pub OPERATOR_ASSIGNMENT {
        id: "operator-assignment",
        description: "Require compound assignment where equivalent",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check operator-assignment.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
