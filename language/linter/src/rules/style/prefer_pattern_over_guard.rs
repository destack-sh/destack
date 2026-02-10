use destack_ast::{self as ast, BinaryOperator, Expression, MatchCase, Pattern};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

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
        code = "LY049",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
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
            let ast::MatchSelector::Pattern {
                pattern: pattern_id,
                guard,
            } = selector
            else {
                continue;
            };
            let Some(guard_expr_id) = guard else {
                continue;
            };

            // check if the pattern is a simple binding and the guard compares it to a literal
            let pattern = ctx.tree.get(*pattern_id);
            let guard_expression = ctx.tree.get(*guard_expr_id);
            if let Some(binding_name) = get_simple_binding_name(pattern)
                && let Some(literal_expression_id) =
                    equality_literal_for_binding(ctx, guard_expression, binding_name)
            {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    PREFER_PATTERN_OVER_GUARD.id,
                    PREFER_PATTERN_OVER_GUARD.code,
                    PREFER_PATTERN_OVER_GUARD.category,
                    severity,
                    "guard comparing binding to literal can be a pattern",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("replace with literal pattern");
                if ctx.compute_fixes
                    && let Some(fix) = prefer_pattern_over_guard_fix(
                        ctx,
                        *pattern_id,
                        *guard_expr_id,
                        literal_expression_id,
                    )
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Get the name of a simple binding pattern, if it is one.
fn get_simple_binding_name(pattern: &Pattern) -> Option<ast::StringId> {
    match pattern {
        Pattern::Binding {
            name,
            pattern: None,
            ..
        } => Some(*name),
        _ => None,
    }
}

/// Return the literal expression when the guard is `binding == literal` or `literal == binding`.
fn equality_literal_for_binding(
    ctx: &LintModuleAstContext<'_>,
    guard: &Expression,
    binding_name: ast::StringId,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let Expression::Binary {
        operator,
        left,
        right,
    } = guard
    else {
        return None;
    };

    // check for == or === (strict equals)
    if !matches!(
        operator,
        BinaryOperator::Equal | BinaryOperator::EqualStrict
    ) {
        return None;
    }

    let left_expression = ctx.tree.get(*left);
    let right_expression = ctx.tree.get(*right);

    // keep one binding side and one literal side
    let left_is_binding = is_path_with_name(ctx, left_expression, binding_name);
    let right_is_binding = is_path_with_name(ctx, right_expression, binding_name);
    let left_is_literal = is_simple_literal(left_expression);
    let right_is_literal = is_simple_literal(right_expression);

    if left_is_binding && right_is_literal {
        return Some(*right);
    }
    if right_is_binding && left_is_literal {
        return Some(*left);
    }

    None
}

/// Check if an expression is a simple path with the given name.
fn is_path_with_name(
    _ctx: &LintModuleAstContext<'_>,
    expression: &Expression,
    name: ast::StringId,
) -> bool {
    let Expression::Path { path, .. } = expression else {
        return false;
    };

    if path.segments.len() != 1 {
        return false;
    }

    path.segments[0] == name
}

/// Check if an expression is a simple literal (number, string, char, bool).
fn is_simple_literal(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ScalarLiteral(_) | Expression::TypeLiteral(_)
    )
}

/// Build a safe guard-to-pattern rewrite fix.
fn prefer_pattern_over_guard_fix(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
    guard_expression_id: ast::LocalNodeId<ast::Expression>,
    literal_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let pattern_span = ctx.tree.get_span(pattern_id);
    let guard_span = ctx.tree.get_span(guard_expression_id);
    if guard_span.end <= pattern_span.end {
        return None;
    }

    // keep source with an explicit `if` guard separator
    let separator_span =
        destack_source::Span::new(pattern_span.file, pattern_span.end, guard_span.start);
    let separator_text = ctx.get_span_text(separator_span);
    if !separator_text.contains("if") {
        return None;
    }

    // replace pattern with the literal and drop trailing `if <guard>`
    let literal_text = ctx.get_span_text(ctx.tree.get_span(literal_expression_id));
    let edits = ctx
        .edit_builder()
        .replace(pattern_span, literal_text)
        .replace(
            destack_source::Span::new(pattern_span.file, pattern_span.end, guard_span.end),
            "",
        )
        .into_edits();
    Some(LintFix::safe("Rewrite guard equality as literal pattern").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_equality_guard_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_equality_guard_detected.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n == 1 => doX()
        _ => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-pattern-over-guard")
            .assert_safe_fixed(
                r#"
function foo(x: int32) {
    match (x) {
        1 => doX()
        _ => doY()
    }
}
"#,
            );
    }

    #[test]
    fn test_strict_equality_guard_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_strict_equality_guard_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_reversed_equality_guard_detected.ds",
            r#"
function foo(x: int32) {
    match x {
        n if 1 == n => doX()
        _ => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-pattern-over-guard")
            .assert_safe_fixed(
                r#"
function foo(x: int32) {
    match (x) {
        1 => doX()
        _ => doY()
    }
}
"#,
            );
    }

    #[test]
    fn test_literal_pattern_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_literal_pattern_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_complex_guard_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_guard_with_different_var_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_guard_with_method_call_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_no_guard_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_ast(
            "prefer_pattern_over_guard/test_inequality_guard_allowed.ds",
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
