use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_declared_or_inferred_type_id, expression_target_symbol, is_any_type,
    symbol_value_type_id_for, unwrap_value_type_id,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow explicit type assertions that do not change semantics.
    ///
    /// Redundant assertions reduce readability and can hide real type issues.
    #[lint(
        id = "no-unnecessary-type-assertion",
        code = "LC032",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoUnnecessaryTypeAssertion,
    "Disallow unnecessary explicit type assertions"
}

impl LintRule for NoUnnecessaryTypeAssertion {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnnecessaryTypeAssertion::meta()
    }

    /// Check module DIR nodes for redundant type assertions.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // scan all assertion expressions
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let Some(assertion) = assertion_expression_operands(ctx.tree, expression_id) else {
                continue;
            };

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // require optional structure
            let Some(target_type_id) =
                assertion_target_type_id(ctx, assertion.target_type_expression)
            else {
                continue;
            };

            // resolve is redundant
            let is_redundant = source_expression_matches_target_type(
                ctx,
                assertion.source_expression,
                assertion.target_type_expression,
                target_type_id,
            );

            // enforce this lint guard
            if !is_redundant {
                continue;
            }

            // resolve diagnostic span
            let span = ctx.get_span(expression_id);
            let diagnostic = redundant_assertion_diagnostic(
                ctx,
                severity,
                span,
                assertion.source_expression,
                "this assertion repeats the existing type",
                ctx.include_fixes,
            );
            ctx.report(diagnostic);
        }
    }
}

/// Compare source expression type against one assertion target type.
fn source_expression_matches_target_type(
    ctx: &LintModuleDirContext<'_>,
    source_expression_id: dir::LocalNodeId<dir::Expression>,
    target_type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
    target_type_id: dir::LocalTypeId,
) -> bool {
    // direct semantic type equality
    if let Some(source_type_id) = source_expression_type_id(ctx, source_expression_id)
        && dir::are_types_equal(source_type_id, target_type_id, ctx.types)
    {
        return true;
    }

    // redundant `any as any`
    let target_is_any = is_any_type(ctx.types, target_type_id)
        || assertion_target_is_explicit_any(ctx.tree, target_type_expression_id);
    target_is_any && source_expression_is_declared_any(ctx, source_expression_id)
}

