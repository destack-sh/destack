use destack_dir::{self as dir, LanguageItem, WellKnownSymbol};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    collect_local_symbol_direct_reference_expression_ids, expression_method_call,
    expression_target_symbol, expression_type_map, is_definitely_non_error_value_type,
    parameter_binding_name_and_symbol,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

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
        let mut visitor = PromiseRejectVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags non-Error Promise rejection payloads.
struct PromiseRejectVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The interned `reject` name.
    reject_name: destack_core::StringId,
    /// The interned `ok` name.
    ok_name: destack_core::StringId,
    /// The interned `err` name.
    err_name: destack_core::StringId,
    /// The well known Promise symbol.
    promise_symbol: dir::GlobalSymbolId,
    /// The built in Error symbol when available.
    error_symbol: Option<dir::GlobalSymbolId>,
    /// The built in Result symbol when available.
    result_symbol: Option<dir::GlobalSymbolId>,
}

impl<'a, 'b> PromiseRejectVisitor<'a, 'b> {
    /// Build a visitor for Promise rejection checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            reject_name: ctx.string_id("reject"),
            ok_name: ctx.string_id("ok"),
            err_name: ctx.string_id("err"),
            promise_symbol: ctx.well_known_symbol(WellKnownSymbol::Promise),
            error_symbol: ctx.get_language_item(LanguageItem::Error),
            result_symbol: ctx.get_language_item(LanguageItem::Result),
            ctx,
            meta,
        }
    }

    /// Walk the module expressions.
    fn run(&mut self) {
        let expression_ids = self.ctx.tree.iter_node_ids_of_type::<dir::Expression>();

        for expression_id in expression_ids {
            self.check_expression(expression_id);
        }
    }

    /// Check one expression for Promise rejection patterns.
    fn check_expression(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        let expression = self.ctx.tree.get(expression_id);

        // direct Promise.reject(...)
        if let dir::Expression::Call { arguments, .. } = expression
            && let Some(method_call) = expression_method_call(self.ctx.tree, expression_id)
            && method_call.method_name == self.reject_name
            && is_promise_receiver(self.ctx, method_call.receiver_id, self.promise_symbol)
        {
            self.check_reject_payload(expression_id, arguments);
        }

        // executor reject(...)
        if let dir::Expression::New {
            left, arguments, ..
        } = expression
            && expression_target_symbol(self.ctx, *left) == Some(self.promise_symbol)
        {
            self.check_executor_reject_calls(arguments);
        }
    }

    /// Check one reject-like call payload.
    fn check_reject_payload(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        if !reject_payload_is_obviously_non_error(
            self.ctx,
            value_id_from_arguments(self.ctx.tree, arguments),
            self.ok_name,
            self.err_name,
            arguments,
            self.error_symbol,
            self.result_symbol,
        ) {
            return;
        }

        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                PREFER_PROMISE_REJECT_ERRORS.id,
                PREFER_PROMISE_REJECT_ERRORS.code,
                PREFER_PROMISE_REJECT_ERRORS.category,
                severity,
                "prefer rejecting with Error objects",
                span,
            )
            .label("pass an Error instance to Promise rejection"),
        );
    }

    /// Check reject(...) calls inside one Promise executor.
    fn check_executor_reject_calls(&mut self, arguments: &[dir::LocalNodeId<dir::Argument>]) {
        let Some(executor_id) = value_id_from_arguments(self.ctx.tree, arguments) else {
            return;
        };
        let Some(executor_declaration_id) = executor_declaration(self.ctx, executor_id) else {
            return;
        };
        let Some(reject_symbol) = executor_reject_symbol(self.ctx, executor_declaration_id) else {
            return;
        };

        let reject_references = collect_local_symbol_direct_reference_expression_ids(
            self.ctx.module.id,
            self.ctx.tree,
            self.ctx.types,
            reject_symbol,
        );
        for reference_id in reject_references {
            let Some(parent) = self.ctx.tree.get_parent(reference_id.id) else {
                continue;
            };
            if parent.ty != dir::NodeType::Expression {
                continue;
            }

            let parent_id = parent.into_typed::<dir::Expression>();
            let parent_expression = self.ctx.tree.get(parent_id);
            let dir::Expression::Call {
                left, arguments, ..
            } = parent_expression
            else {
                continue;
            };
            if *left != reference_id {
                continue;
            }

            self.check_reject_payload(parent_id, arguments);
        }
    }
}

/// Return true when the call receiver is Promise.
fn is_promise_receiver(
    ctx: &LintModuleDirContext<'_>,
    receiver_id: dir::LocalNodeId<dir::Expression>,
    promise_symbol: dir::GlobalSymbolId,
) -> bool {
    let Some(target_symbol) = expression_target_symbol(ctx, receiver_id) else {
        return false;
    };

    target_symbol == promise_symbol
}

