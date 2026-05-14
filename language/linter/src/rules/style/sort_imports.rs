use destack_dir::{self as dir, DependencyBinding, DependencyItem, Expression};
use destack_source::Span;
use destack_workspace::{LintSeverity, SortImportsMemberSyntax};

use crate::rules::common::source_text_contains_comment_token;
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Enforce sorted import declarations.
    ///
    /// Enforce sorted import declarations within modules.
    #[lint(
        id = "sort-imports",
        code = "LY064",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub SortImports,
    "Enforce sorted import declarations"
}

/// One top-level import declaration.
#[derive(Debug, Clone)]
struct ImportInfo {
    /// The root expression span for declaration ordering diagnostics.
    root_expression_id: dir::LocalNodeId<Expression>,
    /// The import expression id.
    import_expression_id: dir::LocalNodeId<Expression>,
    /// The declaration span.
    span: Span,
    /// The import items.
    items: Vec<dir::LocalNodeId<DependencyItem>>,
}

impl LintRule for SortImports {
    /// Return the static lint metadata.
    fn meta(&self) -> &'static LintMeta {
        SortImports::meta()
    }

    /// Check top-level import declarations.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let imports = collect_imports(ctx);

        if !ctx.options.style.sort_imports_ignore_declaration_sort {
            check_declaration_sorting(ctx, meta, &imports);
        }

        if !ctx.options.style.sort_imports_ignore_member_sort {
            for import in &imports {
                check_member_sorting(ctx, meta, import);
            }
        }
    }
}

/// Collect all top-level import declarations in source order.
fn collect_imports(ctx: &LintModuleContext<'_>) -> Vec<ImportInfo> {
    let mut imports = Vec::new();

    for root_expression_id in ctx.roots.iter().copied() {
        let Some(import_expression_id) = top_level_import_expression_id(ctx, root_expression_id)
        else {
            continue;
        };

        let Expression::Import { items, .. } = ctx.dir.get(import_expression_id) else {
            continue;
        };

        imports.push(ImportInfo {
            root_expression_id: root_expression_id,
            import_expression_id,
            span: ctx.dir.get_span(root_expression_id),
            items: items.clone().unwrap_or_default(),
        });
    }

    imports
}

/// Return one top-level import expression id when the root is an import.
fn top_level_import_expression_id(
    ctx: &LintModuleContext<'_>,
    root_expression_id: dir::LocalNodeId<Expression>,
) -> Option<dir::LocalNodeId<Expression>> {
    match ctx.dir.get(root_expression_id) {
        Expression::Import { .. } => Some(root_expression_id),
        _ => None,
    }
}

/// Check declaration ordering against the configured declaration-form order.
fn check_declaration_sorting(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    imports: &[ImportInfo],
) {
    let mut previous_import: Option<&ImportInfo> = None;

    for import in imports {
        if previous_import.is_some()
            && ctx.options.style.sort_imports_allow_separated_groups
            && imports_are_separated_group(ctx, previous_import.unwrap().span, import.span)
        {
            previous_import = None;
        }

        let Some(previous) = previous_import else {
            previous_import = Some(import);
            continue;
        };

        let current_group = member_form_group(ctx, import);
        let previous_group = member_form_group(ctx, previous);
        let current_group_index = member_form_order_index(ctx, current_group);
        let previous_group_index = member_form_order_index(ctx, previous_group);

        if current_group_index < previous_group_index {
            let severity = ctx.get_effective_severity(meta, import.root_expression_id);
            if severity.is_enabled() {
                ctx.report(
                    LintReport::new(
                        SORT_IMPORTS.id,
                        SORT_IMPORTS.code,
                        SORT_IMPORTS.category,
                        severity,
                        format!(
                            "expected `{}` form before `{}` form",
                            member_form_label(current_group),
                            member_form_label(previous_group),
                        ),
                        import.span,
                    )
                    .label("import declarations are out of form order"),
                );
            }
        } else if current_group_index == previous_group_index {
            let Some(current_name) = first_local_member_name(ctx, import) else {
                previous_import = Some(import);
                continue;
            };
            let Some(previous_name) = first_local_member_name(ctx, previous) else {
                previous_import = Some(import);
                continue;
            };

            if normalize_import_name(ctx, &current_name)
                < normalize_import_name(ctx, &previous_name)
            {
                let severity = ctx.get_effective_severity(meta, import.root_expression_id);
                if severity.is_enabled() {
                    ctx.report(
                        LintReport::new(
                            SORT_IMPORTS.id,
                            SORT_IMPORTS.code,
                            SORT_IMPORTS.category,
                            severity,
                            "imports should be sorted alphabetically",
                            import.span,
                        )
                        .label("this import should come earlier"),
                    );
                }
            }
        }

        previous_import = Some(import);
    }
}

