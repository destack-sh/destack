use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expressions_equal;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate conditions in if-else-if chains.
    ///
    /// Having the same condition in multiple branches of an if-else-if chain
    /// is almost always a bug since the later branch will never be reached.
    #[lint(
        id = "no-dupe-else-if",
        code = "LU011",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoDupeElseIf,
    "Disallow duplicate else-if conditions"
}

impl LintRule for NoDupeElseIf {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDupeElseIf::meta()
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
            let mut conditions = vec![*condition];
            collect_else_if_conditions(ctx, *else_expr, &mut conditions);

            // check for duplicates using structural comparison
            for i in 0..conditions.len() {
                for j in (i + 1)..conditions.len() {
                    if expressions_equal(ctx, conditions[i], conditions[j]) {
                        let severity = ctx.get_effective_severity(meta, conditions[j]);
                        if !severity.is_enabled() {
                            continue;
                        }

                        ctx.report(
                            LintDiagnostic::new(
                                NO_DUPE_ELSE_IF.id,
                                NO_DUPE_ELSE_IF.code,
                                NO_DUPE_ELSE_IF.category,
                                severity,
                                "duplicate condition in if-else-if chain",
                                ctx.module.file_id,
                                ctx.tree.get_span(conditions[j]),
                            )
                            .with_label("this condition was already checked above"),
                        );
                    }
                }
            }
        }
    }
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
    conditions: &mut Vec<ast::LocalNodeId<ast::Expression>>,
) {
    let expr = ctx.tree.get(expr_id);
    if let ast::Expression::If {
        kind: ast::IfKind::If,
        condition,
        else_expression,
        ..
    } = expr
    {
        conditions.push(*condition);
        if let Some(else_expr) = else_expression {
            collect_else_if_conditions(ctx, *else_expr, conditions);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_else_if() {
        let test = TestProgram::for_rule(NoDupeElseIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x > 0) {
    a()
} else if (x > 0) {
    b()
}
"#,
        );
        test.result(result).assert_lint("no-dupe-else-if");
    }

    #[test]
    fn test_detects_duplicate_in_longer_chain() {
        let test = TestProgram::for_rule(NoDupeElseIf);
        let result = test.lint_ast(
            "test.ds",
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
        test.result(result).assert_lint("no-dupe-else-if");
    }

    #[test]
    fn test_allows_different_conditions() {
        let test = TestProgram::for_rule(NoDupeElseIf);
        let result = test.lint_ast(
            "test.ds",
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
        test.result(result).assert_no_lint("no-dupe-else-if");
    }

    #[test]
    fn test_allows_simple_if_else() {
        let test = TestProgram::for_rule(NoDupeElseIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x > 0) {
    a()
} else {
    b()
}
"#,
        );
        test.result(result).assert_no_lint("no-dupe-else-if");
    }
}
