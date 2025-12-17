use destack_ast::{self as ast, Mutability};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `var` declarations.
    ///
    /// Use `const` for values that don't change and `let` for loop counters.
    /// In Destack, `var` and `let` are synonyms but `const` is preferred.
    #[lint(
        id = "no-var",
        code = "LY012",
        category = Style,
        level = Ast
    )]
    pub NoVar,
    "Disallow var declarations"
}

impl LintRule for NoVar {
    fn meta(&self) -> &'static crate::LintMeta {
        NoVar::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            if let ast::Expression::Let { mutability, .. } = expr
                && *mutability == Mutability::Mutable
            {
                ctx.report(
                    LintDiagnostic::new(
                        NO_VAR.id,
                        NO_VAR.code,
                        NO_VAR.category,
                        severity,
                        "unexpected `var` declaration",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use `const` instead"),
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
    fn test_detects_var() {
        let test = TestProgram::for_rule(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 1
"#,
        );
        test.result(result).assert_lint("no-var");
    }

    #[test]
    fn test_allows_const() {
        let test = TestProgram::for_rule(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 1
"#,
        );
        test.result(result).assert_no_lint("no-var");
    }

    #[test]
    fn test_detects_var_in_for() {
        let test = TestProgram::for_rule(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (var i = 0; i < 10; i++) {
    console.log(i)
}
"#,
        );
        test.result(result).assert_lint("no-var");
    }
}
