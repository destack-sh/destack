use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{fresh_name_in_symbol_scope, rename_local_symbol_fix};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow value bindings that shadow outer value bindings.
    ///
    /// Shadowing can hide outer variables and makes control flow harder to read.
    #[lint(
        id = "no-shadow",
        code = "LR026",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoShadow,
    "Disallow value bindings that shadow outer bindings"
}

impl LintRule for NoShadow {
    fn meta(&self) -> &'static LintMeta {
        NoShadow::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // scan all local symbols in deterministic order
        for symbol_id in ctx.symbols.symbol_ids() {
            let symbol = ctx.symbols.get_symbol(symbol_id);
            if !symbol_is_shadow_candidate(symbol) {
                continue;
            }

            let Some(shadowed_symbol) = find_shadowed_ancestor(ctx, symbol) else {
                continue;
            };
            let Some(symbol_name) = symbol_name_text(ctx, symbol) else {
                continue;
            };
            let Some((severity, span)) = symbol_declaration_severity_and_span(ctx, meta, symbol)
            else {
                continue;
            };
            if !severity.is_enabled() {
                continue;
            }

            // report one shadowed declaration
            let shadowed_note = symbol_name_text(ctx, shadowed_symbol)
                .map(|name| format!("shadows outer binding `{name}`"))
                .unwrap_or_else(|| "shadows an outer binding".to_string());
            let mut diagnostic = LintReport::new(
                NO_SHADOW.id,
                NO_SHADOW.code,
                NO_SHADOW.category,
                severity,
                format!("shadowed binding `{symbol_name}`"),
                span,
            )
            .label("rename this binding to avoid shadowing")
            .note(shadowed_note);

            // offer a local rename fix when symbol references are directly editable
            if ctx.compute_fixes
                && let Some(replacement_name) =
                    fresh_name_in_symbol_scope(ctx, symbol_id, &symbol_name)
                && let Some(fix) = rename_local_symbol_fix(
                    ctx,
                    symbol_id,
                    &replacement_name,
                    &format!("Rename `{symbol_name}` to `{replacement_name}`"),
                )
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when this symbol can participate in no-shadow checks.
fn symbol_is_shadow_candidate(symbol: &dir::Symbol) -> bool {
    // keep named value-space symbols only
    if symbol.space != dir::SymbolSpace::Value {
        return false;
    }
    if symbol.key.is_none() {
        return false;
    }

    // skip symbols without a lexical declaration anchor
    symbol_has_shadowable_declaration(symbol)
}

/// Return true when the symbol declaration is one lexical binding anchor.
fn symbol_has_shadowable_declaration(symbol: &dir::Symbol) -> bool {
    let Some(declaration) = symbol.declaration else {
        return false;
    };

    matches!(
        declaration.local_id.ty,
        dir::NodeType::Declaration
            | dir::NodeType::Declarator
            | dir::NodeType::Parameter
            | dir::NodeType::Pattern
            | dir::NodeType::PatternField
            | dir::NodeType::DependencyItem
    )
}

/// Return true when two symbols are comparable for no-shadow checks.
fn symbols_shadow_each_other(current: &dir::Symbol, ancestor: &dir::Symbol) -> bool {
    // keep comparable value-space bindings only
    symbol_is_shadow_candidate(current) && symbol_is_shadow_candidate(ancestor)
}

/// Find the nearest shadowed ancestor symbol for one symbol.
fn find_shadowed_ancestor<'a>(
    ctx: &'a LintModuleContext<'_>,
    symbol: &dir::Symbol,
) -> Option<&'a dir::Symbol> {
    let key = symbol.key?;
    let scope = ctx.symbols.get_scope(symbol.scope);
    let mut parent = scope.parent;

    // walk the lexical parent chain and stop at the first shadowed ancestor
    while let Some(parent_cursor) = parent {
        let parent_scope = ctx.symbols.get_scope_by_id(parent_cursor.id);
        let shadowed_symbol_id =
            ctx.symbols
                .find_symbol_up_to(parent_scope, key, parent_cursor.mark);
        if let Some(shadowed_symbol_id) = shadowed_symbol_id {
            let shadowed_symbol = ctx.symbols.get_symbol(shadowed_symbol_id);
            if symbols_shadow_each_other(symbol, shadowed_symbol) {
                return Some(shadowed_symbol);
            }
        }

        parent = parent_scope.parent;
    }

    None
}

/// Return the user-facing symbol name text.
fn symbol_name_text(ctx: &LintModuleContext<'_>, symbol: &dir::Symbol) -> Option<String> {
    let name = symbol.name()?;
    Some(ctx.strings.get(name).to_string())
}