/// Check named import member sorting within one declaration.
fn check_member_sorting(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    import: &ImportInfo,
) {
    let named_items = named_import_items(ctx, import);
    if named_items.len() < 2 {
        return;
    }

    let mut sorted_items = named_items.clone();
    sorted_items.sort_by(|left_id, right_id| {
        normalized_dependency_item_name(ctx, *left_id)
            .cmp(&normalized_dependency_item_name(ctx, *right_id))
    });
    if sorted_items == named_items {
        return;
    }

    let Some(mismatch_index) = named_items
        .iter()
        .zip(sorted_items.iter())
        .position(|(left, right)| left != right)
    else {
        return;
    };

    let mismatch_item_id = named_items[mismatch_index];
    let severity = ctx.get_effective_severity(meta, import.import_expression_id);
    if !severity.is_enabled() {
        return;
    }

    let mismatch_name = dependency_item_name(ctx, mismatch_item_id);
    let mut diagnostic = LintReport::new(
        SORT_IMPORTS.id,
        SORT_IMPORTS.code,
        SORT_IMPORTS.category,
        severity,
        format!("member `{mismatch_name}` should be sorted alphabetically"),
        ctx.dir.get_span(mismatch_item_id),
    )
    .label("import members should be sorted alphabetically");

    if ctx.compute_fixes
        && let Some(fix) = build_member_sort_fix(ctx, &named_items)
    {
        diagnostic = diagnostic.fix(fix);
    }

    ctx.report(diagnostic);
}

/// Build one named-member sorting fix when the specifier region is comment free.
fn build_member_sort_fix(
    ctx: &LintModuleContext<'_>,
    named_items: &[dir::LocalNodeId<DependencyItem>],
) -> Option<LintFix> {
    let first_item_id = *named_items.first()?;
    let last_item_id = *named_items.last()?;
    let first_span = ctx.dir.get_span(first_item_id);
    let last_span = ctx.dir.get_span(last_item_id);
    let region_span = Span::new(ctx.module.file_id, first_span.start, last_span.end);
    let region_text = ctx.get_span_text(region_span);
    if region_text.contains("//") || region_text.contains("/*") {
        return None;
    }

    let mut sorted_items = named_items.to_vec();
    sorted_items.sort_by(|left_id, right_id| {
        normalized_dependency_item_name(ctx, *left_id)
            .cmp(&normalized_dependency_item_name(ctx, *right_id))
    });

    let mut replacement = String::new();
    for (index, item_id) in sorted_items.iter().enumerate() {
        replacement.push_str(ctx.get_span_text(ctx.dir.get_span(*item_id)));

        if let Some(current_original_id) = named_items.get(index)
            && let Some(next_original_id) = named_items.get(index + 1)
        {
            let current_span = ctx.dir.get_span(*current_original_id);
            let next_span = ctx.dir.get_span(*next_original_id);
            replacement.push_str(ctx.get_span_text(Span::new(
                ctx.module.file_id,
                current_span.end,
                next_span.start,
            )));
        }
    }

    let edits = ctx
        .edit_builder()
        .replace(region_span, replacement)
        .into_edits();
    Some(LintFix::safe("Sort import members").with_edits(edits))
}

/// Return the named import items that participate in member sorting.
fn named_import_items(
    ctx: &LintModuleContext<'_>,
    import: &ImportInfo,
) -> Vec<dir::LocalNodeId<DependencyItem>> {
    import
        .items
        .iter()
        .copied()
        .filter(|item_id| {
            matches!(
                ctx.dir.get(*item_id),
                DependencyItem::Item {
                    binding: DependencyBinding::Item,
                    ..
                }
            )
        })
        .collect()
}

/// Return the member form group for one import declaration.
fn member_form_group(ctx: &LintModuleContext<'_>, import: &ImportInfo) -> SortImportsMemberSyntax {
    if import.items.is_empty() {
        return SortImportsMemberSyntax::None;
    }

    if let Some(first_item_id) = import.items.first()
        && item_binding(ctx, *first_item_id) == Some(DependencyBinding::Namespace)
    {
        return SortImportsMemberSyntax::All;
    }

    if import.items.len() == 1 {
        return SortImportsMemberSyntax::Single;
    }

    SortImportsMemberSyntax::Multiple
}

/// Return the dependency binding for one import item.
fn item_binding(
    ctx: &LintModuleContext<'_>,
    item_id: dir::LocalNodeId<DependencyItem>,
) -> Option<DependencyBinding> {
    match ctx.dir.get(item_id) {
        DependencyItem::Item { binding, .. } => Some(*binding),
        DependencyItem::Error => None,
    }
}

/// Return the configured form-group index for one import declaration.
fn member_form_order_index(ctx: &LintModuleContext<'_>, group: SortImportsMemberSyntax) -> usize {
    ctx.options
        .style
        .sort_imports_member_syntax_sort_order
        .iter()
        .position(|candidate| *candidate == group)
        .unwrap_or(usize::MAX)
}

