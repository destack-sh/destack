use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow template interpolation in regular strings.
    pub NO_TEMPLATE_CURLY_IN_STRING {
        id: "no-template-curly-in-string",
        summary: "Disallow template interpolation in regular strings",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-template-curly-in-string.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
