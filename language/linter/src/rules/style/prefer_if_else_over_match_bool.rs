use crate::LintMeta;
use destack_dir::{self as dir, Expression, MatchCase, Pattern, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Suggest using if/else instead of match on booleans.
    ///
    /// Matching on boolean values with `true` and `false` arms is more
    /// idiomatically expressed as an if/else statement.
    ///
    /// ```
    /// // bad
    /// match (condition) {
    ///     true => doX()
    ///     false => doY()
    /// }
    ///
    /// // good
    /// if (condition) {
    ///     doX()
    /// } else {
    ///     doY()
    /// }
    /// ```
    #[lint(
        id = "prefer-if-else-over-match-bool",
        code = "LY040",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferIfElseOverMatchBool,
    "Prefer if/else over match on boolean"
}

impl LintRule for PreferIfElseOverMatchBool {
    fn meta(&self) -> &'static LintMeta {
        PreferIfElseOverMatchBool::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);

            let dir::Expression::Match { value, cases, .. } = expression else {
                continue;
            };

            // look for exactly 2 cases
            if cases.len() != 2 {
                continue;
            }

            // check if we have true/false or false/true patterns
            let first_pattern = get_case_pattern(ctx, cases[0]);
            let second_pattern = get_case_pattern(ctx, cases[1]);

            let is_bool_match = match (first_pattern, second_pattern) {
                (Some(first_bool), Some(second_bool)) => first_bool != second_bool,
                _ => false,
            };

            if !is_bool_match {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintReport::new(
                PREFER_IF_ELSE_OVER_MATCH_BOOL.id,
                PREFER_IF_ELSE_OVER_MATCH_BOOL.code,
                PREFER_IF_ELSE_OVER_MATCH_BOOL.category,
                severity,
                "use `if/else` instead of `match` on boolean",
                ctx.dir.get_span(node_id),
            )
            .label("replace with if/else");
            if ctx.compute_fixes
                && let Some(fix) = prefer_if_else_over_match_bool_fix(ctx, node_id, *value, cases)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Get the boolean literal value from a match case pattern, if it is one.
fn get_case_pattern(
    ctx: &LintModuleContext<'_>,
    case_id: dir::LocalNodeId<MatchCase>,
) -> Option<bool> {
    let case = ctx.dir.get(case_id);

    let selector = match case {
        MatchCase::Block { selector, .. } | MatchCase::Expression { selector, .. } => selector,
    };

    let dir::MatchSelector::Pattern {
        pattern: pattern_id,
        guard,
    } = selector
    else {
        return None;
    };
    if guard.is_some() {
        return None;
    }

    let pattern = ctx.dir.get(*pattern_id);

    // check if it's an expression pattern with a boolean literal
    let Pattern::Expression { value } = pattern else {
        return None;
    };

    let expression = ctx.dir.get(*value);

    match expression {
        Expression::ScalarLiteral(ScalarLiteral::Boolean(b)) => Some(*b),
        _ => None,
    }
}

/// Build a safe rewrite from `match bool` to `if/else` for expression arm bodies.
fn prefer_if_else_over_match_bool_fix(
    ctx: &LintModuleContext<'_>,
    match_expression_id: dir::LocalNodeId<Expression>,
    value_expression_id: dir::LocalNodeId<Expression>,
    case_ids: &[dir::LocalNodeId<MatchCase>],
) -> Option<LintFix> {
    // keep exactly two expression-body boolean cases
    if case_ids.len() != 2 {
        return None;
    }

    let (first_bool, first_body_id) = bool_expression_case_body(ctx, case_ids[0])?;
    let (second_bool, second_body_id) = bool_expression_case_body(ctx, case_ids[1])?;
    if first_bool == second_bool {
        return None;
    }

    // map explicit true/false arm bodies
    let (true_body_id, false_body_id) = if first_bool {
        (first_body_id, second_body_id)
    } else {
        (second_body_id, first_body_id)
    };

    // rewrite the whole match expression to if/else
    let value_text = ctx.get_span_text(ctx.dir.get_span(value_expression_id));
    let true_body_text = ctx.get_span_text(ctx.dir.get_span(true_body_id));
    let false_body_text = ctx.get_span_text(ctx.dir.get_span(false_body_id));
    let replacement =
        format!("if {value_text} {{ {true_body_text} }} else {{ {false_body_text} }}");

    let match_span = ctx.dir.get_span(match_expression_id);
    let mut edit_builder = ctx.edit_builder().replace(match_span, replacement);

    // remove a leading `match ` token when the expression span starts at the selector value
    if match_span.start >= 6 {
        let prefix_span =
            destack_source::Span::new(match_span.file, match_span.start - 6, match_span.start);
        if ctx.get_span_text(prefix_span) == "match " {
            edit_builder = edit_builder.replace(prefix_span, "");
        }
    }

    let edits = edit_builder.into_edits();
    Some(LintFix::safe("Rewrite boolean match to if/else").with_edits(edits))
}

/// Return `(bool_value, body_expression)` for one simple boolean expression case.
fn bool_expression_case_body(
    ctx: &LintModuleContext<'_>,
    case_id: dir::LocalNodeId<MatchCase>,
) -> Option<(bool, dir::LocalNodeId<Expression>)> {
    let case = ctx.dir.get(case_id);
    let (selector, body) = match case {
        MatchCase::Expression { selector, body } => (selector, *body),
        MatchCase::Block { .. } => return None,
    };

    let dir::MatchSelector::Pattern {
        pattern: pattern_id,
        guard,
    } = selector
    else {
        return None;
    };
    if guard.is_some() {
        return None;
    }

    let pattern = ctx.dir.get(*pattern_id);
    let Pattern::Expression { value } = pattern else {
        return None;
    };

    let expression = ctx.dir.get(*value);
    let Expression::ScalarLiteral(ScalarLiteral::Boolean(boolean_value)) = expression else {
        return None;
    };

    Some((*boolean_value, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_match_bool_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_match_bool_detected.ds",
            r#"
function foo(condition: bool) {
    match condition {
        true => doX()
        false => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-if-else-over-match-bool")
            .assert_safe_fixed(
                r#"
function foo(condition: bool) {
    if (condition) { doX() } else { doY() }
}
"#,
            );
    }

    #[test]
    fn test_match_bool_false_first_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_match_bool_false_first_detected.ds",
            r#"
function foo(condition: bool) {
    match condition {
        false => doY()
        true => doX()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-if-else-over-match-bool")
            .assert_safe_fixed(
                r#"
function foo(condition: bool) {
    if (condition) { doX() } else { doY() }
}
"#,
            );
    }

    #[test]
    fn test_if_else_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_if_else_allowed.ds",
            r#"
function foo(condition: bool) {
    if condition {
        doX()
    } else {
        doY()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_non_bool_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_match_non_bool_allowed.ds",
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
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_enum_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_match_enum_allowed.ds",
            r#"
function foo(x: int32?) {
    match x {
        Some(n) => doX(n)
        None => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_bool_with_wildcard_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_match_bool_with_wildcard_allowed.ds",
            r#"
function foo(condition: bool) {
    match condition {
        true => doX()
        _ => doY()
    }
}
"#,
        );
        // using wildcard instead of explicit false is fine
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_bool_with_guard_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_match_bool_with_guard_allowed.ds",
            r#"
function foo(condition: bool) {
    match condition {
        true if isReady() => doX()
        false => doY()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_no_fix_for_block_body_cases() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_no_fix_for_block_body_cases.ds",
            r#"
function foo(condition: bool) {
    match condition {
        true => {
            doX()
        }
        false => {
            doY()
        }
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-if-else-over-match-bool")
            .assert_has_no_fix("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_more_than_two_arms_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_match_more_than_two_arms_allowed.ds",
            r#"
function foo(x: int32) {
    match x {
        1 => doX()
        2 => doY()
        3 => doZ()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }

    #[test]
    fn test_match_bool_same_value_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfElseOverMatchBool);
        let result = test.lint(
            "prefer_if_else_over_match_bool/test_match_bool_same_value_allowed.ds",
            r#"
function foo(condition: bool) {
    match condition {
        true => doX()
        true => doY()
    }
}
"#,
        );
        // both arms matching true doesn't make sense as if/else
        test.result(result)
            .assert_no_lint("prefer-if-else-over-match-bool");
    }
}
