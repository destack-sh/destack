use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow renaming import, export, and destructured assignments to the same name.
    ///
    /// Using `{x: x}` in destructuring is the same as `{x}` and is unnecessarily verbose.
    #[lint(
        id = "no-useless-rename",
        code = "LU055",
        category = Suspicious,
        level = Ast,
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessRename,
    "Disallow useless renaming"
}

impl LintRule for NoUselessRename {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessRename::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // check destructuring pattern fields
        for node_id in ctx.tree.iter_nodes::<ast::PatternField>() {
            let field = ctx.tree.get(node_id);

            // check for Alias pattern where name equals alias
            let ast::PatternField::Alias { name, alias, .. } = field else {
                continue;
            };

            // if name and alias are the same, it's useless
            if name.string() == *alias {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let name_str: String = ctx.strings.get(name.string()).as_ref().to_string();
                let field_span = ctx.tree.get_span(node_id);

                // make fix: replace `x: x` with just `x`
                let edits = ctx
                    .edit_builder()
                    .replace(field_span, name_str.clone())
                    .into_edits();
                let fix = LintFix::safe("Remove useless rename").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_RENAME.id,
                        NO_USELESS_RENAME.code,
                        NO_USELESS_RENAME.category,
                        severity,
                        format!("useless rename: `{name_str}: {name_str}` can be `{name_str}`"),
                        ctx.module.file_id,
                        field_span,
                    )
                    .with_label("this rename is unnecessary")
                    .with_fix(fix),
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
        let test = TestProgram::for_rule_without_builtins(NoUselessRename);
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
        let test = TestProgram::for_rule_without_builtins(NoUselessRename);
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
        let test = TestProgram::for_rule_without_builtins(NoUselessRename);
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
        let test = TestProgram::for_rule_without_builtins(NoUselessRename);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo({ a: a }) {}
"#,
        );
        test.result(result).assert_lint("no-useless-rename");
    }

    #[test]
    fn test_fix_useless_rename() {
        let test = TestProgram::for_rule_without_builtins(NoUselessRename);
        let result = test.lint_ast(
            "test.ds",
            r#"
const { x: x } = obj;
"#,
        );
        test.result(result)
            .assert_lint("no-useless-rename")
            .assert_safe_fixed(
                r#"
const { x } = obj;
"#,
            );
    }
}
