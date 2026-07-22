use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow propagation immediately wrapped in the same result form.
    pub NEEDLESS_QUESTION_MARK {
        id: "needless-question-mark",
        summary: "Disallow propagation immediately wrapped in the same result form",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check needless-question-mark.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
