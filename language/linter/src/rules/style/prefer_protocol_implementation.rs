use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require protocol implementation for members named like protocol operations.
    pub PREFER_PROTOCOL_IMPLEMENTATION {
        id: "prefer-protocol-implementation",
        summary: "Require protocol implementation for members named like protocol operations",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check prefer-protocol-implementation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
