use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_starts_nested_declaration_scope;
use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow nested switch statements.
    ///
    /// Nested switch statements are hard to read and maintain.
    /// Consider extracting the inner switch to a separate function or using a different control flow structure.
    #[lint(
        id = "no-nested-switch",
        code = "LX021",
        category = Complexity,
        level = Ast,
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

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // only check switch statements, not match expressions
            let ast::Expression::Match { form, .. } = ctx.tree.get(node_id) else {
                continue;
            };
            if *form != ast::MatchForm::Switch {
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
                        ctx.tree.get_span(node_id),
                    )
                    .label("consider extracting to a separate function"),
                );
            }
        }
    }
}

/// Check if a switch statement is nested inside another switch.
fn is_nested_in_switch(
    ctx: &LintAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // walk up parent chain and report once any enclosing switch is found
    let mut current = expr_id.id;

    while let Some(parent_raw_id) = ctx.parents.get_by_id(current) {
        let parent_type = ctx.tree.get_node_type(parent_raw_id);

        // stop at method and static-block owners
        if matches!(parent_type, ast::NodeType::Member | ast::NodeType::Property) {
            return false;
        }

        // skip non-expression nodes
        if parent_type != ast::NodeType::Expression {
            current = parent_raw_id;
            continue;
        }

        let parent_id = ast::LocalNodeId::<ast::Expression>::new(parent_raw_id);
        let parent = ctx.tree.get(parent_id);

        // stop once a nested declaration introduces a new callable scope
        if expression_starts_nested_declaration_scope(parent) {
            return false;
        }

        // check if parent is a switch statement
        if let ast::Expression::Match { form, .. } = parent
            && *form == ast::MatchForm::Switch
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
