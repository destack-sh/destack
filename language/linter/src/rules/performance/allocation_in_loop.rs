use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when an allocating operation executes on a repeated path.
    pub ALLOCATION_IN_LOOP {
        id: "allocation-in-loop",
        description: "Warn when an allocating operation executes on a repeated path",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check allocation-in-loop.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
