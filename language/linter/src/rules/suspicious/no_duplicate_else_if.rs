use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::ExpressionDuplicateTracker;
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate conditions in if-else-if chains.
    ///
    /// Having the same condition in multiple branches of an if-else-if chain
    /// is almost always a bug since the later branch will never be reached.
    #[lint(
        id = "no-duplicate-else-if",
        code = "LU010",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoDuplicateElseIf,
    "Disallow duplicate else-if conditions"
}

impl LintRule for NoDuplicateElseIf {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDuplicateElseIf::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // find all if expressions and check their else-if chains
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::If {
                kind: ast::IfKind::If,
                condition,
                else_expression: Some(else_expr),
                ..
            } = expr
            else {
                continue;
            };

            // only process the top-level if (not else-ifs which are nested)
            if is_else_if_of_parent(ctx, node_id) {
                continue;
            }

            // collect all conditions in the chain
            let mut conditions = Vec::new();
            if let ast::IfCondition::Expression { condition } = condition {
                conditions.push(ConditionEntry {
                    condition_id: *condition,
                    owner_if_id: node_id,
                });
            }
            collect_else_if_conditions(ctx, *else_expr, &mut conditions);

            // check for duplicates using structural comparison
            let mut seen = ExpressionDuplicateTracker::new();
            for condition in conditions {
                if seen
                    .find_duplicate_or_insert(ctx, condition.condition_id)
                    .is_some()
                {
                    let severity = ctx.get_effective_severity(meta, condition.condition_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let mut diagnostic = LintDiagnostic::new(
                        NO_DUPLICATE_ELSE_IF.id,
                        NO_DUPLICATE_ELSE_IF.code,
                        NO_DUPLICATE_ELSE_IF.category,
                        severity,
                        "duplicate condition in if-else-if chain",
                        ctx.module.file_id,
                        ctx.tree.get_span(condition.condition_id),
                    )
                    .with_label("this condition was already checked above");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = no_duplicate_else_if_fix(ctx, condition.owner_if_id)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// One condition entry in an if else-if chain.
#[derive(Clone, Copy)]
struct ConditionEntry {
    /// The condition expression.
    condition_id: ast::LocalNodeId<ast::Expression>,
    /// The if expression that owns the condition.
    owner_if_id: ast::LocalNodeId<ast::Expression>,
}

/// Check if this if expression is the else-if of a parent if.
fn is_else_if_of_parent(
    ctx: &LintModuleAstContext<'_>,
    node_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let Some(parent_id) = ctx.parents.get(node_id) else {
        return false;
    };

    if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return false;
    }

    let parent_expr_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent = ctx.tree.get(parent_expr_id);

    // check if parent is an if with this node as its else branch
    matches!(
        parent,
        ast::Expression::If {
            else_expression: Some(else_id),
            ..
        } if *else_id == node_id
    )
}

/// Collect all conditions from else-if branches.
fn collect_else_if_conditions(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    conditions: &mut Vec<ConditionEntry>,
) {
    let expr = ctx.tree.get(expr_id);
    if let ast::Expression::If {
        kind: ast::IfKind::If,
        condition,
        else_expression,
        ..
    } = expr
    {
        if let ast::IfCondition::Expression { condition } = condition {
            conditions.push(ConditionEntry {
                condition_id: *condition,
                owner_if_id: expr_id,
            });
        }
        if let Some(else_expr) = else_expression {
            collect_else_if_conditions(ctx, *else_expr, conditions);
        }
    }
}

/// Build an unsafe fix for one duplicate else-if by replacing it with its fallback branch.
fn no_duplicate_else_if_fix(
    ctx: &LintModuleAstContext<'_>,
    if_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    // keep fixes for nested else-if expressions only
    if !is_else_if_of_parent(ctx, if_expression_id) {
        return None;
    }

    let expression = ctx.tree.get(if_expression_id);
    let ast::Expression::If {
        else_expression: Some(else_expression_id),
        ..
    } = expression
    else {
        return None;
    };

    let replacement = ctx.get_span_text(ctx.tree.get_span(*else_expression_id));
    let edits = ctx
        .edit_builder()
        .replace(ctx.tree.get_span(if_expression_id), replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Remove duplicate else-if branch").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_else_if() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint_ast(
            "no_duplicate_else_if/test_detects_duplicate_else_if.ds",
            r#"
if (x > 0) {
    a()
} else if (x > 0) {
    b()
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-else-if");
    }

    #[test]
    fn test_detects_duplicate_in_longer_chain() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint_ast(
            "no_duplicate_else_if/test_detects_duplicate_in_longer_chain.ds",
            r#"
if (x > 0) {
    a()
} else if (x < 0) {
    b()
} else if (x > 0) {
    c()
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-else-if");
    }

    #[test]
    fn test_fix_rewrites_duplicate_else_if_to_fallback_branch() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint_ast(
            "no_duplicate_else_if/test_fix_rewrites_duplicate_else_if_to_fallback_branch.ds",
            r#"
if (x > 0) {
    a()
} else if (x > 0) {
    b()
} else {
    c()
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-else-if")
            .assert_unsafe_fixed(
                r#"
if (x > 0) {
    a()
} else {
    c()
}
"#,
            );
    }

    #[test]
    fn test_allows_different_conditions() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint_ast(
            "no_duplicate_else_if/test_allows_different_conditions.ds",
            r#"
if (x > 0) {
    a()
} else if (x < 0) {
    b()
} else if (x == 0) {
    c()
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-else-if");
    }

    #[test]
    fn test_allows_simple_if_else() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint_ast(
            "no_duplicate_else_if/test_allows_simple_if_else.ds",
            r#"
if (x > 0) {
    a()
} else {
    b()
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-else-if");
    }

    #[test]
    fn test_no_fix_for_duplicate_terminal_else_if_without_fallback() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint_ast(
            "no_duplicate_else_if/test_no_fix_for_duplicate_terminal_else_if_without_fallback.ds",
            r#"
if (x > 0) {
    a()
} else if (x > 0) {
    b()
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-else-if")
            .assert_has_no_fix("no-duplicate-else-if");
    }
}
