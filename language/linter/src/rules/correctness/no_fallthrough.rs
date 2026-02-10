use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow fallthrough from one switch case to another.
    ///
    /// Unintentional fallthrough in switch statements is a common source of bugs.
    /// If fallthrough is intentional, add a `// fallthrough` comment.
    #[lint(
        id = "no-fallthrough",
        code = "LC014",
        category = Correctness,
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        NoFallthrough::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Match { kind, cases, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // only check switch statements
            if *kind != ast::MatchKind::Switch {
                continue;
            }

            // check each case for fallthrough
            for (i, case_id) in cases.iter().enumerate() {
                // skip the last case (can't fall through)
                if i == cases.len() - 1 {
                    continue;
                }

                let case = ctx.tree.get(*case_id);
                let (body_id, is_block) = match case {
                    ast::MatchCase::Expression { body, .. } => (*body, false),
                    ast::MatchCase::Block { body, .. } => {
                        let body_expression_id = ctx.tree.get(*body).expressions.last();
                        if let Some(id) = body_expression_id {
                            (*id, true)
                        } else {
                            // empty block falls through
                            let severity = ctx.get_effective_severity(meta, node_id);
                            if severity.is_enabled() {
                                let mut diagnostic = LintDiagnostic::new(
                                    NO_FALLTHROUGH.id,
                                    NO_FALLTHROUGH.code,
                                    NO_FALLTHROUGH.category,
                                    severity,
                                    "empty case falls through to next case",
                                    ctx.module.file_id,
                                    ctx.tree.get_span(*case_id),
                                )
                                .with_label("add a `break` statement or `// fallthrough` comment");

                                // compute fixes only when requested by the runner
                                if ctx.compute_fixes
                                    && let Some(fix) = no_fallthrough_fix(ctx, *case_id)
                                {
                                    diagnostic = diagnostic.with_fix(fix);
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

                if !terminates {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    let mut diagnostic = LintDiagnostic::new(
                        NO_FALLTHROUGH.id,
                        NO_FALLTHROUGH.code,
                        NO_FALLTHROUGH.category,
                        severity,
                        "case falls through to next case",
                        ctx.module.file_id,
                        ctx.tree.get_span(*case_id),
                    )
                    .with_label("add a `break` statement or `// fallthrough` comment");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = no_fallthrough_fix(ctx, *case_id)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Build an unsafe fix by inserting `break;` at the end of a switch case.
fn no_fallthrough_fix(
    ctx: &LintModuleAstContext<'_>,
    case_id: ast::LocalNodeId<ast::MatchCase>,
) -> Option<LintFix> {
    let case_span = ctx.tree.get_span(case_id);
    let case_text = ctx.get_span_text(case_span);
    if case_text.trim().is_empty() {
        return None;
    }

    let edits = ctx
        .edit_builder()
        .insert(case_span.end, "\n        break;")
        .into_edits();
    Some(LintFix::r#unsafe("Insert break to prevent fallthrough").with_edits(edits))
}

/// Check if an expression ends with a terminating statement.
fn ends_with_terminating_statement(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        ast::Expression::Statement(inner_id) => is_terminating_statement(ctx, *inner_id),
        _ => is_terminating_statement(ctx, expression_id),
    }
}

/// Check if an expression is a terminating statement.
fn is_terminating_statement(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        ast::Expression::Break { .. } => true,
        ast::Expression::Return { .. } => true,
        ast::Expression::Throw { .. } => true,
        ast::Expression::Continue { .. } => true,
        ast::Expression::Statement(inner_id) => is_terminating_statement(ctx, *inner_id),
        ast::Expression::Parenthesized { expression } => is_terminating_statement(ctx, *expression),
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            if let Some(last_id) = block.expressions.last() {
                ends_with_terminating_statement(ctx, *last_id)
            } else {
                false
            }
        }
        ast::Expression::If {
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
    case 1: {
        console.log("one")

        break
    }
    case 2: {
        console.log("two")
        break
    }
}
"#,
            );
    }

    #[test]
    fn test_fix_inserts_break_for_empty_case() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint_ast(
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
    case 1: break
    case 2: break
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_inserts_break_in_middle_case() {
        let test = TestProgram::for_rule_without_prelude(NoFallthrough);
        let result = test.lint_ast(
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
    case 2: {
        console.log("two")

        break
    }
    case 3: break
}
"#,
            );
    }
}
