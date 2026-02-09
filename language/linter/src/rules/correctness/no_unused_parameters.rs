use std::collections::HashSet;

use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::collect_module_symbol_usage;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow parameters that are never used.
    ///
    /// Unused parameters usually indicate dead API surface or an implementation mismatch.
    #[lint(
        id = "no-unused-parameters",
        code = "LC102",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoUnusedParameters,
    "Disallow unused function and method parameters"
}

impl LintRule for NoUnusedParameters {
    fn meta(&self) -> &'static LintMeta {
        NoUnusedParameters::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let usage = collect_module_symbol_usage(ctx.module_id(), ctx.tree, ctx.types);
        let used_symbols = usage.used_local_symbols(ctx.module_id());

        // inspect all parameters
        for parameter_id in ctx.tree.iter_node_ids_of_type::<dir::Parameter>() {
            let parameter = ctx.tree.get(parameter_id);
            let symbol_id = parameter.symbol();

            // keep value space bindings only
            if !symbol_is_value_binding(ctx, symbol_id) {
                continue;
            }

            // keep callable body parameters only
            if !parameter_requires_usage(ctx.tree, parameter_id) {
                continue;
            }

            // inspect named and pattern parameters separately
            match parameter {
                dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
                    // ignore explicit this parameter names
                    if is_this_parameter_name(ctx, *name) {
                        continue;
                    }

                    // ignore configured placeholder names
                    if parameter_name_is_ignored(ctx, *name) {
                        continue;
                    }

                    // skip used parameter symbols
                    if used_symbols.contains(&symbol_id) {
                        continue;
                    }

                    let severity = ctx.get_effective_severity(meta, parameter_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    // report one unused named parameter
                    let span = ctx.get_span(parameter_id);
                    ctx.report(
                        LintDiagnostic::new(
                            NO_UNUSED_PARAMETERS.id,
                            NO_UNUSED_PARAMETERS.code,
                            NO_UNUSED_PARAMETERS.category,
                            severity,
                            "unused parameter",
                            ctx.module.file_id,
                            span,
                        )
                        .with_label("this parameter is never used"),
                    );
                }
                dir::Parameter::Pattern { pattern, .. }
                | dir::Parameter::VariadicPattern { pattern, .. } => {
                    let mut bindings = HashSet::new();

                    // collect nested pattern bindings from this parameter
                    collect_value_binding_symbols_for_pattern(
                        ctx.tree,
                        *pattern,
                        ctx.symbols,
                        &mut bindings,
                    );

                    // report each unused binding in the parameter pattern
                    for binding_symbol in bindings {
                        // skip used pattern bindings
                        if used_symbols.contains(&binding_symbol) {
                            continue;
                        }

                        let symbol = ctx.symbols.get_symbol(binding_symbol);
                        let Some(name) = symbol.name() else {
                            continue;
                        };

                        // skip configured ignored binding names
                        if parameter_name_is_ignored(ctx, name) {
                            continue;
                        }

                        let Some(node_id) = symbol.primary_declaration else {
                            continue;
                        };

                        // keep declarations in this module only
                        if node_id.module_id != ctx.module_id() {
                            continue;
                        }

                        let local_node_id = node_id.local_id;
                        if local_node_id.ty != dir::NodeType::Pattern
                            && local_node_id.ty != dir::NodeType::PatternField
                        {
                            continue;
                        }

                        let severity = ctx.get_effective_severity(meta, parameter_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        // report one unused pattern binding
                        let span = parameter_binding_span(ctx, parameter_id, local_node_id);
                        ctx.report(
                            LintDiagnostic::new(
                                NO_UNUSED_PARAMETERS.id,
                                NO_UNUSED_PARAMETERS.code,
                                NO_UNUSED_PARAMETERS.category,
                                severity,
                                "unused parameter binding",
                                ctx.module.file_id,
                                span,
                            )
                            .with_label("this parameter binding is never used"),
                        );
                    }
                }
            }
        }
    }
}

