use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow loops that must exit during their first iteration.
    pub LOOP_SINGLE_ITERATION {
        id: "loop-single-iteration",
        summary: "Disallow loops that cannot repeat",
        explanation: "A loop whose control flow cannot reach a second iteration obscures a single conditional execution. Expressing that control flow as a condition makes its cardinality explicit without changing its behavior.",
        example: {
            reported: r#"
declare function ready(): boolean;
declare function process(): void;

while (ready()) {
    process();
    break;
}
"#,
            accepted: r#"
declare function ready(): boolean;
declare function process(): void;

if (ready()) {
    process();
}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check loop-single-iteration.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
