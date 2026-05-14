use regex::Regex;

use destack_dir as dir;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{compiled_no_fallthrough_comment_pattern, fallthrough_comment_matches};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow fallthrough from one switch case to another.
    ///
    /// Unintentional fallthrough in switch statements is a common source of bugs.
    /// If fallthrough is intentional, add a `// fallthrough` comment.
    #[lint(
        id = "no-fallthrough",
        code = "LC014",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoFallthrough,
    "Disallow switch case fallthrough"
}

impl LintRule for NoFallthrough {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoFallthrough::meta()
    }

    /// Check module source nodes for switch fallthrough cases.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let allow_empty_case = ctx.options.correctness.no_fallthrough_allow_empty_case;
        let fallthrough_comment_pattern = compiled_no_fallthrough_comment_pattern(
            ctx.options
                .correctness
                .no_fallthrough_comment_pattern
                .as_deref(),
        );
        let report_unused_comment = ctx.options.correctness.no_fallthrough_report_unused_comment;

        // inspect candidate expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Match { form, cases, .. } = ctx.dir.get(node_id) else {
                continue;
            };

            // only check switch statements
            if *form != dir::MatchForm::Switch {
                continue;
            }

            // check each case for fallthrough
            for (i, case_id) in cases.iter().enumerate() {
                // skip the last case (can't fall through)
                if i == cases.len() - 1 {
                    continue;
                }
                let next_case_id = cases[i + 1];

                // resolve case
                let case = ctx.dir.get(*case_id);
                let (body_id, is_block) = match case {
                    dir::MatchCase::Expression { body, .. } => (*body, false),
                    dir::MatchCase::Block { body, .. } => {
                        let body_expression_id = ctx.dir.get(*body).last_expression();
                        if let Some(id) = body_expression_id {
                            (id, true)
                        } else {
                            let fallthrough_comment_span = fallthrough_comment_between_cases(
                                ctx,
                                *case_id,
                                next_case_id,
                                fallthrough_comment_pattern.as_deref(),
                            );

                            if allow_empty_case {
                                continue;
                            }

                            // allow explicit intentional fallthrough comments
                            if fallthrough_comment_span.is_some() {
                                continue;
                            }

                            // empty block falls through
                            let severity = ctx.get_effective_severity(meta, node_id);
                            if severity.is_enabled() {
                                let mut diagnostic = LintReport::new(
                                    NO_FALLTHROUGH.id,
                                    NO_FALLTHROUGH.code,
                                    NO_FALLTHROUGH.category,
                                    severity,
                                    "empty case falls through to next case",
                                    ctx.dir.get_span(*case_id),
                                )
                                .label("add a `break` statement or `// fallthrough` comment");

                                // compute fixes only when requested by the runner
                                if ctx.compute_fixes
                                    && let Some(fix) = no_fallthrough_fix(ctx, *case_id)
                                {
                                    diagnostic = diagnostic.fix(fix);
                                }

                                ctx.report(diagnostic);
                            }
                            continue;
                        }
                    }
                };

                // check if the case ends with a terminating statement
                let terminates = if is_block {
                    ends_with_terminating_statement(ctx, body_id)
                } else {
                    is_terminating_statement(ctx, body_id)
                };

                // enforce this lint guard
                if !terminates {
                    let fallthrough_comment_span = fallthrough_comment_between_cases(
                        ctx,
                        *case_id,
                        next_case_id,
                        fallthrough_comment_pattern.as_deref(),
                    );

                    // allow explicit intentional fallthrough comments
                    if fallthrough_comment_span.is_some() {
                        continue;
                    }

                    // resolve effective lint severity
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    let mut diagnostic = LintReport::new(
                        NO_FALLTHROUGH.id,
                        NO_FALLTHROUGH.code,
                        NO_FALLTHROUGH.category,
                        severity,
                        "case falls through to next case",
                        ctx.dir.get_span(*case_id),
                    )
                    .label("add a `break` statement or `// fallthrough` comment");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = no_fallthrough_fix(ctx, *case_id)
                    {
                        diagnostic = diagnostic.fix(fix);
                    }

                    ctx.report(diagnostic);
                } else if report_unused_comment
                    && let Some(comment_span) = fallthrough_comment_between_cases(
                        ctx,
                        *case_id,
                        next_case_id,
                        fallthrough_comment_pattern.as_deref(),
                    )
                {
                    report_unused_fallthrough_comment(ctx, meta, *case_id, comment_span);
                }
            }
        }
    }
}

/// Return the intentional fallthrough comment span between two switch cases.
fn fallthrough_comment_between_cases(
    ctx: &LintModuleContext<'_>,
    current_case_id: dir::LocalNodeId<dir::MatchCase>,
    next_case_id: dir::LocalNodeId<dir::MatchCase>,
    fallthrough_comment_pattern: Option<&Regex>,
) -> Option<Span> {
    let current_span = ctx.dir.get_span(current_case_id);
    let next_span = ctx.dir.get_span(next_case_id);
    if current_span.end > next_span.start {
        return None;
    }

    // search all comments in the case gap for intent markers
    ctx.dir.comments().iter().find_map(|comment| {
        let comment_span = comment.span;
        if comment_span.file != ctx.module.file_id {
            return None;
        }
        if comment_span.start < current_span.start || comment_span.end > next_span.start {
            return None;
        }

        // resolve comment text
        let comment_text = ctx.get_span_text(comment_span);
        fallthrough_comment_matches(comment_text, fallthrough_comment_pattern)
            .then_some(comment_span)
    })
}

