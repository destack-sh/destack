use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Enforce filename case style.
    pub FILENAME_CASE {
        id: "filename-case",
        description: "Enforce filename case style",
        category: Style,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check filename-case.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
