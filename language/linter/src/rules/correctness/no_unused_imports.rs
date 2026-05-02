use std::collections::HashMap;

use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{collect_module_resolved_read_symbol_usage, import_item_removal_span};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow imported bindings that are never used.
    ///
    /// Unused imports add noise and can hide stale dependencies.
    #[lint(
        id = "no-unused-imports",
        code = "LC035",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoUnusedImports,
    "Disallow unused import bindings"
}

impl LintRule for NoUnusedImports {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnusedImports::meta()
    }

    /// Check module DIR nodes for unused import bindings.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let read_symbols =
            collect_module_resolved_read_symbol_usage(ctx.module_id(), ctx.tree, ctx.types);
        let import_clauses = collect_import_clauses(ctx);
        let canonical_counts = canonical_import_symbol_counts(ctx, &import_clauses);

        // report each unused import binding
        for clause in &import_clauses {
            for (index, item_id) in clause.items.iter().enumerate() {
                let item = ctx.tree.get(*item_id);
                let Some(symbol_id) = item.symbol() else {
                    continue;
                };

                // enforce this lint guard
                if import_symbol_is_used(ctx, symbol_id, &read_symbols, &canonical_counts) {
                    continue;
                }

                // resolve effective lint severity
                let severity = ctx.get_effective_severity(meta, *item_id);
                if !severity.is_enabled() {
                    continue;
                }

                // resolve diagnostic span
                let span = ctx.get_span(*item_id);
                let mut diagnostic = LintReport::new(
                    NO_UNUSED_IMPORTS.id,
                    NO_UNUSED_IMPORTS.code,
                    NO_UNUSED_IMPORTS.category,
                    severity,
                    "unused import binding",
                    span,
                )
                .label("this import is never used");

                // attach fix when enabled
                if ctx.include_fixes
                    && let Some(fix) = unused_import_fix(ctx, clause, index)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// One import expression clause and its dependency items.
#[derive(Clone)]
struct ImportClause {
    /// The import expression node.
    expression_id: dir::LocalNodeId<dir::Expression>,
    /// Import items in source order.
    items: Vec<dir::LocalNodeId<dir::DependencyItem>>,
}

/// Collect import clauses in this module.
fn collect_import_clauses(ctx: &LintModuleDirContext<'_>) -> Vec<ImportClause> {
    let mut clauses = Vec::new();

    // collect import clauses only
    for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
        let expression = ctx.tree.get(expression_id);
        let dir::Expression::Import {
            items: import_items,
            ..
        } = expression
        else {
            continue;
        };

        clauses.push(ImportClause {
            expression_id,
            items: import_items.clone().unwrap_or_default(),
        });
    }

    clauses
}

/// Count canonical targets for local import bindings.
fn canonical_import_symbol_counts(
    ctx: &LintModuleDirContext<'_>,
    clauses: &[ImportClause],
) -> HashMap<dir::GlobalSymbolId, usize> {
    let mut counts = HashMap::new();

    // count canonical symbols so we only apply canonical fallback for unique mappings
    for clause in clauses {
        for item_id in &clause.items {
            let item = ctx.tree.get(*item_id);
            let Some(symbol_id) = item.symbol() else {
                continue;
            };
            let symbol = ctx.symbols.get_symbol(symbol_id);
            let Some(canonical_id) = symbol.canonical_symbol.or(symbol.target_symbol) else {
                continue;
            };

            *counts.entry(canonical_id).or_default() += 1;
        }
    }

    counts
}

/// Return true when an import symbol is used in this module.
fn import_symbol_is_used(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::LocalSymbolId,
    read_symbols: &std::collections::HashSet<dir::GlobalSymbolId>,
    canonical_counts: &HashMap<dir::GlobalSymbolId, usize>,
) -> bool {
    // direct local usage
    if read_symbols.contains(&symbol_id.into_global(ctx.module_id())) {
        return true;
    }

    // canonical fallback for resolver paths that bypass local import aliases
    let symbol = ctx.symbols.get_symbol(symbol_id);
    let Some(canonical_id) = symbol.canonical_symbol.or(symbol.target_symbol) else {
        return false;
    };
    let is_unique_canonical_binding = canonical_counts
        .get(&canonical_id)
        .is_some_and(|count| *count == 1);
    if !is_unique_canonical_binding {
        return false;
    }

    read_symbols.contains(&canonical_id)
}

/// Build a safe fix for one unused import binding when removal is syntactically local.
fn unused_import_fix(
    ctx: &LintModuleDirContext<'_>,
    clause: &ImportClause,
    item_index: usize,
) -> Option<LintFix> {
    // resolve source spans for the import expression and all clause items
    let import_expression_span = ctx.get_span(clause.expression_id);
    let item_spans: Vec<_> = clause
        .items
        .iter()
        .map(|item_id| ctx.get_span(*item_id))
        .collect();

    // resolve the span to remove one import item safely
    let remove_span = import_item_removal_span(
        ctx.source_text(),
        import_expression_span,
        &item_spans,
        item_index,
    )?;
    if remove_span.is_empty() {
        return None;
    }

    // build fix edits
    let edits = ctx.edit_builder().delete(remove_span).into_edits();
    Some(LintFix::safe("Remove unused import binding").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Report a named import that is never referenced.
    #[test]
    fn test_flags_unused_named_import() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/source.ds" => r#"
export const value = 1;
"#,
                "no_unused_imports/consumer.ds" => r#"
import { value } from "./source.ds";
"#,
            },
            "no_unused_imports/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unused-imports")
            .assert_lint_count("no-unused-imports", 1);
    }

    /// Allow a named import that is referenced.
    #[test]
    fn test_allows_used_named_import() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/source_used.ds" => r#"
