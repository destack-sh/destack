use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirProgram};

declare_lint_stub! {
    /// Require must-use marking for public functions whose value is their only effect.
    pub MUST_USE_CANDIDATE {
        id: "must-use-candidate",
        summary: "Require must-use marking for public functions whose value is their only effect",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: MirProgram(check),
    }
}

/// Check must-use-candidate.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