/// Resolve the semantic type id for one source expression.
fn source_expression_type_id(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalTypeId> {
    // keep this rule aligned with explicit expression types only
    let type_id = expression_declared_or_inferred_type_id(
        ctx.module_id(),
        ctx.tree,
        ctx.types,
        expression_id,
    )?;

    Some(unwrap_value_type_id(ctx.types, type_id))
}

/// Return true when one source expression is explicitly declared as `any`.
fn source_expression_is_declared_any(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // local expression types are enough when present
    if let Some(type_id) =
        expression_declared_or_inferred_type_id(ctx.module_id(), ctx.tree, ctx.types, expression_id)
    {
        let type_id = unwrap_value_type_id(ctx.types, type_id);
        return is_any_type(ctx.types, type_id);
    }

    // symbol backed references can still expose a declared `any` across modules
    let Some(source_symbol_id) = expression_target_symbol(ctx.tree, expression_id) else {
        return false;
    };
    let Some(source_value_type_id) = symbol_value_type_id_for(
        &ctx.repository,
        ctx.revision,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        ctx.types,
        source_symbol_id,
    ) else {
        return false;
    };

    // same module
    if source_value_type_id.module_id == ctx.module_id() {
        let source_type_id = unwrap_value_type_id(ctx.types, source_value_type_id.type_id);
        return is_any_type(ctx.types, source_type_id);
    }

    // cross module
    let Some(module_dir) = ctx.analyzed_dir(source_value_type_id.module_id) else {
        return false;
    };
    let source_type_id = unwrap_value_type_id(&module_dir.types, source_value_type_id.type_id);
    is_any_type(&module_dir.types, source_type_id)
}

/// Return true when the target expression is an explicit `any` type literal.
fn assertion_target_is_explicit_any(
    tree: &dir::NodeTree,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let expression = tree.get(type_expression_id);
    matches!(
        expression,
        dir::TypeExpression::Literal {
            value: dir::TypeLiteral::Any
        }
    )
}

/// Build one redundant assertion diagnostic with a safe fix.
fn redundant_assertion_diagnostic(
    ctx: &LintModuleDirContext<'_>,
    severity: LintSeverity,
    assertion_span: destack_source::Span,
    source_expression: dir::LocalNodeId<dir::Expression>,
    label: &str,
    include_fixes: bool,
) -> LintDiagnostic {
    let diagnostic = LintDiagnostic::new(
        NO_UNNECESSARY_TYPE_ASSERTION.id,
        NO_UNNECESSARY_TYPE_ASSERTION.code,
        NO_UNNECESSARY_TYPE_ASSERTION.category,
        severity,
        "unnecessary type assertion",
        ctx.module.file_id,
        assertion_span,
    )
    .with_label(label);

    // keep a no fix diagnostic when fixes are disabled
    if !include_fixes {
        return diagnostic;
    }

    // replace the full assertion with the source expression text
    let source_span = ctx.get_span(source_expression);
    let source_text = ctx.get_span_text(source_span).to_string();
    let edits = ctx
        .edit_builder()
        .replace(assertion_span, source_text)
        .into_edits();
    let fix = LintFix::safe("Remove redundant assertion").with_edits(edits);
    diagnostic.with_fix(fix)
}

/// One assertion expression shape normalized across DIR phases.
#[derive(Clone, Copy)]
struct AssertionExpressionOperands {
    /// The asserted value expression.
    source_expression: dir::LocalNodeId<dir::Expression>,
    /// The target type expression.
    target_type_expression: dir::LocalNodeId<dir::TypeExpression>,
}

/// Resolve assertion operands for explicit `as` assertions.
fn assertion_expression_operands(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<AssertionExpressionOperands> {
    let expression = tree.get(expression_id);
    match expression {
        dir::Expression::As {
            operator: _,
            source: _,
            expression,
            target_type,
        } => Some(AssertionExpressionOperands {
            source_expression: *expression,
            target_type_expression: *target_type,
        }),
        _ => None,
    }
}

/// Resolve the semantic type id for one assertion target type expression.
fn assertion_target_type_id(
    ctx: &LintModuleDirContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> Option<dir::LocalTypeId> {
    let global_type_expression_id = type_expression_id.into_global_any(ctx.module_id());
    ctx.types.get_declared_type_id(global_type_expression_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Flag redundant assertions that preserve the same type.
    #[test]
    fn test_flags_redundant_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let result = test.lint_dir(
            "no_unnecessary_type_assertion/test_flags_redundant_assertion.ds",
            r#"
let value: string = "ok";
let copy = value as string;
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-assertion");
    }

    /// Allow assertions that change the type.
    #[test]
    fn test_allows_necessary_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let result = test.lint_dir(
            "no_unnecessary_type_assertion/test_allows_necessary_assertion.ds",
            r#"
let value: unknown = "ok";
let narrowed = value as string;
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-type-assertion");
    }

    /// Ignore implicit casts inserted by the checker.
    #[test]
    fn test_ignores_implicit_casts() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let result = test.lint_dir(
            "no_unnecessary_type_assertion/test_ignores_implicit_casts.ds",
            r#"
function id<T>(value: T): T {
    return value;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-type-assertion");
    }

    /// Report redundant assertions for imported return types.
    #[test]
    fn test_flags_cross_module_redundant_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_type_assertion/source.ds" => r#"
export function value(): string {
    return "ok";
}
"#,
                "no_unnecessary_type_assertion/consumer.ds" => r#"
import { value } from "./source.ds";

const output = value() as string;
"#,
            },
            "no_unnecessary_type_assertion/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-type-assertion");
    }

    /// Allow cross module assertions that narrow unions.
    #[test]
    fn test_allows_cross_module_necessary_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_type_assertion/source_union.ds" => r#"
export function value(flag: boolean): string | int32 {
    if flag {
        return "ok";
    }

    return 1;
}
"#,
                "no_unnecessary_type_assertion/consumer_union.ds" => r#"
import { value } from "./source_union.ds";

const output = value(true) as string;
"#,
            },
            "no_unnecessary_type_assertion/consumer_union.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("no-unnecessary-type-assertion");
    }

    /// Ignore non cast type binary operations.
    #[test]
    fn test_ignores_non_cast_type_binary() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let result = test.lint_dir(
            "no_unnecessary_type_assertion/test_ignores_non_cast_type_binary.ds",
            r#"
let value: unknown = "ok";
let check = value is string;
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-type-assertion");
    }

    /// Safely remove a redundant local assertion.
    #[test]
    fn test_fix_redundant_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let result = test.lint_dir(
            "no_unnecessary_type_assertion/test_fix_redundant_assertion.ds",
            r#"
let value: string = "ok";
let copy = value as string;
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-assertion")
            .assert_has_fix("no-unnecessary-type-assertion")
            .assert_safe_fixed(
                r#"
let value: string = "ok";
let copy = value;
"#,
            );
    }

    /// Safely remove a redundant cross module assertion.
    #[test]
    fn test_fix_cross_module_redundant_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_type_assertion/fix_source.ds" => r#"
export function value(): string {
    return "ok";
}
"#,
                "no_unnecessary_type_assertion/fix_consumer.ds" => r#"
import { value } from "./fix_source.ds";

const output = value() as string;
"#,
            },
            "no_unnecessary_type_assertion/fix_consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-type-assertion")
            .assert_has_fix("no-unnecessary-type-assertion")
            .assert_safe_fixed(
                r#"
import { value } from "./fix_source.ds";

const output = value();
"#,
            );
    }

    /// Flag cross module `any as any` assertions through the declared symbol type.
    #[test]
    fn test_flags_cross_module_any_assertion() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_type_assertion/any_source.ds" => r#"
export let value: any = "ok";
"#,
                "no_unnecessary_type_assertion/any_consumer.ds" => r#"
import { value } from "./any_source.ds";

const output = value as any;
"#,
            },
            "no_unnecessary_type_assertion/any_consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-type-assertion");
    }

    /// Safely remove a redundant assertion around a compound expression.
    #[test]
    fn test_fix_redundant_assertion_on_compound_expression() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeAssertion);
        let result = test.lint_dir(
            "no_unnecessary_type_assertion/test_fix_redundant_assertion_on_compound_expression.ds",
            r#"
let value: string = "ok";
let output = (value + "!") as string;
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-assertion")
            .assert_has_fix("no-unnecessary-type-assertion")
            .assert_safe_fixed(
                r#"
let value: string = "ok";
let output = (value + '!');
"#,
            );
    }
}
