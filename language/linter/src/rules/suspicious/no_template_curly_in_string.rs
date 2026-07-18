use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow template interpolation in regular strings.
    pub NO_TEMPLATE_CURLY_IN_STRING {
        id: "no-template-curly-in-string",
        code: "LU031",
        description: "Disallow template interpolation in regular strings",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-template-curly-in-string.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
