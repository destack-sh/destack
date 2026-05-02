use crate::LintMeta;
use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_path_segments, expression_statement_span};
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow assertions on constant values.
    ///
    /// Assertions with constant values are either always true (pointless) or
    /// always false (indicating dead code or a mistake).
    ///
    /// ```
    /// // bad
    /// assert(true)          // always passes, pointless
    /// assert(false)         // always fails, likely a mistake
    /// console.assert(1)     // always passes
    /// console.assert("")    // always fails (empty string is falsy)
    ///
    /// // good
    /// assert(value != null)
    /// assert(isValid())
    /// console.assert(x > 0)
    /// ```
    #[lint(
        id = "no-constant-assertion",
        code = "LU006",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoConstantAssertion,
    "Disallow assertions on constant values"
}

impl LintRule for NoConstantAssertion {
    fn meta(&self) -> &'static LintMeta {
        NoConstantAssertion::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let Expression::Call {
                left, arguments, ..
            } = expression
            else {
                continue;
            };

            // check if calling an assert function
            if !is_assert_call(ctx, *left) {
                continue;
            }

            // check if the first argument is a constant
            let Some(first_arg_id) = arguments.first() else {
                continue;
            };

            let first_arg = ctx.tree.get(*first_arg_id);
            let arg_value = match first_arg {
                ast::Argument::Positional { value, .. } | ast::Argument::Labeled { value, .. } => {
                    *value
                }
                _ => continue,
            };

            let Some(constant_truthiness) = get_constant_truthiness(ctx, arg_value) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let (message, label) = if constant_truthiness {
                (
                    "assertion with constant `true` value is always passing",
                    "this assertion is pointless",
                )
            } else {
                (
                    "assertion with constant `false` value always fails",
                    "this assertion will always fail",
                )
            };

            let mut diagnostic = LintReport::new(
                NO_CONSTANT_ASSERTION.id,
                NO_CONSTANT_ASSERTION.code,
                NO_CONSTANT_ASSERTION.category,
                severity,
                message,
                ctx.tree.get_span(node_id),
            )
            .label(label);

