use std::collections::HashSet;

use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    dependency_item_insert_inline_type_keyword, dependency_item_strip_inline_type_keyword,
    expression_is_in_type_position, import_type_keyword_removal_span,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Enforce consistent type import style.
    ///
    /// Prefer dedicated type imports for bindings that are only used from
    /// type positions.
    #[lint(
        id = "consistent-type-imports",
        code = "LY007",
        category = Style,
        level = Dir,
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
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        ConsistentTypeImports::meta()
    }

    /// Check module DIR nodes for type-only import style.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let options = consistent_type_imports_options(ctx);
        let import_clauses = collect_import_clauses(ctx);
        let tracked_symbols = tracked_import_symbols(ctx, &import_clauses);
        let reference_usage = collect_import_symbol_reference_context_usage(ctx, &tracked_symbols);

        // inspect the module once for import declaration style and type import expressions
        if options.disallow_type_annotations {
            for type_expression_id in ctx.tree.iter_node_ids_of_type::<dir::TypeExpression>() {
                let type_expression = ctx.tree.get(type_expression_id);
                report_type_import_annotation(ctx, meta, type_expression_id, type_expression);
            }
        }

        // apply one declaration style check per import clause
        for clause in &import_clauses {
            report_import_style(ctx, meta, options, clause, &reference_usage);
        }
    }
}

/// Configuration resolved from linter options.
#[derive(Debug, Clone, Copy)]
struct ConsistentTypeImportsOptions {
    /// Prefer explicit type imports.
    prefer_type_imports: bool,
    /// Prefer inline `type` modifiers when type imports are required.
    prefer_inline_type_imports: bool,
    /// Disallow `import("...")` style type annotations.
    disallow_type_annotations: bool,
}

/// Context specific reference usage for one symbol.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SymbolReferenceContextUsage {
    /// Whether the symbol is referenced from a type position.
    has_type_reference: bool,
    /// Whether the symbol is referenced from a value position.
    has_value_reference: bool,
}

impl SymbolReferenceContextUsage {
    /// Record one reference in the requested context.
    fn record_reference(&mut self, is_type_position: bool) {
        if is_type_position {
            self.has_type_reference = true;
        } else {
            self.has_value_reference = true;
        }
    }

    /// Return true when every recorded reference is type only.
    fn is_type_only(self) -> bool {
        self.has_type_reference && !self.has_value_reference
    }
}

/// One import declaration and its dependency items.
#[derive(Debug, Clone)]
struct ImportClause {
    /// The import expression node.
    expression_id: dir::LocalNodeId<dir::Expression>,
    /// The top level import space.
    import_space: dir::DependencySpace,
    /// Whether the import carries trailing arguments or attributes.
    has_arguments: bool,
    /// Import items in source order.
    items: Vec<dir::LocalNodeId<dir::DependencyItem>>,
}

/// Resolve rule options from linter configuration.
fn consistent_type_imports_options(ctx: &LintModuleDirContext<'_>) -> ConsistentTypeImportsOptions {
    ConsistentTypeImportsOptions {
        prefer_type_imports: ctx
            .options
            .style
            .consistent_type_imports_prefer_type_imports,
        prefer_inline_type_imports: ctx
            .options
            .style
            .consistent_type_imports_prefer_inline_type_imports,
        disallow_type_annotations: ctx
            .options
            .style
            .consistent_type_imports_disallow_type_annotations,
    }
}

/// Collect import clauses in source order.
fn collect_import_clauses(ctx: &LintModuleDirContext<'_>) -> Vec<ImportClause> {
    let mut clauses = Vec::new();

    // collect import declarations only
    for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
        let expression = ctx.tree.get(expression_id);
        let dir::Expression::Import {
            space,
            items,
            arguments,
            ..
        } = expression
        else {
            continue;
        };

        clauses.push(ImportClause {
            expression_id,
            import_space: *space,
            has_arguments: arguments.is_some(),
            items: items.clone().unwrap_or_default(),
        });
    }

    clauses
}

/// Collect tracked import binding symbols for one module.
fn tracked_import_symbols(
    ctx: &LintModuleDirContext<'_>,
    clauses: &[ImportClause],
) -> HashSet<dir::GlobalSymbolId> {
    let mut symbols = HashSet::new();

    // keep direct local import aliases only
    for clause in clauses {
        for item_id in &clause.items {
            let item = ctx.tree.get(*item_id);
            let Some(symbol_id) = item.symbol() else {
                continue;
            };

            symbols.insert(symbol_id.into_global(ctx.module_id()));
        }
    }

    symbols
}

