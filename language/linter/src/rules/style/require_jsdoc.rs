use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require documentation on public items.
    ///
    /// Public functions, types, and other declarations should have
    /// documentation to help users understand their purpose and usage.
    #[lint(
        id = "require-jsdoc",
        code = "LY038",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireJsdoc,
    "Require documentation on public items"
}

impl LintRule for RequireJsdoc {
    fn meta(&self) -> &'static crate::LintMeta {
        RequireJsdoc::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // iterate over Expression nodes to find declarations
        // (annotations are attached to Expression nodes, not Declaration nodes)
        for expr_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Declaration(decl_id) = ctx.tree.get(expr_id) else {
                continue;
            };

            let declaration = ctx.tree.get(*decl_id);

            // check if declaration is exported and lacks documentation
            let (is_exported, decl_type) = match declaration {
                Declaration::Function { descriptor, .. } => {
                    (descriptor.export.is_some(), "function")
                }
                Declaration::Struct { descriptor, .. } => (descriptor.export.is_some(), "struct"),
                Declaration::Class { descriptor, .. } => (descriptor.export.is_some(), "class"),
                Declaration::Enum { descriptor, .. } => (descriptor.export.is_some(), "enum"),
                Declaration::Interface { descriptor, .. } => {
                    (descriptor.export.is_some(), "interface")
                }
                Declaration::Type { descriptor, .. } => (descriptor.export.is_some(), "type"),
                _ => continue,
            };

            if !is_exported {
                continue;
            }

            // check if there's a doc annotation for this expression node
            let has_doc = !ctx.tree.get_docs_for(expr_id.id).is_empty();

            if !has_doc {
                let severity = ctx.get_effective_severity(meta, expr_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        REQUIRE_JSDOC.id,
                        REQUIRE_JSDOC.code,
                        REQUIRE_JSDOC.category,
                        severity,
                        format!("public {decl_type} lacks documentation"),
                        ctx.module.file_id,
                        ctx.tree.get_span(expr_id),
                    )
                    .with_label("add documentation comment"),
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
    fn test_exported_function_without_doc_detected() {
        let test = TestProgram::for_rule_without_builtins(RequireJsdoc);
        let result = test.lint_ast(
            "test.ds",
            r#"
export function foo() {}
"#,
        );
        test.result(result).assert_lint("require-jsdoc");
    }

    #[test]
    fn test_exported_function_with_doc_allowed() {
        let test = TestProgram::for_rule_without_builtins(RequireJsdoc);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Does something important.
export function foo() {}
"#,
        );
        test.result(result).assert_no_lint("require-jsdoc");
    }

    #[test]
    fn test_private_function_without_doc_allowed() {
        let test = TestProgram::for_rule_without_builtins(RequireJsdoc);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("require-jsdoc");
    }
}
