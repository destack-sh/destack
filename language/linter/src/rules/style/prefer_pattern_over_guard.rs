use destack_dir as dir;
use destack_repository::LintSeverity;
use destack_source::Span;

use crate::rules::common::{
    expression_target_symbol, expression_unwrap_parenthesized, pattern_binding_name_and_symbol,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Suggest moving match guards into the pattern.
    ///
    /// Some match guards can be more clearly expressed as part of the pattern
    /// itself. For example, guards that compare a bound variable to a literal
    /// can often be replaced with a literal pattern.
    #[lint(
        id = "prefer-pattern-over-guard",
        code = "LY049",
        category = Style,
        level = Dir,
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
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferPatternOverGuard::meta()
    }

    /// Check module DIR match cases for literal equality guards.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect match cases with pattern guards
        for match_case_id in ctx.dir.iter_node_ids_of_type::<dir::MatchCase>() {
            let match_case = ctx.dir.get(match_case_id);
            let selector = match_case.selector();
            let Some(pattern_id) = selector.pattern_id() else {
                continue;
            };
            let Some(guard_expression_id) = selector.guard_expression_id() else {
                continue;
            };

            // keep simple binding patterns whose guard compares the same symbol to one literal
            let Some((_, binding_symbol)) =
                pattern_binding_name_and_symbol(ctx.dir.tree(), &ctx.symbols, pattern_id)
            else {
                continue;
            };
            let Some(literal_expression_id) = equality_literal_for_binding(
                ctx,
                binding_symbol.into_global(ctx.module_id()),
                guard_expression_id,
            ) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, match_case_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintReport::new(
                PREFER_PATTERN_OVER_GUARD.id,
                PREFER_PATTERN_OVER_GUARD.code,
                PREFER_PATTERN_OVER_GUARD.category,
                severity,
                "guard comparing binding to literal can be a pattern",
                ctx.get_span(match_case_id),
            )
            .label("replace with literal pattern");

            // attach the rewrite only when the source still contains one explicit guard separator
            if ctx.compute_fixes
                && let Some(fix) = prefer_pattern_over_guard_fix(
                    ctx,
                    pattern_id,
                    guard_expression_id,
                    literal_expression_id,
                )
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return the literal expression when one guard compares the binding symbol to one literal.
fn equality_literal_for_binding(
    ctx: &LintModuleContext<'_>,
    binding_symbol: dir::GlobalSymbolId,
    guard_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let guard_expression_id = expression_unwrap_parenthesized(ctx.dir.tree(), guard_expression_id);
    let guard_expression = ctx.dir.get(guard_expression_id);
    let dir::Expression::Binary {
        operator,
        left,
        right,
    } = guard_expression
    else {
        return None;
    };

    // keep only equality guards that can become literal patterns
    if !matches!(
        operator,
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
    ) {
        return None;
    }

    let left_id = expression_unwrap_parenthesized(ctx.dir.tree(), *left);
    let right_id = expression_unwrap_parenthesized(ctx.dir.tree(), *right);
    let left_is_binding = expression_target_symbol(ctx, left_id) == Some(binding_symbol);
    let right_is_binding = expression_target_symbol(ctx, right_id) == Some(binding_symbol);
    let left_is_literal = expression_is_simple_literal(ctx.dir.tree(), left_id);
    let right_is_literal = expression_is_simple_literal(ctx.dir.tree(), right_id);

    // keep exactly one binding side and one literal side
    if left_is_binding && right_is_literal {
        return Some(right_id);
    }
    if right_is_binding && left_is_literal {
        return Some(left_id);
    }

    None
}

/// Return true when one expression is a simple literal pattern candidate.
fn expression_is_simple_literal(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);

    matches!(
        expression,
        dir::Expression::ScalarLiteral(_) | dir::Expression::Type { .. }
    )
}

/// Build a safe guard-to-pattern rewrite fix.
fn prefer_pattern_over_guard_fix(
    ctx: &LintModuleContext<'_>,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    guard_expression_id: dir::LocalNodeId<dir::Expression>,
    literal_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let pattern_span = ctx.get_span(pattern_id);
    let guard_span = ctx.get_span(guard_expression_id);
    if guard_span.end <= pattern_span.end {
        return None;
    }

    // keep one explicit `if` separator in the source window
    let separator_span = Span::new(pattern_span.file, pattern_span.end, guard_span.start);
    let separator_text = ctx.get_span_text(separator_span);
    if !separator_text.contains("if") {
        return None;
    }

    // replace the binding pattern and drop the trailing guard text
    let literal_text = ctx.get_span_text(ctx.get_span(literal_expression_id));
    let edits = ctx
        .edit_builder()
        .replace(pattern_span, literal_text)
        .replace(
            Span::new(pattern_span.file, pattern_span.end, guard_span.end),
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
    fn test_detects_equality_guard() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_detects_equality_guard.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n == 1 => 1
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("prefer-pattern-over-guard")
            .assert_safe_fixed(
                r#"
function foo(x: int32) {
    match (x) {
        1 => 1
        _ => 0
    }
}
"#,
            );
    }

    #[test]
    fn test_detects_strict_equality_guard() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_detects_strict_equality_guard.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n === 1 => 1
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_detects_reversed_equality_guard() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_detects_reversed_equality_guard.ds",
            r#"
function foo(x: int32) {
    match x {
        n if 1 == n => 1
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("prefer-pattern-over-guard")
            .assert_safe_fixed(
                r#"
function foo(x: int32) {
    match (x) {
        1 => 1
        _ => 0
    }
}
"#,
            );
    }

    #[test]
    fn test_allows_literal_pattern() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_allows_literal_pattern.ds",
            r#"
function foo(x: int32) {
    match x {
        1 => 1
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_allows_complex_guard() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_allows_complex_guard.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n > 0 && n < 10 => 1
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_allows_guard_with_different_variable() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_allows_guard_with_different_variable.ds",
            r#"
function foo(x: int32, y: int32) {
    match x {
        n if y == 1 => 1
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_allows_non_simple_guard_expression() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_allows_non_simple_guard_expression.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n + 1 == 2 => 1
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_allows_case_without_guard() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_allows_case_without_guard.ds",
            r#"
function foo(x: int32) {
    match x {
        n => n
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }

    #[test]
    fn test_allows_inequality_guard() {
        let test = TestProgram::for_rule_without_prelude(PreferPatternOverGuard);
        let result = test.lint_dir(
            "prefer_pattern_over_guard/test_allows_inequality_guard.ds",
            r#"
function foo(x: int32) {
    match x {
        n if n != 1 => 1
        _ => 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("prefer-pattern-over-guard");
    }
}
