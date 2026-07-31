use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirProgram, Lint, LintResult};

declare_lint_stub! {
    /// Disallow fields written but never read program-wide.
    pub UNREAD_FIELD {
        id: "unread-field",
        summary: "Disallow fields written but never read program-wide",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirProgram(check),
    }
}

/// Check unread-field.
fn check(_program: &DirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
