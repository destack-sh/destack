use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `throw` and `try/catch` in favor of Result types.
    ///
    /// Exception-based error handling can make control flow harder to follow
    /// and doesn't enforce error handling at compile time. Using Result types
    /// makes error handling explicit and type-safe.
    #[lint(
        id = "no-exceptions",
        code = "LR012",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoExceptions,
    "Disallow throw and try/catch (use Result types)"
}

impl LintRule for NoExceptions {
    fn meta(&self) -> &'static LintMeta {
        NoExceptions::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        for (expression_id, expression) in ctx.dir.iter_nodes_of_type::<dir::Expression>() {
            let diagnostic = match expression {
                dir::Expression::Throw { .. } => build_throw_diagnostic(ctx, meta, expression_id),
                dir::Expression::Try { .. } => build_try_diagnostic(ctx, meta, expression_id),
                _ => None,
            };

            if let Some(diagnostic) = diagnostic {
                ctx.report(diagnostic);
            }
        }
    }
}

/// Build one diagnostic for a throw expression.
fn build_throw_diagnostic(
    ctx: &LintModuleContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintReport> {
    // honor per node severity
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return None;
    }

    // return the diagnostic
    Some(
        LintReport::new(
            NO_EXCEPTIONS.id,
            NO_EXCEPTIONS.code,
            NO_EXCEPTIONS.category,
            severity,
            "avoid throw statements",
            ctx.get_span(expression_id),
        )
        .label("use Result type instead of throwing"),
    )
}

/// Build one diagnostic for a try expression.
fn build_try_diagnostic(
    ctx: &LintModuleContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintReport> {
    // honor per node severity
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return None;
    }

    // return the diagnostic
    Some(
        LintReport::new(
            NO_EXCEPTIONS.id,
            NO_EXCEPTIONS.code,
            NO_EXCEPTIONS.category,
            severity,
            "avoid try/catch blocks",
            ctx.get_span(expression_id),
        )
        .label("use Result type instead of catching exceptions"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag throw statements.
    #[test]
    fn test_flags_throw() {
        let test = TestProgram::for_rule_with_prelude(NoExceptions);
        let result = test.lint_dir(
            "no_exceptions/test_flags_throw.ds",
            r#"
function fail(): never {
    throw new Error("failed");
}
"#,
        );
        test.result(result).assert_lint("no-exceptions");
    }

    /// Flag try/catch blocks.
    #[test]
    fn test_flags_try_catch() {
        let test = TestProgram::for_rule_with_prelude(NoExceptions);
        let result = test.lint_dir(
            "no_exceptions/test_flags_try_catch.ds",
            r#"
function risky(): number {
    try {
        return 42;
    } catch (e) {
        return 0;
    }
}
"#,
        );
        test.result(result).assert_lint("no-exceptions");
    }

    /// Flag try/finally blocks.
    #[test]
    fn test_flags_try_finally() {
        let test = TestProgram::for_rule_with_prelude(NoExceptions);
        let result = test.lint_dir(
            "no_exceptions/test_flags_try_finally.ds",
            r#"
function cleanup(): void {
    try {
        doWork();
    } finally {
        cleanupResources();
    }
}
function doWork(): void {}
function cleanupResources(): void {}
"#,
        );
        test.result(result).assert_lint("no-exceptions");
    }

    /// Allow Result-based error handling.
    #[test]
    fn test_allows_result_type() {
        let test = TestProgram::for_rule_with_prelude(NoExceptions);
        let result = test.lint_dir(
            "no_exceptions/test_allows_result_type.ds",
            r#"
function divide(a: number, b: number): Result<number, string> {
    if (b === 0) {
        return Result.Err("division by zero");
    }
    return Result.Ok(a / b);
}
"#,
        );
        test.result(result).assert_no_lint("no-exceptions");
    }

    /// Allow regular function calls without exceptions.
    #[test]
    fn test_allows_normal_code() {
        let test = TestProgram::for_rule_with_prelude(NoExceptions);
        let result = test.lint_dir(
            "no_exceptions/test_allows_normal_code.ds",
            r#"
function add(a: number, b: number): number {
    return a + b;
}
let sum = add(1, 2);
"#,
        );
        test.result(result).assert_no_lint("no-exceptions");
    }

    /// Flag throw in nested scope.
    #[test]
    fn test_flags_throw_in_nested_scope() {
        let test = TestProgram::for_rule_with_prelude(NoExceptions);
        let result = test.lint_dir(
            "no_exceptions/test_flags_throw_in_nested_scope.ds",
            r#"
function check(value: number): void {
    if (value < 0) {
        throw new Error("negative value");
    }
}
"#,
        );
        test.result(result).assert_lint("no-exceptions");
    }

    /// Flag try/catch/finally blocks.
    #[test]
    fn test_flags_try_catch_finally() {
        let test = TestProgram::for_rule_with_prelude(NoExceptions);
        let result = test.lint_dir(
            "no_exceptions/test_flags_try_catch_finally.ds",
            r#"
function process(): number {
    try {
        return 42;
    } catch (e) {
        return 0;
    } finally {
        cleanup();
    }
}
function cleanup(): void {}
"#,
        );
        test.result(result).assert_lint("no-exceptions");
    }

    /// Flag multiple throws.
    #[test]
    fn test_flags_multiple_throws() {
        let test = TestProgram::for_rule_with_prelude(NoExceptions);
        let result = test.lint_dir(
            "no_exceptions/test_flags_multiple_throws.ds",
            r#"
function validate(a: number, b: number): void {
    if (a < 0) {
        throw new Error("a is negative");
    }
    if (b < 0) {
        throw new Error("b is negative");
    }
}
"#,
        );
        test.result(result).assert_lint_count("no-exceptions", 2);
    }
}
