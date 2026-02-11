use std::collections::HashSet;

use destack_dir::{self as dir, WellKnownSymbol};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    canonical_symbol_for, contains_map_with_empty_value_type, expression_type_map,
    is_void_or_never_type,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `Set<K>` over `Map<K, void | never>`.
    ///
    /// A map whose value type is empty usually models key membership and is
    /// better represented by a set.
    #[lint(
        id = "prefer-set-over-empty-map",
        code = "LY079",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Map)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferSetOverEmptyMap,
    "Prefer Set over Map with empty value type"
}

impl LintRule for PreferSetOverEmptyMap {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferSetOverEmptyMap::meta()
    }

    /// Check module DIR nodes for `Map<K, void | never>` usage.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let map_symbols = resolve_map_symbols(ctx);
        if map_symbols.is_empty() {
            return;
        }
        let mut reported_expression_ids = HashSet::new();

        // type aliases: query alias target types directly from the type table
        for (_, declaration) in ctx.tree.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Type {
                descriptor, value, ..
            } = declaration
            else {
                continue;
            };

            let type_symbol = descriptor.symbol.into_global(ctx.module_id());
            let Some(alias_target_type_id) = ctx.types.get_alias_target_type_id(type_symbol) else {
                continue;
            };
            if !type_contains_map_with_empty_value(ctx, alias_target_type_id, &map_symbols) {
                continue;
            }
            if !reported_expression_ids.insert(value.id) {
                continue;
            }

            report_prefer_set_over_empty_map(ctx, meta, *value);
        }

        for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            let should_report = match expression {
                // type annotations are lowered as type values
                dir::Expression::Type { value } => {
                    type_contains_map_with_empty_value(ctx, *value, &map_symbols)
                }
                // type references can carry map static arguments directly
                dir::Expression::LocalReference {
                    target_symbol,
                    static_arguments,
                    ..
                }
                | dir::Expression::ModuleReference {
                    target_symbol,
                    static_arguments,
                    ..
                }
                | dir::Expression::GlobalReference {
                    target_symbol,
                    static_arguments,
                    ..
                } => reference_has_empty_value_argument(
                    ctx,
                    *target_symbol,
                    static_arguments.as_deref(),
                    &map_symbols,
                ),
                // constructor calls keep generic arguments on the new expression
                dir::Expression::New {
                    left,
                    static_arguments,
                    ..
                } => new_map_has_empty_value_argument(ctx, *left, static_arguments, &map_symbols),
                _ => false,
            };
            if !should_report {
                continue;
            }
            if !reported_expression_ids.insert(expression_id.id) {
                continue;
            }

            report_prefer_set_over_empty_map(ctx, meta, expression_id);
        }
    }
}

/// Resolve all concrete `Map` symbols from type and value spaces.
fn resolve_map_symbols(ctx: &LintModuleDirContext<'_>) -> Vec<dir::GlobalSymbolId> {
    let Some(builtins) = ctx.program.builtins.as_ref() else {
        return Vec::new();
    };
    let profile = ctx.program.profile(ctx.profile_id);
    let map_name = ctx
        .program
        .strings
        .intern(WellKnownSymbol::Map.export_name());

    let mut symbols = Vec::new();

    // collect explicit type and value entries first
    for order in [
        dir::SymbolSpaceOrder::TypeOnly,
        dir::SymbolSpaceOrder::ValueOnly,
        dir::SymbolSpaceOrder::TypeThenValue,
        dir::SymbolSpaceOrder::ValueThenType,
    ] {
        if let Some(symbol) = builtins.get_declared_lib_symbol_from(&profile.key, map_name, order)
            && !symbols.contains(&symbol)
        {
            symbols.push(symbol);
        }
    }

    // include the profile well-known entry as an additional candidate
    if let Some(symbol) = ctx.get_well_known_symbol(WellKnownSymbol::Map)
        && !symbols.contains(&symbol)
    {
        symbols.push(symbol);
    }

    symbols
}

/// Report one prefer-set-over-empty-map diagnostic.
fn report_prefer_set_over_empty_map(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
) {
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.get_span(expression_id);
    ctx.report(
        LintDiagnostic::new(
            PREFER_SET_OVER_EMPTY_MAP.id,
            PREFER_SET_OVER_EMPTY_MAP.code,
            PREFER_SET_OVER_EMPTY_MAP.category,
            severity,
            "prefer Set over Map with empty value type",
            ctx.module.file_id,
            span,
        )
        .with_label("use `Set<K>` instead of `Map<K, void | never>`"),
    );
}

/// Return true when one `new` expression constructs `Map<_, void|never>`.
fn new_map_has_empty_value_argument(
    ctx: &LintModuleDirContext<'_>,
    callee_expression_id: dir::LocalNodeId<dir::Expression>,
    static_arguments: &Option<Vec<dir::LocalNodeId<dir::Argument>>>,
    map_symbols: &[dir::GlobalSymbolId],
) -> bool {
    let callee = ctx.tree.get(callee_expression_id);
    let Some(target_symbol) = callee.target_symbol() else {
        return false;
    };
    if !symbol_is_map(ctx, target_symbol, map_symbols) {
        return false;
    }

    let Some(static_arguments) = static_arguments else {
        return false;
    };
    if static_arguments.len() != 2 {
        return false;
    }

    let value_argument = ctx.tree.get(static_arguments[1]);
    let value_expression_id = value_argument.value();
    expression_is_void_or_never_type(ctx, value_expression_id)
}

/// Return true when one type tree contains `Map<_, void | never>`.
fn type_contains_map_with_empty_value(
    ctx: &LintModuleDirContext<'_>,
    type_id: dir::LocalTypeId,
    map_symbols: &[dir::GlobalSymbolId],
) -> bool {
    contains_map_with_empty_value_type(ctx.types, type_id, map_symbols)
}

