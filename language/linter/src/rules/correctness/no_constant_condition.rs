use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow constant expressions in conditions.
    ///
    /// Using a constant expression like `if (true)` or `while (false)` is
    /// usually a mistake. If intentional, consider restructuring the code.
    #[lint(
        id = "no-constant-condition",
        code = "LC003",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoConstantCondition,
    "Disallow constant expressions in conditions"
}

fn is_constant_expression(tree: &ast::NodeTree, expr: &ast::Expression) -> bool {
    match expr {
        ast::Expression::ScalarLiteral(_) => true,
        ast::Expression::Parenthesized { expression } => {
            is_constant_expression(tree, tree.get(*expression))
        }
        ast::Expression::Unary { right, .. } => is_constant_expression(tree, tree.get(*right)),
        _ => false,
    }
}

impl LintRule for NoConstantCondition {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConstantCondition::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let condition_id = match ctx.tree.get(node_id) {
                ast::Expression::If { condition, .. } => *condition,
                ast::Expression::While { condition, .. } => *condition,
                _ => continue,
            };
            let condition = ctx.tree.get(condition_id);
            if is_constant_expression(ctx.tree, condition) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_CONSTANT_CONDITION.id,
                        NO_CONSTANT_CONDITION.code,
                        NO_CONSTANT_CONDITION.category,
                        severity,
                        "unexpected constant condition",
                        ctx.module.file_id,
                        ctx.tree.get_span(condition_id),
                    )
                    .with_label("this condition is always the same"),
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
    fn test_detects_if_true() {
        let test = TestProgram::for_rule(NoConstantCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (true) { foo(); }
"#,
        );
        test.result(result).assert_lint("no-constant-condition");
    }

    #[test]
    fn test_detects_if_false() {
        let test = TestProgram::for_rule(NoConstantCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (false) { foo(); }
"#,
        );
        test.result(result).assert_lint("no-constant-condition");
    }

    #[test]
    fn test_detects_while_true() {
        let test = TestProgram::for_rule(NoConstantCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (true) { foo(); }
"#,
        );
        test.result(result).assert_lint("no-constant-condition");
    }

    #[test]
    fn test_detects_if_number() {
        let test = TestProgram::for_rule(NoConstantCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (1) { foo(); }
"#,
        );
        test.result(result).assert_lint("no-constant-condition");
    }

    #[test]
    fn test_no_constant_with_variable() {
        let test = TestProgram::for_rule(NoConstantCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x) { foo(); }
"#,
        );
        test.result(result).assert_no_lint("no-constant-condition");
    }

    #[test]
    fn test_no_constant_with_comparison() {
        let test = TestProgram::for_rule(NoConstantCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x > 0) { foo(); }
"#,
        );
        test.result(result).assert_no_lint("no-constant-condition");
    }
}