export const value = 1;
"#,
                "no_unused_imports/consumer_used.ds" => r#"
import { value } from "./source_used.ds";

const output = value;
"#,
            },
            "no_unused_imports/consumer_used.ds",
        );

        test.result(diagnostics).assert_no_lint("no-unused-imports");
    }

    /// Report only the unused bindings from a mixed import clause.
    #[test]
    fn test_reports_only_unused_bindings() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/source_mixed.ds" => r#"
export const used = 1;
export const unused = 2;
"#,
                "no_unused_imports/consumer_mixed.ds" => r#"
import { used, unused } from "./source_mixed.ds";

const output = used;
"#,
            },
            "no_unused_imports/consumer_mixed.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unused-imports")
            .assert_lint_count("no-unused-imports", 1);
    }

    /// Treat imports used in type positions as used.
    #[test]
    fn test_allows_import_used_in_type_position() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/source_type.ds" => r#"
export type Payload = {
    value: int32;
};
"#,
                "no_unused_imports/consumer_type.ds" => r#"
import { Payload } from "./source_type.ds";

function usePayload(value: Payload): Payload {
    return value;
}
"#,
            },
            "no_unused_imports/consumer_type.ds",
        );

        test.result(diagnostics).assert_no_lint("no-unused-imports");
    }

    /// Ignore side-effect-only imports with no bindings.
    #[test]
    fn test_ignores_side_effect_only_import() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/source_side_effect.ds" => r#"
export const value = 1;
"#,
                "no_unused_imports/consumer_side_effect.ds" => r#"
import "./source_side_effect.ds";
"#,
            },
            "no_unused_imports/consumer_side_effect.ds",
        );

        test.result(diagnostics).assert_no_lint("no-unused-imports");
    }

    /// Treat namespace imports referenced through members as used.
    #[test]
    fn test_allows_used_namespace_import() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/source_namespace.ds" => r#"
export const value = 1;
"#,
                "no_unused_imports/consumer_namespace.ds" => r#"
import * as sourceModule from "./source_namespace.ds";

const output = sourceModule.value;
"#,
            },
            "no_unused_imports/consumer_namespace.ds",
        );

        test.result(diagnostics).assert_no_lint("no-unused-imports");
    }

    /// Report unused namespace imports.
    #[test]
    fn test_flags_unused_namespace_import() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/source_unused_namespace.ds" => r#"
export const value = 1;
"#,
                "no_unused_imports/consumer_unused_namespace.ds" => r#"
import * as sourceModule from "./source_unused_namespace.ds";
"#,
            },
            "no_unused_imports/consumer_unused_namespace.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unused-imports")
            .assert_lint_count("no-unused-imports", 1);
    }

    /// Safely remove an entire single-binding import.
    #[test]
    fn test_fix_single_binding_import() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/fix_single_source.ds" => r#"
export const value = 1;
"#,
                "no_unused_imports/fix_single_consumer.ds" => r#"
import { value } from "./fix_single_source.ds";

const answer = 1;
"#,
            },
            "no_unused_imports/fix_single_consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unused-imports")
            .assert_has_fix("no-unused-imports")
            .assert_safe_fixed(
                r#"
const answer = 1;
"#,
            );
    }

    /// Safely remove one unused binding from a multi-binding import.
    #[test]
    fn test_fix_removes_one_binding_from_named_list() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/fix_named_source.ds" => r#"
export const used = 1;
export const unused = 2;
"#,
                "no_unused_imports/fix_named_consumer.ds" => r#"
import { used, unused } from "./fix_named_source.ds";

const answer = used;
"#,
            },
            "no_unused_imports/fix_named_consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unused-imports")
            .assert_has_fix("no-unused-imports")
            .assert_safe_fixed(
                r#"
import { used } from "./fix_named_source.ds";

const answer = used;
"#,
            );
    }

    /// Avoid autofix for mixed default-and-named lists where local deletion is ambiguous.
    #[test]
    fn test_no_fix_for_mixed_default_and_named_import() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedImports);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unused_imports/fix_mixed_source.ds" => r#"
export default 1;
export const used = 2;
"#,
                "no_unused_imports/fix_mixed_consumer.ds" => r#"
import defaultValue, { used } from "./fix_mixed_source.ds";

const answer = used;
"#,
            },
            "no_unused_imports/fix_mixed_consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unused-imports")
            .assert_has_no_fix("no-unused-imports");
    }
}
