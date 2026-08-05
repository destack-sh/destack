use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirProgram};

declare_lint_stub! {
    /// Require panic documentation for public functions that may panic.
    pub MISSING_PANICS_DOC {
        id: "missing-panics-doc",
        summary: "Require panic documentation for public functions that may panic",
        category: Style,
        level: Warning,
        fixable: None,
        check: MirProgram(check),
    }
}

/// Check missing-panics-doc.
fn check(_program: &mut MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