/// Return the display label for one member form group.
fn member_form_label(group: SortImportsMemberSyntax) -> &'static str {
    match group {
        SortImportsMemberSyntax::None => "none",
        SortImportsMemberSyntax::All => "all",
        SortImportsMemberSyntax::Multiple => "multiple",
        SortImportsMemberSyntax::Single => "single",
    }
}

/// Return the first local member name for declaration sorting.
fn first_local_member_name(ctx: &LintModuleContext<'_>, import: &ImportInfo) -> Option<String> {
    let first_item_id = *import.items.first()?;
    Some(dependency_item_name(ctx, first_item_id))
}

/// Return one normalized dependency item name for ordering.
fn normalized_dependency_item_name(
    ctx: &LintModuleContext<'_>,
    item_id: dir::LocalNodeId<DependencyItem>,
) -> String {
    normalize_import_name(ctx, &dependency_item_name(ctx, item_id))
}

/// Return one dependency item local name.
fn dependency_item_name(
    ctx: &LintModuleContext<'_>,
    item_id: dir::LocalNodeId<DependencyItem>,
) -> String {
    match ctx.dir.get(item_id) {
        DependencyItem::Item {
            alias: Some(alias), ..
        } => ctx.strings.get(*alias).to_string(),
        DependencyItem::Item {
            name: Some(name), ..
        } => ctx.strings.get(name.string()).to_string(),
        DependencyItem::Item { .. } => "default".to_string(),
        DependencyItem::Error => String::new(),
    }
}

/// Normalize one import ordering name based on the case policy.
fn normalize_import_name(ctx: &LintModuleContext<'_>, name: &str) -> String {
    if ctx.options.style.sort_imports_ignore_case {
        return name.to_ascii_lowercase();
    }

    name.to_string()
}

/// Return true when two imports belong to separated declaration groups.
fn imports_are_separated_group(
    ctx: &LintModuleContext<'_>,
    left_span: Span,
    right_span: Span,
) -> bool {
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    let between_text = ctx.get_span_text(Span::new(
        ctx.module.file_id,
        left_span.end,
        right_span.start,
    ));

    if between_text.bytes().filter(|byte| *byte == b'\n').count() > 1 {
        return true;
    }

    if source_text_contains_comment_token(between_text) {
        return true;
    }

    !between_text.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag out of order named import members.
    #[test]
    fn test_flags_unsorted_import_members() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint(
            "sort_imports/test_flags_unsorted_import_members.ds",
            r#"
import { z, a } from "foo"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    /// Fix out of order named import members.
    #[test]
    fn test_fix_unsorted_import_members() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint(
            "sort_imports/test_fix_unsorted_import_members.ds",
            r#"
import { z, a } from "foo"
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

    /// Flag declaration ordering by default declaration-form order.
    #[test]
    fn test_flags_declaration_form_order() {
        let test = TestProgram::for_rule_without_prelude(SortImports);
        let result = test.lint(
            "sort_imports/test_flags_declaration_form_order.ds",
            r#"
import item from "foo"
import "bar"
"#,
        );
        test.result(result).assert_lint("sort-imports");
    }

    /// Allow separated groups when configured.
    #[test]
    fn test_allows_separated_groups_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.style.sort_imports_allow_separated_groups = true;
        });
        let result = test.lint(
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
            options.style.sort_imports_allow_separated_groups = true;
        });
        let result = test.lint(
            "sort_imports/test_allows_comment_separated_groups_when_enabled.ds",
            r#"
import item from "foo"
// keep groups separate
import "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Allow statement-separated groups when configured.
    #[test]
    fn test_allows_statement_separated_groups_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.style.sort_imports_allow_separated_groups = true;
        });
        let result = test.lint(
            "sort_imports/test_allows_statement_separated_groups_when_enabled.ds",
            r#"
import item from "foo"
boot()
import "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }

    /// Allow reversed declaration-form ordering when configured.
    #[test]
    fn test_allows_custom_member_form_order() {
        let test = TestProgram::for_rule_without_prelude(SortImports).with_options(|options| {
            options.style.sort_imports_member_syntax_sort_order = vec![
                SortImportsMemberSyntax::Single,
                SortImportsMemberSyntax::None,
                SortImportsMemberSyntax::All,
                SortImportsMemberSyntax::Multiple,
            ];
        });
        let result = test.lint(
            "sort_imports/test_allows_custom_member_form_order.ds",
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
            options.style.sort_imports_ignore_declaration_sort = true;
        });
        let result = test.lint(
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
            options.style.sort_imports_ignore_member_sort = true;
        });
        let result = test.lint(
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
            options.style.sort_imports_ignore_case = true;
        });
        let result = test.lint(
            "sort_imports/test_allows_case_insensitive_declaration_order.ds",
            r#"
import a from "foo"
import B from "bar"
"#,
        );
        test.result(result).assert_no_lint("sort-imports");
    }
}
