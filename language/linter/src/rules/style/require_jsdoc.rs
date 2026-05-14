use crate::LintMeta;
use destack_dir::{self as dir, Declaration};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_or_declaration_has_doc;
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Require documentation on public items.
    ///
    /// Public functions, types, and other declarations should have
    /// documentation to help users understand their purpose and usage.
    #[lint(
        id = "require-jsdoc",
        code = "LY062",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireJsdoc,
    "Require documentation on public items"
}

impl LintRule for RequireJsdoc {
    fn meta(&self) -> &'static LintMeta {
        RequireJsdoc::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // iterate over Expression nodes to find declarations
        // (annotations are attached to Expression nodes, not Declaration nodes)
        for expr_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Declaration(decl_id) = ctx.dir.get(expr_id) else {
                continue;
            };

            // check if declaration is exported and lacks documentation
            let declaration_id = *decl_id;
            let declaration = ctx.dir.get(declaration_id);
            let (is_exported, decl_type) = match declaration {
                Declaration::Function(declaration) => (declaration.export.is_some(), "function"),
                Declaration::Struct(declaration) => (declaration.export.is_some(), "struct"),
                Declaration::Class(declaration) => (declaration.export.is_some(), "class"),
                Declaration::Enum(declaration) => (declaration.export.is_some(), "enum"),
                Declaration::Interface(declaration) => (declaration.export.is_some(), "interface"),
                Declaration::Type(declaration) => (declaration.export.is_some(), "type"),
                _ => continue,
            };
            if !is_exported {
                continue;
            }

            // doc ownership can live on the wrapper expression or declaration owner
            let has_doc = expression_or_declaration_has_doc(ctx, expr_id, declaration_id);
            if !has_doc {
                let severity = ctx.get_effective_severity(meta, expr_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        REQUIRE_JSDOC.id,
                        REQUIRE_JSDOC.code,
                        REQUIRE_JSDOC.category,
                        severity,
                        format!("public {decl_type} lacks documentation"),
                        ctx.dir.get_span(expr_id),
                    )
                    .label("add documentation comment"),
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
        let test = TestProgram::for_rule_without_prelude(RequireJsdoc);
        let result = test.lint(
            "require_jsdoc/test_exported_function_without_doc_detected.ds",
            r#"
export function foo() {}
"#,
        );
        test.result(result).assert_lint("require-jsdoc");
    }

    #[test]
    fn test_exported_function_with_doc_allowed() {
        let test = TestProgram::for_rule_without_prelude(RequireJsdoc);
        let result = test.lint(
            "require_jsdoc/test_exported_function_with_doc_allowed.ds",
            r#"
/// Does something important.
export function foo() {}
"#,
        );
        test.result(result).assert_no_lint("require-jsdoc");
    }

    #[test]
    fn test_private_function_without_doc_allowed() {
        let test = TestProgram::for_rule_without_prelude(RequireJsdoc);
        let result = test.lint(
            "require_jsdoc/test_private_function_without_doc_allowed.ds",
            r#"
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("require-jsdoc");
    }
}
