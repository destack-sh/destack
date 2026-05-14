use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::block_is_empty_without_comment;
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of cases in a switch statement.
    ///
    /// Switch statements with many cases can be hard to read and maintain.
    /// Consider using a lookup table, polymorphism, or splitting the logic.
    #[lint(
        id = "max-switch-cases",
        code = "LX012",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxSwitchCases,
    "Limit cases per switch"
}

impl LintRule for MaxSwitchCases {
    fn meta(&self) -> &'static LintMeta {
        MaxSwitchCases::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_switch_cases = ctx.options.complexity.max_switch_cases;

        // check each switch expression against non-empty non-default case count
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Match { form, cases, .. } = ctx.dir.get(node_id) else {
                continue;
            };
            if *form != dir::MatchForm::Switch {
                continue;
            }

            // count only non-default cases with executable body content
            let case_count = cases
                .iter()
                .copied()
                .filter(|case_id| switch_case_counts(ctx.dir.tree(), *case_id))
                .count();
            if case_count > max_switch_cases {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintReport::new(
                        MAX_SWITCH_CASES.id,
                        MAX_SWITCH_CASES.code,
                        MAX_SWITCH_CASES.category,
                        severity,
                        format!("switch has {case_count} cases (max {max_switch_cases})"),
                        ctx.dir.get_span(node_id),
                    )
                    .label("consider using a lookup table or refactoring"),
                );
            }
        }
    }
}

/// Return true when one switch case counts toward the max-switch-cases limit.
fn switch_case_counts(tree: &dir::Tree, case_id: dir::LocalNodeId<dir::MatchCase>) -> bool {
    // resolve selector and skip default cases
    let case = tree.get(case_id);
    if case.selector().is_default() {
        return false;
    }

    // require non-empty case body content
    match case {
        dir::MatchCase::Expression { body, .. } => expression_has_case_content(tree, *body),
        dir::MatchCase::Block { body, .. } => !block_is_empty_without_comment(tree, *body),
    }
}

/// Return true when one case expression body has executable content.
fn expression_has_case_content(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = tree.get(expression_id);
    if let dir::Expression::Block(block_id) = expression {
        return !block_is_empty_without_comment(tree, *block_id);
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_cases() {
        let test = TestProgram::for_rule_without_prelude(MaxSwitchCases)
            .with_options(|options| options.complexity.max_switch_cases = 5);
        let result = test.lint(
            "max_switch_cases/test_detects_too_many_cases.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxSwitchCases);
        let result = test.lint(
            "max_switch_cases/test_allows_few_cases.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxSwitchCases)
            .with_options(|options| options.complexity.max_switch_cases = 3);
        let result = test.lint(
            "max_switch_cases/test_allows_exactly_at_limit.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxSwitchCases)
            .with_options(|options| options.complexity.max_switch_cases = 2);
        // match expressions are not switch statements
        let result = test.lint(
            "max_switch_cases/test_ignores_match_expression.ds",
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

    #[test]
    fn test_excludes_default_case_from_count() {
        let test = TestProgram::for_rule_without_prelude(MaxSwitchCases)
            .with_options(|options| options.complexity.max_switch_cases = 1);
        let result = test.lint(
            "max_switch_cases/test_excludes_default_case_from_count.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    default: break;
}
"#,
        );
        test.result(result).assert_no_lint("max-switch-cases");
    }

    #[test]
    fn test_excludes_empty_cases_from_count() {
        let test = TestProgram::for_rule_without_prelude(MaxSwitchCases)
            .with_options(|options| options.complexity.max_switch_cases = 1);
        let result = test.lint(
            "max_switch_cases/test_excludes_empty_cases_from_count.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("max-switch-cases");
    }
}
