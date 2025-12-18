use destack_ast::{self as ast, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require `Number.isNaN()` instead of comparisons with `NaN`.
    ///
    /// `NaN` is unique in that it is not equal to itself. Comparisons like
    /// `x === NaN` or `x !== NaN` will never work as expected. Use
    /// `Number.isNaN(x)` instead.
    #[lint(
        id = "use-isnan",
        code = "LC019",
        category = Correctness,
        level = Ast
    )]
    pub UseIsnan,
    "Require Number.isNaN() instead of NaN comparisons"
}

impl LintRule for UseIsnan {
    fn meta(&self) -> &'static crate::LintMeta {
        UseIsnan::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check binary comparison expressions
            let Expression::Binary {
                left,
                operator,
                right,
            } = expression
            else {
                continue;
            };

            // only check comparison operators
            if !matches!(
                operator,
                BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::EqualStrict
                    | BinaryOperator::NotEqualStrict
                    | BinaryOperator::LessThan
                    | BinaryOperator::LessThanOrEqual
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterThanOrEqual
            ) {
                continue;
            }

            // check if either side is NaN
            let left_is_nan = is_nan_identifier(ctx, *left);
            let right_is_nan = is_nan_identifier(ctx, *right);
            if left_is_nan || right_is_nan {
                ctx.report(
                    LintDiagnostic::new(
                        USE_ISNAN.id,
                        USE_ISNAN.code,
                        USE_ISNAN.category,
                        severity,
                        "comparison with NaN is always false",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use Number.isNaN() instead"),
                );
            }
        }
    }
}

/// Check if an expression is the identifier `NaN` or `Number.NaN`
fn is_nan_identifier(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expr_id);

    match expression {
        // simple identifier: NaN
        Expression::Path {
            path,
            static_arguments: None,
        } => {
            if path.segments.len() == 1 {
                let name = ctx.strings.get(path.segments[0]);
                return name.as_ref() == "NaN";
            }
            // Number.NaN
            if path.segments.len() == 2 {
                let first = ctx.strings.get(path.segments[0]);
                let second = ctx.strings.get(path.segments[1]);
                return first.as_ref() == "Number" && second.as_ref() == "NaN";
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nan_strict_equal() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.0
if (x === NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_nan_equal() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.0
if (x == NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_nan_not_equal() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.0
if (x !== NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_nan_less_than() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.0
if (x < NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_nan_on_left() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.0
if (NaN === x) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_number_nan() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.0
if (x === Number.NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_allows_normal_comparison() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.0
if (x === 0.0) {}
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_allows_isnan_call() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1.0
if (Number.isNaN(x)) {}
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_allows_nan_variable_name() {
        let test = TestProgram::for_rule(UseIsnan);
        let result = test.lint_ast(
            "test.ds",
            r#"
let NaN = "not a number"
let x = NaN
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }
}
