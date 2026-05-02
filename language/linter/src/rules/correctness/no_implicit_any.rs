use std::collections::HashSet;

use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    collect_pattern_value_binding_symbols, is_any_type, is_error_type, is_infer_var_type,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow implicit `any` in parameters and declarators.
    ///
    /// Missing type annotations on declarations without enough inference
    /// context can silently produce `any` and weaken type safety.
    #[lint(
        id = "no-implicit-any",
        code = "LC018",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoImplicitAny,
    "Disallow implicit `any` types"
}

impl LintRule for NoImplicitAny {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoImplicitAny::meta()
    }

    /// Check module DIR nodes for implicit any declarations and parameters.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // declarators
        for declarator_id in ctx.tree.iter_node_ids_of_type::<dir::Declarator>() {
            if !should_report_declarator(ctx, declarator_id) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, declarator_id);
            if !severity.is_enabled() {
                continue;
            }

            // resolve diagnostic span
            let span = ctx.get_span(declarator_id);
            let mut diagnostic = LintReport::new(
                NO_IMPLICIT_ANY.id,
                NO_IMPLICIT_ANY.code,
                NO_IMPLICIT_ANY.category,
                severity,
                "implicit any in variable declaration",
                span,
            )
            .label("add an explicit type annotation or initializer");

            // compute fixes only when requested by the runner
            if ctx.include_fixes
                && let Some(fix) = no_implicit_any_declarator_fix(ctx, declarator_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }

        // parameters
        for parameter_id in ctx.tree.iter_node_ids_of_type::<dir::Parameter>() {
            if !should_report_parameter(ctx, parameter_id) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, parameter_id);
            if !severity.is_enabled() {
                continue;
            }

            // resolve diagnostic span
            let span = ctx.get_span(parameter_id);
            let mut diagnostic = LintReport::new(
                NO_IMPLICIT_ANY.id,
                NO_IMPLICIT_ANY.code,
                NO_IMPLICIT_ANY.category,
                severity,
                "implicit any in parameter",
                span,
            )
            .label("add an explicit type annotation");

            // compute fixes only when requested by the runner
            if ctx.include_fixes
                && let Some(fix) = no_implicit_any_parameter_fix(ctx, parameter_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build an unsafe fix by annotating a declarator with `unknown`.
fn no_implicit_any_declarator_fix(
    ctx: &LintModuleDirContext<'_>,
    declarator_id: dir::LocalNodeId<dir::Declarator>,
) -> Option<LintFix> {
    let declarator = ctx.tree.get(declarator_id);
    let pattern_span = ctx.get_span(declarator.pattern);
    let edits = ctx
        .edit_builder()
        .insert(pattern_span.end, ": unknown")
        .into_edits();
    Some(LintFix::r#unsafe("Add `unknown` type annotation").with_edits(edits))
}

/// Build an unsafe fix by annotating a parameter with `unknown`.
fn no_implicit_any_parameter_fix(
    ctx: &LintModuleDirContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<LintFix> {
    let parameter = ctx.tree.get(parameter_id);
    if matches!(
        parameter,
        dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. }
    ) {
        return None;
    }

    let parameter_span = ctx.get_span(parameter_id);
    let edits = ctx
        .edit_builder()
        .insert(parameter_span.end, ": unknown")
        .into_edits();
    Some(LintFix::r#unsafe("Add `unknown` type annotation").with_edits(edits))
}

/// Return true when a declarator is an implicit any candidate.
fn should_report_declarator(
    ctx: &LintModuleDirContext<'_>,
    declarator_id: dir::LocalNodeId<dir::Declarator>,
) -> bool {
    let declarator = ctx.tree.get(declarator_id);

    // explicit type annotations are always intentional
    if declarator.ty.is_some() {
        return false;
    }

    // collect declarator bound symbols
    let mut bound_symbols = HashSet::new();
    collect_pattern_value_binding_symbols(
        ctx.tree,
        ctx.symbols,
        declarator.pattern,
        &mut bound_symbols,
    );
    if bound_symbols.is_empty() {
        return false;
    }

    // no initializer means no inference context: always implicit any
    if declarator.value.is_none() {
        return true;
    }

    // with initializers, report if inferred declarator symbols are still any/infer
    let module_id = ctx.module_id();
    for local_symbol in bound_symbols {
        let symbol_id = local_symbol.into_global(module_id);
        let Some(type_id) = ctx.types.get_value_type_id(symbol_id) else {
            return true;
        };

        // enforce this lint guard
        if is_infer_var_type(ctx.types, type_id)
            || is_any_type(ctx.types, type_id)
            || is_error_type(ctx.types, type_id)
        {
            return true;
        }
    }

    false
}

/// Return true when a parameter is an implicit any candidate.
fn should_report_parameter(
    ctx: &LintModuleDirContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> bool {
    let parameter = ctx.tree.get(parameter_id);
    if matches!(
        parameter,
        dir::Parameter::Named {
            default: Some(_),
            ..
        } | dir::Parameter::Pattern {
            default: Some(_),
            ..
        }
    ) {
        return false;
    }

    // typed parameters are always explicit
    if ctx
        .types
        .get_declared_type_id(parameter_id.into_global_any(ctx.module_id()))
        .is_some()
    {
        return false;
    }

    // generic parameters are type parameters, not value parameters
    let symbol_id = parameter.symbol().into_global(ctx.module_id());
    let symbol = ctx.symbols.get_symbol(symbol_id.local_id);
    if symbol.is_static_parameter() {
        return false;
    }

    // contextual or inferred concrete parameter types are fine
    let Some(type_id) = ctx.types.get_value_type_id(symbol_id) else {
        return true;
    };
    if is_infer_var_type(ctx.types, type_id) || is_error_type(ctx.types, type_id) {
        return true;
    }

    is_any_type(ctx.types, type_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag untyped declarators without initializers.
    #[test]
    fn test_flags_untyped_declarator_without_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_flags_untyped_declarator_without_initializer.ts",
            r#"
let value;
"#,
        );
        test.result(result)
            .assert_lint("no-implicit-any")
            .assert_has_fix("no-implicit-any");
    }

    /// Allow declarators with initializers.
    #[test]
    fn test_allows_declarator_with_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_allows_declarator_with_initializer.ts",
            r#"
let value = 1;
"#,
        );
        test.result(result).assert_no_lint("no-implicit-any");
    }

    /// Flag untyped declarators that still infer any from initializer values.
    #[test]
    fn test_flags_untyped_declarator_with_any_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_flags_untyped_declarator_with_any_initializer.ts",
            r#"
let source: any;
let value = source;
"#,
        );
        test.result(result).assert_lint("no-implicit-any");
    }

    /// Flag untyped function parameters.
    #[test]
    fn test_flags_untyped_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_flags_untyped_parameter.ts",
            r#"
