use crate::LintMeta;
use destack_core::StringId;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow default exports.
    ///
    /// Named exports are easier to refactor and provide better IDE support.
    /// Consider using named exports instead of default exports.
    #[lint(
        id = "no-default-export",
        code = "LR009",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable
    )]
    pub NoDefaultExport,
    "Disallow default exports"
}

impl LintRule for NoDefaultExport {
    fn meta(&self) -> &'static LintMeta {
        NoDefaultExport::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let default_name = ctx.string_id("default");

        // check default export dependency items in export and re-export expressions
        for (node_id, item) in ctx.dir.iter_nodes_of_type::<dir::DependencyItem>() {
            if !dependency_item_exports_default(ctx, node_id, item, default_name) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.get_span(node_id);
            ctx.report(
                LintReport::new(
                    NO_DEFAULT_EXPORT.id,
                    NO_DEFAULT_EXPORT.code,
                    NO_DEFAULT_EXPORT.category,
                    severity,
                    "default export",
                    span,
                )
                .label("use named exports instead"),
            );
        }

        // check for declarations with export=Default (export default function/class)
        for (node_id, declaration) in ctx.dir.iter_nodes_of_type::<dir::Declaration>() {
            let export = match declaration {
                dir::Declaration::Global(_) => None,
                dir::Declaration::Module(_) => None,
                dir::Declaration::Type(declaration) => declaration.export,
                dir::Declaration::Struct(declaration) => declaration.export,
                dir::Declaration::Class(declaration) => declaration.export,
                dir::Declaration::Enum(declaration) => declaration.export,
                dir::Declaration::Interface(declaration) => declaration.export,
                dir::Declaration::Extension(declaration) => declaration.export,
                dir::Declaration::Function(declaration) => declaration.export,
            };
            if export == Some(dir::ExportKind::Default) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.get_span(node_id);
                let mut diagnostic = LintReport::new(
                    NO_DEFAULT_EXPORT.id,
                    NO_DEFAULT_EXPORT.code,
                    NO_DEFAULT_EXPORT.category,
                    severity,
                    "default export",
                    span,
                )
                .label("use named exports instead");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = default_export_declaration_fix(ctx, node_id)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return true when a dependency item belongs to an export expression and exports `default`.
fn dependency_item_exports_default(
    ctx: &LintModuleContext<'_>,
    item_id: dir::LocalNodeId<dir::DependencyItem>,
    item: &dir::DependencyItem,
    default_name: StringId,
) -> bool {
    let Some(parent_id) = ctx.dir.get_parent(item_id.id) else {
        return false;
    };
    let Ok(parent_id) = parent_id.try_into_typed::<dir::Expression>() else {
        return false;
    };

    // only inspect export nodes, not imports
    let dir::Expression::Export { .. } = ctx.dir.get(parent_id) else {
        return false;
    };

    match item {
        dir::DependencyItem::Binding {
            binding,
            form,
            name,
            alias,
            ..
        } => {
            if form.unwrap_or(dir::DependencyForm::Plain) != dir::DependencyForm::Plain {
                return false;
            }

            dependency_item_export_name(*binding, *name, *alias, default_name) == Some(default_name)
        }
        dir::DependencyItem::Error => false,
    }
}

/// Return the exported name for one dependency item.
fn dependency_item_export_name(
    binding: dir::DependencyBinding,
    name: Option<dir::Name>,
    alias: Option<StringId>,
    default_name: StringId,
) -> Option<StringId> {
    if alias.is_some() {
        return alias;
    }

    match binding {
        dir::DependencyBinding::Named => name.map(|name| name.string()),
        dir::DependencyBinding::Default => name.map(|name| name.string()).or(Some(default_name)),
        dir::DependencyBinding::Namespace => None,
    }
}

/// Build an unsafe fix that rewrites declaration default exports to named exports.
fn default_export_declaration_fix(
    ctx: &LintModuleContext<'_>,
    declaration_id: dir::LocalNodeId<dir::Declaration>,
) -> Option<LintFix> {
    // only rewrite named function and class declarations
    let declaration = ctx.dir.get(declaration_id);
    match declaration {
        dir::Declaration::Function(declaration) => declaration.name?,
        dir::Declaration::Class(declaration) => declaration.name?,
        _ => return None,
    };

    // rewrite `export default` into `export` for declaration forms
    let declaration_span = ctx.get_span(declaration_id);
    let declaration_text = ctx.get_span_text(declaration_span);
    let declaration_text: &str = declaration_text;
    let replacement = declaration_text.replacen("export default ", "export ", 1);
    if replacement == declaration_text {
        return None;
    }

    let edits = ctx
        .edit_builder()
        .replace(declaration_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Rewrite default declaration export to named export").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Lint one target module in a small dependency set and assert there are no compiler errors.
    fn lint_module_with_modules(
        modules: &[(&str, &str)],
        target_path: &str,
    ) -> (TestProgram, Vec<LintReport>) {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let diagnostics = test.lint_module_dir_with_modules(modules, target_path);
        test.check_clean();
        (test, diagnostics)
    }

    #[test]
    fn test_detects_default_export_function() {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let result = test.lint_dir(
            "no_default_export/test_detects_default_export_function.ds",
            "export default function foo() {}",
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-default-export")
            .assert_unsafe_fixed("export function foo() {}\n");
    }

    #[test]
    fn test_detects_default_export_class() {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let result = test.lint_dir(
            "no_default_export/test_detects_default_export_class.ds",
            "export default class Foo {}",
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-default-export")
            .assert_unsafe_fixed("export class Foo {}\n");
    }

    #[test]
    fn test_allows_named_export_function() {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let result = test.lint_dir(
            "no_default_export/test_allows_named_export_function.ds",
            "export function foo() {}",
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-default-export");
    }

    #[test]
    fn test_allows_named_export_class() {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let result = test.lint_dir(
            "no_default_export/test_allows_named_export_class.ds",
            "export class Foo {}",
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-default-export");
    }

    #[test]
    fn test_detects_default_export_identifier() {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let result = test.lint_dir(
            "no_default_export/test_detects_default_export_identifier.ds",
            "const foo = 1;\nexport default foo;",
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-default-export")
            .assert_has_no_fix("no-default-export");
    }

    #[test]
    fn test_mutation_fix_rewrites_default_export_async_function() {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let result = test.lint_dir(
            "no_default_export/test_mutation_fix_rewrites_default_export_async_function.ds",
            "export default async function foo() { return 1; }",
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-default-export")
            .assert_unsafe_fixed(
                r#"
export async function foo() {
    return 1;
}
"#,
            );
    }

    #[test]
    fn test_detects_default_export_alias_specifier() {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let result = test.lint_dir(
            "no_default_export/test_detects_default_export_alias_specifier.ds",
            "const foo: int32 = 1;\nexport { foo as default };",
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-default-export")
            .assert_has_no_fix("no-default-export");
    }

    #[test]
    fn test_detects_remote_default_re_export() {
        let (test, result) = lint_module_with_modules(
            &[
                (
                    "no_default_export/dep.ds",
                    r#"
export const value: int32 = 1;
export default value;
"#,
                ),
                (
                    "no_default_export/test_detects_remote_default_re_export.ds",
                    r#"
export { default } from "./dep.ds";
"#,
                ),
            ],
            "no_default_export/test_detects_remote_default_re_export.ds",
        );

        test.result(result)
            .assert_lint("no-default-export")
            .assert_has_no_fix("no-default-export");
    }

    #[test]
    fn test_detects_remote_default_alias_re_export() {
        let (test, result) = lint_module_with_modules(
            &[
                (
                    "no_default_export/dep.ds",
                    r#"
export const value: int32 = 1;
"#,
                ),
                (
                    "no_default_export/test_detects_remote_default_alias_re_export.ds",
                    r#"
export { value as default } from "./dep.ds";
"#,
                ),
            ],
            "no_default_export/test_detects_remote_default_alias_re_export.ds",
        );

        test.result(result)
            .assert_lint("no-default-export")
            .assert_has_no_fix("no-default-export");
    }

    #[test]
    fn test_allows_named_alias_of_remote_default_re_export() {
        let (test, result) = lint_module_with_modules(
            &[
                (
                    "no_default_export/dep.ds",
                    r#"
export default 1;
"#,
                ),
                (
                    "no_default_export/test_allows_named_alias_of_remote_default_re_export.ds",
                    r#"
export { default as value } from "./dep.ds";
"#,
                ),
            ],
            "no_default_export/test_allows_named_alias_of_remote_default_re_export.ds",
        );

        test.result(result).assert_no_lint("no-default-export");
    }

    #[test]
    fn test_allows_default_import() {
        let (test, result) = lint_module_with_modules(
            &[
                (
                    "no_default_export/dep.ds",
                    r#"
export default 1;
"#,
                ),
                (
                    "no_default_export/test_allows_default_import.ds",
                    r#"
import value from "./dep.ds";
const local = value;
"#,
                ),
            ],
            "no_default_export/test_allows_default_import.ds",
        );

        test.result(result).assert_no_lint("no-default-export");
    }

    #[test]
    fn test_no_fix_for_anonymous_default_function_declaration() {
        let test = TestProgram::for_rule_without_prelude(NoDefaultExport);
        let result = test.lint_dir(
            "no_default_export/test_no_fix_for_anonymous_default_function_declaration.ds",
            "export default function() { return 1; }",
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-default-export")
            .assert_has_no_fix("no-default-export");
    }

    #[test]
    fn test_allows_named_remote_re_export() {
        let (test, result) = lint_module_with_modules(
            &[
                (
                    "no_default_export/dep.ds",
                    r#"
export const value: int32 = 1;
"#,
                ),
                (
                    "no_default_export/test_allows_named_remote_re_export.ds",
                    r#"
export { value } from "./dep.ds";
"#,
                ),
            ],
            "no_default_export/test_allows_named_remote_re_export.ds",
        );

        test.result(result).assert_no_lint("no-default-export");
    }
}
