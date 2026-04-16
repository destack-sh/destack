use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{BlockDuplicateTracker, ExpressionDuplicateTracker, span_has_comment};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Warn on match arms with identical bodies.
    ///
    /// Having multiple match arms with the same body is often a sign of copy-paste
    /// errors or missed opportunities to combine patterns. Consider using `|` to
    /// combine patterns or extracting the common logic.
    #[lint(
        id = "no-duplicate-match-arms",
        code = "LU011",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoDuplicateMatchArms,
    "Warn on match arms with identical bodies"
}

impl LintRule for NoDuplicateMatchArms {
    fn meta(&self) -> &'static LintMeta {
        NoDuplicateMatchArms::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Match { cases, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // collect expression and block bodies separately
            let mut seen_expression_bodies = ExpressionDuplicateTracker::new();
            let mut seen_block_bodies = BlockDuplicateTracker::new();

            for case_id in cases {
                let case = ctx.tree.get(*case_id);

                // extract body expression from match case
                let is_duplicate = match case {
                    ast::MatchCase::Expression { body, .. } => seen_expression_bodies
                        .find_duplicate_or_insert(ctx, *body)
                        .is_some(),
                    ast::MatchCase::Block { body, .. } => {
                        let block = ctx.tree.get(*body);
                        if block.len() == 1 {
                            seen_expression_bodies
                                .find_duplicate_or_insert(ctx, block.first_expression().unwrap())
                                .is_some()
                        } else {
                            seen_block_bodies
                                .find_duplicate_or_insert(ctx, *body)
                                .is_some()
                        }
                    }
                };

                // check against previously seen bodies
                if is_duplicate {
                    let severity = ctx.get_effective_severity(meta, *case_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let mut diagnostic = LintDiagnostic::new(
                        NO_DUPLICATE_MATCH_ARMS.id,
                        NO_DUPLICATE_MATCH_ARMS.code,
                        NO_DUPLICATE_MATCH_ARMS.category,
                        severity,
                        "duplicate match arm body",
                        ctx.module.file_id,
                        ctx.tree.get_span(*case_id),
                    )
                    .with_label("this arm has the same body as a previous arm");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = duplicate_match_arm_fix(ctx, *case_id)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Build an unsafe fix that removes one duplicate match arm.
fn duplicate_match_arm_fix(
    ctx: &LintAstContext<'_>,
    case_id: ast::LocalNodeId<ast::MatchCase>,
) -> Option<LintFix> {
    let case_span = ctx.tree.get_span(case_id);
    if span_has_comment(ctx.tree, case_span) {
        return None;
    }

    let edits = ctx.edit_builder().delete(case_span).into_edits();
    Some(LintFix::r#unsafe("Remove duplicate match arm").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_match_arms() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "no_duplicate_match_arms/test_detects_duplicate_match_arms.ds",
            r#"
match (x) {
    1 => foo()
    2 => foo()
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-match-arms");
    }

    #[test]
    fn test_allows_different_bodies() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "no_duplicate_match_arms/test_allows_different_bodies.ds",
            r#"
match (x) {
    1 => foo()
    2 => bar()
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-match-arms");
    }

    #[test]
    fn test_detects_duplicate_literals() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "no_duplicate_match_arms/test_detects_duplicate_literals.ds",
            r#"
match (x) {
    1 => 42
    2 => 42
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-match-arms");
    }

    #[test]
    fn test_detects_duplicate_block_bodies() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "no_duplicate_match_arms/test_detects_duplicate_block_bodies.ds",
            r#"
match (x) {
    1 => {
        let y = x + 1;
        y
    }
    2 => {
        let y = x + 1;
        y
    }
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-match-arms");
    }

    #[test]
    fn test_fix_removes_duplicate_match_arm() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "no_duplicate_match_arms/test_fix_removes_duplicate_match_arm.ds",
            r#"
match (x) {
    1 => foo()
    2 => foo()
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-match-arms")
            .assert_unsafe_fixed(
                r#"
match (x) {
    1 => foo()
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_removes_duplicate_block_match_arm() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "no_duplicate_match_arms/test_mutation_fix_removes_duplicate_block_match_arm.ds",
            r#"
match (x) {
    1 => {
        let y = x + 1;
        y
    }
    2 => {
        let y = x + 1;
        y
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-match-arms")
            .assert_unsafe_fixed(
                r#"
match (x) {
    1 => {
        let y = x + 1;
        y
    }
}
"#,
            );
    }

    #[test]
    fn test_reports_without_fix_when_duplicate_arm_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateMatchArms);
        let result = test.lint_ast(
            "no_duplicate_match_arms/test_reports_without_fix_when_duplicate_arm_contains_comment.ds",
            r#"
match (x) {
    1 => {
        // keep
        foo()
    }
    2 => {
        // keep
        foo()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-match-arms")
            .assert_has_no_fix("no-duplicate-match-arms");
    }
}