/// Return true when one map reference has an empty value type argument.
fn reference_has_empty_value_argument(
    ctx: &LintModuleDirContext<'_>,
    target_symbol: dir::GlobalSymbolId,
    static_arguments: Option<&[dir::LocalNodeId<dir::Argument>]>,
    map_symbols: &[dir::GlobalSymbolId],
) -> bool {
    if !symbol_is_map(ctx, target_symbol, map_symbols) {
        return false;
    }

    let Some(static_arguments) = static_arguments else {
        return false;
    };
    if static_arguments.len() != 2 {
        return false;
    }

    let value_argument = ctx.tree.get(static_arguments[1]);
    let value_expression_id = value_argument.value();
    expression_is_void_or_never_type(ctx, value_expression_id)
}

/// Return true when the expression resolves to `void` or `never`.
fn expression_is_void_or_never_type(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    if matches!(
        expression,
        dir::Expression::TypeLiteral {
            value: dir::TypeLiteral::Void | dir::TypeLiteral::Never
        }
    ) {
        return true;
    }

    expression_type_map(
        &ctx.program,
        ctx.profile_id,
        ctx.module_id(),
        ctx.tree,
        ctx.symbols,
        ctx.types,
        expression_id,
        is_void_or_never_type,
    )
    .unwrap_or(false)
}

/// Return true when one symbol resolves to one map symbol candidate.
fn symbol_is_map(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::GlobalSymbolId,
    map_symbols: &[dir::GlobalSymbolId],
) -> bool {
    if map_symbols.contains(&symbol_id) {
        return true;
    }

    canonical_symbol_for(
        &ctx.program,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        symbol_id,
    )
    .is_some_and(|canonical_symbol_id| map_symbols.contains(&canonical_symbol_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Flag map aliases with `void` payloads.
    #[test]
    fn test_flags_map_void_type_alias() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_flags_map_void_type_alias.ds",
            r#"
type Membership = Map<string, void>;
"#,
        );
        test.result(result).assert_lint("prefer-set-over-empty-map");
    }

    /// Flag map aliases with `never` payloads.
    #[test]
    fn test_flags_map_never_type_alias() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_flags_map_never_type_alias.ds",
            r#"
type Membership = Map<string, never>;
"#,
        );
        test.result(result).assert_lint("prefer-set-over-empty-map");
    }

    /// Flag map constructors with `void` payloads.
    #[test]
    fn test_flags_map_void_constructor() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_flags_map_void_constructor.ds",
            r#"
let membership = new Map<string, void>();
"#,
        );
        test.result(result).assert_lint("prefer-set-over-empty-map");
    }

    /// Flag map constructors with `never` payloads.
    #[test]
    fn test_flags_map_never_constructor() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_flags_map_never_constructor.ds",
            r#"
let membership = new Map<string, never>();
"#,
        );
        test.result(result).assert_lint("prefer-set-over-empty-map");
    }

    /// Flag map aliases through local type aliases.
    #[test]
    fn test_flags_map_alias_through_void_type_alias() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_flags_map_alias_through_void_type_alias.ds",
            r#"
type EmptyValue = void;
type Membership = Map<string, EmptyValue>;
"#,
        );
        test.result(result).assert_lint("prefer-set-over-empty-map");
    }

    /// Flag both annotation and constructor when both use empty map values.
    #[test]
    fn test_flags_annotation_and_constructor_for_empty_map() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_flags_annotation_and_constructor_for_empty_map.ds",
            r#"
let membership: Map<string, void> = new Map<string, void>();
"#,
        );
        test.result(result)
            .assert_lint("prefer-set-over-empty-map")
            .assert_lint_count("prefer-set-over-empty-map", 2);
    }

    /// Flag imported type aliases that resolve to empty map values.
    #[test]
    fn test_flags_imported_alias_with_empty_map_value() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "prefer_set_over_empty_map/source.ds" => r#"
export type Membership = Map<string, void>;
"#,
                "prefer_set_over_empty_map/consumer.ds" => r#"
import { Membership } from "./source.ds";

let membership: Membership = new Map<string, void>();
"#,
            },
            "prefer_set_over_empty_map/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("prefer-set-over-empty-map")
            .assert_lint_count("prefer-set-over-empty-map", 1);
    }

    /// Allow maps with concrete value payloads.
    #[test]
    fn test_allows_map_with_value_type() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_allows_map_with_value_type.ds",
            r#"
type Table = Map<string, int32>;
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-set-over-empty-map");
    }

    /// Allow set type aliases.
    #[test]
    fn test_allows_set_alias() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_allows_set_alias.ds",
            r#"
type Membership = Set<string>;
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-set-over-empty-map");
    }

    /// Allow unions when map value types are not empty.
    #[test]
    fn test_allows_map_union_with_non_empty_payload() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_allows_map_union_with_non_empty_payload.ds",
            r#"
type Membership = Map<string, int32> | null;
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-set-over-empty-map");
    }

    /// Flag union aliases that still include empty map payloads.
    #[test]
    fn test_flags_map_union_with_empty_payload_branch() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_flags_map_union_with_empty_payload_branch.ds",
            r#"
type Membership = Map<string, void> | Map<string, int32>;
"#,
        );
        test.result(result).assert_lint("prefer-set-over-empty-map");
    }

    /// Allow map constructors without explicit type arguments.
    #[test]
    fn test_allows_map_constructor_without_type_arguments() {
        let test = TestProgram::for_rule_with_prelude(PreferSetOverEmptyMap);
        let result = test.lint_dir(
            "prefer_set_over_empty_map/test_allows_map_constructor_without_type_arguments.ds",
            r#"
let membership = new Map();
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-set-over-empty-map");
    }
}
