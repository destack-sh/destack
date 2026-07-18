use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer tuple destructuring over indexed access.
    pub PREFER_TUPLE_DESTRUCTURE {
        id: "prefer-tuple-destructure",
        code: "LY059",
        description: "Prefer tuple destructuring over indexed access",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check prefer-tuple-destructure.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
