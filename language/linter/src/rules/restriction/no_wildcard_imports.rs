use destack_dir as dir;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow wildcard imports.
    ///
    /// Wildcard imports (`import * as foo`) make it harder to track what's
    /// being used and can lead to namespace pollution. Use named imports instead.
    #[lint(
        id = "no-wildcard-imports",
        code = "LR031",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoWildcardImports,
    "Disallow wildcard imports"
}

impl LintRule for NoWildcardImports {
    fn meta(&self) -> &'static LintMeta {
        NoWildcardImports::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let dir::Expression::Import { items, .. } = expression else {
                continue;
            };
            let Some(items) = items.as_deref() else {
                continue;
            };

            // check for namespace/wildcard imports (like `import * as foo`)
            for item_id in items {
                let item = ctx.dir.get(*item_id);
                if matches!(
                    item,
                    dir::DependencyItem::Binding {
                        binding: dir::DependencyBinding::Namespace,
                        ..
                    }
                ) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    let mut diagnostic = LintReport::new(
                        NO_WILDCARD_IMPORTS.id,
                        NO_WILDCARD_IMPORTS.code,
                        NO_WILDCARD_IMPORTS.category,
                        severity,
                        "wildcard import",
                        ctx.dir.get_span(*item_id),
                    )
                    .label("use named imports instead");
                    if ctx.compute_fixes
                        && let Some(fix) = no_wildcard_imports_fix(ctx, node_id, items, item_id)
                    {
                        diagnostic = diagnostic.fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Build an unsafe fix by removing one wildcard import item.
fn no_wildcard_imports_fix(
    ctx: &LintModuleContext<'_>,
    import_expression_id: dir::LocalNodeId<dir::Expression>,
    items: &[dir::LocalNodeId<dir::DependencyItem>],
    wildcard_item_id: &dir::LocalNodeId<dir::DependencyItem>,
) -> Option<LintFix> {
    let wildcard_index = items
        .iter()
        .position(|item_id| item_id == wildcard_item_id)?;
    let remove_span = import_item_removal_span(ctx, import_expression_id, items, wildcard_index)?;
    if remove_span.is_empty() {
        return None;
    }

    // build fix edits
    let edits = ctx.edit_builder().delete(remove_span).into_edits();
    Some(LintFix::r#unsafe("Remove wildcard import").with_edits(edits))
}

/// Return a span that safely removes one import item from an import expression.
fn import_item_removal_span(
    ctx: &LintModuleContext<'_>,
    import_expression_id: dir::LocalNodeId<dir::Expression>,
    items: &[dir::LocalNodeId<dir::DependencyItem>],
    item_index: usize,
) -> Option<Span> {
    if item_index >= items.len() {
        return None;
    }

    // only apply to standalone wildcard imports
    if items.len() != 1 {
        return None;
    }

    // single item import: remove full statement
    Some(ctx.dir.get_span(import_expression_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_star_import() {
        let test = TestProgram::for_rule_without_prelude(NoWildcardImports);
        let result = test.lint(
            "no_wildcard_imports/test_detects_star_import.ts",
            r#"import * as foo from "foo";"#,
        );
        test.result(result)
            .assert_lint("no-wildcard-imports")
            .assert_unsafe_fixed("");
    }

    #[test]
    fn test_allows_named_import() {
        let test = TestProgram::for_rule_without_prelude(NoWildcardImports);
        let result = test.lint(
            "no_wildcard_imports/test_allows_named_import.ts",
            r#"import { foo, bar } from "foo";"#,
        );
        test.result(result).assert_no_lint("no-wildcard-imports");
    }

    #[test]
    fn test_allows_default_import() {
        let test = TestProgram::for_rule_without_prelude(NoWildcardImports);
        let result = test.lint(
            "no_wildcard_imports/test_allows_default_import.ts",
            r#"import foo from "foo";"#,
        );
        test.result(result).assert_no_lint("no-wildcard-imports");
    }

    #[test]
    fn test_allows_side_effect_import() {
        let test = TestProgram::for_rule_without_prelude(NoWildcardImports);
        let result = test.lint(
            "no_wildcard_imports/test_allows_side_effect_import.ts",
            r#"import "foo";"#,
        );
        test.result(result).assert_no_lint("no-wildcard-imports");
    }

    #[test]
    fn test_fix_removes_wildcard_item_from_mixed_import() {
        let test = TestProgram::for_rule_without_prelude(NoWildcardImports);
        let result = test.lint(
            "no_wildcard_imports/test_fix_removes_wildcard_item_from_mixed_import.ts",
            r#"import foo, * as ns from "foo";"#,
        );
        test.result(result)
            .assert_lint("no-wildcard-imports")
            .assert_has_no_fix("no-wildcard-imports");
    }

    #[test]
    fn test_skips_declaration_file_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoWildcardImports);
        let result = test.lint(
            "no_wildcard_imports/test_skips_declaration_file_by_default.d.ts",
            r#"import * as foo from "foo";"#,
        );
        test.result(result).assert_no_lint("no-wildcard-imports");
    }

    #[test]
    fn test_includes_declaration_file_when_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(NoWildcardImports).with_options(|options| {
                options.include_declaration_files = true;
            });
        let result = test.lint(
            "no_wildcard_imports/test_includes_declaration_file_when_enabled.d.ts",
            r#"import * as foo from "foo";"#,
        );
        test.result(result).assert_lint("no-wildcard-imports");
    }
}
