use crate::LintMeta;
use destack_dir as dir;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::span_has_comment;
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow match guards that are always true or false.
    ///
    /// A match guard like `if true` or `if false` is redundant or makes the
    /// arm unreachable. If the guard is always true, remove it. If always
    /// false, the arm will never match and should be removed.
    #[lint(
        id = "no-redundant-match-guard",
        code = "LU025",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoRedundantMatchGuard,
    "Disallow match guards that are always true or false"
}

impl LintRule for NoRedundantMatchGuard {
    fn meta(&self) -> &'static LintMeta {
        NoRedundantMatchGuard::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::MatchCase>() {
            let match_case = ctx.dir.get(node_id);
            let selector = match_case.selector();
            let Some(pattern_id) = selector.pattern_id() else {
                continue;
            };
            let Some(guard_id) = selector.guard_expression_id() else {
                continue;
            };

            let Some(is_truthy) = ctx.const_bool(guard_id) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let (message, label) = if is_truthy {
                ("match guard is always true", "this guard can be removed")
            } else {
                ("match guard is always false", "this arm will never match")
            };

            let mut diagnostic = LintReport::new(
                NO_REDUNDANT_MATCH_GUARD.id,
                NO_REDUNDANT_MATCH_GUARD.code,
                NO_REDUNDANT_MATCH_GUARD.category,
                severity,
                message,
                ctx.dir.get_span(guard_id),
            )
            .label(label);

            // remove guards that are always true
            if is_truthy
                && ctx.compute_fixes
                && let Some(fix) = redundant_true_guard_fix(ctx, pattern_id, guard_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a safe fix for one always true match guard.
fn redundant_true_guard_fix(
    ctx: &LintModuleContext<'_>,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    guard_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let pattern_span = ctx.dir.get_span(pattern_id);
    let guard_span = ctx.dir.get_span(guard_id);
    if pattern_span.file != guard_span.file || pattern_span.end > guard_span.end {
        return None;
    }

    let remove_span = Span::new(pattern_span.file, pattern_span.end, guard_span.end);
    if remove_span.is_empty() {
        return None;
    }
    if span_has_comment(ctx.dir.tree(), remove_span) {
        return None;
    }

    let edits = ctx.edit_builder().delete(remove_span).into_edits();
    Some(LintFix::safe("Remove always true match guard").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_guard_true() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_detects_guard_true.ds",
            r#"
match (x) {
    1 if true => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_fix_removes_always_true_guard() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_fix_removes_always_true_guard.ds",
            r#"
match (x) {
    1 if true => "one"
    _ => "other"
}
"#,
        );
        test.result(result)
            .assert_lint("no-redundant-match-guard")
            .assert_has_fix("no-redundant-match-guard")
            .assert_safe_fixed(
                r#"
match (x) {
    1 => "one"
    _ => "other"
}
"#,
            );
    }

    #[test]
    fn test_detects_guard_false() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_detects_guard_false.ds",
            r#"
match (x) {
    1 if false => "one"
    _ => "other"
}
"#,
        );
        test.result(result)
            .assert_lint("no-redundant-match-guard")
            .assert_has_no_fix("no-redundant-match-guard");
    }

    #[test]
    fn test_detects_guard_not_false() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_detects_guard_not_false.ds",
            r#"
match (x) {
    1 if !false => "one"
    _ => "other"
}
"#,
        );
        test.result(result)
            .assert_lint("no-redundant-match-guard")
            .assert_has_fix("no-redundant-match-guard")
            .assert_safe_fixed(
                r#"
match (x) {
    1 => "one"
    _ => "other"
}
"#,
            );
    }

    #[test]
    fn test_detects_guard_zero() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_detects_guard_zero.ds",
            r#"
match (x) {
    1 if 0 => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_detects_guard_one() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_detects_guard_one.ds",
            r#"
match (x) {
    1 if 1 => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_allows_variable_guard() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_allows_variable_guard.ds",
            r#"
match (x) {
    1 if y > 0 => "one"
    _ => "other"
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_allows_no_guard() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_allows_no_guard.ds",
            r#"
match (x) {
    1 => "one"
    _ => "other"
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-redundant-match-guard");
    }

    #[test]
    fn test_true_guard_with_comment_has_no_fix() {
        let test = TestProgram::for_rule_without_prelude(NoRedundantMatchGuard);
        let result = test.lint(
            "no_redundant_match_guard/test_true_guard_with_comment_has_no_fix.ds",
            r#"
match (x) {
    1 if /* keep */ true => "one"
    _ => "other"
}
"#,
        );
        test.result(result)
            .assert_lint("no-redundant-match-guard")
            .assert_has_no_fix("no-redundant-match-guard");
    }
}
