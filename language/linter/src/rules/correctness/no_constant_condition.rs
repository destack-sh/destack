use destack_ast::{Expression, NodeTree};
use destack_source::{FileId, ModuleId};
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
        level = Ast
    )]
    pub NoConstantCondition,
    "Disallow constant expressions in conditions"
}

fn is_constant_expression(tree: &NodeTree, expr: &Expression) -> bool {
    match expr {
        Expression::ScalarLiteral(_) => true,
        Expression::Parenthesized { expression } => {
            let inner = tree.get(*expression);
            is_constant_expression(tree, inner)
        }
        Expression::Unary { right, .. } => {
            let inner = tree.get(*right);
            is_constant_expression(tree, inner)
        }
        _ => false,
    }
}

impl LintRule for NoConstantCondition {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConstantCondition::meta()
    }

    fn check_module_ast(
        &self,
        file_id: FileId,
        _module_id: ModuleId,
        severity: LintSeverity,
        ctx: &mut LintModuleAstContext,
    ) {
        ctx.for_each::<Expression, _>(|tree, _node_id, expression, _span| {
            let condition_id = match expression {
                Expression::If { condition, .. } => Some(*condition),
                Expression::While { condition, .. } => Some(*condition),
                _ => None,
            }?;
            let condition = tree.get(condition_id);
            if is_constant_expression(tree, condition) {
                let condition_span = tree.get_span(condition_id);
                Some(
                    LintDiagnostic::new(
                        NO_CONSTANT_CONDITION.id,
                        NO_CONSTANT_CONDITION.code,
                        NO_CONSTANT_CONDITION.category,
                        severity,
                        "unexpected constant condition",
                        file_id,
                        condition_span,
                    )
                    .with_label("this condition is always the same"),
                )
            } else {
                None
            }
        });
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
