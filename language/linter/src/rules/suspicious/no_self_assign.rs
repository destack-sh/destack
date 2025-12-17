use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow assignments where both sides are the same.
    ///
    /// Assignments like `x = x` have no effect and are likely mistakes.
    #[lint(
        id = "no-self-assign",
        code = "LU016",
        category = Suspicious,
        level = Ast
    )]
    pub NoSelfAssign,
    "Disallow self-assignment"
}

impl LintRule for NoSelfAssign {
    fn meta(&self) -> &'static crate::LintMeta {
        NoSelfAssign::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let file = ctx.program.files.get(ctx.module.file_id);
        let source = file.text();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            let ast::Expression::Assign {
                left,
                operator: ast::AssignOperator::Assign,
                right,
            } = expr
            else {
                continue;
            };

            // compare source text of left and right
            let left_span = ctx.tree.get_span(*left);
            let right_span = ctx.tree.get_span(*right);

            let left_text = get_span_text(source, left_span);
            let right_text = get_span_text(source, right_span);

            if left_text == right_text && !left_text.is_empty() {
                ctx.report(
                    LintDiagnostic::new(
                        NO_SELF_ASSIGN.id,
                        NO_SELF_ASSIGN.code,
                        NO_SELF_ASSIGN.category,
                        severity,
                        format!("self-assignment: `{left_text} = {right_text}`"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this assignment has no effect"),
                );
            }
        }
    }
}

/// Get the source text for a span.
fn get_span_text(source: &str, span: destack_source::Span) -> &str {
    let start = span.start as usize;
    let end = span.end as usize;
    if start < source.len() && end <= source.len() && start <= end {
        &source[start..end]
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_simple_self_assign() {
        let test = TestProgram::for_rule(NoSelfAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
x = x
"#,
        );
        test.result(result).assert_lint("no-self-assign");
    }

    #[test]
    fn test_detects_member_self_assign() {
        let test = TestProgram::for_rule(NoSelfAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
obj.x = obj.x
"#,
        );
        test.result(result).assert_lint("no-self-assign");
    }

    #[test]
    fn test_detects_index_self_assign() {
        let test = TestProgram::for_rule(NoSelfAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
arr[0] = arr[0]
"#,
        );
        test.result(result).assert_lint("no-self-assign");
    }

    #[test]
    fn test_allows_different_assignment() {
        let test = TestProgram::for_rule(NoSelfAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
x = y
"#,
        );
        test.result(result).assert_no_lint("no-self-assign");
    }

    #[test]
    fn test_allows_different_member_assignment() {
        let test = TestProgram::for_rule(NoSelfAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
obj.x = obj.y
"#,
        );
        test.result(result).assert_no_lint("no-self-assign");
    }

    #[test]
    fn test_allows_compound_assignment() {
        let test = TestProgram::for_rule(NoSelfAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
x += x
"#,
        );
        test.result(result).assert_no_lint("no-self-assign");
    }
}
