use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require receiver forms matching the method's name convention.
    pub WRONG_RECEIVER_CONVENTION {
        id: "wrong-receiver-convention",
        summary: "Require receiver forms matching the method's name convention",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check wrong-receiver-convention.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
