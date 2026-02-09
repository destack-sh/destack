use std::collections::HashSet;

use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{collect_parameter_value_binding_symbols, expression_target_symbol};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow reassigning function and method parameters.
    ///
    /// Reassigning parameters makes control flow harder to reason about and can hide bugs.
    #[lint(
        id = "no-parameter-reassignment",
        code = "LR022",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
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

            // report parameter reassignment
            let span = ctx.get_span(expression_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_PARAMETER_REASSIGNMENT.id,
                    NO_PARAMETER_REASSIGNMENT.code,
                    NO_PARAMETER_REASSIGNMENT.category,
                    severity,
                    "parameter reassignment",
                    ctx.module.file_id,
                    span,
                )
                .with_label("do not reassign function parameters"),
            );
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
}
