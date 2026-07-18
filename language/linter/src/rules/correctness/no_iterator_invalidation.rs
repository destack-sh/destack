use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow invalidating a collection while one of its iterators remains live.
    pub NO_ITERATOR_INVALIDATION {
        id: "no-iterator-invalidation",
        code: "LC021",
        description: "Disallow invalidating a collection while one of its iterators remains live",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check no-iterator-invalidation.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
