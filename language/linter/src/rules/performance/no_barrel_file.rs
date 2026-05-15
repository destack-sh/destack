use crate::LintMeta;
use destack_dir::{self as dir, Expression};
use destack_source::FileType;
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

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
        level = Dir,
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        if module_is_declaration_file(ctx) {
            return;
        }

        let meta = self.meta();
        let mut first_value_reexport_id: Option<dir::LocalNodeId<dir::Expression>> = None;
        let mut has_non_reexport_code = false;

        // classify top-level expressions as value re-exports or regular module code
        for node_id in ctx.roots.iter().copied() {
            let expression_id = node_id;
            let expression = ctx.dir.get(expression_id);
            match expression {
                Expression::Export {
                    form,
                    target,
                    items,
                    ..
                } => {
                    if target.is_some() && export_is_value_reexport(ctx, *form, items.as_slice()) {
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
                    ctx.dir.get_span(node_id),
                )
                .label("import directly from source modules instead"),
            );
        }
    }
}

/// Return true when one module path points to a declaration file.
fn module_is_declaration_file(ctx: &LintModuleContext<'_>) -> bool {
    matches!(
        ctx.file.ty,
        FileType::DestackDeclaration | FileType::TypeScriptDeclaration
    )
}

/// Return true when one export-from clause re-exports runtime values.
fn export_is_value_reexport(
    ctx: &LintModuleContext<'_>,
    form: dir::DependencyForm,
    items: &[dir::LocalNodeId<dir::DependencyItem>],
) -> bool {
    if form == dir::DependencyForm::Type {
        return false;
    }

    if items.is_empty() {
        return true;
    }

    items.iter().any(|item_id| {
        let item = ctx.dir.get(*item_id);
        match item {
            dir::DependencyItem::Binding { form, .. } => *form != Some(dir::DependencyForm::Type),
            dir::DependencyItem::Error => true,
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "index.ds",
            r#"
export { foo, type Bar } from "./foo";
"#,
        );
        test.result(result).assert_lint("no-barrel-file");
    }
}
