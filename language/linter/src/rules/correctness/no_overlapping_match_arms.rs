use destack_dir as dir;
use destack_source::LabeledSpan;
use destack_workspace::LintSeverity;

use crate::rules::common::{pattern_is_total, pattern_subsumes_semantically};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow match or switch arms that are subsumed by previous arms.
    ///
    /// When an earlier unguarded arm already matches every value that a later
    /// arm could match, the later arm is unreachable and likely a bug.
    #[lint(
        id = "no-overlapping-match-arms",
        code = "LC048",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoOverlappingMatchArms,
    "Disallow match or switch arms subsumed by previous arms"
}

impl LintRule for NoOverlappingMatchArms {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoOverlappingMatchArms::meta()
    }

    /// Check module DIR nodes for overlapping match arms.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // walk every match or switch expression
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let dir::Expression::Match {
                source: _,
                kind: _,
                value: _,
                cases,
                scope: _,
                symbol: _,
            } = expression
            else {
                continue;
            };
            let mut prior_coverages = Vec::new();

            // compare each arm against prior unguarded coverage
            for case_id in cases {
                let case = ctx.tree.get(*case_id);
                let selector = case.selector();

                // resolve one overlapping prior case when present
                let overlapping_prior_case_id =
                    subsuming_prior_case_id(ctx, prior_coverages.as_slice(), selector);
                if let Some(prior_case_id) = overlapping_prior_case_id {
                    let severity = ctx.get_effective_severity(meta, *case_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let mut diagnostic = LintReport::new(
                        NO_OVERLAPPING_MATCH_ARMS.id,
                        NO_OVERLAPPING_MATCH_ARMS.code,
                        NO_OVERLAPPING_MATCH_ARMS.category,
                        severity,
                        "match arm is subsumed by a previous arm",
                        ctx.get_span(*case_id),
                    )
                    .label("this arm can never be selected")
                    .secondary(LabeledSpan::new(
                        ctx.get_span(prior_case_id),
                        "previous arm already covers every value matched here",
                    ));

                    // attach the delete only when fixes are enabled
                    if ctx.include_fixes
                        && let Some(fix) = overlapping_match_arm_fix(ctx, *case_id)
                    {
                        diagnostic = diagnostic.fix(fix);
                    }

                    ctx.report(diagnostic);
                }

                // keep only unconditional selectors as future subsumers
                if let Some(coverage) = selector_coverage(ctx, selector) {
                    prior_coverages.push(PriorCaseCoverage {
                        case_id: *case_id,
                        coverage,
                    });
                }
            }
        }
    }
}

/// One tracked prior case coverage shape.
#[derive(Debug, Clone, Copy)]
struct PriorCaseCoverage {
    /// The prior case id.
    case_id: dir::LocalNodeId<dir::MatchCase>,
    /// The unconditional coverage of the prior selector.
    coverage: SelectorCoverage,
}

/// One selector coverage shape used for subsumption checks.
#[derive(Debug, Clone, Copy)]
enum SelectorCoverage {
    /// Selector matches all remaining values.
    Any,
    /// Selector matches a specific pattern.
    Pattern(dir::LocalNodeId<dir::Pattern>),
}

/// Return the first prior case that subsumes the current selector.
fn subsuming_prior_case_id(
    ctx: &mut LintModuleDirContext<'_>,
    prior_coverages: &[PriorCaseCoverage],
    selector: &dir::MatchSelector,
) -> Option<dir::LocalNodeId<dir::MatchCase>> {
    // default is subsumed only by one prior total selector
    if selector.is_default() {
        return prior_coverages
            .iter()
            .find(|prior| matches!(prior.coverage, SelectorCoverage::Any))
            .map(|prior| prior.case_id);
    }

    let pattern_id = selector.pattern_id()?;

    // find the first prior unguarded selector that covers this pattern
    prior_coverages
        .iter()
        .find(|prior| match prior.coverage {
            SelectorCoverage::Any => true,
            SelectorCoverage::Pattern(prior_pattern_id) => {
                pattern_subsumes_semantically(ctx, prior_pattern_id, pattern_id)
            }
        })
        .map(|prior| prior.case_id)
}

