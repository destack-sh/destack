use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow breaks that leave a switch where the loop was intended.
    pub INEFFECTIVE_BREAK_IN_SWITCH {
        id: "ineffective-break-in-switch",
        summary: "Disallow breaks that leave a switch where the loop was intended",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check ineffective-break-in-switch.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
