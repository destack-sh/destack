use std::collections::HashSet;

use destack_dir::{self as dir, WellKnownSymbol};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    contains_map_with_empty_value_type, expression_type_map, is_void_or_never_type,
    symbol_matches_any_or_canonical, well_known_symbol_candidates,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

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
        let mut reported_source_ids = HashSet::new();

        // type aliases: query alias target types directly from the type table
        for (declaration_id, declaration) in ctx.tree.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Type(declaration) = declaration else {
                continue;
            };

            let type_symbol = declaration.symbol.into_global(ctx.module_id());
            let Some(alias_target_type_id) = ctx.types.get_alias_target_type_id(type_symbol) else {
                continue;
            };
            if !contains_map_with_empty_value_type(ctx.types, alias_target_type_id, &map_symbols) {
                continue;
            }
            let source_id = ctx.tree.get_source(declaration.value.id);
            if !reported_source_ids.insert(source_id) {
                continue;
            }

            report_prefer_set_over_empty_map(
                ctx,
                meta,
                declaration_id,
                ctx.get_span(declaration.value),
            );
        }

        for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            let should_report = match expression {
                // type annotations are lowered as type values
                dir::Expression::Type {
                    value: _,
                    resolved_type,
                } => contains_map_with_empty_value_type(ctx.types, *resolved_type, &map_symbols),
                // type references can carry map generic arguments directly
                dir::Expression::LocalReference {
                    target_symbol,
                    generic_arguments,
                    ..
                }
                | dir::Expression::ModuleReference {
                    target_symbol,
                    generic_arguments,
                    ..
                }
                | dir::Expression::GlobalReference {
                    target_symbol,
                    generic_arguments,
                    ..
                } => reference_has_empty_value_argument(
                    ctx,
                    *target_symbol,
                    generic_arguments.as_slice(),
                    &map_symbols,
                ),
                // constructor calls keep generic arguments on the new expression
                dir::Expression::New {
                    left,
                    generic_arguments,
                    ..
                } => new_map_has_empty_value_argument(ctx, *left, generic_arguments, &map_symbols),
                _ => false,
            };
            if !should_report {
                continue;
            }
            let source_id = ctx.tree.get_source(expression_id.id);
            if !reported_source_ids.insert(source_id) {
                continue;
            }

            report_prefer_set_over_empty_map(ctx, meta, expression_id, ctx.get_span(expression_id));
        }
    }
}

/// Resolve all concrete `Map` symbols from type and value spaces.
fn resolve_map_symbols(ctx: &LintModuleDirContext<'_>) -> Vec<dir::GlobalSymbolId> {
    let Some(well_known_symbols) = ctx.get_well_known_symbols() else {
        return Vec::new();
    };

    well_known_symbol_candidates(&well_known_symbols, WellKnownSymbol::Map)
}

/// Report one prefer-set-over-empty-map diagnostic.
fn report_prefer_set_over_empty_map<T: dir::Node>(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    node_id: dir::LocalNodeId<T>,
    span: destack_source::Span,
) {
    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    ctx.report(
        LintReport::new(
            PREFER_SET_OVER_EMPTY_MAP.id,
            PREFER_SET_OVER_EMPTY_MAP.code,
            PREFER_SET_OVER_EMPTY_MAP.category,
            severity,
            "prefer Set over Map with empty value type",
            span,
        )
        .label("use `Set<K>` instead of `Map<K, void | never>`"),
    );
}

/// Return true when one `new` expression constructs `Map<_, void|never>`.
fn new_map_has_empty_value_argument(
    ctx: &LintModuleDirContext<'_>,
    callee_expression_id: dir::LocalNodeId<dir::Expression>,
    generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    map_symbols: &[dir::GlobalSymbolId],
) -> bool {
    let callee = ctx.tree.get(callee_expression_id);
    let Some(target_symbol) = callee.target_symbol() else {
        return false;
    };
    if !symbol_is_map(ctx, target_symbol, map_symbols) {
        return false;
    }

    if generic_arguments.len() != 2 {
        return false;
    }

    let Some(value_expression_id) =
        generic_argument_value_expression(ctx.tree, generic_arguments[1])
    else {
        return false;
    };
    expression_is_void_or_never_type(ctx, value_expression_id)
}

/// Return true when one map reference has an empty value type argument.
fn reference_has_empty_value_argument(
    ctx: &LintModuleDirContext<'_>,
    target_symbol: dir::GlobalSymbolId,
    generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    map_symbols: &[dir::GlobalSymbolId],
) -> bool {
    if !symbol_is_map(ctx, target_symbol, map_symbols) {
        return false;
    }

    if generic_arguments.len() != 2 {
        return false;
    }

    let Some(value_expression_id) =
        generic_argument_value_expression(ctx.tree, generic_arguments[1])
    else {
        return false;
    };
    expression_is_void_or_never_type(ctx, value_expression_id)
}

/// Return the value expression for one generic argument.
fn generic_argument_value_expression(
    tree: &dir::Tree,
    generic_argument_id: dir::LocalNodeId<dir::GenericArgument>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let generic_argument = tree.get(generic_argument_id);
    match generic_argument {
        dir::GenericArgument::Type { .. } => None,
        dir::GenericArgument::Value { value } => Some(*value),
        dir::GenericArgument::Error => None,
    }
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
        &ctx.repository,
        ctx.revision,
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
    symbol_matches_any_or_canonical(
        &ctx.repository,
        ctx.revision,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        symbol_id,
        map_symbols,
    )
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
