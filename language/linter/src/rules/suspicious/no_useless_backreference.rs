use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow useless regex backreferences.
    pub NO_USELESS_BACKREFERENCE {
        id: "no-useless-backreference",
        description: "Disallow useless regex backreferences",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-backreference.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
