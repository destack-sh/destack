use destack_dir::{self as dir, Asynchrony, LanguageItem};
use destack_repository::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{function_return_type, is_promise_type};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Require `async` on Promise-returning function bodies.
    ///
    /// Functions with concrete bodies that return Promise values should be
    /// explicitly marked `async` for consistency and readability.
    #[lint(
        id = "promise-function-async",
        code = "LY080",
        category = Style,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Promise)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PromiseFunctionAsync,
    "Require async keyword on Promise-returning functions"
}

impl LintRule for PromiseFunctionAsync {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PromiseFunctionAsync::meta()
    }

    /// Check module DIR nodes for Promise-returning non-async functions.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let Some(promise_symbol) = ctx.get_language_item(LanguageItem::Promise) else {
            return;
        };

        // function declarations
        for (declaration_id, declaration) in ctx.dir.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };
            let Some(function_symbol) = ctx.symbol_for_node(declaration_id) else {
                continue;
            };

            if declaration.body.is_none() {
                continue;
            }
            if declaration.signature.asynchrony == Asynchrony::Async {
                continue;
            }
            if !function_symbol_returns_promise(ctx, function_symbol, promise_symbol)
                && !signature_returns_promise(
                    ctx,
                    declaration.signature.return_type,
                    promise_symbol,
                )
            {
                continue;
            }

            report_promise_function_async(ctx, meta, declaration_id);
        }

        // class and struct methods
        for (member_id, member) in ctx.dir.iter_nodes_of_type::<dir::Member>() {
            let dir::Member::Method {
                signature, body, ..
            } = member
            else {
                continue;
            };
            let Some(function_symbol) = ctx.symbol_for_node(member_id) else {
                continue;
            };

            if body.is_none() {
                continue;
            }
            if signature.asynchrony == Asynchrony::Async {
                continue;
            }
            if !function_symbol_returns_promise(ctx, function_symbol, promise_symbol)
                && !signature_returns_promise(ctx, signature.return_type, promise_symbol)
            {
                continue;
            }

            report_promise_function_async(ctx, meta, member_id);
        }

        // object literal methods
        for (property_id, property) in ctx.dir.iter_nodes_of_type::<dir::Property>() {
            let dir::Property::Method {
                signature, body, ..
            } = property
            else {
                continue;
            };
            let Some(function_symbol) = ctx.symbol_for_node(property_id) else {
                continue;
            };

            if body.is_none() {
                continue;
            }
            if signature.asynchrony == Asynchrony::Async {
                continue;
            }
            if !function_symbol_returns_promise(ctx, function_symbol, promise_symbol)
                && !signature_returns_promise(ctx, signature.return_type, promise_symbol)
            {
                continue;
            }

            report_promise_function_async(ctx, meta, property_id);
        }
    }
}

/// Return true when one function symbol returns Promise.
fn function_symbol_returns_promise(
    ctx: &LintModuleContext<'_>,
    symbol_id: dir::GlobalSymbolId,
    promise_symbol: dir::GlobalSymbolId,
) -> bool {
    let Some(function_type_id) = ctx.types.get_symbol_type_id(symbol_id) else {
        return false;
    };
    let Some(return_type_id) = function_return_type(ctx, function_type_id) else {
        return false;
    };

    is_promise_type(ctx, return_type_id, Some(promise_symbol))
}

/// Return true when one signature return annotation resolves to Promise.
fn signature_returns_promise(
    ctx: &LintModuleContext<'_>,
    return_type_expression_id: Option<dir::LocalNodeId<dir::TypeExpression>>,
    promise_symbol: dir::GlobalSymbolId,
) -> bool {
    let Some(return_type_expression_id) = return_type_expression_id else {
        return false;
    };

    let global_type_expression_id = return_type_expression_id.into_global_any(ctx.module_id());
    let Some(return_type_id) = ctx.types.get_node_type_id(global_type_expression_id) else {
        return false;
    };

    is_promise_type(ctx, return_type_id, Some(promise_symbol))
}