/// Resolve severity and span for the symbol declaration.
fn symbol_declaration_severity_and_span(
    ctx: &LintModuleContext<'_>,
    meta: &LintMeta,
    symbol: &dir::Symbol,
) -> Option<(LintSeverity, destack_source::Span)> {
    let declaration = symbol.declaration?;
    if declaration.module_id != ctx.module_id() {
        return None;
    }

    let declaration_id = declaration.local_id;
    let (severity, span) = match declaration_id.ty {
        dir::NodeType::Declaration => {
            let declaration_id = declaration_id.into_typed::<dir::Declaration>();
            let severity = ctx.get_effective_severity(meta, declaration_id);
            let span = ctx.get_span(declaration_id);
            (severity, span)
        }
        dir::NodeType::Declarator => {
            let declaration_id = declaration_id.into_typed::<dir::Declarator>();
            let severity = ctx.get_effective_severity(meta, declaration_id);
            let span = ctx.get_span(declaration_id);
            (severity, span)
        }
        dir::NodeType::Parameter => {
            let declaration_id = declaration_id.into_typed::<dir::Parameter>();
            let severity = ctx.get_effective_severity(meta, declaration_id);
            let span = ctx.get_span(declaration_id);
            (severity, span)
        }
        dir::NodeType::Pattern => {
            let declaration_id = declaration_id.into_typed::<dir::Pattern>();
            let severity = ctx.get_effective_severity(meta, declaration_id);
            let span = ctx.get_span(declaration_id);
            (severity, span)
        }
        dir::NodeType::PatternField => {
            let declaration_id = declaration_id.into_typed::<dir::PatternField>();
            let severity = ctx.get_effective_severity(meta, declaration_id);
            let span = ctx.get_span(declaration_id);
            (severity, span)
        }
        dir::NodeType::DependencyItem => {
            let declaration_id = declaration_id.into_typed::<dir::DependencyItem>();
            let severity = ctx.get_effective_severity(meta, declaration_id);
            let span = ctx.get_span(declaration_id);
            (severity, span)
        }
        _ => return None,
    };

    Some((severity, span))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Report inner block bindings that shadow outer bindings.
    #[test]
    fn test_flags_block_shadowing() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_dir(
            "no_shadow/test_flags_block_shadowing.ds",
            r#"
function run(): int32 {
    let value = 1;

    {
        let value = 2;
        return value;
    }
}
"#,
        );

        test.result(diagnostics).assert_lint("no-shadow");
    }

    /// Report parameter bindings that shadow outer values.
    #[test]
    fn test_flags_parameter_shadowing() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_dir(
            "no_shadow/test_flags_parameter_shadowing.ds",
            r#"
const token = 1;

function read(token: int32): int32 {
    return token;
}
"#,
        );

        test.result(diagnostics).assert_lint("no-shadow");
    }

    /// Skip shadow diagnostics for local redeclarations already rejected by compiler rules.
    #[test]
    fn test_skips_ts_parameter_redeclaration_conflict() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_dir(
            "no_shadow/test_skips_ts_parameter_redeclaration_conflict.ts",
            r#"
function read(token: number): number {
    let token = 1;
    return token;
}
"#,
        );

        test.result(diagnostics).assert_no_lint("no-shadow");
    }

    /// Keep reporting legal shadowing in destack when local redeclaration checks allow it.
    #[test]
    fn test_keeps_destack_parameter_shadowing_when_policy_allows_redeclare() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_dir(
            "no_shadow/test_keeps_destack_parameter_shadowing_when_policy_allows_redeclare.ds",
            r#"
function read(token: int32): int32 {
    let token = 1;
    return token;
}
"#,
        );

        test.result(diagnostics).assert_lint("no-shadow");
    }

    /// Report local bindings that shadow imported bindings.
    #[test]
    fn test_flags_shadowing_of_import_binding() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_shadow/source.ds" => r#"
export const value = 1;
"#,
                "no_shadow/consumer.ds" => r#"
import { value } from "./source.ds";

function run(): int32 {
    let value = 2;
    return value;
}
"#,
            },
            "no_shadow/consumer.ds",
        );

        test.result(diagnostics).assert_lint("no-shadow");
    }

    /// Allow sibling scope bindings that do not shadow ancestors.
    #[test]
    fn test_allows_sibling_scope_bindings() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_dir(
            "no_shadow/test_allows_sibling_scope_bindings.ds",
            r#"
{
    let value = 1;
}

{
    let value = 2;
}
"#,
        );

        test.result(diagnostics).assert_no_lint("no-shadow");
    }

    /// Allow class members that reuse outer names.
    #[test]
    fn test_allows_class_member_name_reuse() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_dir(
            "no_shadow/test_allows_class_member_name_reuse.ds",
            r#"
const value = 1;

class Box {
    value(): int32 {
        return 2;
    }
}
"#,
        );

        test.result(diagnostics).assert_no_lint("no-shadow");
    }

    /// Provide a rename fix for simple shadowed local bindings.
    #[test]
    fn test_fixes_shadowed_binding_with_local_rename() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_dir(
            "no_shadow/test_fixes_shadowed_binding_with_local_rename.ds",
            r#"
function run(): int32 {
    let value = 1;

    {
        let value = 2;
        return value + value;
    }
}
"#,
        );

        test.result(diagnostics)
            .assert_lint("no-shadow")
            .assert_has_fix("no-shadow")
            .assert_unsafe_fixed(
                r#"
function run(): int32 {
    let value = 1;

    {
        let value_shadow = 2;
        return value_shadow + value_shadow;
    }
}
"#,
            );
    }

    /// Skip rename fixes when the declaration span is ambiguous.
    #[test]
    fn test_skips_fix_for_ambiguous_pattern_declaration() {
        let test = TestProgram::for_rule_without_prelude(NoShadow);
        let diagnostics = test.lint_dir(
            "no_shadow/test_skips_fix_for_ambiguous_pattern_declaration.ds",
            r#"
const value = 1;

function run(input: { value: int32 }): int32 {
    let { value: value } = input;
    return value;
}
"#,
        );

        test.result(diagnostics)
            .assert_lint("no-shadow")
            .assert_has_no_fix("no-shadow");
    }
}
