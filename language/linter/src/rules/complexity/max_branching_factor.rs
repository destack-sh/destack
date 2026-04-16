use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_is_else_if_branch;
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of branches in one conditional.
    ///
    /// Conditionals with many branches are harder to read and understand.
    /// Consider lookup tables, early returns, or splitting logic into helpers.
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
    fn meta(&self) -> &'static LintMeta {
        MaxBranchingFactor::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_branches = ctx.options.complexity.max_branching_factor;

        // scan conditionals and branch based expressions
        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // check top level if chain branch count
            if let ast::Expression::If { .. } = ctx.tree.get(expression_id) {
                if expression_is_else_if_branch(ctx.tree, ctx.parents, expression_id) {
                    continue;
                }

                let branch_count = count_if_chain_branches(ctx, expression_id);
                if branch_count > max_branches {
                    report_branching_violation(
                        ctx,
                        meta,
                        expression_id,
                        "if",
                        branch_count,
                        max_branches,
                    );
                }

                continue;
            }

            // check switch case branch count
            let ast::Expression::Match { cases, .. } = ctx.tree.get(expression_id) else {
                continue;
            };
            let branch_count = cases.len();
            if branch_count > max_branches {
                report_branching_violation(
                    ctx,
                    meta,
                    expression_id,
                    "match",
                    branch_count,
                    max_branches,
                );
            }
        }
    }
}

/// Count branches in one if else chain.
fn count_if_chain_branches(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> usize {
    // resolve one if expression entry point
    let ast::Expression::If {
        else_expression, ..
    } = ctx.tree.get(expression_id)
    else {
        return 0;
    };

    // start with the current if branch
    let mut branch_count = 1;

    // include else if chain and terminal else branch
    let Some(else_expression_id) = else_expression else {
        return branch_count;
    };
    if matches!(
        ctx.tree.get(*else_expression_id),
        ast::Expression::If { .. }
    ) {
        branch_count += count_if_chain_branches(ctx, *else_expression_id);
        return branch_count;
    }

    branch_count + 1
}

/// Report one branching factor violation.
fn report_branching_violation(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    expression_id: ast::LocalNodeId<ast::Expression>,
    expression_kind: &str,
    branch_count: usize,
    max_branches: usize,
) {
    // resolve effective severity
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return;
    }

    // emit one branching overflow diagnostic
    ctx.report(
        LintDiagnostic::new(
            MAX_BRANCHING_FACTOR.id,
            MAX_BRANCHING_FACTOR.code,
            MAX_BRANCHING_FACTOR.category,
            severity,
            format!("{expression_kind} has {branch_count} branches (max {max_branches})"),
            ctx.module.file_id,
            ctx.tree.get_span(expression_id),
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
    fn test_reports_else_if_chain_once() {
        let test = TestProgram::for_rule_without_prelude(MaxBranchingFactor)
            .with_options(|options| options.complexity.max_branching_factor = 2);
        let result = test.lint_ast(
            "max_branching_factor/test_reports_else_if_chain_once.ds",
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
        test.result(result)
            .assert_lint("max-branching-factor")
            .assert_lint_count("max-branching-factor", 1);
    }
}
