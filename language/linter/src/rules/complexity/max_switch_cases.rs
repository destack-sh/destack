use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of cases in a switch statement.
    ///
    /// Switch statements with many cases can be hard to read and maintain.
    /// Consider using a lookup table, polymorphism, or splitting the logic.
    #[lint(
        id = "max-switch-cases",
        code = "LX012",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxSwitchCases,
    "Limit cases per switch"
}

impl LintRule for MaxSwitchCases {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxSwitchCases::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_switch_cases = ctx.options.max_switch_cases;

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Match { kind, cases, .. } = ctx.tree.get(node_id) else {
                continue;
            };
            if *kind != ast::MatchKind::Switch {
                continue;
            }

            let case_count = cases.len();
            if case_count > max_switch_cases {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        MAX_SWITCH_CASES.id,
                        MAX_SWITCH_CASES.code,
                        MAX_SWITCH_CASES.category,
                        severity,
                        format!("switch has {case_count} cases (max {max_switch_cases})"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("consider using a lookup table or refactoring"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_cases() {
        let test = TestProgram::for_rule_without_builtins(MaxSwitchCases)
            .with_options(|options| options.max_switch_cases = 5);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 3: break;
    case 4: break;
    case 5: break;
    case 6: break;
}
"#,
        );
        test.result(result).assert_lint("max-switch-cases");
    }

    #[test]
    fn test_allows_few_cases() {
        let test = TestProgram::for_rule_without_builtins(MaxSwitchCases);
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
        test.result(result).assert_no_lint("max-switch-cases");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_builtins(MaxSwitchCases)
            .with_options(|options| options.max_switch_cases = 3);
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
        test.result(result).assert_no_lint("max-switch-cases");
    }

    #[test]
    fn test_ignores_match_expression() {
        let test = TestProgram::for_rule_without_builtins(MaxSwitchCases)
            .with_options(|options| options.max_switch_cases = 2);
        // match expressions are not switch statements
        let result = test.lint_ast(
            "test.ds",
            r#"
function testMatch(x: int32): int32 {
    match (x) {
        1 => 1
        2 => 2
        3 => 3
        4 => 4
        5 => 5
    }
}
"#,
        );
        test.result(result).assert_no_lint("max-switch-cases");
    }
}
