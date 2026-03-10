use destack_core::StringId;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleDirContext, LintRule, declare_lint};

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
    fn meta(&self) -> &'static crate::LintMeta {
        NoDefaultExport::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let default_name = ctx.program.strings.intern("default");

        // check for DependencyItem nodes with Default mode
        for (node_id, item) in ctx.tree.iter_nodes_of_type::<dir::DependencyItem>() {
            let is_default = dependency_item_is_default(item, default_name);

            if is_default {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.get_span(node_id);
                ctx.report(
                    LintDiagnostic::new(
                        NO_DEFAULT_EXPORT.id,
                        NO_DEFAULT_EXPORT.code,
                        NO_DEFAULT_EXPORT.category,
                        severity,
                        "default export",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("use named exports instead"),
                );
            }
        }

        // check for declarations with export=Default (export default function/class)
        for (node_id, declaration) in ctx.tree.iter_nodes_of_type::<dir::Declaration>() {
            let descriptor = declaration.descriptor();
            if descriptor.export == Some(dir::DependencyMode::Default) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.get_span(node_id);
                let mut diagnostic = LintDiagnostic::new(
                    NO_DEFAULT_EXPORT.id,
                    NO_DEFAULT_EXPORT.code,
                    NO_DEFAULT_EXPORT.category,
                    severity,
                    "default export",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use named exports instead");

                // compute fixes only when requested by the runner
                if ctx.include_fixes
                    && let Some(fix) = default_export_declaration_fix(ctx, node_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return true when a dependency item exports or aliases the `default` binding.
fn dependency_item_is_default(item: &dir::DependencyItem, default_name: StringId) -> bool {
    // check dependency modes that are explicitly default
    if matches!(
        item,
        dir::DependencyItem::Local {
            mode: dir::DependencyMode::Default,
            ..
        } | dir::DependencyItem::UnresolvedLocal {
            mode: dir::DependencyMode::Default,
            ..
        } | dir::DependencyItem::Remote {
            mode: dir::DependencyMode::Default,
            ..
        } | dir::DependencyItem::UnresolvedRemote {
            mode: dir::DependencyMode::Default,
            ..
        } | dir::DependencyItem::Value {
            mode: dir::DependencyMode::Default,
            ..
        }
    ) {
        return true;
    }

    // check local or remote alias forms like `export { foo as default }`
    match item {
        dir::DependencyItem::Local { alias, name, .. }
        | dir::DependencyItem::UnresolvedLocal { alias, name, .. }
        | dir::DependencyItem::Remote { alias, name, .. }
        | dir::DependencyItem::UnresolvedRemote { alias, name, .. } => {
            alias.is_some_and(|alias| alias == default_name)
                || name.is_some_and(|name| name.string() == default_name)
        }
        dir::DependencyItem::Value { .. } => false,
    }
}

/// Build an unsafe fix that rewrites declaration default exports to named exports.
fn default_export_declaration_fix(
    ctx: &LintModuleDirContext<'_>,
    declaration_id: dir::LocalNodeId<dir::Declaration>,
) -> Option<LintFix> {
    // only rewrite named function and class declarations
    let declaration = ctx.tree.get(declaration_id);
    let descriptor = declaration.descriptor();
    descriptor.name?;
    if !matches!(
        declaration,
        dir::Declaration::Function { .. } | dir::Declaration::Class { .. }
    ) {
        return None;
    }

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
            "const foo = 1;\nexport { foo as default };",
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-default-export")
            .assert_has_no_fix("no-default-export");
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
}
