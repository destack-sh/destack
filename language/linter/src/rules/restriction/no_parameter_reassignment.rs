use std::collections::HashSet;

use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{collect_parameter_value_binding_symbols, expression_target_symbol};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow reassigning function and method parameters.
    ///
    /// Reassigning parameters makes control flow harder to reason about and can hide bugs.
    #[lint(
        id = "no-parameter-reassignment",
        code = "LR020",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoParameterReassignment,
    "Disallow reassigning function and method parameters"
}

impl LintRule for NoParameterReassignment {
    fn meta(&self) -> &'static LintMeta {
        NoParameterReassignment::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // collect all value-space parameter symbols for callable bodies
        let parameter_symbols = collect_callable_parameter_symbols(ctx);
        if parameter_symbols.is_empty() {
            return;
        }
        let statement_expression_ids = collect_statement_expression_ids(ctx);

        // inspect assignment expressions and match direct reference targets
        for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            let assigned_expression_id = match expression {
                dir::Expression::Assign { left, .. }
                | dir::Expression::AssignBinary { left, .. } => *left,
                _ => continue,
            };

            let Some(target_symbol) = expression_target_symbol(ctx.tree, assigned_expression_id)
            else {
                continue;
            };
            if !parameter_symbols.contains(&target_symbol) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.get_span(expression_id);
            let mut diagnostic = LintDiagnostic::new(
                NO_PARAMETER_REASSIGNMENT.id,
                NO_PARAMETER_REASSIGNMENT.code,
                NO_PARAMETER_REASSIGNMENT.category,
                severity,
                "parameter reassignment",
                ctx.module.file_id,
                span,
            )
            .with_label("do not reassign function parameters");

            // rewrite standalone assignments to local shadow declarations when redeclaration policy allows it
            if ctx.include_fixes
                && let Some(fix) = no_parameter_reassignment_fix(
                    ctx,
                    expression_id,
                    expression,
                    target_symbol,
                    &statement_expression_ids,
                )
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build one unsafe fix by converting one parameter assignment into a local shadow declaration.
fn no_parameter_reassignment_fix(
    ctx: &LintModuleDirContext<'_>,
    assignment_expression_id: dir::LocalNodeId<dir::Expression>,
    assignment_expression: &dir::Expression,
    target_symbol: dir::GlobalSymbolId,
    statement_expression_ids: &HashSet<u32>,
) -> Option<LintFix> {
    if target_symbol.module_id != ctx.module_id() {
        return None;
    }
    if !statement_expression_ids.contains(&assignment_expression_id.id) {
        return None;
    }

    let assignment_span = ctx.get_span(assignment_expression_id);
    if parameter_is_used_after_span(ctx, target_symbol, assignment_span.end) {
        return None;
    }

    let dir::Expression::Assign { right, .. } = assignment_expression else {
        return None;
    };

    let symbol = ctx.symbols.get_symbol(target_symbol.local_id);
    let symbol_name_id = symbol.name()?;
    let symbol_name = ctx.program.strings.get(symbol_name_id).to_string();
    if !is_simple_identifier(&symbol_name) {
        return None;
    }

    let right_text = ctx.get_span_text(ctx.get_span(*right)).trim().to_string();
    if right_text.is_empty() {
        return None;
    }
    let shadow_name = unique_shadow_name(ctx, assignment_expression_id, &symbol_name);
    let replacement_text = format!("let {shadow_name} = {right_text}");
    let edits = ctx
        .edit_builder()
        .replace(ctx.get_span(assignment_expression_id), replacement_text)
        .into_edits();
    Some(
        LintFix::r#unsafe("Introduce local shadow instead of parameter reassignment")
            .with_edits(edits),
    )
}

/// Return true when one parameter symbol is referenced after a source offset.
fn parameter_is_used_after_span(
    ctx: &LintModuleDirContext<'_>,
    parameter_symbol: dir::GlobalSymbolId,
    offset: u32,
) -> bool {
    for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
        if expression.target_symbol() != Some(parameter_symbol) {
            continue;
        }

        let span = ctx.get_span(expression_id);
        if span.start >= offset {
            return true;
        }
    }

    false
}

/// Collect expression ids that appear directly as block statements.
fn collect_statement_expression_ids(ctx: &LintModuleDirContext<'_>) -> HashSet<u32> {
    let mut expression_ids = HashSet::new();
    for (_, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
        if let dir::Expression::Statement { statement } = expression {
            expression_ids.insert(statement.id);
        }
    }
    expression_ids
}

/// Return true when one text is a simple identifier.
fn is_simple_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

/// Build one unique local shadow name for a reassigned parameter.
fn unique_shadow_name(
    ctx: &LintModuleDirContext<'_>,
    insertion_expression_id: dir::LocalNodeId<dir::Expression>,
    parameter_name: &str,
) -> String {
    let (_, scope, mark) = ctx.symbols.get_scope(insertion_expression_id, ctx.tree);

    let base_name = format!("{parameter_name}Shadow");
    let base_name_id = ctx.program.strings.intern(&base_name);
    let base_name_key = dir::StaticKey::Name(base_name_id);
    if ctx
        .symbols
        .find_active_symbol_up_to(scope, base_name_key, mark)
        .is_none()
    {
        return base_name;
    }

    let mut suffix = 2_u32;
    loop {
        let candidate = format!("{base_name}{suffix}");
        let candidate_id = ctx.program.strings.intern(&candidate);
        let candidate_key = dir::StaticKey::Name(candidate_id);
        if ctx
            .symbols
            .find_active_symbol_up_to(scope, candidate_key, mark)
            .is_none()
        {
            return candidate;
        }
        suffix += 1;
        if suffix > 1024 {
            return base_name;
        }
    }
}

