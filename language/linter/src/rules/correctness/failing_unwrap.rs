use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow unwrapping values control flow already proves absent or failed.
    pub FAILING_UNWRAP {
        id: "failing-unwrap",
        summary: "Disallow unwrapping values control flow already proves absent or failed",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check failing-unwrap.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
