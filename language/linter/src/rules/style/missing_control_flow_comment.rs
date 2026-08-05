use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require intent comments before substantial control flow.
    pub MISSING_CONTROL_FLOW_COMMENT {
        id: "missing-control-flow-comment",
        summary: "Require intent comments before substantial control flow",
        explanation: "A substantial branch or loop should state the operation it performs when the code alone does not provide a short local name. The comment must describe intent rather than restate the condition, and trivial guards remain exempt.",
        example: {
            reported: r#"
function sum(limit: int32): int32 {
    let index = 0;
    let total = 0;
    while (index < limit) {
        total += index;
        index += 1;
    }

    return total;
}
"#,
            accepted: r#"
function sum(limit: int32): int32 {
    let index = 0;
    let total = 0;

    // accumulate every value below the limit
    while (index < limit) {
        total += index;
        index += 1;
    }

    return total;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check missing-control-flow-comment.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
