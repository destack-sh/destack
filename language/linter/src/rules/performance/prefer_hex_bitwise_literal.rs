use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer hexadecimal literals for bitwise operands.
    pub PREFER_HEX_BITWISE_LITERAL {
        id: "prefer-hex-bitwise-literal",
        summary: "Prefer hexadecimal literals for bitwise operands",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check prefer-hex-bitwise-literal.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
