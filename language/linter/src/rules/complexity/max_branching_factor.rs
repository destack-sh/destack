use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of branches in a single conditional.
    ///
    /// Conditionals with many branches are harder to read and understand.
    /// Consider using a lookup table, early returns, or breaking into smaller functions.
    #[lint(
        id = "max-branching-factor",
        code = "LX003",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxBranchingFactor,
    "Limit branches in conditionals"
}

impl LintRule for MaxBranchingFactor {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxBranchingFactor::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_branches = ctx.options.max_branching_factor;

        // check if expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            match expression {
                // count if-else-if chains
                ast::Expression::If { .. } => {
                    let branch_count = count_if_branches(ctx, node_id);
                    if branch_count > max_branches {
                        report_violation(ctx, meta, node_id, branch_count, max_branches, "if");
                    }
                }

                // count match arms
                ast::Expression::Match { cases, .. } => {
                    let branch_count = cases.len();
                    if branch_count > max_branches {
                        report_violation(ctx, meta, node_id, branch_count, max_branches, "match");
                    }
                }

                _ => {}
            }
        }
    }
}

/// Count branches in an if-else-if chain.
fn count_if_branches(
    ctx: &LintModuleAstContext<'_>,
    node_id: ast::LocalNodeId<ast::Expression>,
) -> usize {
    let expression = ctx.tree.get(node_id);
    let ast::Expression::If {
        else_expression, ..
    } = expression
    else {
        return 0;
    };

    // start with 1 for the initial if branch
    let mut count = 1;

    // follow else-if chain
    if let Some(else_id) = else_expression {
        let else_expr = ctx.tree.get(*else_id);

        // check if else is another if (else-if)
        if matches!(else_expr, ast::Expression::If { .. }) {
            count += count_if_branches(ctx, *else_id);
        } else {
            // final else branch
            count += 1;
        }
    }

    count
}

/// Report a branching factor violation.
fn report_violation(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    node_id: ast::LocalNodeId<ast::Expression>,
    branch_count: usize,
    max_branches: usize,
    kind: &str,
) {
    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    ctx.report(
        LintDiagnostic::new(
            MAX_BRANCHING_FACTOR.id,
            MAX_BRANCHING_FACTOR.code,
            MAX_BRANCHING_FACTOR.category,
            severity,
            format!("{kind} has {branch_count} branches (max {max_branches})"),
            ctx.module.file_id,
            ctx.tree.get_span(node_id),
        )
        .with_label("consider simplifying this conditional"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_many_if_branches() {
        let test = TestProgram::for_rule_without_prelude(MaxBranchingFactor);
        let result = test.lint_ast(
            "max_branching_factor/test_flags_many_if_branches.ds",
            r#"
function check(x: int32) {
    if (x == 1) {
        return "one";
    } else if (x == 2) {
        return "two";
    } else if (x == 3) {
        return "three";
    } else if (x == 4) {
        return "four";
    } else if (x == 5) {
        return "five";
    } else if (x == 6) {
        return "six";
    } else if (x == 7) {
        return "seven";
    } else if (x == 8) {
        return "eight";
    } else if (x == 9) {
        return "nine";
    } else if (x == 10) {
        return "ten";
    } else {
        return "other";
    }
}
"#,
        );
        test.result(result).assert_lint("max-branching-factor");
    }

    #[test]
    fn test_flags_many_match_arms() {
        let test = TestProgram::for_rule_without_prelude(MaxBranchingFactor);
        let result = test.lint_ast(
            "max_branching_factor/test_flags_many_match_arms.ds",
            r#"
function check(x: int32) {
    match (x) {
        1 => console.log("one")
        2 => console.log("two")
        3 => console.log("three")
        4 => console.log("four")
        5 => console.log("five")
        6 => console.log("six")
        7 => console.log("seven")
        8 => console.log("eight")
        9 => console.log("nine")
        10 => console.log("ten")
        _ => console.log("other")
    }
}
"#,
        );
        test.result(result).assert_lint("max-branching-factor");
    }

    #[test]
    fn test_allows_few_if_branches() {
        let test = TestProgram::for_rule_without_prelude(MaxBranchingFactor);
        let result = test.lint_ast(
            "max_branching_factor/test_allows_few_if_branches.ds",
            r#"
function check(x: int32) {
    if (x == 1) {
        return "one";
    } else if (x == 2) {
        return "two";
    } else {
        return "other";
    }
}
"#,
        );
        test.result(result).assert_no_lint("max-branching-factor");
    }

    #[test]
    fn test_allows_few_match_arms() {
        let test = TestProgram::for_rule_without_prelude(MaxBranchingFactor);
        let result = test.lint_ast(
            "max_branching_factor/test_allows_few_match_arms.ds",
            r#"
function check(x: int32) {
    match (x) {
        1 => console.log("one")
        2 => console.log("two")
        _ => console.log("other")
    }
}
"#,
        );
        test.result(result).assert_no_lint("max-branching-factor");
    }

    #[test]
    fn test_allows_simple_if() {
        let test = TestProgram::for_rule_without_prelude(MaxBranchingFactor);
        let result = test.lint_ast(
            "max_branching_factor/test_allows_simple_if.ds",
            r#"
function check(x: boolean) {
    if (x) {
        return "yes";
    }
    return "no";
}
"#,
        );
        test.result(result).assert_no_lint("max-branching-factor");
    }
}