/// Return tracked coverage for one selector, or none when guarded.
fn selector_coverage(
    ctx: &mut LintModuleDirContext<'_>,
    selector: &dir::MatchSelector,
) -> Option<SelectorCoverage> {
    // default always matches every remaining value
    if selector.is_default() {
        return Some(SelectorCoverage::Any);
    }

    let pattern_id = selector.pattern_id()?;

    // guarded selectors do not subsume later arms unconditionally
    if selector.has_guard() {
        return None;
    }

    // wildcard and unconstrained bindings are total
    if pattern_is_total(ctx.tree, pattern_id) {
        return Some(SelectorCoverage::Any);
    }

    Some(SelectorCoverage::Pattern(pattern_id))
}

/// Build an unsafe fix that removes one subsumed match arm.
fn overlapping_match_arm_fix(
    ctx: &LintModuleDirContext<'_>,
    case_id: dir::LocalNodeId<dir::MatchCase>,
) -> Option<LintFix> {
    let case_span = ctx.get_span(case_id);
    let edits = ctx.edit_builder().replace(case_span, "").into_edits();
    Some(LintFix::r#unsafe("Remove subsumed match arm").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_match_arm_after_wildcard() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_flags_match_arm_after_wildcard.ds",
            r#"
function classify(x: int32): int32 {
    return match (x) {
        _ => 0
        1 => 1
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_flags_match_arm_after_unconstrained_binding() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_flags_match_arm_after_unconstrained_binding.ds",
            r#"
function classify(x: int32): int32 {
    return match (x) {
        value => value
        1 => 1
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_allows_guarded_wildcard_before_specific_arm() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_allows_guarded_wildcard_before_specific_arm.ds",
            r#"
function classify(x: int32): int32 {
    return match (x) {
        _ if x > 0 => 0
        1 => 1
        _ => 2
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_flags_switch_case_after_default() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_flags_switch_case_after_default.ds",
            r#"
function classify(x: int32) {
    switch (x) {
        default: 0
        case 1: 1
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_allows_switch_default_last() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_allows_switch_default_last.ds",
            r#"
function classify(x: int32) {
    switch (x) {
        case 1: 1
        default: 0
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_flags_duplicate_match_expression_arm() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_flags_duplicate_match_expression_arm.ds",
            r#"
function classify(x: int32): int32 {
    return match (x) {
        1 => 0
        1 => 1
        _ => 2
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_flags_union_subsuming_literal_arm() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_flags_union_subsuming_literal_arm.ds",
            r#"
function classify(x: int32): int32 {
    return match (x) {
        1 | 2 => 0
        2 => 1
        _ => 3
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_flags_guarded_arm_after_total_previous_arm() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_flags_guarded_arm_after_total_previous_arm.ds",
            r#"
function classify(x: int32): int32 {
    return match (x) {
        _ => 0
        1 if true => 1
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_allows_guarded_prior_duplicate_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_allows_guarded_prior_duplicate_pattern.ds",
            r#"
function classify(x: int32): int32 {
    return match (x) {
        1 if x > 1 => 0
        1 => 1
        _ => 2
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_flags_default_after_wildcard() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_flags_default_after_wildcard.ds",
            r#"
function classify(x: int32): int32 {
    return match (x) {
        _ => 0
        _ => 1
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-overlapping-match-arms");
    }

    #[test]
    fn test_fix_removes_subsumed_switch_case() {
        let test = TestProgram::for_rule_without_prelude(NoOverlappingMatchArms);
        let result = test.lint_dir(
            "no_overlapping_match_arms/test_fix_removes_subsumed_switch_case.ds",
            r#"
function classify(x: int32) {
    switch (x) {
        default: 0
        case 1: 1
    }
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-overlapping-match-arms")
            .assert_unsafe_fixed(
                r#"
function classify(x: int32) {
    switch (x) {
        default: 0
    }
}
"#,
            );
    }
}
