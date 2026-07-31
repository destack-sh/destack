use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow contradictory or ineffective file open options.
    pub NONSENSICAL_OPEN_OPTIONS {
        id: "nonsensical-open-options",
        summary: "Disallow contradictory or ineffective file open options",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check nonsensical-open-options.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
