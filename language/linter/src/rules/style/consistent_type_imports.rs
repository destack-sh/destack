use destack_ast::{self as ast, DependencyKind, DependencyMode, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    dependency_item_insert_inline_type_keyword, dependency_item_strip_inline_type_keyword,
    import_type_keyword_removal_span,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce consistent type import style.
    ///
    /// Prefer `import type { Foo }` over `import { type Foo }` for type-only imports.
    /// This makes it clearer that the import is only used for type checking.
    #[lint(
        id = "consistent-type-imports",
        code = "LY007",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub ConsistentTypeImports,
    "Enforce consistent type import style"
}

impl LintRule for ConsistentTypeImports {
    fn meta(&self) -> &'static crate::LintMeta {
        ConsistentTypeImports::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let options = consistent_type_imports_options(ctx);

        // inspect every expression for import declarations and `import(...)` type annotations
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // report forbidden `import(...)` type annotations
            if options.disallow_type_annotations {
                report_type_import_annotation(ctx, meta, node_id, expr);
            }

            // apply one import declaration style check
            if let Expression::Import { kind, items, .. } = expr {
                report_import_style(ctx, meta, options, node_id, *kind, items);
            }
        }
    }
}

/// Configuration resolved from linter options.
#[derive(Debug, Clone, Copy)]
struct ConsistentTypeImportsOptions {
    /// Prefer explicit type import syntax.
    prefer_type_imports: bool,
    /// Prefer inline `type` modifiers when type imports are required.
    prefer_inline_type_imports: bool,
    /// Disallow `import("...")` style type annotations.
    disallow_type_annotations: bool,
}

/// Resolve rule options from linter configuration.
fn consistent_type_imports_options(ctx: &LintModuleAstContext<'_>) -> ConsistentTypeImportsOptions {
    ConsistentTypeImportsOptions {
        prefer_type_imports: ctx.options.consistent_type_imports_prefer_type_imports,
        prefer_inline_type_imports: ctx
            .options
            .consistent_type_imports_prefer_inline_type_imports,
        disallow_type_annotations: ctx
            .options
            .consistent_type_imports_disallow_type_annotations,
    }
}

/// Report one diagnostic for forbidden `import(...)` type annotations.
fn report_type_import_annotation(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    node_id: ast::LocalNodeId<ast::Expression>,
    expression: &ast::Expression,
) {
    // skip non type import expressions
    if !matches!(expression, Expression::TypeImport { .. }) {
        return;
    }

    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    let diagnostic = LintDiagnostic::new(
        CONSISTENT_TYPE_IMPORTS.id,
        CONSISTENT_TYPE_IMPORTS.code,
        CONSISTENT_TYPE_IMPORTS.category,
        severity,
        "`import(...)` type annotations are forbidden",
        ctx.module.file_id,
        ctx.tree.get_span(node_id),
    )
    .with_label("use named type imports instead of `import(...)`");
    ctx.report(diagnostic);
}

