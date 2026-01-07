use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `throw` and `try/catch` in favor of Result types.
    ///
    /// Exception-based error handling can make control flow harder to follow
    /// and doesn't enforce error handling at compile time. Using Result types
    /// makes error handling explicit and type-safe.
    #[lint(
        id = "no-exceptions",
        code = "LR037",
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

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoExceptionsVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags throw and try/catch expressions.
struct NoExceptionsVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoExceptionsVisitor<'a, 'b> {
    /// Build a visitor for no-exceptions checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Report a throw expression.
    fn report_throw(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_EXCEPTIONS.id,
                NO_EXCEPTIONS.code,
                NO_EXCEPTIONS.category,
                severity,
                "avoid throw statements",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use Result type instead of throwing"),
        );
    }

    /// Report a try/catch expression.
    fn report_try(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_EXCEPTIONS.id,
                NO_EXCEPTIONS.code,
                NO_EXCEPTIONS.category,
                severity,
                "avoid try/catch blocks",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use Result type instead of catching exceptions"),
        );
    }
}

impl NodeVisitor for NoExceptionsVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for exception-related expressions
        match expression {
            dir::Expression::Throw { .. } => self.report_throw(id),
            dir::Expression::Try { .. } => self.report_try(id),
            _ => {}
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
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
            "test.ds",
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
            "test.ds",
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
            "test.ds",
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
            "test.ds",
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
            "test.ds",
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
            "test.ds",
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
            "test.ds",
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
            "test.ds",
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