/// Report one unused fallthrough comment.
fn report_unused_fallthrough_comment(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    case_id: dir::LocalNodeId<dir::MatchCase>,
    comment_span: Span,
) {
    let severity = ctx.get_effective_severity(meta, case_id);
    if !severity.is_enabled() {
        return;
    }

    ctx.report(
        LintReport::new(
            NO_FALLTHROUGH.id,
            NO_FALLTHROUGH.code,
            NO_FALLTHROUGH.category,
            severity,
            "unused fallthrough comment",
            comment_span,
        )
        .label("this case cannot fall through, so the comment is misleading"),
    );
}

/// Build an unsafe fix by inserting `break;` at the end of a switch case.
fn no_fallthrough_fix(
    ctx: &LintModuleContext<'_>,
    case_id: dir::LocalNodeId<dir::MatchCase>,
) -> Option<LintFix> {
    let case_span = ctx.dir.get_span(case_id);
    let case_text = ctx.get_span_text(case_span);
    if case_text.trim().is_empty() {
        return None;
    }

    // append break at the end of this case body
    let edits = ctx
        .edit_builder()
        .insert(case_span.end, "\n        break;")
        .into_edits();
    Some(LintFix::r#unsafe("Insert break to prevent fallthrough").with_edits(edits))
}

/// Check if an expression ends with a terminating statement.
fn ends_with_terminating_statement(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    is_terminating_statement(ctx, expression_id)
}

/// Check if an expression is a terminating statement.
fn is_terminating_statement(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = ctx.dir.get(expression_id);
    match expression {
        dir::Expression::Break { .. } => true,
        dir::Expression::Return { .. } => true,
        dir::Expression::Throw { .. } => true,
        dir::Expression::Continue { .. } => true,
        dir::Expression::Parenthesized { expression } => is_terminating_statement(ctx, *expression),
        dir::Expression::Block(block_id) => {
            let block = ctx.dir.get(*block_id);
            if let Some(last_id) = block.last_expression() {
                ends_with_terminating_statement(ctx, last_id)
            } else {
                false
            }
        }
        dir::Expression::If {
            then_expression,
            else_expression: Some(else_id),
            ..
        } => {
            // if both branches terminate, the if terminates
            ends_with_terminating_statement(ctx, *then_expression)
                && ends_with_terminating_statement(ctx, *else_id)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_fallthrough() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_detects_fallthrough.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        console.log("one");
    case 2:
        console.log("two");
        break;
}
"#,
        );
        test.result(result)
            .assert_lint("no-fallthrough")
            .assert_has_fix("no-fallthrough");
    }

    #[test]
    fn test_allows_break() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_allows_break.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        console.log("one");
        break;
    case 2:
        console.log("two");
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_allows_return() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_allows_return.ds",
            r#"
function foo(x: int32): int32 {
    switch (x) {
        case 1:
            return 1;
        case 2:
            return 2;
    }
    return 0;
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_allows_throw() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_allows_throw.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        throw "error";
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_allows_last_case_without_break() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_allows_last_case_without_break.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        console.log("one");
        break;
    case 2:
        console.log("two");
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_ignores_match() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_ignores_match.ds",
            r#"
let x = 1;
match (x) {
    1 => console.log("one")
    2 => console.log("two")
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_fix_inserts_break_for_fallthrough_case() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_fix_inserts_break_for_fallthrough_case.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        console.log("one");
    case 2:
        console.log("two");
        break;
}
"#,
        );
        test.result(result)
            .assert_lint("no-fallthrough")
            .assert_unsafe_fixed(
                r#"
let x = 1;
switch (x) {
    case 1:
        console.log("one")

        break
    case 2:
        console.log("two")
        break
}
"#,
            );
    }

    #[test]
    fn test_fix_inserts_break_for_empty_case() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_fix_inserts_break_for_empty_case.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
    case 2:
        break;
}
"#,
        );
        test.result(result)
            .assert_lint("no-fallthrough")
            .assert_unsafe_fixed(
                r#"
let x = 1;
switch (x) {
    case 1:

        break
    case 2: break
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_inserts_break_in_middle_case() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_mutation_fix_inserts_break_in_middle_case.ds",
            r#"
let x = 2;
switch (x) {
    case 1:
        break;
    case 2:
        console.log("two");
    case 3:
        break;
}
"#,
        );
        test.result(result)
            .assert_lint("no-fallthrough")
            .assert_unsafe_fixed(
                r#"
let x = 2;
switch (x) {
    case 1: break
    case 2:
        console.log("two")

        break
    case 3: break
}
"#,
            );
    }

    #[test]
    fn test_allows_fallthrough_comment() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_allows_fallthrough_comment.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        log("one");
        // falls through
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_allows_fallthrough_block_comment() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_allows_fallthrough_block_comment.ds",
            r#"
let x = 1;
switch (x) {
    case 1: {
        log("one");
        /* fallthrough */
    }
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_allows_empty_case_when_configured() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough).with_options(|options| {
            options.correctness.no_fallthrough_allow_empty_case = true;
        });
        let result = test.lint(
            "no_fallthrough/test_allows_empty_case_when_configured.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_allows_custom_fallthrough_comment_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough).with_options(|options| {
            options.correctness.no_fallthrough_comment_pattern = Some("no break".to_string());
        });
        let result = test.lint(
            "no_fallthrough/test_allows_custom_fallthrough_comment_pattern.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        log("one");
        // no break
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }

    #[test]
    fn test_reports_unused_fallthrough_comment_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough).with_options(|options| {
            options.correctness.no_fallthrough_report_unused_comment = true;
        });
        let result = test.lint(
            "no_fallthrough/test_reports_unused_fallthrough_comment_when_enabled.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        break;
        // fallthrough
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_lint("no-fallthrough");
    }

    #[test]
    fn test_ignores_unused_fallthrough_comment_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint(
            "no_fallthrough/test_ignores_unused_fallthrough_comment_by_default.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        break;
        // fallthrough
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-fallthrough");
    }
}
