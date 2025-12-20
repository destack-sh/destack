use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow default exports.
    ///
    /// Named exports are easier to refactor and provide better IDE support.
    /// Consider using named exports instead of default exports.
    #[lint(
        id = "no-default-export",
        code = "LR015",
        category = Restriction,
        level = Dir,
        fixable = No,
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

        // check for DependencyItem nodes with Default mode
        for (node_id, item) in ctx.tree.iter_nodes_of_type::<dir::DependencyItem>() {
            let is_default = match item {
                dir::DependencyItem::Local { mode, .. } => *mode == dir::DependencyMode::Default,
                dir::DependencyItem::UnresolvedLocal { mode, .. } => {
                    *mode == dir::DependencyMode::Default
                }
                dir::DependencyItem::Remote { mode, .. } => *mode == dir::DependencyMode::Default,
                dir::DependencyItem::UnresolvedRemote { mode, .. } => {
                    *mode == dir::DependencyMode::Default
                }
                dir::DependencyItem::Value { mode, .. } => *mode == dir::DependencyMode::Default,
            };

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LintLevel;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_default_export_function() {
        let test = TestProgram::for_rule_without_builtins(NoDefaultExport);
        let result = test.lint(
            "test.ds",
            "export default function foo() {}",
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-default-export");
    }

    #[test]
    fn test_detects_default_export_class() {
        let test = TestProgram::for_rule_without_builtins(NoDefaultExport);
        let result = test.lint("test.ds", "export default class Foo {}", LintLevel::Dir);
        test.check_clean();
        test.result(result).assert_lint("no-default-export");
    }

    #[test]
    fn test_allows_named_export_function() {
        let test = TestProgram::for_rule_without_builtins(NoDefaultExport);
        let result = test.lint("test.ds", "export function foo() {}", LintLevel::Dir);
        test.check_clean();
        test.result(result).assert_no_lint("no-default-export");
    }

    #[test]
    fn test_allows_named_export_class() {
        let test = TestProgram::for_rule_without_builtins(NoDefaultExport);
        let result = test.lint("test.ds", "export class Foo {}", LintLevel::Dir);
        test.check_clean();
        test.result(result).assert_no_lint("no-default-export");
    }

    #[test]
    fn test_detects_default_export_identifier() {
        let test = TestProgram::for_rule_without_builtins(NoDefaultExport);
        let result = test.lint(
            "test.ds",
            "const foo = 1;\nexport default foo;",
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-default-export");
    }
}
