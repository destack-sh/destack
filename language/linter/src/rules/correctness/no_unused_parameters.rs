use std::collections::HashSet;

use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    collect_module_read_symbol_usage, collect_parameter_value_binding_symbols,
    parameter_binding_span,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow parameters that are never used.
    ///
    /// Unused parameters usually indicate dead API surface or an implementation mismatch.
    #[lint(
        id = "no-unused-parameters",
        code = "LC036",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoUnusedParameters,
    "Disallow unused function and method parameters"
}

impl LintRule for NoUnusedParameters {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnusedParameters::meta()
    }

    /// Check module DIR nodes for unused function and method parameters.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let read_symbols =
            collect_module_read_symbol_usage(ctx.module_id(), ctx.dir.tree(), ctx.resolutions);

        // inspect all parameters
        for parameter_id in ctx.dir.iter_node_ids_of_type::<dir::Parameter>() {
            let parameter = ctx.dir.get(parameter_id);
            let Some(symbol_id) = ctx.local_symbol_for_node(parameter_id) else {
                continue;
            };

            // keep value space bindings only
            if !symbol_is_value_binding(ctx, symbol_id) {
                continue;
            }

            // keep callable body parameters only
            if !parameter_requires_usage(ctx.dir.tree(), parameter_id) {
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
                    if read_symbols.contains(&symbol_id.into_global(ctx.module_id())) {
                        continue;
                    }

                    // resolve effective lint severity
                    let severity = ctx.get_effective_severity(meta, parameter_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    // report one unused named parameter
                    let span = ctx.get_span(parameter_id);
                    let mut diagnostic = LintReport::new(
                        NO_UNUSED_PARAMETERS.id,
                        NO_UNUSED_PARAMETERS.code,
                        NO_UNUSED_PARAMETERS.category,
                        severity,
                        "unused parameter",
                        span,
                    )
                    .label("this parameter is never used");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = unused_named_parameter_fix(ctx, parameter_id)
                    {
                        diagnostic = diagnostic.fix(fix);
                    }

                    ctx.report(diagnostic);
                }
                dir::Parameter::Pattern { .. } | dir::Parameter::VariadicPattern { .. } => {
                    let mut bindings = HashSet::new();

                    // collect nested pattern bindings from this parameter
                    collect_parameter_value_binding_symbols(
                        ctx.dir.tree(),
                        &ctx.symbols,
                        parameter_id,
                        &mut bindings,
                    );

                    // keep only pattern local bindings
                    bindings.remove(&symbol_id);

                    // report each unused binding in the parameter pattern
                    let mut binding_symbols = bindings.into_iter().collect::<Vec<_>>();

                    // keep diagnostics deterministic for stable snapshots
                    binding_symbols.sort_unstable_by_key(|binding_symbol| {
                        let symbol = ctx.symbols.get_symbol(*binding_symbol);
                        symbol
                            .declaration
                            .map_or(u32::MAX, |node_id| node_id.local_id.id)
                    });

                    // inspect candidate nodes
                    for binding_symbol in binding_symbols {
                        // skip used pattern bindings
                        if read_symbols.contains(&binding_symbol.into_global(ctx.module_id())) {
                            continue;
                        }

                        // resolve symbol
                        let symbol = ctx.symbols.get_symbol(binding_symbol);
                        let Some(name) = symbol.name() else {
                            continue;
                        };

                        // skip configured ignored binding names
                        if parameter_name_is_ignored(ctx, name) {
                            continue;
                        }

                        // require optional structure
                        let Some(node_id) = symbol.declaration else {
                            continue;
                        };

                        // keep declarations in this module only
                        if node_id.module_id != ctx.module_id() {
                            continue;
                        }

                        // resolve local node id
                        let local_node_id = node_id.local_id;
                        if local_node_id.ty != dir::NodeType::Pattern
                            && local_node_id.ty != dir::NodeType::PatternField
                        {
                            continue;
                        }

                        // resolve effective lint severity
                        let severity = ctx.get_effective_severity(meta, parameter_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        // report one unused pattern binding
                        let span = parameter_binding_span(ctx, parameter_id, local_node_id);
                        ctx.report(
                            LintReport::new(
                                NO_UNUSED_PARAMETERS.id,
                                NO_UNUSED_PARAMETERS.code,
                                NO_UNUSED_PARAMETERS.category,
                                severity,
                                "unused parameter binding",
                                span,
                            )
                            .label("this parameter binding is never used"),
                        );
                    }
                }
                dir::Parameter::Error => continue,
            }
        }
    }
}

