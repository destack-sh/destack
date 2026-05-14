use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_starts_nested_declaration_scope;
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow nested switch statements.
    ///
    /// Nested switch statements are hard to read and maintain.
    /// Consider extracting the inner switch to a separate function or using a different control flow structure.
    #[lint(
        id = "no-nested-switch",
        code = "LX021",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNestedSwitch,
    "Disallow nested switch statements"
}

impl LintRule for NoNestedSwitch {
    fn meta(&self) -> &'static LintMeta {
        NoNestedSwitch::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            // only check switch statements, not match expressions
            let dir::Expression::Match { form, .. } = ctx.dir.get(node_id) else {
                continue;
            };
            if *form != dir::MatchForm::Switch {
                continue;
            }
            // check if this switch is nested inside another switch
            if is_nested_in_switch(ctx, node_id) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        NO_NESTED_SWITCH.id,
                        NO_NESTED_SWITCH.code,
                        NO_NESTED_SWITCH.category,
                        severity,
                        "nested switch statement",
                        ctx.dir.get_span(node_id),
                    )
                    .label("consider extracting to a separate function"),
                );
            }
        }
    }
}

/// Check if a switch statement is nested inside another switch.
fn is_nested_in_switch(
    ctx: &LintModuleContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // walk up parent chain and report once any enclosing switch is found
    let mut current = expr_id.id;

    while let Some(parent_raw_id) = ctx.dir.get_parent_id(current) {
        let parent_type = ctx.dir.get_node_type(parent_raw_id);

        // stop at method and static-block owners
        if matches!(parent_type, dir::NodeType::Member | dir::NodeType::Property) {
            return false;
        }

        // skip non-expression nodes
        if parent_type != dir::NodeType::Expression {
            current = parent_raw_id;
            continue;
        }

        let parent_id = dir::LocalNodeId::<dir::Expression>::new(parent_raw_id);
        let parent = ctx.dir.get(parent_id);

        // stop once a nested declaration introduces a new callable scope
        if expression_starts_nested_declaration_scope(parent) {
            return false;
        }

        // check if parent is a switch statement
        if let dir::Expression::Match { form, .. } = parent
            && *form == dir::MatchForm::Switch
        {
            return true;
        }

        current = parent_raw_id;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nested_switch() {
        let test = TestProgram::for_rule_without_prelude(NoNestedSwitch);
        let result = test.lint(
            "no_nested_switch/test_detects_nested_switch.ds",
            r#"
let x = 1;
let y = 2;
switch (x) {
    case 1:
        switch (y) {
            case 1: break;
            case 2: break;
        }
        break;
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_lint("no-nested-switch");
    }

    #[test]
    fn test_allows_single_switch() {
        let test = TestProgram::for_rule_without_prelude(NoNestedSwitch);
        let result = test.lint(
            "no_nested_switch/test_allows_single_switch.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 3: break;
}
"#,
        );
        test.result(result).assert_no_lint("no-nested-switch");
    }

    #[test]
    fn test_allows_switch_in_separate_function() {
        let test = TestProgram::for_rule_without_prelude(NoNestedSwitch);
        let result = test.lint(
            "no_nested_switch/test_allows_switch_in_separate_function.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        function handleCase1(y: int32) {
            switch (y) {
                case 1: break;
                case 2: break;
            }
        }
        handleCase1(2);
        break;
    case 2:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-nested-switch");
    }

    #[test]
    fn test_allows_sequential_switches() {
        let test = TestProgram::for_rule_without_prelude(NoNestedSwitch);
        let result = test.lint(
            "no_nested_switch/test_allows_sequential_switches.ds",
            r#"
let x = 1;
let y = 2;
switch (x) {
    case 1: break;
    case 2: break;
}
switch (y) {
    case 1: break;
    case 2: break;
}
"#,
        );
        test.result(result).assert_no_lint("no-nested-switch");
    }

    #[test]
    fn test_allows_switch_in_nested_lambda_inside_switch() {
        let test = TestProgram::for_rule_without_prelude(NoNestedSwitch);
        let result = test.lint(
            "no_nested_switch/test_allows_switch_in_nested_lambda_inside_switch.ds",
            r#"
let x = 1;
switch (x) {
    case 1:
        const handler = () => {
            switch (x) {
                case 1: break;
                default: break;
            }
        };
        break;
    default:
        break;
}
"#,
        );
        test.result(result).assert_no_lint("no-nested-switch");
    }
}
