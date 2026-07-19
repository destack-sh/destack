use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Enforce default parameters to be last.
    pub DEFAULT_PARAM_LAST {
        id: "default-param-last",
        description: "Enforce default parameters to be last",
        category: Style,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check default-param-last.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
