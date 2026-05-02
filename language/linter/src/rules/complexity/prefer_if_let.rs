use crate::LintMeta;
use destack_ast::{self as ast, MatchCase};
use destack_workspace::LintSeverity;

use crate::rules::common::pattern_matches_all;
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Suggest if-let over single-arm match.
    ///
    /// A match expression with a single pattern arm followed by a wildcard can be more clearly written as an if-let expression.
    #[lint(
        id = "prefer-if-let",
        code = "LX025",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferIfLet,
    "Prefer if-let over single-arm match"
}

impl LintRule for PreferIfLet {
    fn meta(&self) -> &'static LintMeta {
        PreferIfLet::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::Match { value, cases, .. } = expression else {
                continue;
            };

            // look for exactly 2 cases
            if cases.len() != 2 {
                continue;
            }

            // check if second case is a wildcard or default
            let first_case = ctx.tree.get(cases[0]);
            let second_case = ctx.tree.get(cases[1]);

            // keep first case as plain pattern without guard
            let first_selector = first_case.selector();
            if !matches!(first_selector, ast::MatchSelector::Pattern { .. })
                || first_selector.has_guard()
            {
                continue;
            }

            // allow wildcard and default second cases
            let second_selector = second_case.selector();
            let is_wildcard = second_selector.is_default()
                || second_selector
                    .pattern_id()
                    .map(|pattern_id| pattern_matches_all(ctx, pattern_id))
                    .unwrap_or(false);

            if !is_wildcard {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintReport::new(
                PREFER_IF_LET.id,
                PREFER_IF_LET.code,
                PREFER_IF_LET.category,
                severity,
                "match with single pattern and wildcard can be if-let",
                ctx.tree.get_span(node_id),
            )
            .label("use if-let instead");
            if ctx.compute_fixes
                && let Some(fix) = prefer_if_let_fix(ctx, node_id, *value, first_case, second_case)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a safe match-to-if-let rewrite.
fn prefer_if_let_fix(
    ctx: &LintAstContext<'_>,
    match_expression_id: ast::LocalNodeId<ast::Expression>,
    value_expression_id: ast::LocalNodeId<ast::Expression>,
    first_case: &MatchCase,
    second_case: &MatchCase,
) -> Option<LintFix> {
    let value_text = ctx
        .get_span_text(ctx.tree.get_span(value_expression_id))
        .to_string();
    if value_text.trim().is_empty() {
        return None;
    }

    let (first_selector, first_body_text) = match first_case {
        MatchCase::Expression { selector, body } => (selector, expression_body_text(ctx, *body)?),
        MatchCase::Block { selector, body } => (selector, block_body_text(ctx, *body)?),
    };
    let ast::MatchSelector::Pattern {
        pattern,
        guard: None,
    } = first_selector
    else {
        return None;
    };

    let pattern_text = ctx.get_span_text(ctx.tree.get_span(*pattern)).to_string();
    if pattern_text.trim().is_empty() {
        return None;
    }

    let second_body_text = match second_case {
        MatchCase::Expression { body, .. } => expression_body_text(ctx, *body)?,
        MatchCase::Block { body, .. } => block_body_text(ctx, *body)?,
    };
    let else_is_empty = second_body_text.trim() == "{}";

    let mut replacement_text = format!("if let {pattern_text} = {value_text} {first_body_text}");
    if !else_is_empty {
        replacement_text.push_str(" else ");
        replacement_text.push_str(&second_body_text);
    }

    let match_span = ctx.tree.get_span(match_expression_id);
    let mut edit_builder = ctx.edit_builder().replace(match_span, replacement_text);

    // remove the `match ` prefix when the expression span starts at the selector value
    if match_span.start >= 6 {
        let prefix_span =
            destack_source::Span::new(match_span.file, match_span.start - 6, match_span.start);
        if ctx.get_span_text(prefix_span) == "match " {
            edit_builder = edit_builder.replace(prefix_span, "");
        }
    }

    let edits = edit_builder.into_edits();
    Some(LintFix::safe("Rewrite to if-let").with_edits(edits))
}

/// Render one expression case body for use as an if branch body.
fn expression_body_text(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<String> {
    let body_text = ctx
        .get_span_text(ctx.tree.get_span(expression_id))
        .to_string();
    if body_text.trim().is_empty() {
        return None;
    }

    Some(format!("{{ {body_text} }}"))
}

/// Render one block case body for use as an if branch body.
fn block_body_text(
    ctx: &LintAstContext<'_>,
    block_id: ast::LocalNodeId<ast::Block>,
) -> Option<String> {
    let body_text = ctx.get_span_text(ctx.tree.get_span(block_id)).to_string();
    if body_text.trim().is_empty() {
        return None;
    }

    Some(body_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_single_arm_match_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferIfLet);
        let result = test.lint_ast(
            "prefer_if_let/test_single_arm_match_detected.ds",
            r#"
function foo(x: int32?) {
    match (x) {
        Some(n) => console.log(n)
        _ => {}
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-if-let");
    }

    #[test]
    fn test_if_let_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfLet);
        let result = test.lint_ast(
            "prefer_if_let/test_if_let_allowed.ds",
            r#"
function foo(x: int32?) {
    if let Some(n) = x {
        console.log(n)
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-if-let");
    }

    #[test]
    fn test_multi_arm_match_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferIfLet);
        let result = test.lint_ast(
            "prefer_if_let/test_multi_arm_match_allowed.ds",
            r#"
function foo(x: int32) {
    match (x) {
        1 => console.log("one")
        2 => console.log("two")
        _ => console.log("other")
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-if-let");
    }

    #[test]
    fn test_fix_rewrites_single_pattern_match_to_if_let() {
        let test = TestProgram::for_rule_without_prelude(PreferIfLet);
        let result = test.lint_ast(
            "prefer_if_let/test_fix_rewrites_single_pattern_match_to_if_let.ds",
            r#"
function run(x: int32?) {
    match (x) {
        Some(n) => console.log(n)
        _ => {}
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-if-let")
            .assert_safe_fixed(
                r#"
function run(x: int32?) {
    if let Some(n) = (x) { console.log(n) }
}
"#,
            );
    }

    #[test]
    fn test_no_fix_for_match_guard_case() {
        let test = TestProgram::for_rule_without_prelude(PreferIfLet);
        let result = test.lint_ast(
            "prefer_if_let/test_no_fix_for_match_guard_case.ds",
            r#"
function run(x: int32?) {
    match (x) {
        Some(n) if n > 0 => console.log(n)
        _ => {}
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-if-let");
    }

    #[test]
    fn test_fix_preserves_non_empty_wildcard_as_else_branch() {
        let test = TestProgram::for_rule_without_prelude(PreferIfLet);
        let result = test.lint_ast(
            "prefer_if_let/test_fix_preserves_non_empty_wildcard_as_else_branch.ds",
            r#"
function run(x: int32?) {
    match (x) {
        Some(n) => console.log(n)
        _ => console.log("none")
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-if-let")
            .assert_safe_fixed(
                r#"
function run(x: int32?) {
    if let Some(n) = (x) { console.log(n) } else { console.log("none") }
}
"#,
            );
    }
}