/// Report one diagnostic for one import declaration based on selected style options.
fn report_import_style(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static LintMeta,
    options: ConsistentTypeImportsOptions,
    import_id: ast::LocalNodeId<ast::Expression>,
    import_kind: DependencyKind,
    items: &[ast::LocalNodeId<ast::DependencyItem>],
) {
    // extract reusable import item classification
    let inline_type_items = inline_type_item_ids(ctx, items);
    let all_inline_types = import_all_inline_type_items(ctx, items);
    let supports_inline_style = import_supports_inline_style(ctx, items);

    // enforce value import mode without any type modifiers
    if !options.prefer_type_imports {
        if import_kind != DependencyKind::Type && inline_type_items.is_empty() {
            return;
        }

        let severity = ctx.get_effective_severity(meta, import_id);
        if !severity.is_enabled() {
            return;
        }

        let mut diagnostic = LintDiagnostic::new(
            CONSISTENT_TYPE_IMPORTS.id,
            CONSISTENT_TYPE_IMPORTS.code,
            CONSISTENT_TYPE_IMPORTS.category,
            severity,
            "use value imports without `type` modifiers",
            ctx.module.file_id,
            ctx.tree.get_span(import_id),
        )
        .with_label("remove type import modifiers");
        if ctx.compute_fixes
            && let Some(fix) =
                consistent_type_import_no_type_fix(ctx, import_id, import_kind, &inline_type_items)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        ctx.report(diagnostic);
        return;
    }

    // enforce inline `type` style when configured
    if options.prefer_inline_type_imports {
        if import_kind != DependencyKind::Type || !supports_inline_style {
            return;
        }

        let severity = ctx.get_effective_severity(meta, import_id);
        if !severity.is_enabled() {
            return;
        }

        let mut diagnostic = LintDiagnostic::new(
            CONSISTENT_TYPE_IMPORTS.id,
            CONSISTENT_TYPE_IMPORTS.code,
            CONSISTENT_TYPE_IMPORTS.category,
            severity,
            "use inline `type` specifiers for type-only imports",
            ctx.module.file_id,
            ctx.tree.get_span(import_id),
        )
        .with_label("prefer inline `type` modifiers");
        if ctx.compute_fixes
            && let Some(fix) = consistent_type_import_inline_fix(ctx, import_id, items)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        ctx.report(diagnostic);
        return;
    }

    // enforce top level `import type` style for type only named imports
    if import_kind == DependencyKind::Type || !all_inline_types {
        return;
    }

    let severity = ctx.get_effective_severity(meta, import_id);
    if !severity.is_enabled() {
        return;
    }

    let mut diagnostic = LintDiagnostic::new(
        CONSISTENT_TYPE_IMPORTS.id,
        CONSISTENT_TYPE_IMPORTS.code,
        CONSISTENT_TYPE_IMPORTS.category,
        severity,
        "use `import type { ... }` for type-only imports",
        ctx.module.file_id,
        ctx.tree.get_span(import_id),
    )
    .with_label("prefer top-level `import type`");
    if ctx.compute_fixes
        && let Some(fix) = consistent_type_import_top_level_fix(ctx, import_id, items)
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Return import item ids that use inline `type` modifiers.
fn inline_type_item_ids(
    ctx: &LintModuleAstContext<'_>,
    items: &[ast::LocalNodeId<ast::DependencyItem>],
) -> Vec<ast::LocalNodeId<ast::DependencyItem>> {
    items
        .iter()
        .copied()
        .filter(|item_id| {
            let item = ctx.tree.get(*item_id);
            item.kind == Some(DependencyKind::Type)
        })
        .collect()
}

/// Return whether every import item uses an inline `type` modifier.
fn import_all_inline_type_items(
    ctx: &LintModuleAstContext<'_>,
    items: &[ast::LocalNodeId<ast::DependencyItem>],
) -> bool {
    !items.is_empty()
        && items.iter().all(|item_id| {
            let item = ctx.tree.get(*item_id);
            item.kind == Some(DependencyKind::Type)
        })
}

/// Return whether all import items support inline `type` modifiers.
fn import_supports_inline_style(
    ctx: &LintModuleAstContext<'_>,
    items: &[ast::LocalNodeId<ast::DependencyItem>],
) -> bool {
    !items.is_empty()
        && items.iter().all(|item_id| {
            let item = ctx.tree.get(*item_id);
            item.mode == DependencyMode::Item
        })
}

/// Build one fix that rewrites inline type imports into top-level `import type`.
fn consistent_type_import_top_level_fix(
    ctx: &LintModuleAstContext<'_>,
    import_id: ast::LocalNodeId<ast::Expression>,
    items: &[ast::LocalNodeId<ast::DependencyItem>],
) -> Option<LintFix> {
    let import_span = ctx.tree.get_span(import_id);
    let import_text = ctx.get_span_text(import_span);
    let keyword_offset = import_text.find("import")?;

    // only auto-fix canonical import statements with leading trivia only
    if !import_text[..keyword_offset].trim().is_empty() {
        return None;
    }

    let mut builder = ctx.edit_builder().insert(
        import_span.start + keyword_offset as u32 + "import".len() as u32,
        " type",
    );

    // strip inline `type` prefixes from each import item
    for item_id in items {
        let item = ctx.tree.get(*item_id);
        if item.kind != Some(DependencyKind::Type) {
            continue;
        }

        let item_span = ctx.tree.get_span(*item_id);
        let item_text = ctx.get_span_text(item_span);
        let rewritten_text = dependency_item_strip_inline_type_keyword(item_text)?;
        builder = builder.replace(item_span, rewritten_text);
    }

    let edits = builder.into_edits();
    Some(LintFix::safe("Rewrite to `import type` syntax").with_edits(edits))
}

/// Build one fix that rewrites top-level `import type` into inline `type` specifiers.
fn consistent_type_import_inline_fix(
    ctx: &LintModuleAstContext<'_>,
    import_id: ast::LocalNodeId<ast::Expression>,
    items: &[ast::LocalNodeId<ast::DependencyItem>],
) -> Option<LintFix> {
    // only convert named import items because inline `type` only applies there
    if !import_supports_inline_style(ctx, items) {
        return None;
    }

    let import_span = ctx.tree.get_span(import_id);
    let import_text = ctx.get_span_text(import_span);
    let removal_span = import_type_keyword_removal_span(import_span, import_text)?;
    let mut builder = ctx.edit_builder().replace(removal_span, "");

    // add inline `type` to each named item
    for item_id in items {
        let item = ctx.tree.get(*item_id);
        if item.mode != DependencyMode::Item {
            return None;
        }

        let item_span = ctx.tree.get_span(*item_id);
        let item_text = ctx.get_span_text(item_span);
        let rewritten_text = dependency_item_insert_inline_type_keyword(item_text);
        builder = builder.replace(item_span, rewritten_text);
    }

    let edits = builder.into_edits();
    Some(LintFix::safe("Rewrite to inline type import syntax").with_edits(edits))
}

/// Build one fix that rewrites `import type` and inline type specifiers to value imports.
fn consistent_type_import_no_type_fix(
    ctx: &LintModuleAstContext<'_>,
    import_id: ast::LocalNodeId<ast::Expression>,
    import_kind: DependencyKind,
    inline_type_items: &[ast::LocalNodeId<ast::DependencyItem>],
) -> Option<LintFix> {
    let import_span = ctx.tree.get_span(import_id);
    let import_text = ctx.get_span_text(import_span);
    let mut builder = ctx.edit_builder();

    // remove one top level type keyword for `import type`
    if import_kind == DependencyKind::Type {
        let removal_span = import_type_keyword_removal_span(import_span, import_text)?;
        builder = builder.replace(removal_span, "");
    }

    // remove inline type markers in import items
    for item_id in inline_type_items {
        let item_span = ctx.tree.get_span(*item_id);
        let item_text = ctx.get_span_text(item_span);
        let rewritten_text = dependency_item_strip_inline_type_keyword(item_text)?;
        builder = builder.replace(item_span, rewritten_text);
    }

    let edits = builder.into_edits();
    Some(LintFix::safe("Rewrite to value import syntax").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_import_type() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_allows_import_type.ds",
            r#"
import type { Foo, Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_detects_inline_type_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_detects_inline_type_imports.ds",
            r#"
import { type Foo, type Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_has_fix("consistent-type-imports");
    }

    #[test]
    fn test_allows_mixed_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        // mixed imports are allowed (can't use `import type` for these)
        let result = test.lint_ast(
            "consistent_type_imports/test_allows_mixed_imports.ds",
            r#"
import { Foo, type Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_allows_value_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_allows_value_imports.ds",
            r#"
import { foo, bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_allows_namespace_import() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_allows_namespace_import.ds",
            r#"
import * as foo from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_fix_rewrites_inline_type_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_fix_rewrites_inline_type_imports.ds",
            r#"
import { type Foo, type Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import type { Foo, Bar } from "foo";
"#,
            );
    }

    #[test]
    fn test_fix_rewrites_alias_type_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_fix_rewrites_alias_type_imports.ds",
            r#"
import { type Foo as FooModel, type Bar as BarModel } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import type { Foo as FooModel, Bar as BarModel } from "foo";
"#,
            );
    }

    #[test]
    fn test_mutation_detects_single_inline_type_import() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_mutation_detects_single_inline_type_import.ds",
            r#"
import { type OnlyType } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import type { OnlyType } from "foo";
"#,
            );
    }

    #[test]
    fn test_no_type_mode_flags_import_type() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports)
            .with_options(|options| options.consistent_type_imports_prefer_type_imports = false);
        let result = test.lint_ast(
            "consistent_type_imports/test_no_type_mode_flags_import_type.ds",
            r#"
import type { Foo } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import { Foo } from "foo";
"#,
            );
    }

    #[test]
    fn test_no_type_mode_flags_inline_type_specifier() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports)
            .with_options(|options| options.consistent_type_imports_prefer_type_imports = false);
        let result = test.lint_ast(
            "consistent_type_imports/test_no_type_mode_flags_inline_type_specifier.ds",
            r#"
import { type Foo, Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import { Foo, Bar } from "foo";
"#,
            );
    }

    #[test]
    fn test_no_type_mode_allows_value_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports)
            .with_options(|options| options.consistent_type_imports_prefer_type_imports = false);
        let result = test.lint_ast(
            "consistent_type_imports/test_no_type_mode_allows_value_imports.ds",
            r#"
import { Foo, Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_inline_mode_allows_inline_type_specifiers() {
        let test =
            TestProgram::for_rule_without_prelude(ConsistentTypeImports).with_options(|options| {
                options.consistent_type_imports_prefer_inline_type_imports = true
            });
        let result = test.lint_ast(
            "consistent_type_imports/test_inline_mode_allows_inline_type_specifiers.ds",
            r#"
import { type Foo, type Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }

    #[test]
    fn test_inline_mode_flags_top_level_type_import() {
        let test =
            TestProgram::for_rule_without_prelude(ConsistentTypeImports).with_options(|options| {
                options.consistent_type_imports_prefer_inline_type_imports = true
            });
        let result = test.lint_ast(
            "consistent_type_imports/test_inline_mode_flags_top_level_type_import.ds",
            r#"
import type { Foo, Bar } from "foo"
"#,
        );
        test.result(result)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import { type Foo, type Bar } from "foo";
"#,
            );
    }

    #[test]
    fn test_disallow_type_annotations_flags_import_type_expressions() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let result = test.lint_ast(
            "consistent_type_imports/test_disallow_type_annotations_flags_import_type_expressions.ds",
            r#"
type Foo = import("foo").Foo
"#,
        );
        test.result(result).assert_lint("consistent-type-imports");
    }

    #[test]
    fn test_disallow_type_annotations_can_be_disabled() {
        let test =
            TestProgram::for_rule_without_prelude(ConsistentTypeImports).with_options(|options| {
                options.consistent_type_imports_disallow_type_annotations = false
            });
        let result = test.lint_ast(
            "consistent_type_imports/test_disallow_type_annotations_can_be_disabled.ds",
            r#"
type Foo = import("foo").Foo
"#,
        );
        test.result(result)
            .assert_no_lint("consistent-type-imports");
    }
}
