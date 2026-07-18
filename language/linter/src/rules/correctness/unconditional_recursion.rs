use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirProgram};

declare_lint! {
    /// Disallow functions that recurse before every possible return.
    pub UNCONDITIONAL_RECURSION {
        id: "unconditional-recursion",
        code: "LC050",
        description: "Disallow functions that recurse before every possible return",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check unconditional-recursion.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
