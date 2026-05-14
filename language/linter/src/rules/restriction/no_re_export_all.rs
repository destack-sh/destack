use crate::LintMeta;
use destack_dir::{self as dir, DependencyBinding, Expression};
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `export * from "..."`.
    ///
    /// Re-exporting everything from a module can make it hard to track what
    /// is being exported and can lead to unexpected exports. It also makes
    /// tree-shaking less effective and can increase bundle sizes.
    #[lint(
        id = "no-re-export-all",
        code = "LR024",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoReExportAll,
    "Disallow export * from"
}

impl LintRule for NoReExportAll {
    fn meta(&self) -> &'static LintMeta {
        NoReExportAll::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);

            // check export expressions
            let Expression::Export {
                space,
                target: Some(_target),
                items,
                ..
            } = expression
            else {
                continue;
            };

            // check if any item is a namespace re-export (export *)
            for item_id in items {
                let item = ctx.dir.get(*item_id);
                if dependency_item_is_value_namespace_re_export(*space, item) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        break;
                    }

                    // both `export * from` and `export * as foo from` re-export the full module surface
                    ctx.report(
                        LintReport::new(
                            NO_RE_EXPORT_ALL.id,
                            NO_RE_EXPORT_ALL.code,
                            NO_RE_EXPORT_ALL.category,
                            severity,
                            "avoid using export * from",
                            ctx.dir.get_span(node_id),
                        )
                        .label("use named exports instead"),
                    );
                    break; // only report once per export statement
                }
            }
        }
    }
}

/// Return true when one dependency item re-exports the full value namespace.
fn dependency_item_is_value_namespace_re_export(
    export_kind: dir::DependencySpace,
    item: &dir::DependencyItem,
) -> bool {
    let dir::DependencyItem::Item {
        binding,
        space,
        alias: _,
        ..
    } = item
    else {
        return false;
    };
    if *binding != DependencyBinding::Namespace {
        return false;
    }

    (*space).unwrap_or(export_kind) != dir::DependencySpace::Type
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_export_star() {
        let test = TestProgram::for_rule_without_prelude(NoReExportAll);
        let result = test.lint(
            "no_re_export_all/test_detects_export_star.ds",
            r#"
export * from "./module"
"#,
        );
        test.result(result).assert_lint("no-re-export-all");
    }

    #[test]
    fn test_detects_export_star_from_path() {
        let test = TestProgram::for_rule_without_prelude(NoReExportAll);
        let result = test.lint(
            "no_re_export_all/test_detects_export_star_from_path.ds",
            r#"
export * from "some/path"
"#,
        );
        test.result(result).assert_lint("no-re-export-all");
    }

    #[test]
    fn test_detects_export_star_as() {
        let test = TestProgram::for_rule_without_prelude(NoReExportAll);
        let result = test.lint(
            "no_re_export_all/test_detects_export_star_as.ds",
            r#"
export * as utils from "./utils"
"#,
        );
        test.result(result).assert_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_named_exports() {
        let test = TestProgram::for_rule_without_prelude(NoReExportAll);
        let result = test.lint(
            "no_re_export_all/test_allows_named_exports.ds",
            r#"
export { foo, bar } from "./module"
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_default_export() {
        let test = TestProgram::for_rule_without_prelude(NoReExportAll);
        let result = test.lint(
            "no_re_export_all/test_allows_default_export.ds",
            r#"
export default foo
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_local_exports() {
        let test = TestProgram::for_rule_without_prelude(NoReExportAll);
        let result = test.lint(
            "no_re_export_all/test_allows_local_exports.ds",
            r#"
export { foo, bar }
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_export_type_star() {
        let test = TestProgram::for_rule_without_prelude(NoReExportAll);
        let result = test.lint(
            "no_re_export_all/test_allows_export_type_star.ds",
            r#"
export type * from "./types"
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }

    #[test]
    fn test_allows_export_type_star_as() {
        let test = TestProgram::for_rule_without_prelude(NoReExportAll);
        let result = test.lint(
            "no_re_export_all/test_allows_export_type_star_as.ds",
            r#"
export type * as utils from "./types"
"#,
        );
        test.result(result).assert_no_lint("no-re-export-all");
    }
}