/// Resolve a precise report span for one unused binding inside a parameter pattern.
fn parameter_binding_span(
    ctx: &LintModuleDirContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    local_node_id: dir::LocalNodeIdAny,
) -> destack_source::Span {
    // prefer pattern node spans when available
    if local_node_id.ty == dir::NodeType::Pattern {
        return ctx.get_span(local_node_id.into_typed::<dir::Pattern>());
    }

    // then prefer pattern field spans
    if local_node_id.ty == dir::NodeType::PatternField {
        return ctx.get_span(local_node_id.into_typed::<dir::PatternField>());
    }

    // otherwise report at parameter span
    ctx.get_span(parameter_id)
}

/// Return true when this symbol is a value-space binding.
fn symbol_is_value_binding(ctx: &LintModuleDirContext<'_>, symbol_id: dir::LocalSymbolId) -> bool {
    let symbol = ctx.symbols.get_symbol(symbol_id);
    matches!(
        symbol.space,
        dir::SymbolSpace::Value | dir::SymbolSpace::TypeValue
    )
}

/// Return true when the parameter belongs to a declaration or member body.
fn parameter_requires_usage(
    tree: &dir::NodeTree,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> bool {
    let mut parent = tree.get_parent(parameter_id.id);

    // walk ancestors until we find the owning callable node
    while let Some(parent_id) = parent {
        // function declarations require usage when they have a body
        match parent_id.ty {
            dir::NodeType::Declaration => {
                let declaration = tree.get(parent_id.into_typed::<dir::Declaration>());
                if let dir::Declaration::Function { body, .. } = declaration {
                    return body.is_some();
                }
            }
            dir::NodeType::Member => {
                let member = tree.get(parent_id.into_typed::<dir::Member>());
                if let dir::Member::Method { body, .. } = member {
                    return body.is_some();
                }
            }
            _ => {}
        }

        // continue walking toward the root
        parent = tree.get_parent(parent_id.id);
    }

    false
}

/// Return true when the parameter name is `this`.
fn is_this_parameter_name(ctx: &LintModuleDirContext<'_>, name: dir::StringId) -> bool {
    ctx.program.strings.get(name) == "this"
}

/// Return true when the parameter name should be ignored by configuration.
fn parameter_name_is_ignored(ctx: &LintModuleDirContext<'_>, name: dir::StringId) -> bool {
    let text = ctx.program.strings.get(name);
    ctx.options
        .ignored_unused_parameter_prefixes
        .iter()
        .any(|prefix| !prefix.is_empty() && text.starts_with(prefix))
}

/// Collect value-space binding symbols for a pattern subtree.
fn collect_value_binding_symbols_for_pattern(
    tree: &dir::NodeTree,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    symbols: &dir::SymbolTable,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    let pattern = tree.get(pattern_id);

    // record the current pattern binding
    if let Some(symbol_id) = pattern.symbol() {
        let symbol = symbols.get_symbol(symbol_id);
        if matches!(
            symbol.space,
            dir::SymbolSpace::Value | dir::SymbolSpace::TypeValue
        ) {
            bindings.insert(symbol_id);
        }
    }

    // recurse into nested patterns
    match pattern {
        dir::Pattern::Wildcard | dir::Pattern::Expression { .. } => {}
        dir::Pattern::Must(inner)
        | dir::Pattern::ReferenceOf { right: inner, .. }
        | dir::Pattern::ValueOf { right: inner, .. } => {
            collect_value_binding_symbols_for_pattern(tree, *inner, symbols, bindings);
        }
        dir::Pattern::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_value_binding_symbols_for_pattern(tree, *start, symbols, bindings);
            }
            if let Some(end) = end {
                collect_value_binding_symbols_for_pattern(tree, *end, symbols, bindings);
            }
        }
        dir::Pattern::Tuple { fields }
        | dir::Pattern::TaggedTuple { fields, .. }
        | dir::Pattern::Array { fields }
        | dir::Pattern::Object { fields }
        | dir::Pattern::TaggedObject { fields, .. } => {
            for field_id in fields {
                collect_value_binding_symbols_for_field(tree, *field_id, symbols, bindings);
            }
        }
        dir::Pattern::Union { patterns } => {
            for pattern_id in patterns {
                collect_value_binding_symbols_for_pattern(tree, *pattern_id, symbols, bindings);
            }
        }
        dir::Pattern::Binding { pattern, .. } => {
            if let Some(pattern) = pattern {
                collect_value_binding_symbols_for_pattern(tree, *pattern, symbols, bindings);
            }
        }
    }
}

