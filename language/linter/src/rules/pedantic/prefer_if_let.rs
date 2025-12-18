use destack_ast::{self as ast, MatchCase};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest if-let over single-arm match.
    ///
    /// A match expression with a single pattern arm followed by a wildcard
    /// can be more clearly written as an if-let expression.
    #[lint(
        id = "prefer-if-let",
        code = "LD010",
        category = Pedantic,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub PreferIfLet,
    "Prefer if-let over single-arm match"
}

impl LintRule for PreferIfLet {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferIfLet::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::Match { cases, .. } = expression else {
                continue;
            };

            // look for exactly 2 cases
            if cases.len() != 2 {
                continue;
            }

            // check if second case is a wildcard
            let second_case = ctx.tree.get(cases[1]);
            let is_wildcard = match second_case {
                MatchCase::Block { pattern, .. } | MatchCase::Expression { pattern, .. } => {
                    let pat = ctx.tree.get(*pattern);
                    matches!(pat, ast::Pattern::Wildcard)
                }
            };

            if !is_wildcard {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    PREFER_IF_LET.id,
                    PREFER_IF_LET.code,
                    PREFER_IF_LET.category,
                    severity,
                    "match with single pattern and wildcard can be if-let",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("use if-let instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_single_arm_match_detected() {
        let test = TestProgram::for_rule(PreferIfLet);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(PreferIfLet);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(PreferIfLet);
        let result = test.lint_ast(
            "test.ds",
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
}
