use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow renaming import, export, and destructured assignments to the same name.
    ///
    /// Using `{x: x}` in destructuring is the same as `{x}` and is unnecessarily verbose.
    #[lint(
        id = "no-useless-rename",
        code = "LU008",
        category = Suspicious,
        level = Ast
    )]
    pub NoUselessRename,
    "Disallow useless renaming"
}

impl LintRule for NoUselessRename {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessRename::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // check destructuring pattern fields
        for node_id in ctx.tree.iter_nodes::<ast::PatternField>() {
            let field = ctx.tree.get(node_id);

            // check for Alias pattern where name equals alias
            let ast::PatternField::Alias { name, alias, .. } = field else {
                continue;
            };

            // if name and alias are the same, it's useless
            if name.string() == *alias {
                let name_str: String = ctx.strings.get(name.string()).as_ref().to_string();
                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_RENAME.id,
                        NO_USELESS_RENAME.code,
                        NO_USELESS_RENAME.category,
                        severity,
                        format!("useless rename: `{name_str}: {name_str}` can be `{name_str}`"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this rename is unnecessary"),
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
    fn test_detects_useless_rename_destructure() {
        let test = TestProgram::for_rule(NoUselessRename);
        let result = test.lint_ast(
            "test.ds",
            r#"
const { x: x } = obj
"#,
        );
        test.result(result).assert_lint("no-useless-rename");
    }

    #[test]
    fn test_allows_actual_rename() {
        let test = TestProgram::for_rule(NoUselessRename);
        let result = test.lint_ast(
            "test.ds",
            r#"
const { x: y } = obj
"#,
        );
        test.result(result).assert_no_lint("no-useless-rename");
    }

    #[test]
    fn test_allows_simple_destructure() {
        let test = TestProgram::for_rule(NoUselessRename);
        let result = test.lint_ast(
            "test.ds",
            r#"
const { x } = obj
"#,
        );
        test.result(result).assert_no_lint("no-useless-rename");
    }

    #[test]
    fn test_detects_useless_rename_in_function_param() {
        let test = TestProgram::for_rule(NoUselessRename);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo({ a: a }) {}
"#,
        );
        test.result(result).assert_lint("no-useless-rename");
    }
}
