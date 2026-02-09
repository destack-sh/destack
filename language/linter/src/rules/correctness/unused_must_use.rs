use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_has_decorator, expression_unwrap_parenthesized};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow ignoring return values from `@mustUse` APIs.
    ///
    /// Functions marked as `@mustUse` signal that dropping the return value
    /// is almost always a bug.
    #[lint(
        id = "unused-must-use",
        code = "LC056",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub UnusedMustUse,
    "Disallow ignoring return values of @mustUse functions"
}

impl LintRule for UnusedMustUse {
    fn meta(&self) -> &'static LintMeta {
        UnusedMustUse::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = UnusedMustUseVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor for ignored must-use return checks.
struct UnusedMustUseVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> UnusedMustUseVisitor<'a, 'b> {
    /// Build a visitor for ignored must-use return checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one statement for ignored must-use results.
    fn check_statement(
        &mut self,
        statement_id: dir::LocalNodeId<dir::Expression>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);
        if !matches!(
            expression,
            dir::Expression::Call { .. } | dir::Expression::New { .. }
        ) {
            return;
        }

        let has_must_use = expression_has_decorator(
            &self.ctx.program,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            self.ctx.types,
            expression_id,
            expression,
            |decorators| decorators.is_must_use,
        );
        if !has_must_use {
            return;
        }

        let severity = self.ctx.get_effective_severity(self.meta, statement_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(statement_id);
        self.ctx.report(
            LintDiagnostic::new(
                UNUSED_MUST_USE.id,
                UNUSED_MUST_USE.code,
                UNUSED_MUST_USE.category,
                severity,
                "ignored return value from @mustUse API",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use, return, or explicitly handle this result"),
        );
    }
}

impl NodeVisitor for UnusedMustUseVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if let dir::Expression::Statement { statement } = expression {
            self.check_statement(id, *statement);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag ignored @mustUse function calls.
    #[test]
    fn test_flags_ignored_must_use_call() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_flags_ignored_must_use_call.ds",
            r#"
@mustUse
function parse(): Result<int32, string> {
    return Result.ok(1)
}

parse()
"#,
        );
        test.result(result).assert_lint("unused-must-use");
    }

    /// Allow @mustUse return values when assigned.
    #[test]
    fn test_allows_used_must_use_call() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_allows_used_must_use_call.ds",
            r#"
@mustUse
function parse(): Result<int32, string> {
    return Result.ok(1)
}

let value = parse()
"#,
        );
        test.result(result).assert_no_lint("unused-must-use");
    }

    /// Flag ignored @mustUse constructor calls.
    #[test]
    fn test_flags_ignored_must_use_constructor() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_flags_ignored_must_use_constructor.ds",
            r#"
@mustUse
class Token {
    value: int32

    constructor(value: int32) {
        this.value = value
    }
}

new Token(1)
"#,
        );
        test.result(result).assert_lint("unused-must-use");
    }

    /// Allow non must-use calls.
    #[test]
    fn test_allows_non_must_use_call() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_allows_non_must_use_call.ds",
            r#"
function ping(): void {
}

ping()
"#,
        );
        test.result(result).assert_no_lint("unused-must-use");
    }
}
