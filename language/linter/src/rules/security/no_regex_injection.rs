use crate::rules::declare_lint;
use crate::{LinterError, MirProgramContext};

declare_lint! {
    /// Disallow untrusted values flowing into dynamic regular expressions.
    pub NO_REGEX_INJECTION {
        id: "no-regex-injection",
        code: "LS007",
        description: "Disallow untrusted values flowing into dynamic regular expressions",
        category: Security,
        level: Error,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check no-regex-injection.
fn check(context: MirProgramContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