/// Collect type and value reference contexts for tracked import symbols in one module.
fn collect_import_symbol_reference_context_usage(
    ctx: &LintModuleDirContext<'_>,
    tracked_symbols: &HashSet<dir::GlobalSymbolId>,
) -> std::collections::HashMap<dir::GlobalSymbolId, SymbolReferenceContextUsage> {
    let mut usage: std::collections::HashMap<dir::GlobalSymbolId, SymbolReferenceContextUsage> =
        std::collections::HashMap::new();

    // inspect all direct symbol references once in tree order
    for (expression_id, _) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
        let Some(symbol_id) = ctx.expression_target_symbol(expression_id) else {
            continue;
        };
        if !tracked_symbols.contains(&symbol_id) {
            continue;
        }

        let is_type_position = expression_is_in_type_position(ctx.tree, expression_id);
        usage
            .entry(symbol_id)
            .or_default()
            .record_reference(is_type_position);
    }

    usage
}

/// Report one diagnostic for forbidden `import(...)` type annotations.
fn report_type_import_annotation(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &'static LintMeta,
    node_id: dir::LocalNodeId<dir::TypeExpression>,
    type_expression: &dir::TypeExpression,
) {
    // skip non type import expressions
    if !matches!(type_expression, dir::TypeExpression::Import { .. }) {
        return;
    }

    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    let diagnostic = LintReport::new(
        CONSISTENT_TYPE_IMPORTS.id,
        CONSISTENT_TYPE_IMPORTS.code,
        CONSISTENT_TYPE_IMPORTS.category,
        severity,
        "`import(...)` type annotations are forbidden",
        ctx.get_span(node_id),
    )
    .label("use named type imports instead of `import(...)`");
    ctx.report(diagnostic);
}

