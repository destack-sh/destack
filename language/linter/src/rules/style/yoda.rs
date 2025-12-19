use destack_ast::{self as ast, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{is_comparison_operator, is_literal};
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow "Yoda" conditions.
    ///
    /// Yoda conditions are comparisons where the literal value comes first,
    /// like `"red" === color` instead of `color === "red"`. While syntactically
    /// valid, they can be confusing and are less natural to read.
    #[lint(
        id = "yoda",
        code = "LY035",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub Yoda,
    "Disallow Yoda conditions"
}

impl LintRule for Yoda {
    fn meta(&self) -> &'static crate::LintMeta {
        Yoda::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let Expression::Binary {
                left,
                operator,
                right,
            } = expression
            else {
                continue;
            };

            // only check comparison operators
            if !is_comparison_operator(operator) {
                continue;
            }

            // check for yoda condition: literal on left, non-literal on right
            let left_expression = ctx.tree.get(*left);
            let right_expression = ctx.tree.get(*right);
            if is_literal(left_expression) && !is_literal(right_expression) {
                // make fix: flip comparison
                let expr_span = ctx.tree.get_span(node_id);
                let left_span = ctx.tree.get_span(*left);
                let right_span = ctx.tree.get_span(*right);
                let left_text = ctx.get_span_text(left_span);
                let right_text = ctx.get_span_text(right_span);
                let flipped_op = flip_operator(operator);
                let replacement = format!("{right_text} {flipped_op} {left_text}");
                let edits = ctx
                    .edit_builder()
                    .replace(expr_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Flip comparison").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        YODA.id,
                        YODA.code,
                        YODA.category,
                        severity,
                        "unexpected literal on the left side of comparison",
                        ctx.module.file_id,
                        expr_span,
                    )
                    .with_label("move the literal to the right side")
                    .with_fix(fix),
                );
            }
        }
    }
}

/// Flip a comparison operator for yoda fix (e.g., < becomes >).
fn flip_operator(operator: &BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Equal => "==",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::EqualStrict => "===",
        BinaryOperator::NotEqualStrict => "!==",
        BinaryOperator::LessThan => ">",
        BinaryOperator::LessThanOrEqual => ">=",
        BinaryOperator::GreaterThan => "<",
        BinaryOperator::GreaterThanOrEqual => "<=",
        _ => unreachable!("only called for comparison operators"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_yoda_equality() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if ("red" === color) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_strict_equality() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (5 === x) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_less_than() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (10 < x) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_detects_yoda_null_check() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (null === value) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("yoda");
    }

    #[test]
    fn test_allows_normal_comparison() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (color === "red") {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_variable_comparison() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (a === b) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_literal_to_literal() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (5 === 5) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_allows_non_comparison_operators() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 5 + a;
"#,
        );
        test.result(result).assert_no_lint("yoda");
    }

    #[test]
    fn test_fix_yoda_equality() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (5 === x) { y() }
"#,
        );
        test.result(result).assert_lint("yoda").assert_safe_fixed(
            r#"
if (x === 5) { y() }
"#,
        );
    }

    #[test]
    fn test_fix_yoda_less_than() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (10 < x) { y() }
"#,
        );
        test.result(result).assert_lint("yoda").assert_safe_fixed(
            r#"
if (x > 10) { y() }
"#,
        );
    }

    #[test]
    fn test_fix_yoda_greater_than_or_equal() {
        let test = TestProgram::for_rule(Yoda);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (10 >= x) { y() }
"#,
        );
        test.result(result).assert_lint("yoda").assert_safe_fixed(
            r#"
if (x <= 10) { y() }
"#,
        );
    }
}
