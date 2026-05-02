use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_is_else_if_branch, expression_is_equal, if_expression_branch_chain,
};
use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow identical if/else branches.
    ///
    /// When the if and else branches have identical code, the conditional
    /// is pointless and indicates a copy-paste error or unfinished logic.
    #[lint(
        id = "no-identical-branches",
        code = "LU018",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoIdenticalBranches,
    "Disallow identical if/else branches"
}

impl LintRule for NoIdenticalBranches {
    fn meta(&self) -> &'static LintMeta {
        NoIdenticalBranches::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::If { kind, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // statement-style if chains:
            // only evaluate the chain root, not nested else-if children
            if *kind == ast::IfKind::If
                && expression_is_else_if_branch(ctx.tree, ctx.parents, node_id)
            {
                continue;
            }

            // normalize branches and require a complete conditional
            let Some(branch_chain) = if_expression_branch_chain(ctx.tree, node_id) else {
                continue;
            };
            if !branch_chain.ends_with_else || branch_chain.branch_expressions.len() < 2 {
                continue;
            }

            // report only when every branch body is structurally identical
            let first_branch = branch_chain.branch_expressions[0];
            let branches_are_identical = branch_chain
                .branch_expressions
                .iter()
                .skip(1)
                .all(|branch_id| expression_is_equal(ctx, first_branch, *branch_id));
            if !branches_are_identical {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintReport::new(
                    NO_IDENTICAL_BRANCHES.id,
                    NO_IDENTICAL_BRANCHES.code,
                    NO_IDENTICAL_BRANCHES.category,
                    severity,
                    "identical conditional branches",
                    ctx.tree.get_span(node_id),
                )
                .label("this conditional evaluates to the same branch body"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_identical_branches() {
        let test = TestProgram::for_rule_without_prelude(NoIdenticalBranches);
        let result = test.lint_ast(
            "no_identical_branches/test_detects_identical_branches.ds",
            r#"
if (x) {
    doSomething();
} else {
    doSomething();
}
"#,
        );
        test.result(result).assert_lint("no-identical-branches");
    }

    #[test]
    fn test_allows_different_branches() {
        let test = TestProgram::for_rule_without_prelude(NoIdenticalBranches);
        let result = test.lint_ast(
            "no_identical_branches/test_allows_different_branches.ds",
            r#"
if (x) {
    doA();
} else {
    doB();
}
"#,
        );
        test.result(result).assert_no_lint("no-identical-branches");
    }

    #[test]
    fn test_allows_if_without_else() {
        let test = TestProgram::for_rule_without_prelude(NoIdenticalBranches);
        let result = test.lint_ast(
            "no_identical_branches/test_allows_if_without_else.ds",
            r#"
if (x) {
    doSomething();
}
"#,
        );
        test.result(result).assert_no_lint("no-identical-branches");
    }

    #[test]
    fn test_detects_identical_ternary() {
        let test = TestProgram::for_rule_without_prelude(NoIdenticalBranches);
        let result = test.lint_ast(
            "no_identical_branches/test_detects_identical_ternary.ds",
            r#"
let x = cond ? value : value;
"#,
        );
        test.result(result).assert_lint("no-identical-branches");
    }

    #[test]
    fn test_detects_identical_else_if_chain() {
        let test = TestProgram::for_rule_without_prelude(NoIdenticalBranches);
        let result = test.lint_ast(
            "no_identical_branches/test_detects_identical_else_if_chain.ds",
            r#"
if (a) {
    first();
} else if (b) {
    first();
} else {
    first();
}
"#,
        );
        test.result(result)
            .assert_lint_count("no-identical-branches", 1);
    }

    #[test]
    fn test_allows_else_if_chain_when_not_all_identical() {
        let test = TestProgram::for_rule_without_prelude(NoIdenticalBranches);
        let result = test.lint_ast(
            "no_identical_branches/test_allows_else_if_chain_when_not_all_identical.ds",
            r#"
if (a) {
    first();
} else if (b) {
    second();
} else {
    second();
}
"#,
        );
        test.result(result).assert_no_lint("no-identical-branches");
    }
}