/// Build an unsafe fix by prefixing an unused named parameter with `_`.
fn unused_named_parameter_fix(
    ctx: &LintModuleContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<LintFix> {
    let parameter_span = ctx.get_span(parameter_id);
    let parameter_text = ctx.get_span_text(parameter_span);
    let mut insert_offset = 0usize;

    // skip variadic markers when present
    if parameter_text.starts_with("...") {
        insert_offset += 3;
    }

    // skip leading trivia before the binding name
    let bytes = parameter_text.as_bytes();
    while insert_offset < bytes.len() && bytes[insert_offset].is_ascii_whitespace() {
        insert_offset += 1;
    }

    // stop when there is no parameter name
    if insert_offset >= bytes.len() {
        return None;
    }

    // inspect the first character of the name token
    let first_char = bytes[insert_offset] as char;

    // skip already ignored bindings
    if first_char == '_' {
        return None;
    }

    // keep plain identifier starts only
    if !first_char.is_ascii_alphabetic() && first_char != '$' {
        return None;
    }

    // insert one leading underscore at the parameter name position
    let insert_position = parameter_span.start + insert_offset as u32;
    let edits = ctx.edit_builder().insert(insert_position, "_").into_edits();
    Some(LintFix::r#unsafe("Prefix unused parameter with `_`").with_edits(edits))
}

/// Return true when this symbol is a value space binding.
fn symbol_is_value_binding(ctx: &LintModuleContext<'_>, symbol_id: dir::LocalSymbolId) -> bool {
    let symbol = ctx.symbols.get_symbol(symbol_id);
    symbol.kind.is_visible_in(dir::SymbolSpace::Value)
}

/// Return true when the parameter belongs to a declaration or member body.
fn parameter_requires_usage(
    tree: &dir::Tree,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> bool {
    let mut parent = tree.get_parent(parameter_id.id);

    // walk ancestors until we find the owning callable node
    while let Some(parent_id) = parent {
        // function declarations require usage when they have a body
        match parent_id.ty {
            dir::NodeType::Declaration => {
                let declaration = tree.get(parent_id.into_typed::<dir::Declaration>());
                if let dir::Declaration::Function(declaration) = declaration {
                    return declaration.body.is_some();
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
fn is_this_parameter_name(ctx: &LintModuleContext<'_>, name: dir::StringId) -> bool {
    ctx.strings.get(name) == "this"
}

/// Return true when the parameter name should be ignored by configuration.
fn parameter_name_is_ignored(ctx: &LintModuleContext<'_>, name: dir::StringId) -> bool {
    let text = ctx.strings.get(name);
    if text == "_" {
        return true;
    }

    ctx.options()
        .correctness
        .ignored_unused_parameter_prefixes
        .iter()
        .any(|prefix| !prefix.is_empty() && text.starts_with(prefix))
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

    /// Allow underscore placeholder parameters by default.
    #[test]
    fn test_allows_underscore_placeholder_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_allows_underscore_placeholder_parameter.ds",
            r#"
function run(_: int32): int32 {
    return 1;
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
                options.correctness.ignored_unused_parameter_prefixes = vec!["ignored".to_string()];
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

    /// Unsafely fix one unused named parameter by prefixing `_`.
    #[test]
    fn test_fix_prefixes_unused_named_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_fix_prefixes_unused_named_parameter.ds",
            r#"
function run(value: int32): int32 {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-parameters")
            .assert_unsafe_fixed(
                r#"
function run(_value: int32): int32 {
    return 1;
}
"#,
            );
    }

    /// Keep destructured parameter diagnostics without auto-fix.
    #[test]
    fn test_no_fix_for_destructured_parameter_binding() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_no_fix_for_destructured_parameter_binding.ds",
            r#"
function run({ value }: { value: int32 }): int32 {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("no-unused-parameters")
            .assert_has_no_fix("no-unused-parameters");
    }

    /// Flag write-only parameters that are never read.
    #[test]
    fn test_flags_write_only_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_flags_write_only_parameter.ds",
            r#"
function run(value: int32): int32 {
    value = 1;
    return 2;
}
"#,
        );
        test.result(result).assert_lint("no-unused-parameters");
    }

    /// Flag standalone compound assignment as write-only parameter usage.
    #[test]
    fn test_flags_compound_write_only_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_flags_compound_write_only_parameter.ds",
            r#"
function run(value: int32): int32 {
    value += 1;
    return 2;
}
"#,
        );
        test.result(result).assert_lint("no-unused-parameters");
    }

    /// Flag standalone increment as write-only parameter usage.
    #[test]
    fn test_flags_increment_write_only_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnusedParameters);
        let result = test.lint_dir(
            "no_unused_parameters/test_flags_increment_write_only_parameter.ds",
            r#"
function run(value: int32): int32 {
    value++;
    return 2;
}
"#,
        );
        test.result(result).assert_lint("no-unused-parameters");
    }
}
