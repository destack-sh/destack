use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintDiagnostic, LintMeta, LintRule, declare_lint};

declare_lint! {
    /// Disallow namespace declarations.
    ///
    /// TypeScript namespaces are a legacy feature. Use ES modules (import/export)
    /// instead for better tree shaking and standard module semantics.
    #[lint(
        id = "no-namespace",
        code = "LR017",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoNamespace,
    "Disallow namespace declarations"
}

impl LintRule for NoNamespace {
    fn meta(&self) -> &'static LintMeta {
        NoNamespace::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate declarations
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            if !matches!(declaration, Declaration::Namespace { .. }) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_NAMESPACE.id,
                    NO_NAMESPACE.code,
                    NO_NAMESPACE.category,
                    severity,
                    "namespace declaration is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use ES modules instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_namespace() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace);
        let result = test.lint_ast(
            "no_namespace/test_detects_namespace.ts",
            r#"
namespace MyNamespace {
    export const foo = 1;
}
"#,
        );
        test.result(result).assert_lint("no-namespace");
    }

    #[test]
    fn test_allows_module_exports() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace);
        let result = test.lint_ast(
            "no_namespace/test_allows_module_exports.ts",
            r#"
export const foo = 1;
export function bar() {}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }

    #[test]
    fn test_skips_declaration_file_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace);
        let result = test.lint_ast(
            "no_namespace/test_skips_declaration_file_by_default.d.ts",
            r#"
declare namespace External {
    const value: number;
}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }

    #[test]
    fn test_includes_declaration_file_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace).with_options(|options| {
            options.include_declaration_files = true;
        });
        let result = test.lint_ast(
            "no_namespace/test_includes_declaration_file_when_enabled.d.ts",
            r#"
declare namespace External {
    const value: number;
}
"#,
        );
        test.result(result).assert_lint("no-namespace");
    }
}
