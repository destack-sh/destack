use destack_ast::{self as ast, DependencyItem, DependencyMode, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce sorted import declarations.
    ///
    /// This rule enforces two kinds of sorting:
    ///
    /// 1. **Member sorting**: imported names within each import should be sorted
    ///    alphabetically (e.g., `{ a, b, z }` not `{ z, a, b }`)
    ///
    /// 2. **Declaration grouping**: imports should be grouped by type and sorted
    ///    alphabetically within each group:
    ///    - External packages (e.g., `"react"`, `"lodash"`)
    ///    - Internal/aliased paths (e.g., `"@/utils"`, `"~/lib"`)
    ///    - Parent imports (e.g., `"../utils"`)
    ///    - Sibling imports (e.g., `"./local"`)
    ///
    /// ```
    /// // bad
    /// import { z, a } from "./local"
    /// import { foo } from "external"
    ///
    /// // good
    /// import { foo } from "external"
    /// import { a, z } from "./local"
    /// ```
    #[lint(
        id = "sort-imports",
        code = "LY062",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub SortImports,
    "Enforce sorted import declarations"
}

/// Import group ordering (lower = should come first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ImportGroup {
    /// External packages (no leading `.`, `@/`, `~/`, or `#`)
    External = 0,
    /// Internal/aliased paths (`@/`, `~/`, `#`)
    Internal = 1,
    /// Parent imports (`../`)
    Parent = 2,
    /// Sibling imports (`./`)
    Sibling = 3,
}

impl ImportGroup {
    fn from_target(target: &str) -> Self {
        if target.starts_with("../") {
            ImportGroup::Parent
        } else if target.starts_with("./") || target == "." {
            ImportGroup::Sibling
        } else if target.starts_with("@/") || target.starts_with("~/") || target.starts_with('#') {
            ImportGroup::Internal
        } else {
            ImportGroup::External
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ImportGroup::External => "external",
            ImportGroup::Internal => "internal",
            ImportGroup::Parent => "parent",
            ImportGroup::Sibling => "sibling",
        }
    }
}

impl LintRule for SortImports {
    fn meta(&self) -> &'static crate::LintMeta {
        SortImports::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // collect all imports with their info
        struct ImportInfo {
            target: String,
            group: ImportGroup,
            node_id: ast::LocalNodeId<Expression>,
            items: Vec<ast::LocalNodeId<DependencyItem>>,
        }

        let mut imports: Vec<ImportInfo> = Vec::new();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            if let Expression::Import { target, items, .. } = expression {
                let target_str = ctx.strings.get(*target).to_string();
                let group = ImportGroup::from_target(&target_str);
                imports.push(ImportInfo {
                    target: target_str,
                    group,
                    node_id,
                    items: items.clone(),
                });
            }
        }

        // check member sorting within each import
        for import in &imports {
            check_member_sorting(ctx, severity, import.node_id, &import.items);
        }

        // check declaration ordering (grouping + alphabetical within groups)
        for i in 1..imports.len() {
            let prev = &imports[i - 1];
            let current = &imports[i];

            // check group ordering
            if current.group < prev.group {
                ctx.report(
                    LintDiagnostic::new(
                        SORT_IMPORTS.id,
                        SORT_IMPORTS.code,
                        SORT_IMPORTS.category,
                        severity,
                        format!(
                            "{} import `{}` should come before {} import `{}`",
                            current.group.name(),
                            current.target,
                            prev.group.name(),
                            prev.target
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(current.node_id),
                    )
                    .with_label(format!(
                        "{} imports should come before {} imports",
                        current.group.name(),
                        prev.group.name()
                    )),
                );
            } else if current.group == prev.group {
                // same group: check alphabetical ordering
                if prev.target.to_lowercase() > current.target.to_lowercase() {
                    ctx.report(
                        LintDiagnostic::new(
                            SORT_IMPORTS.id,
                            SORT_IMPORTS.code,
                            SORT_IMPORTS.category,
                            severity,
                            format!(
                                "import `{}` should come before `{}`",
                                current.target, prev.target
                            ),
                            ctx.module.file_id,
                            ctx.tree.get_span(current.node_id),
                        )
                        .with_label("imports should be sorted alphabetically within their group"),
                    );
                }
            }
        }
    }
}

/// Check that imported members within an import are sorted alphabetically.
fn check_member_sorting(
    ctx: &mut LintModuleAstContext<'_>,
    severity: LintSeverity,
    _import_node_id: ast::LocalNodeId<Expression>,
    items: &[ast::LocalNodeId<DependencyItem>],
) {
    // get the sortable names for each item
    let mut named_items: Vec<(String, ast::LocalNodeId<DependencyItem>)> = Vec::new();

    for item_id in items {
        let item = ctx.tree.get(*item_id);

        // skip namespace imports (import * as foo) - they don't participate in member sorting
        if item.mode == DependencyMode::Namespace {
            continue;
        }

        // use alias if present, otherwise use name
        let sort_name = if let Some(alias) = item.alias {
            ctx.strings.get(alias).to_string()
        } else if let Some(name) = item.name {
            ctx.strings.get(name).to_string()
        } else {
            // default import without alias
            "default".to_string()
        };

        named_items.push((sort_name, *item_id));
    }

    // check if members are sorted
    for i in 1..named_items.len() {
        let (prev_name, _) = &named_items[i - 1];
        let (current_name, current_id) = &named_items[i];

        if prev_name.to_lowercase() > current_name.to_lowercase() {
            ctx.report(
                LintDiagnostic::new(
                    SORT_IMPORTS.id,
                    SORT_IMPORTS.code,
                    SORT_IMPORTS.category,
                    severity,
                    format!("member `{current_name}` should come before `{prev_name}`"),
                    ctx.module.file_id,
                    ctx.tree.get_span(*current_id),
                )
                .with_label("import members should be sorted alphabetically"),
            );
            // only report once per import to avoid noise
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_unsorted_members_detected() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { z, a, m } from "utils"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    #[test]
    fn test_sorted_members_allowed() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { a, m, z } from "utils"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    #[test]
    fn test_member_sorting_case_insensitive() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { Alpha, beta, Gamma } from "utils"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    #[test]
    fn test_single_member_allowed() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { foo } from "utils"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    #[test]
    fn test_external_before_sibling_required() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { local } from "./local"
import { external } from "external"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    #[test]
    fn test_correct_group_order_allowed() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { external } from "external"
import { internal } from "@/internal"
import { parent } from "../parent"
import { sibling } from "./sibling"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    #[test]
    fn test_internal_before_parent_required() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { parent } from "../parent"
import { internal } from "@/internal"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    #[test]
    fn test_parent_before_sibling_required() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { sibling } from "./sibling"
import { parent } from "../parent"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    #[test]
    fn test_alphabetical_within_external_group() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { z } from "zod"
import { a } from "axios"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    #[test]
    fn test_alphabetical_within_sibling_group() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { z } from "./z"
import { a } from "./a"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    #[test]
    fn test_sorted_within_groups_allowed() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { a } from "axios"
import { z } from "zod"
import { a } from "./a"
import { z } from "./z"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    // === Mixed tests ===

    #[test]
    fn test_both_member_and_declaration_issues() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { z, a } from "./local"
import { foo } from "external"
"#,
        );
        // should detect both: unsorted members AND wrong group order
        test.result(result).assert_lint_count("sort-imports", 2);
    }

    #[test]
    fn test_internal_alias_paths() {
        let test = TestProgram::for_rule(SortImports);
        let result = test.lint_ast(
            "test.ds",
            r#"
import { a } from "external"
import { b } from "@/components"
import { c } from "~/utils"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }
}
