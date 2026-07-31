use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow asymmetric operands in chains of similar comparisons.
    pub SUSPICIOUS_OPERAND_GROUPING {
        id: "suspicious-operand-grouping",
        summary: "Disallow asymmetric operands in chains of similar comparisons",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check suspicious-operand-grouping.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