/// Report one diagnostic for one import declaration based on selected style options.
fn report_import_style(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &'static LintMeta,
    options: ConsistentTypeImportsOptions,
    clause: &ImportClause,
    reference_usage: &std::collections::HashMap<dir::GlobalSymbolId, SymbolReferenceContextUsage>,
) {
    let inline_type_item_ids = clause_inline_type_item_ids(ctx, clause);
    let has_top_level_type_keyword =
        import_declaration_has_top_level_type_keyword(ctx, clause.expression_id);

    // enforce value imports without type modifiers
    if !options.prefer_type_imports {
        if !has_top_level_type_keyword && inline_type_item_ids.is_empty() {
            return;
        }

        let severity = ctx.get_effective_severity(meta, clause.expression_id);
        if !severity.is_enabled() {
            return;
        }

        let mut diagnostic = LintReport::new(
            CONSISTENT_TYPE_IMPORTS.id,
            CONSISTENT_TYPE_IMPORTS.code,
            CONSISTENT_TYPE_IMPORTS.category,
            severity,
            "use value imports without `type` modifiers",
            ctx.get_span(clause.expression_id),
        )
        .label("remove type import modifiers");
        if ctx.include_fixes
            && let Some(fix) =
                consistent_type_import_no_type_fix(ctx, clause.expression_id, &inline_type_item_ids)
        {
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
        return;
    }

    let semantic_type_only_item_ids =
        clause_semantic_type_only_item_ids(ctx, clause, reference_usage);

    // prefer inline `type` modifiers when configured
    if options.prefer_inline_type_imports {
        if has_top_level_type_keyword && clause_supports_inline_style(ctx, clause) {
            let severity = ctx.get_effective_severity(meta, clause.expression_id);
            if !severity.is_enabled() {
                return;
            }

            let mut diagnostic = LintReport::new(
                CONSISTENT_TYPE_IMPORTS.id,
                CONSISTENT_TYPE_IMPORTS.code,
                CONSISTENT_TYPE_IMPORTS.category,
                severity,
                "use inline `type` specifiers for type-only imports",
                ctx.get_span(clause.expression_id),
            )
            .label("prefer inline `type` modifiers");
            if ctx.include_fixes
                && let Some(fix) =
                    consistent_type_import_inline_fix(ctx, clause.expression_id, &clause.items)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
            return;
        }

        let selected_item_ids =
            clause_missing_inline_type_only_item_ids(ctx, clause, &semantic_type_only_item_ids);
        if selected_item_ids.is_empty() || !item_ids_support_inline_style(ctx, &selected_item_ids) {
            return;
        }

        report_partial_type_only_imports(
            ctx,
            meta,
            clause,
            &selected_item_ids,
            "prefer inline `type` modifiers",
            consistent_type_import_inline_fix(ctx, clause.expression_id, &selected_item_ids),
        );
        return;
    }

    // canonicalize all inline `type` imports into one top level `import type`
    if clause.import_space != dir::DependencySpace::Type
        && clause_all_items_are_inline_type(ctx, clause)
    {
        let severity = ctx.get_effective_severity(meta, clause.expression_id);
        if !severity.is_enabled() {
            return;
        }

        let mut diagnostic = LintReport::new(
            CONSISTENT_TYPE_IMPORTS.id,
            CONSISTENT_TYPE_IMPORTS.code,
            CONSISTENT_TYPE_IMPORTS.category,
            severity,
            "use `import type { ... }` for type-only imports",
            ctx.get_span(clause.expression_id),
        )
        .label("prefer top-level `import type`");
        if ctx.include_fixes
            && let Some(fix) =
                consistent_type_import_top_level_fix(ctx, clause.expression_id, &clause.items)
        {
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
        return;
    }

    // convert whole declarations when every binding is type only
    if clause.import_space != dir::DependencySpace::Type
        && semantic_type_only_item_ids.len() == clause.items.len()
        && !semantic_type_only_item_ids.is_empty()
    {
        let severity = ctx.get_effective_severity(meta, clause.expression_id);
        if !severity.is_enabled() {
            return;
        }

        let mut diagnostic = LintReport::new(
            CONSISTENT_TYPE_IMPORTS.id,
            CONSISTENT_TYPE_IMPORTS.code,
            CONSISTENT_TYPE_IMPORTS.category,
            severity,
            "all imports in this declaration are only used as types",
            ctx.get_span(clause.expression_id),
        )
        .label("prefer top-level `import type`");
        if ctx.include_fixes
            && !clause.has_arguments
            && let Some(fix) =
                consistent_type_import_top_level_fix(ctx, clause.expression_id, &clause.items)
        {
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
        return;
    }

    // accept declarations that already use top level `import type`
    if clause.import_space == dir::DependencySpace::Type {
        return;
    }

    // fall back to inline `type` markers for mixed declarations
    let selected_item_ids =
        clause_missing_inline_type_only_item_ids(ctx, clause, &semantic_type_only_item_ids);
    if selected_item_ids.is_empty() || !item_ids_support_inline_style(ctx, &selected_item_ids) {
        return;
    }

    report_partial_type_only_imports(
        ctx,
        meta,
        clause,
        &selected_item_ids,
        "mark these bindings with inline `type`",
        consistent_type_import_inline_fix(ctx, clause.expression_id, &selected_item_ids),
    );
}

/// Report one diagnostic for a mixed declaration with type only bindings.
fn report_partial_type_only_imports(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &'static LintMeta,
    clause: &ImportClause,
    item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
    label: &str,
    fix: Option<LintFix>,
) {
    let severity = ctx.get_effective_severity(meta, clause.expression_id);
    if !severity.is_enabled() {
        return;
    }

    let item_names = format_import_item_names(ctx, item_ids);
    let mut diagnostic = LintReport::new(
        CONSISTENT_TYPE_IMPORTS.id,
        CONSISTENT_TYPE_IMPORTS.code,
        CONSISTENT_TYPE_IMPORTS.category,
        severity,
        format!("imports `{item_names}` are only used as types"),
        ctx.get_span(clause.expression_id),
    )
    .label(label);
    if ctx.include_fixes
        && let Some(fix) = fix
    {
        diagnostic = diagnostic.fix(fix);
    }

    ctx.report(diagnostic);
}

/// Return import item ids that use inline `type` modifiers.
fn clause_inline_type_item_ids(
    ctx: &LintModuleDirContext<'_>,
    clause: &ImportClause,
) -> Vec<dir::LocalNodeId<dir::DependencyItem>> {
    clause
        .items
        .iter()
        .copied()
        .filter(|item_id| {
            let item_span = ctx.get_span(*item_id);
            let item_text = ctx.get_span_text(item_span);
            dependency_item_strip_inline_type_keyword(item_text).is_some()
        })
        .collect()
}

/// Return semantic type only item ids that still use value imports.
fn clause_semantic_type_only_item_ids(
    ctx: &LintModuleDirContext<'_>,
    clause: &ImportClause,
    reference_usage: &std::collections::HashMap<dir::GlobalSymbolId, SymbolReferenceContextUsage>,
) -> Vec<dir::LocalNodeId<dir::DependencyItem>> {
    clause
        .items
        .iter()
        .copied()
        .filter(|item_id| import_item_is_semantic_type_only(ctx, *item_id, reference_usage))
        .collect()
}

/// Return type only item ids that still need inline `type` markers.
fn clause_missing_inline_type_only_item_ids(
    ctx: &LintModuleDirContext<'_>,
    clause: &ImportClause,
    semantic_type_only_item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
) -> Vec<dir::LocalNodeId<dir::DependencyItem>> {
    semantic_type_only_item_ids
        .iter()
        .copied()
        .filter(|item_id| {
            if clause.import_space == dir::DependencySpace::Type {
                return true;
            }

            let item = ctx.tree.get(*item_id);
            item_space(item) != Some(dir::DependencySpace::Type)
        })
        .collect()
}

/// Return true when every import item already uses inline `type`.
fn clause_all_items_are_inline_type(ctx: &LintModuleDirContext<'_>, clause: &ImportClause) -> bool {
    !clause.items.is_empty()
        && clause.items.iter().all(|item_id| {
            let item = ctx.tree.get(*item_id);
            item_space(item) == Some(dir::DependencySpace::Type)
        })
}

/// Return true when every import item supports inline `type` modifiers.
fn clause_supports_inline_style(ctx: &LintModuleDirContext<'_>, clause: &ImportClause) -> bool {
    item_ids_support_inline_style(ctx, &clause.items)
}

/// Return true when every selected item supports inline `type` modifiers.
fn item_ids_support_inline_style(
    ctx: &LintModuleDirContext<'_>,
    item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
) -> bool {
    !item_ids.is_empty()
        && item_ids.iter().all(|item_id| {
            let item = ctx.tree.get(*item_id);
            item_binding(item) == Some(dir::DependencyBinding::Item)
        })
}

/// Return true when one import item is only used from type positions.
fn import_item_is_semantic_type_only(
    ctx: &LintModuleDirContext<'_>,
    item_id: dir::LocalNodeId<dir::DependencyItem>,
    reference_usage: &std::collections::HashMap<dir::GlobalSymbolId, SymbolReferenceContextUsage>,
) -> bool {
    let item = ctx.tree.get(item_id);
    let Some(symbol_id) = item.symbol() else {
        return false;
    };

    let usage = reference_usage
        .get(&symbol_id.into_global(ctx.module_id()))
        .copied()
        .unwrap_or_default();

    usage.is_type_only()
}

/// Return the dependency item kind when present.
fn item_space(item: &dir::DependencyItem) -> Option<dir::DependencySpace> {
    match item {
        dir::DependencyItem::Item { space, .. } => Some(*space),
        dir::DependencyItem::Value { .. } | dir::DependencyItem::Error => None,
    }
}

/// Return the dependency item binding.
fn item_binding(item: &dir::DependencyItem) -> Option<dir::DependencyBinding> {
    match item {
        dir::DependencyItem::Item { binding, .. } | dir::DependencyItem::Value { binding, .. } => {
            Some(*binding)
        }
        dir::DependencyItem::Error => None,
    }
}

/// Format import item names for diagnostics.
fn format_import_item_names(
    ctx: &LintModuleDirContext<'_>,
    item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
) -> String {
    item_ids
        .iter()
        .map(|item_id| import_item_name(ctx, *item_id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Resolve one readable import item name.
fn import_item_name(
    ctx: &LintModuleDirContext<'_>,
    item_id: dir::LocalNodeId<dir::DependencyItem>,
) -> String {
    let item = ctx.tree.get(item_id);
    if let Some(symbol_id) = item.symbol()
        && let Some(name_id) = ctx.symbols.get_symbol(symbol_id).name()
    {
        return ctx.strings.get(name_id).to_string();
    }

    ctx.get_span_text(ctx.get_span(item_id)).trim().to_string()
}

/// Build one fix that rewrites a declaration to top level `import type`.
fn consistent_type_import_top_level_fix(
    ctx: &LintModuleDirContext<'_>,
    import_id: dir::LocalNodeId<dir::Expression>,
    items: &[dir::LocalNodeId<dir::DependencyItem>],
) -> Option<LintFix> {
    let import_span = ctx.get_span(import_id);
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
        if item_space(item) != Some(dir::DependencySpace::Type) {
            continue;
        }

        let item_span = ctx.get_span(*item_id);
        let item_text = ctx.get_span_text(item_span);
        let rewritten_text = dependency_item_strip_inline_type_keyword(item_text)?;
        builder = builder.replace(item_span, rewritten_text);
    }

    let edits = builder.into_edits();
    Some(LintFix::safe("Rewrite to `import type`").with_edits(edits))
}

/// Build one fix that rewrites selected items to inline `type` specifiers.
fn consistent_type_import_inline_fix(
    ctx: &LintModuleDirContext<'_>,
    import_id: dir::LocalNodeId<dir::Expression>,
    item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
) -> Option<LintFix> {
    let mut builder = ctx.edit_builder();
    let import_span = ctx.get_span(import_id);
    let import_text = ctx.get_span_text(import_span);

    // remove the top level type keyword first when rewriting `import type`
    if let Some(removal_span) = import_type_keyword_removal_span(import_span, import_text) {
        builder = builder.replace(removal_span, "");
    }

    // add inline `type` to each selected item
    for item_id in item_ids {
        let item = ctx.tree.get(*item_id);
        if item_binding(item) != Some(dir::DependencyBinding::Item) {
            return None;
        }

        let item_span = ctx.get_span(*item_id);
        let item_text = ctx.get_span_text(item_span);
        let rewritten_text = dependency_item_insert_inline_type_keyword(item_text);
        builder = builder.replace(item_span, rewritten_text);
    }

    let edits = builder.into_edits();
    Some(LintFix::safe("Rewrite to inline type imports").with_edits(edits))
}

/// Build one fix that rewrites `import type` and inline specifiers to value imports.
fn consistent_type_import_no_type_fix(
    ctx: &LintModuleDirContext<'_>,
    import_id: dir::LocalNodeId<dir::Expression>,
    inline_type_item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
) -> Option<LintFix> {
    let import_span = ctx.get_span(import_id);
    let import_text = ctx.get_span_text(import_span);

    // rewrite plain `import type` declarations in one shot
    let rewritten_import_text = import_text.replacen("import type", "import", 1);
    if inline_type_item_ids.is_empty() && rewritten_import_text != import_text {
        let edits = ctx
            .edit_builder()
            .replace(import_span, rewritten_import_text)
            .into_edits();
        return Some(LintFix::safe("Rewrite to value imports").with_edits(edits));
    }

    let mut builder = ctx.edit_builder();

    // remove one top level type keyword for `import type`
    if let Some(removal_span) = import_type_keyword_removal_span(import_span, import_text) {
        builder = builder.replace(removal_span, "");
    }

    // remove inline type markers in import items
    for item_id in inline_type_item_ids {
        let item_span = ctx.get_span(*item_id);
        let item_text = ctx.get_span_text(item_span);
        let rewritten_text = dependency_item_strip_inline_type_keyword(item_text)?;
        builder = builder.replace(item_span, rewritten_text);
    }

    let edits = builder.into_edits();
    Some(LintFix::safe("Rewrite to value imports").with_edits(edits))
}

/// Return true when one import declaration spells a top level `import type`.
fn import_declaration_has_top_level_type_keyword(
    ctx: &LintModuleDirContext<'_>,
    import_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let import_span = ctx.get_span(import_id);
    let import_text = ctx.get_span_text(import_span);
    import_type_keyword_removal_span(import_span, import_text).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Allow a declaration that already uses top level `import type`.
    #[test]
    fn test_allows_import_type() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "consistent_type_imports/source_type.ds" => r#"
export type Foo = number;
export type Bar = string;
"#,
                "consistent_type_imports/consumer_type.ds" => r#"
import type { Foo, Bar } from "./source_type.ds";

type Example = (Foo, Bar);
"#,
            },
            "consistent_type_imports/consumer_type.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("consistent-type-imports");
    }

    /// Rewrite all inline type specifiers to top level `import type`.
    #[test]
    fn test_detects_inline_type_imports() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "consistent_type_imports/source_inline.ds" => r#"
export type Foo = number;
export type Bar = string;
"#,
                "consistent_type_imports/consumer_inline.ds" => r#"
import { type Foo, type Bar } from "./source_inline.ds";

type Example = (Foo, Bar);
"#,
            },
            "consistent_type_imports/consumer_inline.ds",
        );

        test.result(diagnostics)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import type { Foo, Bar } from "./source_inline.ds";

type Example = (Foo, Bar);
"#,
            );
    }

    /// Convert value imports that are only used in type positions.
    #[test]
    fn test_detects_value_imports_used_only_as_types() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "consistent_type_imports/source_semantic.ds" => r#"
