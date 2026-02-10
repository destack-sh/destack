use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow chained assignment expressions.
    ///
    /// Chained assignments like `a = b = c` can be confusing about which
    /// variables are being modified. Write each assignment on its own line
    /// for clarity.
    #[lint(
        id = "no-multi-assign",
        code = "LX019",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoMultiAssign,
    "Disallow chained assignments"
}

impl LintRule for NoMultiAssign {
    fn meta(&self) -> &'static crate::LintMeta {
        NoMultiAssign::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Assign { right, .. } = ctx.tree.get(expression_id) else {
                continue;
            };

            // check if the right-hand side is also an assignment
            if is_assignment(ctx, *right) {
                let severity = ctx.get_effective_severity(meta, expression_id);
                if !severity.is_enabled() {
                    continue;
                }

                let span = ctx.tree.get_span(expression_id);
                let mut diagnostic = LintDiagnostic::new(
                    NO_MULTI_ASSIGN.id,
                    NO_MULTI_ASSIGN.code,
                    NO_MULTI_ASSIGN.category,
                    severity,
                    "chained assignment expression",
                    ctx.module.file_id,
                    span,
                )
                .with_label("write each assignment on its own line");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = no_multi_assign_fix(ctx, expression_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build a safe fix for one chained assignment statement with simple paths.
fn no_multi_assign_fix(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    // keep statement scoped assignments only
    let parent_id = ctx.parents.get(expression_id)?;
    if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return None;
    }
    let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent_expression = ctx.tree.get(parent_expression_id);
    let ast::Expression::Statement(statement_id) = parent_expression else {
        return None;
    };
    if *statement_id != expression_id {
        return None;
    }

    // collect chained assignments and final rhs
    let mut left_ids = Vec::new();
    let final_rhs_id = collect_assignment_chain(ctx, expression_id, &mut left_ids)?;
    if left_ids.len() < 2 {
        return None;
    }

    // collect lhs texts and keep simple paths only
    let mut left_texts = Vec::new();
    for left_id in left_ids {
        let left_expression = ctx.tree.get(left_id);
        if !matches!(left_expression, ast::Expression::Path { .. }) {
            return None;
        }

        let left_span = ctx.tree.get_span(left_id);
        left_texts.push(ctx.get_span_text(left_span).to_string());
    }

    let final_rhs_span = ctx.tree.get_span(final_rhs_id);
    let final_rhs_text = ctx.get_span_text(final_rhs_span).to_string();

    // build sequential assignments from inner to outer
    let mut rewrites = Vec::new();
    let mut previous_value = final_rhs_text;
    for left_text in left_texts.into_iter().rev() {
        rewrites.push(format!("{left_text} = {previous_value};"));
        previous_value = left_text;
    }

    let replacement = rewrites.join("\n");
    let statement_span = ctx.tree.get_span(parent_expression_id);
    let edits = ctx
        .edit_builder()
        .replace(statement_span, replacement)
        .into_edits();
    Some(LintFix::safe("Split chained assignment into sequential assignments").with_edits(edits))
}

/// Collect left sides for a chained assignment and return the final rhs expression.
fn collect_assignment_chain(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    left_ids: &mut Vec<ast::LocalNodeId<ast::Expression>>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let expression_id = unwrap_parenthesized_expression(ctx, expression_id);
    let expression = ctx.tree.get(expression_id);
    let ast::Expression::Assign { left, right, .. } = expression else {
        return Some(expression_id);
    };

    left_ids.push(*left);
    collect_assignment_chain(ctx, *right, left_ids)
}

/// Unwrap parenthesized expressions.
fn unwrap_parenthesized_expression(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> ast::LocalNodeId<ast::Expression> {
    let mut current = expression_id;

    // peel parenthesized layers to access nested assignments
    loop {
        let expression = ctx.tree.get(current);
        let ast::Expression::Parenthesized { expression } = expression else {
            return current;
        };
        current = *expression;
    }
}

/// check if an expression is an assignment (possibly wrapped in parentheses)
fn is_assignment(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        ast::Expression::Assign { .. } => true,
        ast::Expression::Parenthesized { expression } => is_assignment(ctx, *expression),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_chained_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_detects_chained_assignment.ds",
            r#"
let a: int32;
let b: int32;
let c: int32;
a = (b = (c = 1));
"#,
        );
        test.result(result).assert_lint("no-multi-assign");
    }

    #[test]
    fn test_detects_simple_chain() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_detects_simple_chain.ds",
            r#"
let a: int32;
let b: int32;
a = (b = 1);
"#,
        );
        test.result(result).assert_lint("no-multi-assign");
    }

    #[test]
    fn test_detects_parenthesized_chain() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_detects_parenthesized_chain.ds",
            r#"
let a: int32;
let b: int32;
a = (b = 1);
"#,
        );
        test.result(result).assert_lint("no-multi-assign");
    }

    #[test]
    fn test_allows_separate_assignments() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_allows_separate_assignments.ds",
            r#"
let a: int32;
let b: int32;
a = 1;
b = 1;
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }

    #[test]
    fn test_allows_assignment_in_declaration() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_allows_assignment_in_declaration.ds",
            r#"
let a = 1;
let b = 2;
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }

    #[test]
    fn test_allows_compound_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_allows_compound_assignment.ds",
            r#"
let a = 1;
a += 2;
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }

    #[test]
    fn test_fix_splits_simple_chained_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_fix_splits_simple_chained_assignment.ds",
            r#"
let a: int32;
let b: int32;
a = (b = 1);
"#,
        );
        test.result(result)
            .assert_lint("no-multi-assign")
            .assert_safe_fixed(
                r#"
let a: int32;
let b: int32;
b = 1;
a = b;
"#,
            );
    }

    #[test]
    fn test_fix_splits_nested_chained_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_fix_splits_nested_chained_assignment.ds",
            r#"
let a: int32;
let b: int32;
let c: int32;
a = (b = (c = 1));
"#,
        );
        test.result(result)
            .assert_lint("no-multi-assign")
            .assert_safe_fixed(
                r#"
let a: int32;
let b: int32;
let c: int32;
c = 1;
b = c;
a = b;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_chained_assignment_used_as_value() {
        let test = TestProgram::for_rule_without_prelude(NoMultiAssign);
        let result = test.lint_ast(
            "no_multi_assign/test_no_fix_for_chained_assignment_used_as_value.ds",
            r#"
let a: int32;
let b: int32;
let x = (a = (b = 1));
"#,
        );
        test.result(result)
            .assert_lint("no-multi-assign")
            .assert_has_no_fix("no-multi-assign");
    }
}
