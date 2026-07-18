use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow fallbacks that cannot affect a spread.
    pub NO_USELESS_SPREAD_FALLBACK {
        id: "no-useless-spread-fallback",
        code: "LU046",
        description: "Disallow fallbacks that cannot affect a spread",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-spread-fallback.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
