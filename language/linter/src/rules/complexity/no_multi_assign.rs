use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow chained assignment expressions.
    ///
    /// Chained assignments like `a = b = c` can be confusing about which
    /// variables are being modified. Write each assignment on its own line
    /// for clarity.
    #[lint(
        id = "no-multi-assign",
        code = "LX007",
        category = Complexity,
        level = Ast,
        fixable = No,
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
                ctx.report(
                    LintDiagnostic::new(
                        NO_MULTI_ASSIGN.id,
                        NO_MULTI_ASSIGN.code,
                        NO_MULTI_ASSIGN.category,
                        severity,
                        "chained assignment expression",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("write each assignment on its own line"),
                );
            }
        }
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
        let test = TestProgram::for_rule(NoMultiAssign);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(NoMultiAssign);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(NoMultiAssign);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(NoMultiAssign);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(NoMultiAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
let a = 1;
let b = 2;
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }

    #[test]
    fn test_allows_compound_assignment() {
        let test = TestProgram::for_rule(NoMultiAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
let a = 1;
a += 2;
"#,
        );
        test.result(result).assert_no_lint("no-multi-assign");
    }
}
