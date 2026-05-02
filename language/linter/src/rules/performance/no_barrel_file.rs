use crate::LintMeta;
use destack_ast::{self as ast, Expression};
use destack_source::FileType;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow barrel files that re-export everything.
    ///
    /// Barrel files (like `index.ts` files that only re-export from other modules)
    /// can hurt tree-shaking and bundle size because bundlers may have difficulty
    /// determining which exports are actually used.
    ///
    /// bad:
    /// ```
    /// export * from "./foo";
    /// export * from "./bar";
    /// export { baz } from "./baz";
    /// ```
    ///
    /// good: Import directly from the source modules.
    #[lint(
        id = "no-barrel-file",
        code = "LP005",
        category = Performance,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoBarrelFile,
    "Disallow barrel files"
}

impl LintRule for NoBarrelFile {
    fn meta(&self) -> &'static LintMeta {
        NoBarrelFile::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        if module_is_declaration_file(ctx) {
            return;
        }

        let meta = self.meta();
        let mut first_value_reexport_id: Option<ast::LocalNodeId<ast::Expression>> = None;
        let mut has_non_reexport_code = false;

        // classify top-level expressions as value re-exports or regular module code
        for &node_id in ctx.roots {
            let expression_id = node_id;
            let expression = ctx.tree.get(expression_id);
            match expression {
                Expression::Export {
                    kind,
                    target,
                    items,
                    ..
                } => {
                    if target.is_some() && export_is_value_reexport(ctx, *kind, items.as_slice()) {
                        if first_value_reexport_id.is_none() {
                            first_value_reexport_id = Some(expression_id);
                        }
                        continue;
                    }

                    has_non_reexport_code = true;
                }
                Expression::Import { .. } => {}
                Expression::Declaration(_) => {
                    // declarations indicate real code
                    has_non_reexport_code = true;
                }
                Expression::Let { .. } => {
                    // let bindings indicate real code
                    has_non_reexport_code = true;
                }
                Expression::LetElse { .. } => {
                    // let else bindings indicate real code
                    has_non_reexport_code = true;
                }
                Expression::Using { .. } => {
                    // using bindings indicate real code
                    has_non_reexport_code = true;
                }
                _ => {
                    // other top-level expressions are rare but indicate non-barrel (?)
                    has_non_reexport_code = true;
                }
            }
        }

        // report files that only re-export value bindings from other modules
        if !has_non_reexport_code && let Some(node_id) = first_value_reexport_id {
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                return;
            }
            ctx.report(
                LintReport::new(
                    NO_BARREL_FILE.id,
                    NO_BARREL_FILE.code,
                    NO_BARREL_FILE.category,
                    severity,
                    "barrel file re-exporting other modules hurts tree-shaking",
                    ctx.tree.get_span(node_id),
                )
                .label("import directly from source modules instead"),
            );
        }
    }
}

/// Return true when one module path points to a declaration file.
fn module_is_declaration_file(ctx: &LintAstContext<'_>) -> bool {
    matches!(
        ctx.file.ty,
        FileType::DestackDeclaration | FileType::TypeScriptDeclaration
    )
}

/// Return true when one export-from clause re-exports runtime values.
fn export_is_value_reexport(
    ctx: &LintAstContext<'_>,
    kind: ast::DependencyKind,
    items: &[ast::LocalNodeId<ast::DependencyItem>],
) -> bool {
    if kind == ast::DependencyKind::Type {
        return false;
    }

    if items.is_empty() {
        return true;
    }

    items.iter().any(|item_id| {
        let item = ctx.tree.get(*item_id);
        match item {
            ast::DependencyItem::Item { kind, .. } => *kind != Some(ast::DependencyKind::Type),
            ast::DependencyItem::Error => true,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_barrel_file_with_star_exports() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export * from "./foo"
export * from "./bar"
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }

    #[test]
    fn test_detects_barrel_file_with_named_exports() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export { foo } from "./foo"
export { bar, baz } from "./bar"
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }

    #[test]
    fn test_detects_mixed_barrel_file() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export * from "./foo"
export { bar } from "./bar"
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }

    #[test]
    fn test_flags_single_reexport() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export * from "./foo"
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_module_with_real_code() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "module.ds",
            r#"
export * from "./utils"

export function main() {
    console.log("hello")
}
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_local_exports() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "module.ds",
            r#"
export function foo() {}
export const bar = 42
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_mixed_local_and_reexport() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "module.ds",
            r#"
export { foo } from "./foo"
export function bar() {}
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_type_only_reexports() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export type * from "./foo";
export type { Bar } from "./bar";
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }

    #[test]
    fn test_allows_declaration_file_reexports() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "index.d.ts",
            r#"
export * from "./foo";
"#,
        );
        test.result(result).assert_no_lint("no-barrel-file");
    }

    #[test]
    fn test_detects_mixed_type_and_value_reexport_items() {
        let test = TestProgram::for_rule_without_prelude(NoBarrelFile);
        let result = test.lint_ast(
            "index.ds",
            r#"
export { foo, type Bar } from "./foo";
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }
}
