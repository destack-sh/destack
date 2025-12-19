use destack_ast::{self as ast, DependencyKind, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce consistent type import style.
    ///
    /// Prefer `import type { Foo }` over `import { type Foo }` for type-only imports.
    /// This makes it clearer that the import is only used for type checking.
    #[lint(
        id = "consistent-type-imports",
        code = "LY024",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub ConsistentTypeImports,
    "Enforce consistent type import style"
}

impl LintRule for ConsistentTypeImports {
    fn meta(&self) -> &'static crate::LintMeta {
        ConsistentTypeImports::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for import expressions
            let Expression::Import { kind, items, .. } = expr else {
                continue;
            };

            // if the import is already `import type`, skip
            if *kind == DependencyKind::Type {
                continue;
            }

            // check if ALL items are type imports
            let all_type_imports = !items.is_empty()
                && items.iter().all(|item_id| {
                    let item = ctx.tree.get(*item_id);
                    item.kind == Some(DependencyKind::Type)
                });

            if all_type_imports {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        CONSISTENT_TYPE_IMPORTS.id,
                        CONSISTENT_TYPE_IMPORTS.code,
                        CONSISTENT_TYPE_IMPORTS.category,
                        severity,
                        "use `import type { ... }` for type-only imports",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("prefer top-level `import type`"),
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
    fn test_allows_import_type() {
        let test = TestProgram::for_rule(ConsistentTypeImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import type { Foo, Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_detects_inline_type_imports() {
        let test = TestProgram::for_rule(ConsistentTypeImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { type Foo, type Bar } from "foo"
"#,
        );
        test.result(result).assert_lint("consistent-type-imports");
    }

    #[test]
    fn test_allows_mixed_imports() {
        let test = TestProgram::for_rule(ConsistentTypeImports);
        // mixed imports are allowed (can't use `import type` for these)
        let result = test.lint_ast(
            "test.ds",
            r#"
import { Foo, type Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_allows_value_imports() {
        let test = TestProgram::for_rule(ConsistentTypeImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { foo, bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_allows_namespace_import() {
        let test = TestProgram::for_rule(ConsistentTypeImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import * as foo from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }
}
