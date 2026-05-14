use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_import_target_static_specifier;
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow relative parent path imports.
    ///
    /// Parent relative imports couple modules to directory layout and make
    /// moves or package extraction harder.
    #[lint(
        id = "no-relative-parent-imports",
        code = "LR034",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoRelativeParentImports,
    "Disallow parent-relative import paths"
}

impl LintRule for NoRelativeParentImports {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoRelativeParentImports::meta()
    }

    /// Check module DIR nodes for parent relative imports.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoRelativeParentImportsVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that checks import targets for parent relative paths.
struct NoRelativeParentImportsVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoRelativeParentImportsVisitor<'a, 'b> {
    /// Build a visitor for parent relative import checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one expression for a parent relative import target.
    fn check_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        let Some(target_id) = expression_import_target_static_specifier(expression) else {
            return;
        };

        // resolve target text
        let target_text = self.ctx.strings.get(target_id).to_string();
        if !is_relative_parent_specifier(target_text.as_ref()) {
            return;
        }

        // resolve effective lint severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // resolve diagnostic span
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_RELATIVE_PARENT_IMPORTS.id,
                NO_RELATIVE_PARENT_IMPORTS.code,
                NO_RELATIVE_PARENT_IMPORTS.category,
                severity,
                format!("parent relative import `{target_text}`"),
                span,
            )
            .label("avoid importing through parent relative paths")
            .note("prefer package aliases or rooted module paths"),
        );
    }
}

impl NodeVisitor for NoRelativeParentImportsVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.check_expression(id, expression);
        walk_expression(self, tree, id, expression);
    }
}

/// Return true when the import specifier traverses to a parent directory.
fn is_relative_parent_specifier(specifier: &str) -> bool {
    // normalize leading current directory segments
    let mut remainder = specifier;
    loop {
        if let Some(stripped) = remainder.strip_prefix("./") {
            remainder = stripped;
            continue;
        }
        if let Some(stripped) = remainder.strip_prefix(".\\") {
            remainder = stripped;
            continue;
        }
        break;
    }

    remainder == ".." || remainder.starts_with("../") || remainder.starts_with("..\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Report parent relative import specifiers.
    #[test]
    fn test_flags_parent_relative_import() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/shared.ds" => r#"
export const value = 1;
"#,
                "no_relative_parent_imports/feature/consumer.ds" => r#"
import { value } from "../shared.ds";
value;
"#,
            },
            "no_relative_parent_imports/feature/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-relative-parent-imports");
    }

    /// Report parent relative re export specifiers.
    #[test]
    fn test_flags_parent_relative_re_export() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/shared.ds" => r#"
export const value = 1;
"#,
                "no_relative_parent_imports/feature/index.ds" => r#"
export { value } from "../shared.ds";
"#,
            },
            "no_relative_parent_imports/feature/index.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-relative-parent-imports");
    }

    /// Report parent relative imports with windows separators.
    #[test]
    fn test_flags_parent_relative_windows_import() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/windows/shared.ds" => r#"
export const value = 1;
"#,
                "no_relative_parent_imports/windows/consumer.ds" => r#"
import { value } from "..\\shared.ds";
value;
"#,
            },
            "no_relative_parent_imports/windows/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-relative-parent-imports");
    }

    /// Report parent relative imports with leading current directory segments.
    #[test]
    fn test_flags_parent_relative_import_after_current_directory_prefix() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/prefixed/shared.ds" => r#"
export const value = 1;
"#,
                "no_relative_parent_imports/prefixed/consumer.ds" => r#"
import { value } from "./../shared.ds";
value;
"#,
            },
            "no_relative_parent_imports/prefixed/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-relative-parent-imports");
    }

    /// Report direct parent directory imports without trailing segments.
    #[test]
    fn test_flags_direct_parent_directory_import() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let result = test.lint_dir(
            "no_relative_parent_imports/test_flags_direct_parent_directory_import.ds",
            r#"
import parent from "..";
parent;
"#,
        );
        test.result(result)
            .assert_lint("no-relative-parent-imports");
    }

    /// Allow sibling relative imports.
    #[test]
    fn test_allows_sibling_relative_import() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/feature/shared.ds" => r#"
export const value = 1;
"#,
                "no_relative_parent_imports/feature/consumer.ds" => r#"
import { value } from "./shared.ds";
value;
"#,
            },
            "no_relative_parent_imports/feature/consumer.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("no-relative-parent-imports");
    }

    /// Allow package style imports.
    #[test]
    fn test_allows_package_import() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let result = test.lint_dir(
            "no_relative_parent_imports/test_allows_package_import.ds",
            r#"
import { safe } from "public/safe";
safe;
"#,
        );
        test.result(result)
            .assert_no_lint("no-relative-parent-imports");
    }

    /// Allow imports from current directory segments only.
    #[test]
    fn test_allows_current_directory_segments() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/current/shared.ds" => r#"
export const value = 1;
"#,
                "no_relative_parent_imports/current/consumer.ds" => r#"
import { value } from "././shared.ds";
value;
"#,
            },
            "no_relative_parent_imports/current/consumer.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("no-relative-parent-imports");
    }

    /// Report side effect imports through parent relative paths.
    #[test]
    fn test_flags_parent_relative_side_effect_import() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/side_effect/shared.ds" => r#"
export const value = 1;
"#,
                "no_relative_parent_imports/side_effect/consumer.ds" => r#"
import "../shared.ds";
"#,
            },
            "no_relative_parent_imports/side_effect/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-relative-parent-imports");
    }

    /// Report export star through parent relative paths.
    #[test]
    fn test_flags_parent_relative_export_star() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/export_star/shared.ds" => r#"
export const value = 1;
"#,
                "no_relative_parent_imports/export_star/index.ds" => r#"
export * from "../shared.ds";
"#,
            },
            "no_relative_parent_imports/export_star/index.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-relative-parent-imports");
    }

    /// Allow specifiers that only begin with two dots.
    #[test]
    fn test_allows_double_dot_prefix_package_name() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let result = test.lint_dir(
            "no_relative_parent_imports/test_allows_double_dot_prefix_package_name.ds",
            r#"
import value from "..pkg/shared";
value;
"#,
        );
        test.result(result)
            .assert_no_lint("no-relative-parent-imports");
    }

    /// Skip declaration files by default.
    #[test]
    fn test_skips_declaration_files_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/types/shared.d.ts" => r#"
export declare const value: int32;
"#,
                "no_relative_parent_imports/types/index.d.ts" => r#"
import { value } from "../types/shared.d.ts";
export { value };
"#,
            },
            "no_relative_parent_imports/types/index.d.ts",
        );

        test.result(diagnostics)
            .assert_no_lint("no-relative-parent-imports");
    }

    /// Include declaration files when configured.
    #[test]
    fn test_includes_declaration_files_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoRelativeParentImports)
            .with_options(|options| options.include_declaration_files = true);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_relative_parent_imports/types/shared.d.ts" => r#"
export declare const value: int32;
"#,
                "no_relative_parent_imports/types/index.d.ts" => r#"
import { value } from "../types/shared.d.ts";
export { value };
"#,
            },
            "no_relative_parent_imports/types/index.d.ts",
        );

        test.result(diagnostics)
            .assert_lint("no-relative-parent-imports");
    }
}
