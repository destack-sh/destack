use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow namespace declarations.
    ///
    /// TypeScript namespaces are a legacy feature. Use ES modules (import/export)
    /// instead for better tree-shaking and standard module semantics.
    #[lint(
        id = "no-namespace",
        code = "LR011",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoNamespace,
    "Disallow namespace declarations"
}

impl LintRule for NoNamespace {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNamespace::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            if !matches!(declaration, Declaration::Namespace { .. }) {
                continue;
            }

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
        let test = TestProgram::for_rule_without_builtins(NoNamespace);
        let result = test.lint_ast(
            "test.ts",
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
        let test = TestProgram::for_rule_without_builtins(NoNamespace);
        let result = test.lint_ast(
            "test.ts",
            r#"
export const foo = 1;
export function bar() {}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }
}
