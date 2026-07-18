use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when significant destruction is extended by a scrutinee.
    pub SIGNIFICANT_DROP_IN_SCRUTINEE {
        id: "significant-drop-in-scrutinee",
        code: "LU051",
        description: "Warn when significant destruction is extended by a scrutinee",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check significant-drop-in-scrutinee.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
