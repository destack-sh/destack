use destack_ast::{self as ast, DependencyMode, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `export * from "..."`.
    ///
    /// Re-exporting everything from a module can make it hard to track what
    /// is being exported and can lead to unexpected exports. It also makes
    /// tree-shaking less effective and can increase bundle sizes.
    #[lint(
        id = "no-re-export-all",
        code = "LR015",
        category = Restriction,
        level = Ast
    )]
    pub NoReExportAll,
    "Disallow export * from"
}

impl LintRule for NoReExportAll {
    fn meta(&self) -> &'static crate::LintMeta {
        NoReExportAll::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check export expressions
            let Expression::Export {
                target: Some(_target),
                items,
                ..
            } = expression
            else {
                continue;
            };

            // check if any item is a namespace re-export (export *)
            for item_id in items {
                let item = ctx.tree.get(*item_id);
                if item.mode == DependencyMode::Namespace && item.alias.is_none() {
                    // this is `export * from "..."` (not `export * as foo from "..."`)
                    ctx.report(
                        LintDiagnostic::new(
                            NO_RE_EXPORT_ALL.id,
                            NO_RE_EXPORT_ALL.code,
                            NO_RE_EXPORT_ALL.category,
                            severity,
                            "avoid using export * from",
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("use named exports instead"),
                    );
                    break; // only report once per export statement
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_export_star() {
        let test = TestProgram::for_rule(NoReExportAll);
        let result = test.lint_ast(
            "test.ds",
            r#"
export * from "./module"
"#,
        );
        test.result(result).assert_lint("no-re-export-all");
    }

    #[test]
    fn test_detects_export_star_from_path() {
        let test = TestProgram::for_rule(NoReExportAll);
        let result = test.lint_ast(
            "test.ds",
            r#"
export * from "some/path"
"#,
        );
        test.result(result).assert_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_export_star_as() {
        let test = TestProgram::for_rule(NoReExportAll);
        let result = test.lint_ast(
            "test.ds",
            r#"
export * as utils from "./utils"
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_named_exports() {
        let test = TestProgram::for_rule(NoReExportAll);
        let result = test.lint_ast(
            "test.ds",
            r#"
export { foo, bar } from "./module"
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_default_export() {
        let test = TestProgram::for_rule(NoReExportAll);
        let result = test.lint_ast(
            "test.ds",
            r#"
export default foo
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_local_exports() {
        let test = TestProgram::for_rule(NoReExportAll);
        let result = test.lint_ast(
            "test.ds",
            r#"
export { foo, bar }
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }
}