/// Collect value-space binding symbols for a pattern field.
fn collect_value_binding_symbols_for_field(
    tree: &dir::NodeTree,
    field_id: dir::LocalNodeId<dir::PatternField>,
    symbols: &dir::SymbolTable,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    let field = tree.get(field_id);

    // record direct field bindings
    if let Some(symbol_id) = field.symbol() {
        let symbol = symbols.get_symbol(symbol_id);
        if matches!(
            symbol.space,
            dir::SymbolSpace::Value | dir::SymbolSpace::TypeValue
        ) {
            bindings.insert(symbol_id);
        }
    }

    // recurse into nested field patterns
    match field {
        dir::PatternField::Named {
            pattern: Some(pattern),
            ..
        }
        | dir::PatternField::Computed {
            pattern: Some(pattern),
            ..
        }
        | dir::PatternField::Positional { pattern } => {
            collect_value_binding_symbols_for_pattern(tree, *pattern, symbols, bindings);
        }
        dir::PatternField::Spread {
            pattern: Some(pattern),
            ..
        } => {
            collect_value_binding_symbols_for_pattern(tree, *pattern, symbols, bindings);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag an unused named function parameter.
    #[test]
    fn test_flags_unused_named_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_flags_unused_named_parameter.ds",
            r#"
function run(value: int32): int32 {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("no-unused-parameters");
    }

    /// Allow a used named function parameter.
    #[test]
    fn test_allows_used_named_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_allows_used_named_parameter.ds",
            r#"
function run(value: int32): int32 {
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-parameters");
    }

    /// Ignore underscore-prefixed parameters by default.
    #[test]
    fn test_ignores_underscored_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_ignores_underscored_parameter.ds",
            r#"
function run(_value: int32): int32 {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-parameters");
    }

    /// Respect custom ignored prefixes.
    #[test]
    fn test_respects_custom_ignored_prefixes() {
        let test =
            TestProgram::for_rule_without_prelude(NoUnusedParameters).with_options(|options| {
                options.ignored_unused_parameter_prefixes = vec!["ignored".to_string()];
            });
        let result = test.lint_dir(
            "no_unused_parameters/test_respects_custom_ignored_prefixes.ds",
            r#"
function run(ignoredValue: int32): int32 {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-parameters");
    }

    /// Flag unused parameter bindings introduced by destructuring.
    #[test]
    fn test_flags_unused_pattern_binding_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_flags_unused_pattern_binding_parameter.ds",
            r#"
function run({ value }: { value: int32 }): int32 {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("no-unused-parameters");
    }

    /// Allow used parameter bindings introduced by destructuring.
    #[test]
    fn test_allows_used_pattern_binding_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_allows_used_pattern_binding_parameter.ds",
            r#"
function run({ value }: { value: int32 }): int32 {
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-parameters");
    }

    /// Ignore explicit `this` parameters.
    #[test]
    fn test_ignores_this_parameter() {
        let test = TestProgram::for_rule_with_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_ignores_this_parameter.ts",
            r#"
function run(this: { value: number }, value: number): number {
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-parameters");
    }

    /// Ignore parameters in declaration-only signatures.
    #[test]
    fn test_ignores_declaration_only_signature_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_ignores_declaration_only_signature_parameter.ds",
            r#"
interface Service {
    run(value: int32): int32;
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-parameters");
    }

    /// Report each unused named binding in a destructured parameter.
    #[test]
    fn test_reports_each_unused_pattern_binding() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_reports_each_unused_pattern_binding.ds",
            r#"
function run({ first, second }: { first: int32; second: int32 }): int32 {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-parameters")
            .assert_lint_count("no-unused-parameters", 2);
    }

    /// Respect ignored prefixes for destructured parameter bindings.
    #[test]
    fn test_respects_ignored_prefixes_for_pattern_bindings() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_respects_ignored_prefixes_for_pattern_bindings.ds",
            r#"
function run({ _first, second }: { _first: int32; second: int32 }): int32 {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-parameters")
            .assert_lint_count("no-unused-parameters", 1);
    }
}
