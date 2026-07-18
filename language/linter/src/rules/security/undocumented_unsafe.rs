use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require a safety rationale for every unsafe declaration and expression.
    pub UNDOCUMENTED_UNSAFE {
        id: "undocumented-unsafe",
        code: "LS011",
        description: "Require a safety rationale for every unsafe declaration and expression",
        category: Security,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check undocumented-unsafe.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