/// Collect parameter symbols for function and method bodies.
fn collect_callable_parameter_symbols(
    ctx: &LintModuleDirContext<'_>,
) -> HashSet<dir::GlobalSymbolId> {
    let mut symbols = HashSet::new();

    // inspect function declarations
    for declaration_id in ctx.tree.iter_node_ids_of_type::<dir::Declaration>() {
        let declaration = ctx.tree.get(declaration_id);
        let dir::Declaration::Function {
            signature,
            body: Some(_),
            ..
        } = declaration
        else {
            continue;
        };

        collect_signature_parameter_symbols(ctx, signature, &mut symbols);
    }

    // inspect class and extension methods
    for member_id in ctx.tree.iter_node_ids_of_type::<dir::Member>() {
        let member = ctx.tree.get(member_id);
        let dir::Member::Method {
            signature,
            body: Some(_),
            ..
        } = member
        else {
            continue;
        };

        collect_signature_parameter_symbols(ctx, signature, &mut symbols);
    }

    symbols
}

/// Collect parameter symbols for one function signature.
fn collect_signature_parameter_symbols(
    ctx: &LintModuleDirContext<'_>,
    signature: &dir::FunctionSignature,
    symbols: &mut HashSet<dir::GlobalSymbolId>,
) {
    for parameter_id in &signature.dynamic_parameters {
        let mut local_symbols = HashSet::new();

        // include named symbols and nested pattern binding symbols
        collect_parameter_value_binding_symbols(
            ctx.tree,
            ctx.symbols,
            *parameter_id,
            &mut local_symbols,
        );

        for local_symbol in local_symbols {
            symbols.insert(local_symbol.into_global(ctx.module_id()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag direct reassignment of a named parameter.
    #[test]
    fn test_flags_named_parameter_reassignment() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_flags_named_parameter_reassignment.ds",
            r#"
function run(value: int32): int32 {
    value = value + 1;
    return value;
}
"#,
        );
        test.result(result).assert_lint("no-parameter-reassignment");
    }

    /// Flag compound assignment reassignment for parameters.
    #[test]
    fn test_flags_compound_parameter_reassignment() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_flags_compound_parameter_reassignment.ds",
            r#"
function run(value: int32): int32 {
    value += 1;
    return value;
}
"#,
        );
        test.result(result).assert_lint("no-parameter-reassignment");
    }

    /// Flag reassignment of bindings introduced by parameter patterns.
    #[test]
    fn test_flags_pattern_parameter_binding_reassignment() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_flags_pattern_parameter_binding_reassignment.ds",
            r#"
function run({ value }: { value: int32 }): int32 {
    value = value + 1;
    return value;
}
"#,
        );
        test.result(result).assert_lint("no-parameter-reassignment");
    }

    /// Allow assignment to a shadowed local binding.
    #[test]
    fn test_allows_shadowed_local_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_allows_shadowed_local_assignment.ds",
            r#"
function run(value: int32): int32 {
    let value = 0;
    value = 1;
    return value;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-parameter-reassignment");
    }

    /// Allow writing through parameter properties by default.
    #[test]
    fn test_allows_parameter_property_write() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_allows_parameter_property_write.ds",
            r#"
function run(values: int32[]): int32 {
    values[0] = 1;
    return values[0];
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-parameter-reassignment");
    }

    /// Flag reassignment from nested closures.
    #[test]
    fn test_flags_outer_parameter_reassignment_in_nested_function() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_flags_outer_parameter_reassignment_in_nested_function.ds",
            r#"
function run(value: int32): int32 {
    const update = (): void => {
        value = value + 1;
    };

    update();
    return value;
}
"#,
        );
        test.result(result).assert_lint("no-parameter-reassignment");
    }

    /// Rewrite simple parameter reassignment to a local shadow declaration.
    #[test]
    fn test_fix_rewrites_parameter_assignment_to_local_shadow() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_fix_rewrites_parameter_assignment_to_local_shadow.ds",
            r#"
function run(value: int32): int32 {
    value = 1;
    return 0;
}
"#,
        );
        test.result(result)
            .assert_lint("no-parameter-reassignment")
            .assert_has_fix("no-parameter-reassignment")
            .assert_unsafe_fixed(
                r#"
function run(value: int32): int32 {
    let valueShadow = 1;
    return 0;
}
"#,
            );
    }

    /// Skip fixes when reassigned parameters are read later.
    #[test]
    fn test_no_fix_when_parameter_is_used_after_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_no_fix_when_parameter_is_used_after_assignment.ds",
            r#"
function run(value: int32): int32 {
    value = 1;
    return value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-parameter-reassignment")
            .assert_has_no_fix("no-parameter-reassignment");
    }

    /// Skip fixes when parameter reassignment appears in expression position.
    #[test]
    fn test_no_fix_for_expression_position_reassignment() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_no_fix_for_expression_position_reassignment.ds",
            r#"
function run(value: int32): int32 {
    return value = 1;
}
"#,
        );
        test.result(result)
            .assert_lint("no-parameter-reassignment")
            .assert_has_no_fix("no-parameter-reassignment");
    }

    /// Pick a unique shadow suffix when preferred replacement is already bound.
    #[test]
    fn test_fix_uses_suffix_when_shadow_name_exists() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_fix_uses_suffix_when_shadow_name_exists.ds",
            r#"
function run(value: int32): int32 {
    let valueShadow = 0;
    value = 1;
    return valueShadow;
}
"#,
        );
        test.result(result)
            .assert_lint("no-parameter-reassignment")
            .assert_has_fix("no-parameter-reassignment")
            .assert_unsafe_fixed(
                r#"
function run(value: int32): int32 {
    let valueShadow = 0;
    let valueShadow2 = 1;
    return valueShadow;
}
"#,
            );
    }
}
