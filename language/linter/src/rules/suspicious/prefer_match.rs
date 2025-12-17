use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest using `match` instead of complex if-else-if chains.
    ///
    /// When comparing the same value against multiple possibilities,
    /// a `match` expression is often clearer and ensures exhaustiveness.
    #[lint(
        id = "prefer-match",
        code = "LU020",
        category = Suspicious,
        level = Ast
    )]
    pub PreferMatch,
    "Prefer match over complex if-else-if"
}

// minimum number of else-if branches to trigger the suggestion
const MIN_BRANCHES: usize = 3;

impl LintRule for PreferMatch {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferMatch::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let file = ctx.program.files.get(ctx.module.file_id);
        let source = file.text();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // only check top-level if expressions (not nested else-ifs)
            let ast::Expression::If {
                kind: ast::IfKind::If,
                condition,
                else_expression: Some(else_expr),
                ..
            } = expr
            else {
                continue;
            };

            // skip if this is an else-if of a parent
            if is_else_if_of_parent(ctx, node_id) {
                continue;
            }

            // count branches and check if conditions compare same variable
            let mut branch_count = 1;
            let mut current_else = Some(*else_expr);
            let base_var = get_comparison_var(ctx, source, *condition);

            if base_var.is_none() {
                continue;
            }

            let base_var = base_var.unwrap();

            while let Some(else_id) = current_else {
                let else_expr = ctx.tree.get(else_id);

                if let ast::Expression::If {
                    kind: ast::IfKind::If,
                    condition: else_cond,
                    else_expression: next_else,
                    ..
                } = else_expr
                {
                    // check if this else-if compares the same variable
                    let else_var = get_comparison_var(ctx, source, *else_cond);
                    if else_var.as_deref() != Some(base_var.as_str()) {
                        // different variable, not a good match candidate
                        break;
                    }

                    branch_count += 1;
                    current_else = *next_else;
                } else {
                    // else block (not else-if)
                    branch_count += 1;
                    break;
                }
            }

            if branch_count >= MIN_BRANCHES {
                ctx.report(
                    LintDiagnostic::new(
                        PREFER_MATCH.id,
                        PREFER_MATCH.code,
                        PREFER_MATCH.category,
                        severity,
                        format!(
                            "consider using `match` for this {branch_count}-branch if-else chain"
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("a `match` expression would be clearer here"),
                );
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

    matches!(
        parent,
        ast::Expression::If {
            else_expression: Some(else_id),
            ..
        } if *else_id == node_id
    )
}

/// Get the variable being compared in a condition (if it's a simple equality check).
fn get_comparison_var(
    ctx: &LintModuleAstContext<'_>,
    source: &str,
    condition_id: ast::LocalNodeId<ast::Expression>,
) -> Option<String> {
    let condition = ctx.tree.get(condition_id);

    // handle parenthesized conditions
    if let ast::Expression::Parenthesized { expression } = condition {
        return get_comparison_var(ctx, source, *expression);
    }

    // look for equality comparisons
    let ast::Expression::Binary {
        left,
        operator,
        right,
    } = condition
    else {
        return None;
    };

    // only consider == and === comparisons
    if !matches!(
        operator,
        ast::BinaryOperator::Equal | ast::BinaryOperator::EqualStrict
    ) {
        return None;
    }

    // get the variable from either side (prefer left side)
    let left_span = ctx.tree.get_span(*left);
    let right_span = ctx.tree.get_span(*right);

    // check if left is a simple identifier (Path)
    let left_expr = ctx.tree.get(*left);
    if matches!(left_expr, ast::Expression::Path { .. }) {
        let start = left_span.start as usize;
        let end = left_span.end as usize;
        if start < source.len() && end <= source.len() {
            return Some(source[start..end].to_string());
        }
    }

    // check if right is a simple identifier
    let right_expr = ctx.tree.get(*right);
    if matches!(right_expr, ast::Expression::Path { .. }) {
        let start = right_span.start as usize;
        let end = right_span.end as usize;
        if start < source.len() && end <= source.len() {
            return Some(source[start..end].to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_long_if_else_chain() {
        let test = TestProgram::for_rule(PreferMatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x == 1) {
    a()
} else if (x == 2) {
    b()
} else if (x == 3) {
    c()
}
"#,
        );
        test.result(result).assert_lint("prefer-match");
    }

    #[test]
    fn test_detects_with_else_block() {
        let test = TestProgram::for_rule(PreferMatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x == 1) {
    a()
} else if (x == 2) {
    b()
} else {
    c()
}
"#,
        );
        test.result(result).assert_lint("prefer-match");
    }

    #[test]
    fn test_allows_short_if_else() {
        let test = TestProgram::for_rule(PreferMatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x == 1) {
    a()
} else {
    b()
}
"#,
        );
        test.result(result).assert_no_lint("prefer-match");
    }

    #[test]
    fn test_allows_different_variables() {
        let test = TestProgram::for_rule(PreferMatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x == 1) {
    a()
} else if (y == 2) {
    b()
} else if (z == 3) {
    c()
}
"#,
        );
        test.result(result).assert_no_lint("prefer-match");
    }

    #[test]
    fn test_allows_non_equality_conditions() {
        let test = TestProgram::for_rule(PreferMatch);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x > 1) {
    a()
} else if (x > 2) {
    b()
} else if (x > 3) {
    c()
}
"#,
        );
        test.result(result).assert_no_lint("prefer-match");
    }
}
