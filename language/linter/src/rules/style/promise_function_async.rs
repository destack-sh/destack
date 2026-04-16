use destack_dir::{self as dir, Asynchrony, WellKnownSymbol};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    function_return_type, is_promise_type_with_candidates, well_known_symbol_candidates,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

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
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Promise)],
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
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let promise_symbols = resolve_promise_symbols(ctx);
        if promise_symbols.is_empty() {
            return;
        }

        // function declarations
        for (declaration_id, declaration) in ctx.tree.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };
            let function_symbol = declaration.symbol.into_global(ctx.module_id());

            if declaration.body.is_none() {
                continue;
            }
            if declaration.signature.asynchrony == Asynchrony::Async {
                continue;
            }
            if !function_symbol_returns_promise(ctx, function_symbol, &promise_symbols)
                && !signature_returns_promise(
                    ctx,
                    declaration.signature.return_type,
                    &promise_symbols,
                )
            {
                continue;
            }

            report_promise_function_async(ctx, meta, declaration_id);
        }

        // class and struct methods
        for (member_id, member) in ctx.tree.iter_nodes_of_type::<dir::Member>() {
            let dir::Member::Method {
                signature,
                body,
                symbol,
                ..
            } = member
            else {
                continue;
            };
            let function_symbol = symbol.into_global(ctx.module_id());

            if body.is_none() {
                continue;
            }
            if signature.asynchrony == Asynchrony::Async {
                continue;
            }
            if !function_symbol_returns_promise(ctx, function_symbol, &promise_symbols)
                && !signature_returns_promise(ctx, signature.return_type, &promise_symbols)
            {
                continue;
            }

            report_promise_function_async(ctx, meta, member_id);
        }

        // object literal methods
        for (property_id, property) in ctx.tree.iter_nodes_of_type::<dir::Property>() {
            let dir::Property::Method {
                signature,
                body,
                symbol,
                ..
            } = property
            else {
                continue;
            };
            let function_symbol = symbol.into_global(ctx.module_id());

            if body.is_none() {
                continue;
            }
            if signature.asynchrony == Asynchrony::Async {
                continue;
            }
            if !function_symbol_returns_promise(ctx, function_symbol, &promise_symbols)
                && !signature_returns_promise(ctx, signature.return_type, &promise_symbols)
            {
                continue;
            }

            report_promise_function_async(ctx, meta, property_id);
        }
    }
}

/// Resolve all concrete Promise symbols from type and value spaces.
fn resolve_promise_symbols(ctx: &LintModuleDirContext<'_>) -> Vec<dir::GlobalSymbolId> {
    let Some(well_known_symbols) = ctx.get_well_known_symbols() else {
        return Vec::new();
    };

    well_known_symbol_candidates(&well_known_symbols, WellKnownSymbol::Promise)
}

/// Return true when one type resolves to Promise.
fn type_is_promise_with_symbols(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    promise_symbols: &[dir::GlobalSymbolId],
) -> bool {
    is_promise_type_with_candidates(types, type_id, promise_symbols)
}

/// Return true when one function symbol returns Promise.
fn function_symbol_returns_promise(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::GlobalSymbolId,
    promise_symbols: &[dir::GlobalSymbolId],
) -> bool {
    let Some(function_type_id) = ctx.types.get_type_id_for_symbol(ctx.symbols, symbol_id) else {
        return false;
    };
    let Some(return_type_id) = function_return_type(ctx.types, function_type_id) else {
        return false;
    };

    type_is_promise_with_symbols(ctx.types, return_type_id, promise_symbols)
}

/// Return true when one signature return annotation resolves to Promise.
fn signature_returns_promise(
    ctx: &LintModuleDirContext<'_>,
    return_type_expression_id: Option<dir::LocalNodeId<dir::TypeExpression>>,
    promise_symbols: &[dir::GlobalSymbolId],
) -> bool {
    let Some(return_type_expression_id) = return_type_expression_id else {
        return false;
    };

    let global_type_expression_id = return_type_expression_id.into_global_any(ctx.module_id());
    let Some(return_type_id) = ctx.types.get_declared_type_id(global_type_expression_id) else {
        return false;
    };

    type_is_promise_with_symbols(ctx.types, return_type_id, promise_symbols)
}

/// Report one Promise-function-async diagnostic.
fn report_promise_function_async<T: dir::Node>(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    node_id: dir::LocalNodeId<T>,
) {
    let severity = ctx.get_effective_severity(meta, dir::LocalNodeId::<T>::new(node_id.id));
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.get_span(node_id);
    ctx.report(
        LintDiagnostic::new(
            PROMISE_FUNCTION_ASYNC.id,
            PROMISE_FUNCTION_ASYNC.code,
            PROMISE_FUNCTION_ASYNC.category,
            severity,
            "Promise-returning function should be marked async",
            ctx.module.file_id,
            span,
        )
        .with_label("add `async` to this function"),
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
