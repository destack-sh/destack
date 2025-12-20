use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::is_equal;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate case labels in switch statements.
    ///
    /// Having duplicate case labels in a switch statement is almost always a mistake.
    /// Only the first matching case will be executed.
    #[lint(
        id = "no-duplicate-case",
        code = "LC009",
        category = Correctness,
        level = Ast,
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
            let mut seen: Vec<ast::LocalNodeId<ast::Expression>> = Vec::new();
            for case_id in cases {
                let case = ctx.tree.get(*case_id);
                let pattern_id = match case {
                    ast::MatchCase::Expression { pattern, .. } => pattern,
                    ast::MatchCase::Block { pattern, .. } => pattern,
                };

                let pattern = ctx.tree.get(*pattern_id);
                let Some(expr_id) = pattern_to_expression(pattern) else {
                    continue;
                };

                // check against all previously seen expressions
                let is_duplicate = seen.iter().any(|&prev| is_equal(ctx, prev, expr_id));

                if is_duplicate {
                    let severity = ctx.get_effective_severity(meta, node_id);
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
                } else {
                    seen.push(expr_id);
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
        let test = TestProgram::for_rule_without_builtins(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoDuplicateCase);
        let result = test.lint_ast(
            "test.ds",
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