/// Report one Promise-function-async diagnostic.
fn report_promise_function_async<T: dir::Node>(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    node_id: dir::LocalNodeId<T>,
) {
    let severity = ctx.get_effective_severity(meta, dir::LocalNodeId::<T>::new(node_id.id));
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.get_span(node_id);
    ctx.report(
        LintReport::new(
            PROMISE_FUNCTION_ASYNC.id,
            PROMISE_FUNCTION_ASYNC.code,
            PROMISE_FUNCTION_ASYNC.category,
            severity,
            "Promise-returning function should be marked async",
            span,
        )
        .label("add `async` to this function"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Flag non-async functions that return Promise.
    #[test]
    fn test_flags_sync_function_returning_promise() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_flags_sync_function_returning_promise.ds",
            r#"
function load(): Promise<int32> {
    return Promise.resolve(1);
}
"#,
        );
        test.result(result).assert_lint("promise-function-async");
    }

    /// Allow async functions that return Promise.
    #[test]
    fn test_allows_async_function_returning_promise() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_allows_async_function_returning_promise.ds",
            r#"
async function load(): Promise<int32> {
    return Promise.resolve(1);
}
"#,
        );
        test.result(result).assert_no_lint("promise-function-async");
    }

    /// Allow non-Promise returning sync functions.
    #[test]
    fn test_allows_sync_non_promise_function() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_allows_sync_non_promise_function.ds",
            r#"
function load(): int32 {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("promise-function-async");
    }

    /// Flag non-async class methods that return Promise.
    #[test]
    fn test_flags_sync_method_returning_promise() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_flags_sync_method_returning_promise.ds",
            r#"
class Service {
    fetch(): Promise<int32> {
        return Promise.resolve(1);
    }
}
"#,
        );
        test.result(result).assert_lint("promise-function-async");
    }

    /// Flag non-async object methods that return Promise.
    #[test]
    fn test_flags_sync_object_method_returning_promise() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_flags_sync_object_method_returning_promise.ds",
            r#"
let service = {
    fetch(): Promise<int32> {
        return Promise.resolve(1);
    },
};
"#,
        );
        test.result(result).assert_lint("promise-function-async");
    }

    /// Flag Promise aliases from local type aliases.
    #[test]
    fn test_flags_sync_function_returning_local_promise_alias() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_flags_sync_function_returning_local_promise_alias.ds",
            r#"
type LoadResult = Promise<int32>;

function load(): LoadResult {
    return Promise.resolve(1);
}
"#,
        );
        test.result(result).assert_lint("promise-function-async");
    }

    /// Flag Promise aliases imported from another module.
    #[test]
    fn test_flags_sync_function_returning_imported_promise_alias() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "promise_function_async/source.ds" => r#"
export type LoadResult = Promise<int32>;
"#,
                "promise_function_async/consumer.ds" => r#"
import { LoadResult } from "./source.ds";

function load(): LoadResult {
    return Promise.resolve(1);
}
"#,
            },
            "promise_function_async/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("promise-function-async")
            .assert_lint_count("promise-function-async", 1);
    }

    /// Allow union return annotations that include Promise.
    #[test]
    fn test_allows_sync_function_returning_promise_union() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_allows_sync_function_returning_promise_union.ds",
            r#"
function load(flag: boolean): Promise<int32> | null {
    if (flag) {
        return Promise.resolve(1);
    }

    return null;
}
"#,
        );
        test.result(result).assert_no_lint("promise-function-async");
    }

    /// Allow sync methods without Promise return types.
    #[test]
    fn test_allows_sync_object_method_without_promise_return() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_allows_sync_object_method_without_promise_return.ds",
            r#"
let service = {
    fetch(): int32 {
        return 1;
    },
};
"#,
        );
        test.result(result).assert_no_lint("promise-function-async");
    }

    /// Flag sync functions with inferred Promise return types.
    #[test]
    fn test_flags_sync_function_with_inferred_promise_return() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_flags_sync_function_with_inferred_promise_return.ds",
            r#"
function load() {
    return Promise.resolve(1);
}
"#,
        );
        test.result(result).assert_lint("promise-function-async");
    }

    /// Allow declaration-only signatures without function bodies.
    #[test]
    fn test_allows_declaration_only_promise_signature() {
        let test = TestProgram::for_rule_with_prelude(PromiseFunctionAsync);
        let result = test.lint_dir(
            "promise_function_async/test_allows_declaration_only_promise_signature.ds",
            r#"
declare function load(): Promise<int32>;
"#,
        );
        test.result(result).assert_no_lint("promise-function-async");
    }
}
