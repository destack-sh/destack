use destack_ast::{self as ast, DependencyItem, Expression};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    ImportDeclarationKey, categorize_import, sort_import_declaration_indices,
};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintMeta, LintRule, declare_lint};

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
        code = "LY064",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub SortImports,
    "Enforce sorted import declarations"
}

/// A top-level import declaration with source metadata.
#[derive(Debug, Clone)]
struct ImportInfo {
    /// The root expression ID (statement or import) for diagnostics.
    root_expression_id: ast::LocalNodeId<Expression>,
    /// The import expression ID for member lookup.
    import_expression_id: ast::LocalNodeId<Expression>,
    /// The source span for this declaration.
    span: Span,
    /// The import target path.
    target: String,
    /// Whether this is a side-effect-only import.
    is_side_effect: bool,
    /// The declaration source text.
    text: String,
    /// The imported dependency items.
    items: Vec<ast::LocalNodeId<DependencyItem>>,
}

impl LintRule for SortImports {
    /// Return the static lint metadata.
    fn meta(&self) -> &'static LintMeta {
        SortImports::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let imports = collect_top_level_imports(ctx);

        // check member sorting within each import declaration
        for import in &imports {
            check_member_sorting(ctx, meta, import);
        }

        // check declaration ordering with shared canonical import sort order
        check_declaration_sorting(ctx, meta, &imports);
    }
}

/// Collect contiguous top-level import declarations.
fn collect_top_level_imports(ctx: &LintAstContext<'_>) -> Vec<ImportInfo> {
    let mut imports = Vec::new();

    // inspect candidate syntax nodes
    for root_expression_id in ctx.roots {
        let root_expression = ctx.tree.get(*root_expression_id);
        let import_expression = match root_expression {
            Expression::Import {
                source: ast::ImportSource::ImportStatement | ast::ImportSource::ImportEquals,
                ..
            } => Some(*root_expression_id),
            Expression::Statement(inner_expression_id) => {
                let inner_expression = ctx.tree.get(*inner_expression_id);
                if matches!(
                    inner_expression,
                    Expression::Import {
                        source: ast::ImportSource::ImportStatement
                            | ast::ImportSource::ImportEquals,
                        ..
                    }
                ) {
                    Some(*inner_expression_id)
                } else {
                    None
                }
            }
            _ => None,
        };

        // require optional structure
        let Some(import_expression_id) = import_expression else {
            if !imports.is_empty() {
                break;
            }
            continue;
        };

        // resolve import expression
        let import_expression = ctx.tree.get(import_expression_id);
        let Expression::Import {
            target: ast::ImportTarget::String(target),
            items,
            ..
        } = import_expression
        else {
            continue;
        };

        // resolve diagnostic span
        let span = ctx.tree.get_span(*root_expression_id);
        let text = ctx.get_span_text(span).trim_end().to_string();
        let target = ctx.strings.get(*target).to_string();

        imports.push(ImportInfo {
            root_expression_id: *root_expression_id,
            import_expression_id,
            span,
            target,
            is_side_effect: items.is_empty(),
            text,
            items: items.clone(),
        });
    }

    imports
}

/// Check that imported members within an import are in canonical order.
fn check_member_sorting(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    import: &ImportInfo,
) {
    if import.items.len() < 2 {
        return;
    }

    // build canonical member order
    let mut sorted_items = import.items.clone();
    sorted_items.sort_by(|left_id, right_id| compare_dependency_items(ctx, *left_id, *right_id));
    if sorted_items == import.items {
        return;
    }

    // report only the first mismatch in this declaration
    let mismatch_position = import
        .items
        .iter()
        .zip(sorted_items.iter())
        .position(|(left, right)| left != right);
    let Some(mismatch_position) = mismatch_position else {
        return;
    };

    // resolve mismatch item id
    let mismatch_item_id = import.items[mismatch_position];
    let expected_item_id = sorted_items[mismatch_position];
    let mismatch_name = dependency_item_sort_name(ctx, mismatch_item_id);
    let expected_name = dependency_item_sort_name(ctx, expected_item_id);

    // resolve effective lint severity
    let severity = ctx.get_effective_severity(meta, import.import_expression_id);
    if !severity.is_enabled() {
        return;
    }

    ctx.report(
        LintDiagnostic::new(
            SORT_IMPORTS.id,
            SORT_IMPORTS.code,
            SORT_IMPORTS.category,
            severity,
            format!("member `{mismatch_name}` should come before `{expected_name}`"),
            ctx.module.file_id,
            ctx.tree.get_span(mismatch_item_id),
        )
        .with_label("import members should be in canonical order"),
    );
}

