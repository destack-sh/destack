use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirProgram};

declare_lint! {
    /// Disallow untrusted values flowing into dynamic regular expressions.
    pub NO_REGEX_INJECTION {
        id: "no-regex-injection",
        description: "Disallow untrusted values flowing into dynamic regular expressions",
        category: Security,
        level: Error,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check no-regex-injection.
fn check(_program: &MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
