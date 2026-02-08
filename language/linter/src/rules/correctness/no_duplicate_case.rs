use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::ExpressionDuplicateTracker;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate case labels in switch statements.
    ///
    /// Having duplicate case labels in a switch statement is almost always a mistake.
    /// Only the first matching case will be executed.
    #[lint(
        id = "no-duplicate-case",
        code = "LC018",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoDuplicateCase,
    "Disallow duplicate case labels"
}

impl LintRule for NoDuplicateCase {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDuplicateCase::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Match { kind, cases, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // only check switch statements, not match expressions
            if *kind != ast::MatchKind::Switch {
                continue;
            }

            // collect case expression IDs and check for duplicates
            let mut seen = ExpressionDuplicateTracker::new();
            for case_id in cases {
                let case = ctx.tree.get(*case_id);
                let selector = match case {
                    ast::MatchCase::Expression { selector, .. } => selector,
                    ast::MatchCase::Block { selector, .. } => selector,
                };

                // skip default cases
                let ast::MatchSelector::Pattern {
                    pattern: pattern_id,
                    ..
                } = selector
                else {
                    continue;
                };

                let pattern = ctx.tree.get(*pattern_id);
                let Some(expr_id) = pattern_to_expression(pattern) else {
                    continue;
                };

                // check against all previously seen expressions
                if seen.find_duplicate_or_insert(ctx, expr_id).is_some() {
                    let severity = ctx.get_effective_severity(meta, expr_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    ctx.report(
                        LintDiagnostic::new(
                            NO_DUPLICATE_CASE.id,
                            NO_DUPLICATE_CASE.code,
                            NO_DUPLICATE_CASE.category,
                            severity,
                            "duplicate case label",
                            ctx.module.file_id,
                            ctx.tree.get_span(*case_id),
                        )
                        .with_label("this case was already handled"),
                    );
                }
            }
        }
    }
}

/// Extract the expression from a pattern if it's an expression pattern.
fn pattern_to_expression(pattern: &ast::Pattern) -> Option<ast::LocalNodeId<ast::Expression>> {
    match pattern {
        ast::Pattern::Expression { value } => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_integer_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_detects_duplicate_integer_case.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 1: break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_detects_duplicate_string_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_detects_duplicate_string_case.ds",
            r#"
let x = "a";
switch (x) {
    case "a": break;
    case "b": break;
    case "a": break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_detects_duplicate_boolean_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_detects_duplicate_boolean_case.ds",
            r#"
let x = true;
switch (x) {
    case true: break;
    case false: break;
    case true: break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_allows_unique_cases() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_allows_unique_cases.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 3: break;
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-case");
    }

    #[test]
    fn test_ignores_match_expression() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_ignores_match_expression.ds",
            r#"
let x = 1;
match (x) {
    1 => 1
    2 => 2
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-case");
    }
}
