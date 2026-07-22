use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow unwrap after control flow already proves the present case.
    pub UNNECESSARY_UNWRAP {
        id: "unnecessary-unwrap",
        summary: "Disallow unwrap after control flow already proves the present case",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check unnecessary-unwrap.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
