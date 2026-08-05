use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer methods when the first parameter establishes a receiver.
    pub PREFER_METHOD {
        id: "prefer-method",
        summary: "Prefer methods when the first parameter establishes a receiver",
        explanation: "A function whose first parameter supplies the operation's canonical nominal owner should be an inherent or extension method. Symmetric operations, protocol implementations, constructors, callbacks, intrinsics, and generated declarations are exempt.",
        example: {
            reported: r#"
struct Counter {
    value: int32;
}

function increment(counter: &Counter): void {
    counter.value += 1;
}
"#,
            accepted: r#"
struct Counter {
    value: int32;
}

extension of Counter {
    increment(this: &Counter): void {
        this.value += 1;
    }
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check prefer-method.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
