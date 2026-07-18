use crate::rules::declare_lint;
use crate::{LinterError, MirProgramContext};

declare_lint! {
    /// Disallow values carrying a source taint from reaching matching sinks.
    pub NO_TAINTED_SINK {
        id: "no-tainted-sink",
        code: "LS010",
        description: "Disallow values carrying a source taint from reaching matching sinks",
        category: Security,
        level: Error,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check no-tainted-sink.
fn check(context: MirProgramContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