/// Check declaration ordering and report one canonical-order diagnostic.
fn check_declaration_sorting(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    imports: &[ImportInfo],
) {
    if imports.len() < 2 {
        return;
    }

    // build shared declaration sort keys
    let declaration_keys: Vec<_> = imports
        .iter()
        .map(|import| ImportDeclarationKey {
            target: import.target.as_str(),
            is_side_effect: import.is_side_effect,
        })
        .collect();
    let order = sort_import_declaration_indices(&declaration_keys);

    // exit early when declarations are already in canonical order
    let mismatch_position = order
        .iter()
        .copied()
        .enumerate()
        .find_map(|(position, import_index)| (position != import_index).then_some(position));
    let Some(mismatch_position) = mismatch_position else {
        return;
    };

    // compare the current import with the expected import at this position
    let import = &imports[mismatch_position];
    let expected = &imports[order[mismatch_position]];
    let severity = ctx.get_effective_severity(meta, import.root_expression_id);
    if !severity.is_enabled() {
        return;
    }

    // build one declaration ordering diagnostic
    let mut diagnostic = LintDiagnostic::new(
        SORT_IMPORTS.id,
        SORT_IMPORTS.code,
        SORT_IMPORTS.category,
        severity,
        format!(
            "import `{}` should come before `{}`",
            expected.target, import.target
        ),
        ctx.module.file_id,
        import.span,
    )
    .with_label("import declarations are not in canonical order");

    // add a module level declaration reorder fix when enabled
    if ctx.compute_fixes
        && let Some(fix) = build_declaration_fix(ctx, imports, &order)
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Build a declaration reorder fix for the contiguous top-level import block.
fn build_declaration_fix(
    ctx: &LintAstContext<'_>,
    imports: &[ImportInfo],
    order: &[usize],
) -> Option<LintFix> {
    let first = imports.first()?;
    let last = imports.last()?;
    let block_span = Span::new(ctx.module.file_id, first.span.start, last.span.end);

    // rebuild declarations in canonical order and preserve group spacing
    let mut replacement = String::new();
    for (position, import_index) in order.iter().copied().enumerate() {
        let import = &imports[import_index];

        // keep each import separated by at least one newline
        if !replacement.is_empty() {
            replacement.push('\n');
        }
        replacement.push_str(&import.text);

        // insert one extra blank line between import groups
        if let Some(next_index) = order.get(position + 1).copied() {
            let next_import = &imports[next_index];
            let needs_blank = !next_import.is_side_effect
                && (import.is_side_effect
                    || categorize_import(import.target.as_str())
                        != categorize_import(next_import.target.as_str()));
            if needs_blank {
                replacement.push('\n');
            }
        }
    }

    // skip no op edits
    if ctx.get_span_text(block_span) == replacement {
        return None;
    }

    // replace the contiguous import block with canonical ordering
    let edits = ctx
        .edit_builder()
        .replace(block_span, replacement)
        .into_edits();
    Some(LintFix::safe("Organize import declarations").with_edits(edits))
}

/// Resolve a stable sort key name for a dependency item.
fn dependency_item_sort_name(
    ctx: &LintAstContext<'_>,
    item_id: ast::LocalNodeId<DependencyItem>,
) -> String {
    let item = ctx.tree.get(item_id);
    // prefer alias names because that is what downstream code references
    if let ast::DependencyItem::Item {
        alias: Some(alias), ..
    } = item
    {
        return ctx.strings.get(*alias).to_string();
    }

    // otherwise use the imported member name
    if let ast::DependencyItem::Item {
        name: Some(name), ..
    } = item
    {
        return ctx.strings.get(name.string()).to_string();
    }

    "default".to_string()
}

/// Compare dependency items using canonical import member ordering.
fn compare_dependency_items(
    ctx: &LintAstContext<'_>,
    left_id: ast::LocalNodeId<DependencyItem>,
    right_id: ast::LocalNodeId<DependencyItem>,
) -> std::cmp::Ordering {
    let left_item = ctx.tree.get(left_id);
    let right_item = ctx.tree.get(right_id);

    // place type imports before value imports
    let left_is_type = matches!(
        left_item,
        ast::DependencyItem::Item {
            kind: Some(ast::DependencyKind::Type),
            ..
        }
    );
    let right_is_type = matches!(
        right_item,
        ast::DependencyItem::Item {
            kind: Some(ast::DependencyKind::Type),
            ..
        }
    );
    match (left_is_type, right_is_type) {
        (true, false) => return std::cmp::Ordering::Less,
        (false, true) => return std::cmp::Ordering::Greater,
        _ => {}
    }

    // compare by case insensitive key and keep deterministic case order
    let left_name = dependency_item_sort_name(ctx, left_id);
    let right_name = dependency_item_sort_name(ctx, right_id);
    let left_lower = left_name.to_ascii_lowercase();
    let right_lower = right_name.to_ascii_lowercase();
    match left_lower.cmp(&right_lower) {
        std::cmp::Ordering::Equal => left_name.cmp(&right_name),
        ordering => ordering,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Detect unsorted import members.
    #[test]
    fn test_unsorted_members_detected() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint_ast(
            "sort_imports/test_unsorted_members_detected.ds",
            r#"
import { z, a, m } from "utils"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    /// Allow already sorted import members.
    #[test]
    fn test_sorted_members_allowed() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint_ast(
            "sort_imports/test_sorted_members_allowed.ds",
            r#"
import { a, m, z } from "utils"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Sort member names case-insensitively.
    #[test]
    fn test_member_sorting_case_insensitive() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint_ast(
            "sort_imports/test_member_sorting_case_insensitive.ds",
            r#"
import { Alpha, beta, Gamma } from "utils"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Allow single-member imports.
    #[test]
    fn test_single_member_allowed() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint_ast(
            "sort_imports/test_single_member_allowed.ds",
            r#"
import { foo } from "utils"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    /// Require external imports before sibling imports.
    #[test]
    fn test_external_before_sibling_required() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint_ast(
            "sort_imports/test_external_before_sibling_required.ds",
            r#"
import { local } from "./local"
import { external } from "external"
"#,
        );
        test.result(result)
            .assert_lint("sort-imports")
            .assert_safe_fixed(
                r#"
import { a, z } from "foo";
"#,
            );
    }

    /// Allow declarations in canonical group order.
    #[test]
    fn test_flags_unsorted_import_members() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint_ast(
            "sort_imports/test_flags_unsorted_import_members.ds",
            r#"
import { z, a } from "foo"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Fix out of order named import members.
    #[test]
    fn test_fix_unsorted_import_members() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint_ast(
            "sort_imports/test_fix_unsorted_import_members.ds",
            r#"
import { z, a } from "foo"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    /// Flag declaration ordering by default syntax order.
    #[test]
    fn test_flags_declaration_syntax_order() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint_ast(
            "sort_imports/test_flags_declaration_syntax_order.ds",
            r#"
import item from "foo"
import "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Allow separated groups when configured.
    #[test]
    fn test_allows_separated_groups_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.sort_imports_allow_separated_groups = true;
        });
        let result = test.lint_ast(
            "sort_imports/test_allows_separated_groups_when_enabled.ds",
            r#"
import item from "foo"

import "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Allow comment-separated groups when configured.
    #[test]
    fn test_allows_comment_separated_groups_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.sort_imports_allow_separated_groups = true;
        });
        let result = test.lint_ast(
            "sort_imports/test_allows_comment_separated_groups_when_enabled.ds",
            r#"
import item from "foo"
// keep groups separate
import "bar"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    /// Allow statement-separated groups when configured.
    #[test]
    fn test_allows_statement_separated_groups_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.sort_imports_allow_separated_groups = true;
        });
        let result = test.lint_ast(
            "sort_imports/test_allows_statement_separated_groups_when_enabled.ds",
            r#"
import item from "foo"
boot()
import "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Allow reversed syntax ordering when configured.
    #[test]
    fn test_allows_custom_member_syntax_order() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.sort_imports_member_syntax_sort_order = vec![
                SortImportsMemberSyntax::Single,
                SortImportsMemberSyntax::None,
                SortImportsMemberSyntax::All,
                SortImportsMemberSyntax::Multiple,
            ];
        });
        let result = test.lint_ast(
            "sort_imports/test_allows_custom_member_syntax_order.ds",
            r#"
import item from "foo"
import "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Allow declaration ordering when it is ignored.
    #[test]
    fn test_allows_declaration_order_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.sort_imports_ignore_declaration_sort = true;
        });
        let result = test.lint_ast(
            "sort_imports/test_allows_declaration_order_when_ignored.ds",
            r#"
import item from "foo"
import "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Allow member ordering when it is ignored.
    #[test]
    fn test_allows_member_order_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.sort_imports_ignore_member_sort = true;
        });
        let result = test.lint_ast(
            "sort_imports/test_allows_member_order_when_ignored.ds",
            r#"
import { z, a } from "foo"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Respect case insensitive ordering when configured.
    #[test]
    fn test_allows_case_insensitive_declaration_order() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.sort_imports_ignore_case = true;
        });
        let result = test.lint_ast(
            "sort_imports/test_allows_case_insensitive_declaration_order.ds",
            r#"
import a from "foo"
import B from "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }
}
