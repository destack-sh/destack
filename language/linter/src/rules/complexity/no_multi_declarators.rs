use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow multiple declarators in a single let/const statement.
    ///
    /// Multiple declarators in a single statement like `let a = 1, b = 2` can be
    /// harder to read and maintain. Each binding should be its own statement for
    /// better clarity and easier modification.
    #[lint(
        id = "no-multi-declarators",
        code = "LX008",
        category = Complexity,
        level = Ast
    )]
    pub NoMultiDeclarators,
    "Disallow multiple declarators in let/const statements"
}

impl LintRule for NoMultiDeclarators {
    fn meta(&self) -> &'static crate::LintMeta {
        NoMultiDeclarators::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Let { declarators, .. } = ctx.tree.get(expression_id) else {
                continue;
            };

            // check if there are multiple declarators
            if declarators.len() > 1 {
                let span = ctx.tree.get_span(expression_id);
                ctx.report(
                    LintDiagnostic::new(
                        NO_MULTI_DECLARATORS.id,
                        NO_MULTI_DECLARATORS.code,
                        NO_MULTI_DECLARATORS.category,
                        severity,
                        "multiple declarators in single statement",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("split into separate statements"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_multiple_declarators_with_let() {
        let test = TestProgram::for_rule(NoMultiDeclarators);
        let result = test.lint_ast(
            "test.ds",
            r#"
let a = 1, b = 2;
"#,
        );
        test.result(result).assert_lint("no-multi-declarators");
    }

    #[test]
    fn test_detects_multiple_declarators_with_const() {
        let test = TestProgram::for_rule(NoMultiDeclarators);
        let result = test.lint_ast(
            "test.ds",
            r#"
const a = 1, b = 2, c = 3;
"#,
        );
        test.result(result).assert_lint("no-multi-declarators");
    }

    #[test]
    fn test_detects_multiple_declarators_with_var() {
        let test = TestProgram::for_rule(NoMultiDeclarators);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 1, y = 2;
"#,
        );
        test.result(result).assert_lint("no-multi-declarators");
    }

    #[test]
    fn test_detects_multiple_declarators_without_values() {
        let test = TestProgram::for_rule(NoMultiDeclarators);
        let result = test.lint_ast(
            "test.ds",
            r#"
let a: int32, b: int32;
"#,
        );
        test.result(result).assert_lint("no-multi-declarators");
    }

    #[test]
    fn test_allows_single_declarator_with_let() {
        let test = TestProgram::for_rule(NoMultiDeclarators);
        let result = test.lint_ast(
            "test.ds",
            r#"
let a = 1;
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }

    #[test]
    fn test_allows_single_declarator_with_const() {
        let test = TestProgram::for_rule(NoMultiDeclarators);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 42;
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }

    #[test]
    fn test_allows_multiple_separate_statements() {
        let test = TestProgram::for_rule(NoMultiDeclarators);
        let result = test.lint_ast(
            "test.ds",
            r#"
let a = 1;
let b = 2;
let c = 3;
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }

    #[test]
    fn test_allows_single_declarator_without_value() {
        let test = TestProgram::for_rule(NoMultiDeclarators);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x: int32;
"#,
        );
        test.result(result).assert_no_lint("no-multi-declarators");
    }
}