            // compute fixes only when requested by the runner
            if ctx.compute_fixes
                && constant_truthiness
                && let Some(fix) = constant_true_assertion_fix(ctx, node_id, arg_value)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a safe fix by removing one standalone constant-true assertion statement.
fn constant_true_assertion_fix(
    ctx: &LintAstContext<'_>,
    call_expression_id: ast::LocalNodeId<Expression>,
    argument_expression_id: ast::LocalNodeId<Expression>,
) -> Option<LintFix> {
    let argument_expression = ctx.tree.get(argument_expression_id);
    if !matches!(
        argument_expression,
        Expression::ScalarLiteral(ScalarLiteral::Boolean(true))
    ) {
        return None;
    }

    let statement_span = expression_statement_span(ctx.tree, ctx.parents, call_expression_id)?;
    let edits = ctx.edit_builder().delete(statement_span).into_edits();
    Some(LintFix::suggestion("Remove constant-true assertion").with_edits(edits))
}

/// Check if an expression is a call to an assert function.
fn is_assert_call(ctx: &LintAstContext<'_>, callee_id: ast::LocalNodeId<Expression>) -> bool {
    let callee = ctx.tree.get(callee_id);

    match callee {
        // direct or qualified call: assert(...) or Debug.assert(...)
        Expression::Identifier { .. }
        | Expression::QualifiedReference { .. }
        | Expression::Member { .. } => {
            let Some(path_segments) = expression_path_segments(ctx.tree, callee_id) else {
                return false;
            };

            if let Some(last_segment) = path_segments.last() {
                let name = ctx.strings.get(*last_segment);
                matches!(name.as_ref(), "assert" | "ok" | "strictEqual" | "deepEqual")
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Get the constant truthiness of an expression, if it can be determined at compile time.
fn get_constant_truthiness(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
) -> Option<bool> {
    let expression = ctx.tree.get(expression_id);

    match expression {
        // boolean literals
        Expression::ScalarLiteral(ScalarLiteral::Boolean(value)) => Some(*value),

        // numeric literals: 0 is falsy, non-zero is truthy
        Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => Some(*value != 0),
        Expression::ScalarLiteral(ScalarLiteral::Float(value)) => Some(*value != 0.0),
        Expression::ScalarLiteral(ScalarLiteral::Bigint(value)) => Some(*value != 0),

        // string literals: empty string is falsy, non-empty is truthy
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            let s = ctx.strings.get(*string_id);
            Some(!s.is_empty())
        }

        // null is falsy
        Expression::ScalarLiteral(ScalarLiteral::Null) => Some(false),

        // empty optional sentinel types are falsy
        Expression::Type {
            value: type_expression_id,
        } => {
            let type_expression = ctx.tree.get(*type_expression_id);
            match type_expression {
                ast::TypeExpression::Literal {
                    value: ast::TypeLiteral::Null | ast::TypeLiteral::Undefined,
                } => Some(false),
                _ => None,
            }
        }

        // parenthesized: unwrap
        Expression::Parenthesized { expression } => get_constant_truthiness(ctx, *expression),

        // empty array literal is truthy (arrays are objects)
        Expression::ArrayExpression { elements } if elements.is_empty() => Some(true),

        // empty object literal is truthy
        Expression::ObjectExpression { properties, .. } if properties.is_empty() => Some(true),

        // empty tuple is truthy
        Expression::TupleExpression { elements } if elements.is_empty() => Some(true),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_assert_true_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_true_detected.ds",
            r#"
assert(true)
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_fix_removes_assert_true_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_fix_removes_assert_true_statement.ds",
            r#"
assert(true)
const value = 1
"#,
        );
        test.result(result)
            .assert_lint("no-constant-assertion")
            .assert_suggested_fixed(
                r#"
const value = 1;
"#,
            );
    }

    #[test]
    fn test_assert_false_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_false_detected.ds",
            r#"
assert(false)
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_no_fix_for_assert_false() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_no_fix_for_assert_false.ds",
            r#"
assert(false)
"#,
        );
        test.result(result)
            .assert_lint("no-constant-assertion")
            .assert_has_no_fix("no-constant-assertion");
    }

    #[test]
    fn test_console_assert_true_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_console_assert_true_detected.ds",
            r#"
console.assert(true)
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_mutation_fix_removes_console_assert_true_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_mutation_fix_removes_console_assert_true_statement.ds",
            r#"
console.assert(true)
run()
"#,
        );
        test.result(result)
            .assert_lint("no-constant-assertion")
            .assert_suggested_fixed(
                r#"
run();
"#,
            );
    }

    #[test]
    fn test_console_assert_false_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_console_assert_false_detected.ds",
            r#"
console.assert(false)
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_assert_zero_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_zero_detected.ds",
            r#"
assert(0)
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_assert_nonzero_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_nonzero_detected.ds",
            r#"
assert(1)
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_assert_empty_string_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_empty_string_detected.ds",
            r#"
assert("")
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_assert_non_empty_string_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_non_empty_string_detected.ds",
            r#"
assert("hello")
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_assert_null_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_null_detected.ds",
            r#"
assert(null)
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_assert_variable_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_variable_allowed.ds",
            r#"
assert(isValid)
"#,
        );
        test.result(result).assert_no_lint("no-constant-assertion");
    }

    #[test]
    fn test_assert_comparison_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_comparison_allowed.ds",
            r#"
assert(x > 0)
"#,
        );
        test.result(result).assert_no_lint("no-constant-assertion");
    }

    #[test]
    fn test_assert_function_call_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_assert_function_call_allowed.ds",
            r#"
assert(validate())
"#,
        );
        test.result(result).assert_no_lint("no-constant-assertion");
    }

    #[test]
    fn test_non_assert_call_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_non_assert_call_allowed.ds",
            r#"
log(true)
"#,
        );
        test.result(result).assert_no_lint("no-constant-assertion");
    }

    #[test]
    fn test_debug_assert_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_debug_assert_detected.ds",
            r#"
Debug.assert(true)
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }

    #[test]
    fn test_parenthesized_true_detected() {
        let test = TestProgram::for_rule_without_prelude(NoConstantAssertion);
        let result = test.lint_ast(
            "no_constant_assertion/test_parenthesized_true_detected.ds",
            r#"
assert((true))
"#,
        );
        test.result(result).assert_lint("no-constant-assertion");
    }
}
