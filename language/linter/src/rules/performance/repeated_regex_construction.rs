use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow repeated construction of the same regular expression.
    pub REPEATED_REGEX_CONSTRUCTION {
        id: "repeated-regex-construction",
        summary: "Disallow repeated construction of the same regular expression",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check repeated-regex-construction.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