/// Return true when the reject payload is clearly non-Error.
fn reject_payload_is_obviously_non_error(
    ctx: &LintModuleDirContext<'_>,
    value_id: Option<dir::LocalNodeId<dir::Expression>>,
    ok_name: destack_core::StringId,
    err_name: destack_core::StringId,
    arguments: &[dir::LocalNodeId<dir::Argument>],
    error_symbol: Option<dir::GlobalSymbolId>,
    result_symbol: Option<dir::GlobalSymbolId>,
) -> bool {
    if arguments.is_empty() {
        return !ctx
            .options
            .style
            .prefer_promise_reject_errors_allow_empty_reject;
    }

    let Some(value_id) = value_id else {
        return false;
    };

    // direct literal cases
    if expression_is_non_error_literal(ctx.tree, value_id) {
        return true;
    }

    // explicit result constructors always produce non error payload values
    if expression_is_result_constructor_call(ctx, value_id, result_symbol, ok_name, err_name) {
        return true;
    }

    // typed primitive and nominal non-error cases
    expression_type_map(
        ctx.artifacts.as_ref(),
        ctx.profile_id,
        ctx.module_id(),
        ctx.tree,
        ctx.types,
        value_id,
        |types, type_id| {
            is_definitely_non_error_value_type(types, type_id, error_symbol, result_symbol)
        },
    )
    .unwrap_or(false)
}

/// Return the first positional argument expression id when available.
fn value_id_from_arguments(
    tree: &dir::Tree,
    arguments: &[dir::LocalNodeId<dir::Argument>],
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let first_argument = tree.get(*arguments.first()?);
    let dir::Argument::Positional { value, .. } = first_argument else {
        return None;
    };

    Some(*value)
}

/// Resolve one Promise executor declaration from an inline or local callable reference.
fn executor_declaration(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Declaration>> {
    let expression = ctx.tree.get(expression_id);

    // inline callables
    if let dir::Expression::Declaration(declaration) = expression {
        return Some(*declaration);
    }

    // local callable references
    let target_symbol = expression_target_symbol(ctx, expression_id)?;
    if target_symbol.module_id != ctx.module.id {
        return None;
    }

    let symbol_entry = ctx.symbols.get_symbol(target_symbol.local_id);
    let declaration = symbol_entry.declaration?;
    if declaration.module_id != ctx.module.id {
        return None;
    }
    if declaration.local_id.ty != dir::NodeType::Declaration {
        return None;
    }

    Some(declaration.into_local_typed())
}

/// Resolve the reject parameter symbol from one Promise executor declaration.
fn executor_reject_symbol(
    ctx: &LintModuleDirContext<'_>,
    declaration_id: dir::LocalNodeId<dir::Declaration>,
) -> Option<dir::LocalSymbolId> {
    let declaration = ctx.tree.get(declaration_id);
    let dir::Declaration::Function(declaration) = declaration else {
        return None;
    };
    declaration.body?;

    let reject_parameter_id = *declaration.signature.parameters.get(1)?;
    let (_, reject_symbol) = parameter_binding_name_and_symbol(ctx.tree, reject_parameter_id)?;
    Some(reject_symbol)
}

/// Return true when the expression is `Result.ok(...)` or `Result.err(...)`.
fn expression_is_result_constructor_call(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    result_symbol: Option<dir::GlobalSymbolId>,
    ok_name: destack_core::StringId,
    err_name: destack_core::StringId,
) -> bool {
    let Some(result_symbol) = result_symbol else {
        return false;
    };

    let Some(method_call) = expression_method_call(ctx.tree, expression_id) else {
        return false;
    };
    if method_call.method_name != ok_name && method_call.method_name != err_name {
        return false;
    }

    let Some(receiver_symbol) = expression_target_symbol(ctx, method_call.receiver_id) else {
        return false;
    };

    receiver_symbol == result_symbol
}

/// Return true when one expression is a non-Error literal payload.
fn expression_is_non_error_literal(
    tree: &dir::Tree,
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

    /// Allow Promise.reject without argument when configured.
    #[test]
    fn test_allows_promise_reject_without_argument_when_enabled() {
        let test =
            TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors).with_options(|options| {
                options
                    .style
                    .prefer_promise_reject_errors_allow_empty_reject = true
            });
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_allows_promise_reject_without_argument_when_enabled.ds",
            r#"
Promise.reject();
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
    }

    /// Flag executor reject calls with primitive payloads.
    #[test]
    fn test_flags_executor_reject_string_literal() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_flags_executor_reject_string_literal.ds",
            r#"
let task = new Promise((resolve, reject) => {
    reject("oops");
});
"#,
        );
        test.result(result)
            .assert_lint("prefer-promise-reject-errors");
    }

    /// Allow executor reject without argument when configured.
    #[test]
    fn test_allows_executor_reject_without_argument_when_enabled() {
        let test =
            TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors).with_options(|options| {
                options
                    .style
                    .prefer_promise_reject_errors_allow_empty_reject = true
            });
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_allows_executor_reject_without_argument_when_enabled.ds",
            r#"
let task = new Promise((resolve, reject) => {
    reject();
});
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
    }

    /// Ignore shadowed reject bindings inside Promise executors.
    #[test]
    fn test_ignores_shadowed_executor_reject_binding() {
        let test = TestProgram::for_rule_with_prelude(PreferPromiseRejectErrors);
        let result = test.lint_dir(
            "prefer_promise_reject_errors/test_ignores_shadowed_executor_reject_binding.ds",
            r#"
let task = new Promise((resolve, reject) => {
    let reject = (value: string) => value;
    reject("oops");
});
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-promise-reject-errors");
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