export class Foo {}
export class Bar {}
"#,
                "consistent_type_imports/consumer_semantic.ds" => r#"
import { Foo, Bar } from "./source_semantic.ds";

type FooExample = Foo;
type BarExample = Bar;
"#,
            },
            "consistent_type_imports/consumer_semantic.ds",
        );

        test.result(diagnostics)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import type { Foo, Bar } from "./source_semantic.ds";

type FooExample = Foo;
type BarExample = Bar;
"#,
            );
    }

    /// Mark only the type only bindings in a mixed declaration.
    #[test]
    fn test_detects_mixed_type_only_bindings() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "consistent_type_imports/source_mixed.ds" => r#"
export class Foo {}
export class Bar {}
"#,
                "consistent_type_imports/consumer_mixed.ds" => r#"
import { Foo, Bar } from "./source_mixed.ds";

type Example = Foo;
const value = Bar;
"#,
            },
            "consistent_type_imports/consumer_mixed.ds",
        );

        test.result(diagnostics)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import { type Foo, Bar } from "./source_mixed.ds";

type Example = Foo;
const value = Bar;
"#,
            );
    }

    /// Keep mixed declarations that already mark the type only binding inline.
    #[test]
    fn test_allows_mixed_inline_type_bindings() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "consistent_type_imports/source_mixed_inline.ds" => r#"
