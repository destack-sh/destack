use destack_ast::{self as ast, AssignOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer compound assignment operators.
    ///
    /// Use `x += 1` instead of `x = x + 1` for brevity and clarity.
    #[lint(
        id = "operator-assignment",
        code = "LY016",
        category = Style,
        level = Ast
    )]
    pub OperatorAssignment,
    "Prefer compound assignment operators"
}

impl LintRule for OperatorAssignment {
    fn meta(&self) -> &'static crate::LintMeta {
        OperatorAssignment::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for Assign expressions with plain `=`
            let ast::Expression::Assign {
                left,
                operator: AssignOperator::Assign,
                right,
            } = expr
            else {
                continue;
            };

            // check if left is a path
            let left_expr = ctx.tree.get(*left);
            let ast::Expression::Path {
                path: left_path, ..
            } = left_expr
            else {
                continue;
            };

            // check if right is a binary expression
            let right_expr = ctx.tree.get(*right);
            let ast::Expression::Binary {
                left: bin_left,
                operator,
                right: _,
            } = right_expr
            else {
                continue;
            };

            // check if operator is one that has compound form
            let compound_op = match operator {
                ast::BinaryOperator::Add => Some("+="),
                ast::BinaryOperator::Subtract => Some("-="),
                ast::BinaryOperator::Multiply => Some("*="),
                ast::BinaryOperator::Divide => Some("/="),
                ast::BinaryOperator::Remainder => Some("%="),
                ast::BinaryOperator::ElementwiseAnd => Some("&="),
                ast::BinaryOperator::ElementwiseOr => Some("|="),
                ast::BinaryOperator::ElementwiseXor => Some("^="),
                ast::BinaryOperator::ShiftLeft => Some("<<="),
                ast::BinaryOperator::ShiftRight => Some(">>="),
                ast::BinaryOperator::Exponent => Some("**="),
                _ => None,
            };

            let Some(compound_op) = compound_op else {
                continue;
            };

            // check if left side of binary is same identifier as assignment target
            let bin_left_expr = ctx.tree.get(*bin_left);
            let ast::Expression::Path {
                path: bin_left_path,
                ..
            } = bin_left_expr
            else {
                continue;
            };

            // compare the paths
            if paths_equal(left_path, bin_left_path) {
                ctx.report(
                    LintDiagnostic::new(
                        OPERATOR_ASSIGNMENT.id,
                        OPERATOR_ASSIGNMENT.code,
                        OPERATOR_ASSIGNMENT.category,
                        severity,
                        format!("assignment can be simplified with `{compound_op}`"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label(format!("use `{compound_op}` instead")),
                );
            }
        }
    }
}

fn paths_equal(a: &ast::Path, b: &ast::Path) -> bool {
    if a.segments.len() != b.segments.len() {
        return false;
    }
    // compare segment names
    a.segments
        .iter()
        .zip(b.segments.iter())
        .all(|(seg_a, seg_b)| seg_a == seg_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_add_assignment() {
        let test = TestProgram::for_rule(OperatorAssignment);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 1
x = x + 1
"#,
        );
        test.result(result).assert_lint("operator-assignment");
    }

    #[test]
    fn test_detects_multiply_assignment() {
        let test = TestProgram::for_rule(OperatorAssignment);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 2
x = x * 3
"#,
        );
        test.result(result).assert_lint("operator-assignment");
    }

    #[test]
    fn test_allows_compound_assignment() {
        let test = TestProgram::for_rule(OperatorAssignment);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 1
x += 1
"#,
        );
        test.result(result).assert_no_lint("operator-assignment");
    }

    #[test]
    fn test_allows_different_variable() {
        let test = TestProgram::for_rule(OperatorAssignment);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 1
var y = 2
x = y + 1
"#,
        );
        test.result(result).assert_no_lint("operator-assignment");
    }

    #[test]
    fn test_allows_non_compound_operators() {
        let test = TestProgram::for_rule(OperatorAssignment);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 1
x = x == 1
"#,
        );
        test.result(result).assert_no_lint("operator-assignment");
    }
}
