use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expressions_equal;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow identical if/else branches.
    ///
    /// When the if and else branches have identical code, the conditional
    /// is pointless and indicates a copy-paste error or unfinished logic.
    #[lint(
        id = "no-identical-branches",
        code = "LU025",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoIdenticalBranches,
    "Disallow identical if/else branches"
}

impl LintRule for NoIdenticalBranches {
    fn meta(&self) -> &'static crate::LintMeta {
        NoIdenticalBranches::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::If {
                then_expression,
                else_expression: Some(else_expression),
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // check if then and else are identical
            if expressions_equal(ctx, *then_expression, *else_expression) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_IDENTICAL_BRANCHES.id,
                        NO_IDENTICAL_BRANCHES.code,
                        NO_IDENTICAL_BRANCHES.category,
                        severity,
                        "identical if and else branches",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("these branches have the same code"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_identical_branches() {
        let test = TestProgram::for_rule(NoIdenticalBranches);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x) {
    doSomething();
} else {
    doSomething();
}
"#,
        );
        test.result(result).assert_lint("no-identical-branches");
    }

    #[test]
    fn test_allows_different_branches() {
        let test = TestProgram::for_rule(NoIdenticalBranches);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x) {
    doA();
} else {
    doB();
}
"#,
        );
        test.result(result).assert_no_lint("no-identical-branches");
    }

    #[test]
    fn test_allows_if_without_else() {
        let test = TestProgram::for_rule(NoIdenticalBranches);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x) {
    doSomething();
}
"#,
        );
        test.result(result).assert_no_lint("no-identical-branches");
    }

    #[test]
    fn test_detects_identical_ternary() {
        let test = TestProgram::for_rule(NoIdenticalBranches);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = cond ? value : value;
"#,
        );
        test.result(result).assert_lint("no-identical-branches");
    }
}
