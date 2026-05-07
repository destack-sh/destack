use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    collect_callable_parameter_value_binding_symbols, expression_assignment_target,
    expression_is_standalone_statement, expression_target_symbol, fresh_name_in_expression_scope,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

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

        // collect all value space parameter symbols for callable bodies
        let parameter_symbols = collect_callable_parameter_value_binding_symbols(
            ctx.module_id(),
            ctx.tree,
            ctx.symbols,
        );

        // skip when there are no callable parameter symbols
        if parameter_symbols.is_empty() {
            return;
        }

        // inspect assignment expressions and match direct reference targets
        for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            // require an assignment-style target expression
            let Some(assigned_expression_id) = expression_assignment_target(ctx.tree, expression)
            else {
                continue;
            };

            // resolve the assigned symbol from the target
            let Some(target_symbol) = expression_target_symbol(ctx, assigned_expression_id) else {
                continue;
            };

            // keep only symbols that belong to callable parameters
            if !parameter_symbols.contains(&target_symbol) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, expression_id);

            // skip disabled diagnostics
            if !severity.is_enabled() {
                continue;
            }

            // build the parameter reassignment diagnostic
            let span = ctx.get_span(expression_id);
            let mut diagnostic = LintReport::new(
                NO_PARAMETER_REASSIGNMENT.id,
                NO_PARAMETER_REASSIGNMENT.code,
                NO_PARAMETER_REASSIGNMENT.category,
                severity,
                "parameter reassignment",
                span,
            )
            .label("do not reassign function parameters");

            // rewrite standalone assignments to local shadow declarations when redeclaration policy allows it
            if ctx.include_fixes
                && let Some(fix) =
                    no_parameter_reassignment_fix(ctx, expression_id, expression, target_symbol)
            {
                diagnostic = diagnostic.fix(fix);
            }

            // report the diagnostic
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
) -> Option<LintFix> {
    // keep only local module parameter symbols
    if target_symbol.module_id != ctx.module_id() {
        return None;
    }

    // keep only standalone statement assignments
    if !expression_is_standalone_statement(ctx.tree, assignment_expression_id) {
        return None;
    }

    // keep assignments where the parameter is not read later
    let assignment_span = ctx.get_span(assignment_expression_id);

    // skip fixes when the parameter is still used later
    if parameter_is_used_after_span(ctx, target_symbol, assignment_span.end) {
        return None;
    }

    // keep direct assignment expressions
    let dir::Expression::Assign { right, .. } = assignment_expression else {
        return None;
    };

    // resolve the parameter symbol name
    let symbol = ctx.symbols.get_symbol(target_symbol.local_id);
    let symbol_name_id = symbol.name()?;
    let symbol_name = ctx.strings.get(symbol_name_id).to_string();

    // resolve non-empty replacement text from the right side
    let right_text = ctx.get_span_text(ctx.get_span(*right)).trim().to_string();

    // keep only assignments with non-empty right side text
    if right_text.is_empty() {
        return None;
    }

    // build a unique local shadow assignment replacement
    let shadow_name =
        fresh_name_in_expression_scope(ctx, assignment_expression_id, &symbol_name, "Shadow")?;
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
    // scan resolved expression targets for the same parameter symbol
    for (expression_id, _) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
        // keep only expressions targeting the same parameter symbol
        if ctx.expression_target_symbol(expression_id) != Some(parameter_symbol) {
            continue;
        }

        // report usage when it occurs after the assignment offset
        let span = ctx.get_span(expression_id);

        // flag a later parameter read/write usage
        if span.start >= offset {
            return true;
        }
    }

    false
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

    /// Flag unary updates on parameters.
    #[test]
    fn test_flags_unary_parameter_update() {
        let test = TestProgram::for_rule_without_prelude(NoParameterReassignment);
        let result = test.lint_dir(
            "no_parameter_reassignment/test_flags_unary_parameter_update.ds",
            r#"
function run(value: int32): int32 {
    value++;
    return value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-parameter-reassignment")
            .assert_has_no_fix("no-parameter-reassignment");
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
