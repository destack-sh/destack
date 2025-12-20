use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow wildcard imports.
    ///
    /// Wildcard imports (`import * as foo`) make it harder to track what's
    /// being used and can lead to namespace pollution. Use named imports instead.
    #[lint(
        id = "no-wildcard-imports",
        code = "LR022",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoWildcardImports,
    "Disallow wildcard imports"
}

impl LintRule for NoWildcardImports {
    fn meta(&self) -> &'static crate::LintMeta {
        NoWildcardImports::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::Import { items, .. } = expression else {
                continue;
            };

            // check for namespace/wildcard imports (like `import * as foo`)
            for item_id in items {
                let item = ctx.tree.get(*item_id);
                if item.mode == ast::DependencyMode::Namespace {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    ctx.report(
                        LintDiagnostic::new(
                            NO_WILDCARD_IMPORTS.id,
                            NO_WILDCARD_IMPORTS.code,
                            NO_WILDCARD_IMPORTS.category,
                            severity,
                            "wildcard import",
                            ctx.module.file_id,
                            ctx.tree.get_span(*item_id),
                        )
                        .with_label("use named imports instead"),
                    );
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
    fn test_detects_star_import() {
        let test = TestProgram::for_rule_without_builtins(NoWildcardImports);
        let result = test.lint_ast("test.ts", r#"import * as foo from "foo";"#);
        test.result(result).assert_lint("no-wildcard-imports");
    }

    #[test]
    fn test_allows_named_import() {
        let test = TestProgram::for_rule_without_builtins(NoWildcardImports);
        let result = test.lint_ast("test.ts", r#"import { foo, bar } from "foo";"#);
        test.result(result).assert_no_lint("no-wildcard-imports");
    }

    #[test]
    fn test_allows_default_import() {
        let test = TestProgram::for_rule_without_builtins(NoWildcardImports);
        let result = test.lint_ast("test.ts", r#"import foo from "foo";"#);
        test.result(result).assert_no_lint("no-wildcard-imports");
    }

    #[test]
    fn test_allows_side_effect_import() {
        let test = TestProgram::for_rule_without_builtins(NoWildcardImports);
        let result = test.lint_ast("test.ts", r#"import "foo";"#);
        test.result(result).assert_no_lint("no-wildcard-imports");
    }
}
