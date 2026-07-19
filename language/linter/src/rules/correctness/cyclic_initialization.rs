use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirProgram};

declare_lint! {
    /// Disallow unsafe cycles in target initialization.
    pub CYCLIC_INITIALIZATION {
        id: "cyclic-initialization",
        description: "Disallow unsafe cycles in target initialization",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check cyclic-initialization.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
