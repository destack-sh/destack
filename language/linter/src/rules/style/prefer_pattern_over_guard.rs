use destack_ast::{self as ast, BinaryOperator, Expression, MatchCase, Pattern};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest moving match guards into the pattern.
    ///
    /// Some match guards can be more clearly expressed as part of the pattern
    /// itself. For example, guards that compare a bound variable to a literal
    /// can often be replaced with a literal pattern.
    ///
    /// ```
    /// // bad
    /// match x {
    ///     n if n == 1 => doX()
    ///     _ => doY()
    /// }
    ///
    /// // good
    /// match x {
    ///     1 => doX()
    ///     _ => doY()
    /// }
    /// ```
    #[lint(
        id = "prefer-pattern-over-guard",
        code = "LY056",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferPatternOverGuard,
    "Prefer pattern over match guard"
}

impl LintRule for PreferPatternOverGuard {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferPatternOverGuard::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::MatchCase>() {
            let match_case = ctx.tree.get(node_id);

            let selector = match match_case {
                MatchCase::Expression { selector, .. } => selector,
                MatchCase::Block { selector, .. } => selector,
            };

            // only check pattern selectors with guards
            let ast::MatchSelector::Pattern { pattern: pattern_id, guard } = selector else {
                continue;
            };
            let Some(guard_expr_id) = guard else {
                continue;
            };

            // check if the pattern is a simple binding and the guard compares it to a literal
            let pattern = ctx.tree.get(*pattern_id);
            let guard_expression = ctx.tree.get(*guard_expr_id);
            if let Some(binding_name) = get_simple_binding_name(ctx, pattern)
                && is_equality_with_literal(ctx, guard_expression, &binding_name)
            {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        PREFER_PATTERN_OVER_GUARD.id,
                        PREFER_PATTERN_OVER_GUARD.code,
                        PREFER_PATTERN_OVER_GUARD.category,
                        severity,
                        "guard comparing binding to literal can be a pattern",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("replace with literal pattern"),
                );
            }
        }
    }
}

/// Get the name of a simple binding pattern, if it is one.
fn get_simple_binding_name(ctx: &LintModuleAstContext<'_>, pattern: &Pattern) -> Option<String> {
    match pattern {
        Pattern::Binding {
            name,
            pattern: None,
            ..
        } => Some(ctx.strings.get(*name).to_string()),
        _ => None,
    }
}

/// Check if the guard is an equality comparison of the binding to a literal.
fn is_equality_with_literal(
    ctx: &LintModuleAstContext<'_>,
    guard: &Expression,
    binding_name: &str,
) -> bool {
    let Expression::Binary {
        operator,
        left,
        right,
    } = guard
    else {
        return false;
    };

    // check for == or === (strict equals)
    if !matches!(
        operator,
        BinaryOperator::Equal | BinaryOperator::EqualStrict
    ) {
        return false;
    }

    let left_expression = ctx.tree.get(*left);
    let right_expression = ctx.tree.get(*right);

    // check if one side is the binding and the other is a literal
    let left_is_binding = is_path_with_name(ctx, left_expression, binding_name);
    let right_is_binding = is_path_with_name(ctx, right_expression, binding_name);
    let left_is_literal = is_simple_literal(left_expression);
    let right_is_literal = is_simple_literal(right_expression);

    (left_is_binding && right_is_literal) || (right_is_binding && left_is_literal)
}

/// Check if an expression is a simple path with the given name.
fn is_path_with_name(ctx: &LintModuleAstContext<'_>, expression: &Expression, name: &str) -> bool {
    let Expression::Path { path, .. } = expression else {
        return false;
    };

    if path.segments.len() != 1 {
        return false;
    }

    let path_name = ctx.strings.get(path.segments[0]);
    path_name.as_ref() == name
}

/// Check if an expression is a simple literal (number, string, char, bool).
fn is_simple_literal(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ScalarLiteral(_) | Expression::TypeLiteral(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_equality_guard_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n == 1 => doX()
        _ => doY()
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_strict_equality_guard_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n === 1 => doX()
        _ => doY()
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_reversed_equality_guard_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        n if 1 == n => doX()
        _ => doY()
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_literal_pattern_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        1 => doX()
        _ => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_complex_guard_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n > 0 && n < 10 => doX()
        _ => doY()
    }
}
"#,
        );
        // complex guards can't easily be expressed as patterns
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_guard_with_different_var_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32, y: int32) {
    match x {
        n if y == 1 => doX()
        _ => doY()
    }
}
"#,
        );
        // guard uses different variable
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_guard_with_method_call_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: string) {
    match x {
        s if s.isEmpty() => doX()
        _ => doY()
    }
}
"#,
        );
        // method call guards can't be patterns
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_no_guard_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        n => doX(n)
        _ => doY()
    }
}
"#,
        );
        // no guard at all
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_inequality_guard_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferPatternOverGuard);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n != 1 => doX()
        _ => doY()
    }
}
"#,
        );
        // inequality is not as simple to express as pattern
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }
}