export class Foo {}
export class Bar {}
"#,
                "consistent_type_imports/consumer_mixed_inline.ds" => r#"
import { type Foo, Bar } from "./source_mixed_inline.ds";

type Example = Foo;
const value = Bar;
"#,
            },
            "consistent_type_imports/consumer_mixed_inline.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("consistent-type-imports");
    }

    /// Remove type imports when the rule prefers value imports.
    #[test]
    fn test_no_type_mode_flags_import_type() {
        let test =
            TestProgram::for_rule_without_prelude(ConsistentTypeImports).with_options(|options| {
                options.style.consistent_type_imports_prefer_type_imports = false
            });
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "consistent_type_imports/source_no_type.ds" => r#"
export type Foo = number;
"#,
                "consistent_type_imports/consumer_no_type.ds" => r#"
import type { Foo } from "./source_no_type.ds";

type Example = Foo;
"#,
            },
            "consistent_type_imports/consumer_no_type.ds",
        );

        test.result(diagnostics)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import { Foo } from "./source_no_type.ds";

type Example = Foo;
"#,
            );
    }

    /// Prefer inline `type` modifiers when configured.
    #[test]
    fn test_inline_mode_flags_top_level_type_import() {
        let test =
            TestProgram::for_rule_without_prelude(ConsistentTypeImports).with_options(|options| {
                options
                    .style
                    .consistent_type_imports_prefer_inline_type_imports = true
            });
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "consistent_type_imports/source_inline_mode.ds" => r#"
export type Foo = number;
export type Bar = string;
"#,
                "consistent_type_imports/consumer_inline_mode.ds" => r#"
import type { Foo, Bar } from "./source_inline_mode.ds";

type Example = (Foo, Bar);
"#,
            },
            "consistent_type_imports/consumer_inline_mode.ds",
        );

        test.result(diagnostics)
            .assert_lint("consistent-type-imports")
            .assert_safe_fixed(
                r#"
import { type Foo, type Bar } from "./source_inline_mode.ds";

type Example = (Foo, Bar);
"#,
            );
    }

    /// Report forbidden `import(...)` type annotations by default.
    #[test]
    fn test_disallow_type_annotations_flags_import_type_expressions() {
        let test = TestProgram::for_rule_without_prelude(ConsistentTypeImports);
        let diagnostics = test.lint_dir(
            "consistent_type_imports/test_disallow_type_annotations.ds",
            r#"
type Foo = import("foo").Foo;
"#,
        );

        test.result(diagnostics)
            .assert_lint("consistent-type-imports");
    }

    /// Allow `import(...)` type annotations when the option disables the check.
    #[test]
    fn test_disallow_type_annotations_can_be_disabled() {
        let test =
            TestProgram::for_rule_without_prelude(ConsistentTypeImports).with_options(|options| {
                options
                    .style
                    .consistent_type_imports_disallow_type_annotations = false
            });
        let diagnostics = test.lint_dir(
            "consistent_type_imports/test_allow_type_annotations.ds",
            r#"
type Foo = import("foo").Foo;
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("consistent-type-imports");
    }
}
