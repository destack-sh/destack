use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow nested switch statements.
    ///
    /// Nested switch statements are hard to read and maintain. Consider
    /// extracting the inner switch to a separate function or using a
    /// different control flow structure.
    #[lint(
        id = "no-nested-switch",
        code = "LX020",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNestedSwitch,
    "Disallow nested switch statements"
}

impl LintRule for NoNestedSwitch {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNestedSwitch::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // only check switch statements, not match expressions
            let ast::Expression::Match { kind, .. } = ctx.tree.get(node_id) else {
                continue;
            };
            if *kind != ast::MatchKind::Switch {
                continue;
            }
            // check if this switch is nested inside another switch
            if is_nested_in_switch(ctx, node_id) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_NESTED_SWITCH.id,
                        NO_NESTED_SWITCH.code,
                        NO_NESTED_SWITCH.category,
                        severity,
                        "nested switch statement",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("consider extracting to a separate function"),
                );
            }
        }
    }
}

/// Check if a switch statement is nested inside another switch.
fn is_nested_in_switch(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let mut current = expr_id.id;

    while let Some(parent_raw_id) = ctx.parents.get_by_id(current) {
        let parent_type = ctx.tree.get_node_type(parent_raw_id);

        // Skip non-expression nodes
        if parent_type != ast::NodeType::Expression {
            current = parent_raw_id;
            continue;
        }

        let parent_id = ast::LocalNodeId::<ast::Expression>::new(parent_raw_id);
        let parent = ctx.tree.get(parent_id);

        // check if parent is a switch statement
        if let ast::Expression::Match { kind, .. } = parent
            && *kind == ast::MatchKind::Switch
        {
            return true;
        }

        // Stop at function boundaries
        if let ast::Expression::Declaration(decl_id) = parent {
            let decl = ctx.tree.get(*decl_id);
            if matches!(decl, ast::Declaration::Function { .. }) {
                return false;
            }
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
        let test = TestProgram::for_rule_without_builtins(NoNestedSwitch);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoNestedSwitch);
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
        test.result(result).assert_no_lint("no-nested-switch");
    }

    #[test]
    fn test_allows_switch_in_separate_function() {
        let test = TestProgram::for_rule_without_builtins(NoNestedSwitch);
        let result = test.lint_ast(
            "test.ds",
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
        // The inner switch is in a separate function, so it's not nested
        test.result(result).assert_no_lint("no-nested-switch");
    }

    #[test]
    fn test_allows_sequential_switches() {
        let test = TestProgram::for_rule_without_builtins(NoNestedSwitch);
        let result = test.lint_ast(
            "test.ds",
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
}