function identity(value) {
    return value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-implicit-any")
            .assert_has_fix("no-implicit-any");
    }

    /// Allow explicitly typed parameters.
    #[test]
    fn test_allows_typed_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_allows_typed_parameter.ts",
            r#"
function identity(value: number): number {
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("no-implicit-any");
    }

    /// Allow parameters with default values.
    #[test]
    fn test_allows_parameter_with_default() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_allows_parameter_with_default.ts",
            r#"
function f(value = 1) {
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("no-implicit-any");
    }

    /// Allow context typed callback parameters.
    #[test]
    fn test_allows_contextual_callback_parameter() {
        let test = TestProgram::for_rule_with_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_allows_contextual_callback_parameter.ts",
            r#"
let values: number[] = [1, 2, 3];
let mapped = values.map((value) => value + 1);
"#,
        );
        test.result(result).assert_no_lint("no-implicit-any");
    }

    /// Ignore static type parameters.
    #[test]
    fn test_ignores_static_type_parameters() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_ignores_static_type_parameters.ts",
            r#"
function id<T>(value: T): T {
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("no-implicit-any");
    }

    /// Unsafely annotate untyped declarators with unknown.
    #[test]
    fn test_fix_untyped_declarator_without_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_fix_untyped_declarator_without_initializer.ts",
            r#"
let value;
"#,
        );
        test.result(result)
            .assert_lint("no-implicit-any")
            .assert_unsafe_fixed(
                r#"
let value: unknown;
"#,
            );
    }

    /// Unsafely annotate untyped parameters with unknown.
    #[test]
    fn test_fix_untyped_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_fix_untyped_parameter.ts",
            r#"
function identity(value) {
    return value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-implicit-any")
            .assert_unsafe_fixed(
                r#"
function identity(value: unknown) {
    return value;
}
"#,
            );
    }

    /// Flag untyped variadic parameters without offering a broken scalar fix.
    #[test]
    fn test_flags_variadic_parameter_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_flags_variadic_parameter_without_fix.ts",
            r#"
function collect(...values) {
    return values;
}
"#,
        );
        test.result(result)
            .assert_lint("no-implicit-any")
            .assert_has_no_fix("no-implicit-any");
    }

    /// Mutation: annotate destructured parameters when implicitly any typed.
    #[test]
    fn test_mutation_fix_pattern_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitAny);
        let result = test.lint_dir(
            "no_implicit_any/test_mutation_fix_pattern_parameter.ts",
            r#"
function select({ value }) {
    return value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-implicit-any")
            .assert_unsafe_fixed(
                r#"
function select({ value }: unknown) {
    return value;
}
"#,
            );
    }
}
