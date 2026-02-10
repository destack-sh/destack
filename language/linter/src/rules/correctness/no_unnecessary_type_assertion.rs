use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_declared_or_inferred_type_id, expression_is_any_typed, expression_target_symbol,
    expression_unwrap_parenthesized, is_any_type, symbol_value_type_id_for,
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
    fn meta(&self) -> &'static LintMeta {
        NoUnnecessaryTypeAssertion::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // scan all assertion expressions
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let Some(assertion) = assertion_expression_operands(ctx.tree, expression_id) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // explicit identity casts are always redundant
            if assertion.is_explicit_identity {
                let span = ctx.get_span(expression_id);
                let diagnostic = redundant_assertion_diagnostic(
                    ctx,
                    severity,
                    span,
                    assertion.source_expression,
                    "this assertion does not change the type",
                );
                ctx.report(diagnostic);
                continue;
            }

            let Some(target_type_id) = assertion_operand_type_id(ctx, assertion.target_expression)
            else {
                continue;
            };

            let is_redundant = source_expression_matches_target_type(
                ctx,
                assertion.source_expression,
                assertion.target_expression,
                target_type_id,
            );

            if !is_redundant {
                continue;
            }

            let span = ctx.get_span(expression_id);
            let diagnostic = redundant_assertion_diagnostic(
                ctx,
                severity,
                span,
                assertion.source_expression,
                "this assertion repeats the existing type",
            );
            ctx.report(diagnostic);
        }
    }
}

/// Compare source expression type against one assertion target type.
fn source_expression_matches_target_type(
    ctx: &LintModuleDirContext<'_>,
    source_expression_id: dir::LocalNodeId<dir::Expression>,
    target_expression_id: dir::LocalNodeId<dir::Expression>,
    target_type_id: dir::LocalTypeId,
) -> bool {
    let target_is_any = is_any_type(ctx.types, target_type_id)
        || assertion_target_is_explicit_any(ctx.tree, target_expression_id);

    // prefer expression type comparisons from the current module
    if let Some(source_type_id) = assertion_operand_type_id(ctx, source_expression_id)
        && dir::are_types_equal(source_type_id, target_type_id, ctx.types)
    {
        return true;
    }

    // preserve `any as any` behavior when flow types are narrower than declared any
    if target_is_any
        && expression_is_any_typed(
            ctx.module_id(),
            ctx.tree,
            ctx.symbols,
            ctx.types,
            source_expression_id,
        )
    {
        return true;
    }

    // fallback: compare the source symbol value type when expression types are unavailable
    let Some(source_symbol_id) = expression_target_symbol(ctx.tree, source_expression_id) else {
        return false;
    };
    let Some(source_value_type_id) = symbol_value_type_id_for(
        &ctx.program,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        ctx.types,
        source_symbol_id,
    ) else {
        return false;
    };

    // same module: compare semantic type equality directly
    if source_value_type_id.module_id == ctx.module_id() {
        let source_type_id = unwrap_type_value(ctx.types, source_value_type_id.type_id);
        return dir::are_types_equal(source_type_id, target_type_id, ctx.types);
    }

    // cross module fallback: only accept this path for `any as any`
    if !target_is_any {
        return false;
    }

    let module_ref = ctx.program.modules.get(source_value_type_id.module_id);
    let module = module_ref.read();
    let Some(module_dir) = module.dir_maybe(ctx.profile_id) else {
        return false;
    };
    let types = module_dir.types.read();
    let source_type_id = unwrap_type_value(&types, source_value_type_id.type_id);
    is_any_type(&types, source_type_id)
}

/// Return true when the target expression is an explicit `any` type literal.
fn assertion_target_is_explicit_any(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    matches!(
        expression,
        dir::Expression::TypeLiteral {
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
) -> LintDiagnostic {
    let source_span = ctx.get_span(source_expression);
    let source_text = ctx.get_span_text(source_span).to_string();
    let edits = ctx
        .edit_builder()
        .replace(assertion_span, source_text)
        .into_edits();
    let fix = LintFix::safe("Remove redundant assertion").with_edits(edits);

    LintDiagnostic::new(
        NO_UNNECESSARY_TYPE_ASSERTION.id,
        NO_UNNECESSARY_TYPE_ASSERTION.code,
        NO_UNNECESSARY_TYPE_ASSERTION.category,
        severity,
        "unnecessary type assertion",
        ctx.module.file_id,
        assertion_span,
    )
    .with_label(label)
    .with_fix(fix)
}

/// One assertion expression shape normalized across DIR phases.
#[derive(Clone, Copy)]
struct AssertionExpressionOperands {
    /// The asserted value expression.
    source_expression: dir::LocalNodeId<dir::Expression>,
    /// The target type expression.
    target_expression: dir::LocalNodeId<dir::Expression>,
    /// Whether this came from an explicit identity cast node.
    is_explicit_identity: bool,
}

/// Resolve assertion operands for explicit cast nodes and pre reify type binary casts.
fn assertion_expression_operands(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<AssertionExpressionOperands> {
    let expression = tree.get(expression_id);
    match expression {
        // explicit cast in reified DIR
        dir::Expression::Cast {
            operator,
            source,
            value,
            target_type,
        } => (*source == dir::CastSource::Explicit).then_some(AssertionExpressionOperands {
            source_expression: *value,
            target_expression: *target_type,
            is_explicit_identity: *operator == dir::CastOperator::Identity,
        }),

        // pre reify `x as T` in analyzed DIR
        dir::Expression::TypeBinary {
            left,
            operator: dir::TypeBinaryOperator::Cast,
            right,
        } => Some(AssertionExpressionOperands {
            source_expression: *left,
            target_expression: *right,
            is_explicit_identity: false,
        }),

        _ => None,
    }
}

/// Resolve the semantic type id for one assertion operand expression.
fn assertion_operand_type_id(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalTypeId> {
    let type_id = expression_declared_or_inferred_type_id(
        ctx.module_id(),
        ctx.tree,
        ctx.types,
        expression_id,
    )
    .or_else(|| ctx.expression_type_id(expression_id))?;

    Some(unwrap_type_value(ctx.types, type_id))
}

/// Unwrap nested `Type::Value` wrappers to reach the underlying type.
fn unwrap_type_value(types: &dir::TypeTable, mut type_id: dir::LocalTypeId) -> dir::LocalTypeId {
    loop {
        let dir::Type::Value { value } = types.get_type(type_id) else {
            return type_id;
        };

        if *value == type_id {
            return type_id;
        }

        type_id = *value;
    }
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
