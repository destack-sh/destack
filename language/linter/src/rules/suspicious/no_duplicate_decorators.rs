use crate::LintMeta;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{ExpressionDuplicateTracker, span_has_comment};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate decorators on the same target.
    ///
    /// Only flags decorators that are structurally identical (same name and arguments).
    /// Decorators with different arguments are allowed, supporting stackable decorators.
    #[lint(
        id = "no-duplicate-decorators",
        code = "LU009",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoDuplicateDecorators,
    "Disallow duplicate decorators on the same target"
}

impl LintRule for NoDuplicateDecorators {
    fn meta(&self) -> &'static LintMeta {
        NoDuplicateDecorators::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // check each decorated node
        for decorators in ctx.tree.get_all_decorators().values() {
            // collect decorator ids for comparison
            let mut seen = ExpressionDuplicateTracker::new();

            for decorator_id in decorators {
                let decorator = ctx.tree.get(*decorator_id);

                // check against all previously seen decorators
                // report if we found a duplicate
                if seen
                    .find_duplicate_or_insert(ctx, decorator.expression)
                    .is_some()
                {
                    let severity = ctx.get_effective_severity(meta, *decorator_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    // extract decorator name for the message
                    // ("unknown" shouldn't happen, but not our business here)
                    let name = ctx
                        .decorator_name(*decorator_id)
                        .unwrap_or_else(|| "<unknown>".to_string());

                    let span = ctx.tree.get_span(*decorator_id);
                    let mut diagnostic = LintDiagnostic::new(
                        NO_DUPLICATE_DECORATORS.id,
                        NO_DUPLICATE_DECORATORS.code,
                        NO_DUPLICATE_DECORATORS.category,
                        severity,
                        format!("duplicate decorator '@{name}'"),
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("this decorator is already applied with identical arguments");
                    if ctx.compute_fixes && !span_has_comment(ctx.tree, span) {
                        let fix_span = duplicate_decorator_fix_span(ctx, span);
                        let fix =
                            LintFix::suggestion("Remove duplicate decorator").delete(fix_span);
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Return a deletion span that removes one duplicate decorator line cleanly.
fn duplicate_decorator_fix_span(ctx: &LintAstContext<'_>, annotation_span: Span) -> Span {
    let source = ctx.source_text().as_bytes();
    let mut start = annotation_span.start as usize;
    let mut end = annotation_span.end as usize;

    // include line indentation when decorator starts a standalone line
    let mut line_start = start;
    while line_start > 0 && source[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    if source[line_start..start]
        .iter()
        .all(|byte| *byte == b' ' || *byte == b'\t')
    {
        start = line_start;
    }

    // consume trailing spaces and one newline
    while end < source.len()
        && (source[end] == b' ' || source[end] == b'\t' || source[end] == b'\r')
    {
        end += 1;
    }
    if end < source.len() && source[end] == b'\n' {
        end += 1;
    }

    Span::new(annotation_span.file, start as u32, end as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_duplicate_decorator() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_flags_duplicate_decorator.ds",
            r#"
@inline
@inline
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_flags_duplicate_decorator_with_same_args() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_flags_duplicate_decorator_with_same_args.ds",
            r#"
@cache(100)
@cache(100)
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_allows_different_decorators() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_allows_different_decorators.ds",
            r#"
@inline
@deprecated
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_allows_same_decorator_different_args() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_allows_same_decorator_different_args.ds",
            r#"
@validate({ min: 1 })
@validate({ max: 100 })
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_allows_same_decorator_different_numeric_args() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_allows_same_decorator_different_numeric_args.ds",
            r#"
@cache(10)
@cache(20)
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_allows_single_decorator() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_allows_single_decorator.ds",
            r#"
@inline
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_flags_triple_decorator() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_flags_triple_decorator.ds",
            r#"
@inline
@inline
@inline
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint_count("no-duplicate-decorators", 2);
    }

    #[test]
    fn test_flags_only_exact_duplicates_among_multiple() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_flags_only_exact_duplicates_among_multiple.ds",
            r#"
@cache(10)
@cache(20)
@cache(10)
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint_count("no-duplicate-decorators", 1);
    }

    #[test]
    fn test_fix_removes_duplicate_decorator() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_fix_removes_duplicate_decorator.ds",
            r#"
@inline
@inline
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-decorators")
            .assert_has_fix("no-duplicate-decorators")
            .assert_suggested_fixed(
                r#"
@inline
function foo() {}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_collapses_multiple_duplicate_decorators() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_mutation_fix_collapses_multiple_duplicate_decorators.ds",
            r#"
@inline
@inline
@inline
@deprecated
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint_count("no-duplicate-decorators", 2)
            .assert_has_fix("no-duplicate-decorators")
            .assert_suggested_fixed(
                r#"
@inline
@deprecated
function foo() {}
"#,
            );
    }

    #[test]
    fn test_reports_without_fix_when_duplicate_decorator_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "no_duplicate_decorators/test_reports_without_fix_when_duplicate_decorator_contains_comment.ds",
            r#"
@cache(/* keep */ 100)
@cache(/* keep */ 100)
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-decorators")
            .assert_has_no_fix("no-duplicate-decorators");
    }
}
