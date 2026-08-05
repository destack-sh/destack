use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirProgram};

declare_lint_stub! {
    /// Disallow reachable panics in public APIs.
    pub PANIC_IN_PUBLIC_API {
        id: "panic-in-public-api",
        summary: "Disallow reachable panics in public APIs",
        explanation: "An exported function should represent expected failure in its return type rather than terminate its caller. Report explicit and implicit panic operations reachable from the public call graph, including force unwraps, out-of-bounds access, and overflowing operations.",
        example: {
            reported: r#"
export function divide(value: int32, divisor: int32): int32 {
    if (divisor === 0) {
        panic("division by zero");
    }

    return value / divisor;
}
"#,
            accepted: r#"
export function divide(value: int32, divisor: int32): Result<int32, string> {
    if (divisor === 0) {
        return Result.err("division by zero");
    }

    return Result.ok(value / divisor);
}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirProgram(check),
    }
}

/// Check panic-in-public-api.
fn check(_program: &mut MirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
