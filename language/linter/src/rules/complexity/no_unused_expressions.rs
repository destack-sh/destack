use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::has_side_effects;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow expressions that have no effect.
    ///
    /// Expressions that are evaluated but never used are likely mistakes or
    /// leftover code. This rule flags expressions that don't produce side effects
    /// and whose values are discarded.
    ///
    /// bad:
    /// ```
    /// x + 1;  // computed but not used
    /// "hello";  // string literal with no effect
    /// a && b;  // result discarded
    /// ```
    ///
    /// good:
    /// ```
    /// let y = x + 1;  // value is used
    /// console.log("hello");  // has side effect
    /// if (a && b) { ... }  // used in condition
    /// ```
    #[lint(
        id = "no-unused-expressions",
        code = "LX011",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUnusedExpressions,
    "Disallow expressions without effect"
}

impl LintRule for NoUnusedExpressions {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnusedExpressions::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // check each root expression in the module
        for &root_id in ctx.roots {
            check_expression_statement(ctx, meta, root_id);
        }

        // also check expressions inside blocks
        for node_id in ctx.tree.iter_nodes::<ast::Block>() {
            let block = ctx.tree.get(node_id);
            for &child_id in &block.expressions {
                check_expression_statement(ctx, meta, child_id);
            }
        }
    }
}

/// Check if an expression statement is unused and report it.
fn check_expression_statement(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    expr_id: ast::LocalNodeId<ast::Expression>,
) {
    // skip expressions that have side effects or are useful
    if !has_side_effects(ctx, expr_id) {
        let severity = ctx.get_effective_severity(meta, expr_id);
        if !severity.is_enabled() {
            return;
        }

        ctx.report(
            LintDiagnostic::new(
                NO_UNUSED_EXPRESSIONS.id,
                NO_UNUSED_EXPRESSIONS.code,
                NO_UNUSED_EXPRESSIONS.category,
                severity,
                "expression has no effect",
                ctx.module.file_id,
                ctx.tree.get_span(expr_id),
            )
            .with_label("this expression does nothing"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_unused_literal() {
        let test = TestProgram::for_rule_without_builtins(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds", r#"
5
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_string() {
        let test = TestProgram::for_rule_without_builtins(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
"hello"
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_binary() {
        let test = TestProgram::for_rule_without_builtins(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
x + 1
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_identifier() {
        let test = TestProgram::for_rule_without_builtins(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds", r#"
x
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_function_call() {
        let test = TestProgram::for_rule_without_builtins(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
doSomething()
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_assignment() {
        let test = TestProgram::for_rule_without_builtins(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
x = 5
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_let_binding() {
        let test = TestProgram::for_rule_without_builtins(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 5
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_return() {
        let test = TestProgram::for_rule_without_builtins(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    return 5
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }
}
