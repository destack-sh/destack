use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require expected messages on panic-expecting tests.
    pub REQUIRE_EXPECTED_FAILURE_MESSAGE {
        id: "require-expected-failure-message",
        summary: "Require expected messages on panic-expecting tests",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check require-expected-failure-message.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
