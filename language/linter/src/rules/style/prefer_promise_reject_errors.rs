use destack_builtin::LanguageSymbol;
use destack_dir::{self as dir, WellKnownSymbol};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    canonical_symbol_for, expression_method_call, expression_target_symbol, expression_type_map,
    is_definitely_non_error_value_type,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer Error objects in Promise rejections.
    ///
    /// Rejecting with primitives makes downstream error handling inconsistent
    /// and loses stack and metadata semantics.
    #[lint(
        id = "prefer-promise-reject-errors",
        code = "LY076",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Promise)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferPromiseRejectErrors,
    "Prefer Error objects in Promise rejections"
}

impl LintRule for PreferPromiseRejectErrors {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferPromiseRejectErrors::meta()
    }

    /// Check module DIR nodes for Promise.reject calls with non-Error payloads.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let reject_name = ctx.program.strings.intern("reject");
        let promise_symbol = ctx.well_known_symbol(WellKnownSymbol::Promise);
        let error_symbol = ctx.get_language_symbol(LanguageSymbol::Error);
        let result_symbol = ctx.get_language_symbol(LanguageSymbol::Result);

        for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::Call {
                dynamic_arguments, ..
            } = expression
            else {
                continue;
            };
            let Some(method_call) = expression_method_call(ctx.tree, expression_id) else {
                continue;
            };

            if method_call.method_name != reject_name {
                continue;
            }
            if !is_promise_receiver(ctx, method_call.receiver_id, promise_symbol) {
                continue;
            }
            if !reject_payload_is_obviously_non_error(
                ctx,
                dynamic_arguments,
                error_symbol,
                result_symbol,
            ) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.get_span(expression_id);
            ctx.report(
                LintDiagnostic::new(
                    PREFER_PROMISE_REJECT_ERRORS.id,
                    PREFER_PROMISE_REJECT_ERRORS.code,
                    PREFER_PROMISE_REJECT_ERRORS.category,
                    severity,
                    "prefer rejecting with Error objects",
                    ctx.module.file_id,
                    span,
                )
                .with_label("pass an Error instance to Promise.reject"),
            );
        }
    }
}

/// Return true when the call receiver is Promise.
fn is_promise_receiver(
    ctx: &LintModuleDirContext<'_>,
    receiver_id: dir::LocalNodeId<dir::Expression>,
    promise_symbol: dir::GlobalSymbolId,
) -> bool {
    let Some(target_symbol) = expression_target_symbol(ctx.tree, receiver_id) else {
        return false;
    };

    if target_symbol == promise_symbol {
        return true;
    }

    canonical_symbol_for(
        &ctx.program,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        target_symbol,
    )
    .is_some_and(|canonical_symbol| canonical_symbol == promise_symbol)
}

/// Return true when the reject payload is clearly non-Error.
fn reject_payload_is_obviously_non_error(
    ctx: &LintModuleDirContext<'_>,
    dynamic_arguments: &[dir::LocalNodeId<dir::Argument>],
    error_symbol: Option<dir::GlobalSymbolId>,
    result_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    if dynamic_arguments.is_empty() {
        return true;
    }

    let first_argument = ctx.tree.get(dynamic_arguments[0]);
    let dir::Argument::Positional { value, .. } = first_argument else {
        return false;
    };
    let value_id = *value;

    // direct literal cases
    if expression_is_non_error_literal(ctx.tree, value_id) {
        return true;
    }

    // typed primitive and nominal non-error cases
    expression_type_map(
        &ctx.program,
        ctx.profile_id,
        ctx.module_id(),
        ctx.tree,
        ctx.symbols,
        ctx.types,
        value_id,
        |types, type_id| {
            is_definitely_non_error_value_type(types, type_id, error_symbol, result_symbol)
        },
    )
    .unwrap_or(false)
}

/// Return true when one expression is a non-Error literal payload.
fn expression_is_non_error_literal(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = tree.get(expression_id);
    matches!(
        expression,
        dir::Expression::ScalarLiteral { .. }
            | dir::Expression::TemplateExpression { .. }
            | dir::Expression::TypeLiteral {
                value: dir::TypeLiteral::Null | dir::TypeLiteral::Undefined
            }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Flag Promise.reject with string literals.
    #[test]
    fn test_flags_promise_reject_string_literal() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_flags_promise_reject_string_literal.ds",
            r#"
Promise.reject("oops");
"#,
        );
        test.result(result)
            .assert_lint("prefer-promise-reject-errors");
    }

    /// Flag Promise.reject without argument.
    #[test]
    fn test_flags_promise_reject_without_argument() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_flags_promise_reject_without_argument.ds",
            r#"
Promise.reject();
"#,
        );
        test.result(result)
            .assert_lint("prefer-promise-reject-errors");
    }

    /// Flag Promise.reject with primitive typed variables.
    #[test]
    fn test_flags_promise_reject_primitive_typed_variable() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_flags_promise_reject_primitive_typed_variable.ds",
            r#"
let message: string = "oops";
Promise.reject(message);
"#,
        );
        test.result(result)
            .assert_lint("prefer-promise-reject-errors");
    }

    /// Allow Promise.reject with Error instances.
    #[test]
    fn test_allows_promise_reject_error_object() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_allows_promise_reject_error_object.ds",
            r#"
Promise.reject(new Error("oops"));
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
    }

    /// Allow Promise.reject with object values when type is not definitely primitive.
    #[test]
    fn test_allows_promise_reject_object_literal() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_allows_promise_reject_object_literal.ds",
            r#"
Promise.reject({ message: "oops" });
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
    }

    /// Allow Promise.reject with custom Error implementations.
    #[test]
    fn test_allows_promise_reject_custom_error_implementation() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_allows_promise_reject_custom_error_implementation.ds",
            r#"
struct AppError implements Error {
    message: string;
}

Promise.reject(AppError { message: "oops" });
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
    }

    /// Flag Promise.reject with Result values.
    #[test]
    fn test_flags_promise_reject_result_error_value() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_flags_promise_reject_result_error_value.ds",
            r#"
Promise.reject(Result.err("oops"));
"#,
        );
        test.result(result)
            .assert_lint("prefer-promise-reject-errors");
    }

    /// Allow Promise.reject with unions that may include Error.
    #[test]
    fn test_allows_promise_reject_union_with_error() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_allows_promise_reject_union_with_error.ds",
            r#"
let value: string | Error = new Error("oops");
Promise.reject(value);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
    }

    /// Allow Promise.reject with imported Error aliases.
    #[test]
    fn test_allows_promise_reject_imported_error_alias() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "prefer_promise_reject_errors/source.ds" => r#"
export type AppError = Error;
"#,
                "prefer_promise_reject_errors/consumer.ds" => r#"
import { AppError } from "./source.ds";

let error_value: AppError = new Error("oops");
Promise.reject(error_value);
"#,
            },
            "prefer_promise_reject_errors/consumer.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-promise-reject-errors");
    }

    /// Ignore non-Promise reject methods.
    #[test]
    fn test_ignores_non_promise_reject_method() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_ignores_non_promise_reject_method.ds",
            r#"
let logger = {
    reject(value: string) {
        return value;
    },
};

logger.reject("oops");
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
    }

    /// Allow Promise.reject with unknown values.
    #[test]
    fn test_allows_promise_reject_unknown_typed_value() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_allows_promise_reject_unknown_typed_value.ds",
            r#"
let value: unknown = "oops";
Promise.reject(value);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
    }
}
