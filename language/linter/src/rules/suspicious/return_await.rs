use destack_dir as dir;
use destack_dir::LanguageItem;
use destack_workspace::{LintSeverity, ReturnAwaitMode};

use crate::rules::common::{
    expression_affects_error_handling_context, expression_affects_resource_management_context,
    expression_is_any_typed, expression_is_inside_async_callable, expression_is_promise_like,
    expression_type_or_call_return_type_map, expression_unwrap_parenthesized,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow redundant `return await` in async callables.
    ///
    /// In async functions, `return await value` is usually redundant and can be
    /// simplified to `return value` when it is not inside a try context.
    #[lint(
        id = "return-await",
        code = "LU045",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub ReturnAwait,
    "Disallow redundant `return await` in async callables"
}

impl LintRule for ReturnAwait {
    fn meta(&self) -> &'static LintMeta {
        ReturnAwait::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let promise_symbol = ctx.get_language_item(LanguageItem::Promise);
        let configuration = return_await_configuration(ctx.options.correctness.return_await_mode);

        // inspect return expressions
        for return_expression_id in ctx.dir.iter_node_ids_of_type::<dir::Expression>() {
            let return_expression = ctx.dir.get(return_expression_id);
            let dir::Expression::Return {
                value: Some(value_expression_id),
            } = return_expression
            else {
                continue;
            };

            // keep async callable returns only
            if !expression_is_inside_async_callable(ctx.dir.tree(), return_expression_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, return_expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // inspect all conditional return branches for this return statement
            // keep try-catch and resource-management contexts aligned to source policy
            let in_control_flow_sensitive_context =
                expression_affects_error_handling_context(ctx.dir.tree(), return_expression_id)
                    || expression_affects_resource_management_context(
                        ctx.dir.tree(),
                        return_expression_id,
                    );
            let mut possible_return_values = Vec::new();
            collect_possible_return_values(
                ctx.dir.tree(),
                *value_expression_id,
                &mut possible_return_values,
            );
            for possible_return_value_id in possible_return_values {
                check_return_value_expression(
                    ctx,
                    meta,
                    promise_symbol,
                    configuration,
                    return_expression_id,
                    possible_return_value_id,
                    in_control_flow_sensitive_context,
                );
            }
        }

        // inspect concise async expression bodies as implicit returns
        for body_expression_id in async_callable_expression_bodies(ctx.dir.tree()) {
            // keep try-catch and resource-management contexts aligned to source policy
            let in_control_flow_sensitive_context =
                expression_affects_error_handling_context(ctx.dir.tree(), body_expression_id)
                    || expression_affects_resource_management_context(
                        ctx.dir.tree(),
                        body_expression_id,
                    );
            let mut possible_return_values = Vec::new();
            collect_possible_return_values(
                ctx.dir.tree(),
                body_expression_id,
                &mut possible_return_values,
            );
            for possible_return_value_id in possible_return_values {
                check_return_value_expression(
                    ctx,
                    meta,
                    promise_symbol,
                    configuration,
                    body_expression_id,
                    possible_return_value_id,
                    in_control_flow_sensitive_context,
                );
            }
        }
    }
}

/// One await expectation for a return context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AwaitExpectation {
    /// Require `await` for Promise like returns.
    Require,
    /// Do not enforce either await or no-await.
    DontCare,
    /// Forbid `await` for Promise like returns.
    Forbid,
}

/// One return-await policy configuration for error and ordinary contexts.
#[derive(Debug, Clone, Copy)]
struct ReturnAwaitConfiguration {
    /// Await expectation when explicit error handling context is active.
    error_handling_context: AwaitExpectation,
    /// Await expectation in ordinary contexts.
    ordinary_context: AwaitExpectation,
}

/// Resolve one return-await policy configuration from linter options.
fn return_await_configuration(mode: ReturnAwaitMode) -> ReturnAwaitConfiguration {
    match mode {
        ReturnAwaitMode::Always => ReturnAwaitConfiguration {
            error_handling_context: AwaitExpectation::Require,
            ordinary_context: AwaitExpectation::Require,
        },
        ReturnAwaitMode::ErrorHandlingCorrectnessOnly => ReturnAwaitConfiguration {
            error_handling_context: AwaitExpectation::Require,
            ordinary_context: AwaitExpectation::DontCare,
        },
        ReturnAwaitMode::InTryCatch => ReturnAwaitConfiguration {
            error_handling_context: AwaitExpectation::Require,
            ordinary_context: AwaitExpectation::Forbid,
        },
        ReturnAwaitMode::Never => ReturnAwaitConfiguration {
            error_handling_context: AwaitExpectation::Forbid,
            ordinary_context: AwaitExpectation::Forbid,
        },
    }
}

/// One certainty level for whether an expression is Promise-like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThenableCertainty {
    /// This expression is definitely Promise-like.
    Always,
    /// This expression may be Promise-like.
    Maybe,
    /// This expression is definitely not Promise-like.
    Never,
}

/// One return value shape classification.
#[derive(Debug, Clone, Copy)]
enum ReturnValueKind {
    /// The return value is explicitly awaited.
    Awaited {
        await_expression_id: dir::LocalNodeId<dir::Expression>,
        awaited_value_id: dir::LocalNodeId<dir::Expression>,
    },
    /// The return value is not explicitly awaited.
    Plain {
        value_id: dir::LocalNodeId<dir::Expression>,
    },
}

impl ReturnValueKind {
    /// Return the effective return value expression id.
    fn value_expression_id(self) -> dir::LocalNodeId<dir::Expression> {
        match self {
            ReturnValueKind::Awaited {
                awaited_value_id, ..
            } => awaited_value_id,
            ReturnValueKind::Plain { value_id } => value_id,
        }
    }
}

/// Classify one return value expression as awaited or plain.
fn classify_return_value(
    tree: &dir::Tree,
    value_expression_id: dir::LocalNodeId<dir::Expression>,
) -> ReturnValueKind {
    if let Some((await_expression_id, awaited_value_id)) =
        explicit_await_expression(tree, value_expression_id)
    {
        return ReturnValueKind::Awaited {
            await_expression_id,
            awaited_value_id,
        };
    }

    let value_id = expression_unwrap_parenthesized(tree, value_expression_id);
    ReturnValueKind::Plain { value_id }
}

/// Check one return value expression against return-await policy.
fn check_return_value_expression(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    promise_symbol: Option<dir::GlobalSymbolId>,
    configuration: ReturnAwaitConfiguration,
    report_node_id: dir::LocalNodeId<dir::Expression>,
    value_expression_id: dir::LocalNodeId<dir::Expression>,
    in_try_context: bool,
) {
    // honor per node severity before deeper analysis
    let severity = ctx.get_effective_severity(meta, report_node_id);
    if !severity.is_enabled() {
        return;
    }

    // classify the returned expression shape
    let returned_value_kind = classify_return_value(ctx.dir.tree(), value_expression_id);
    let return_value_id = returned_value_kind.value_expression_id();
    let thenable_certainty = expression_thenable_certainty(ctx, return_value_id, promise_symbol);

    // always disallow awaiting non-thenables
    if let ReturnValueKind::Awaited {
        await_expression_id,
        awaited_value_id,
    } = returned_value_kind
        && thenable_certainty == ThenableCertainty::Never
    {
        let mut diagnostic = LintReport::new(
            RETURN_AWAIT.id,
            RETURN_AWAIT.code,
            RETURN_AWAIT.category,
            severity,
            "returning an awaited value that is not a promise is not allowed",
            ctx.get_span(await_expression_id),
        )
        .label("remove await from this non-promise return value");
        if ctx.compute_fixes {
            let awaited_span = ctx.get_span(awaited_value_id);
            let awaited_text = ctx.get_span_text(awaited_span).to_string();
            let edits = ctx
                .edit_builder()
                .replace(ctx.get_span(await_expression_id), awaited_text)
                .into_edits();
            let fix = LintFix::suggestion("Remove await from non-promise return").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
        return;
    }

    // keep uncertain thenables and non-thenables out of await policy checks
    if thenable_certainty != ThenableCertainty::Always {
        return;
    }

    // resolve await expectation for the current context
    let expectation = if in_try_context {
        configuration.error_handling_context
    } else {
        configuration.ordinary_context
    };

    // report missing await where policy requires it
    if expectation == AwaitExpectation::Require
        && let ReturnValueKind::Plain { value_id } = returned_value_kind
    {
        let return_value_span = ctx.get_span(value_id);
        let mut diagnostic = LintReport::new(
            RETURN_AWAIT.id,
            RETURN_AWAIT.code,
            RETURN_AWAIT.category,
            severity,
            "returning an awaited promise is required in this context",
            return_value_span,
        )
        .label("add await so this return follows configured await policy");
        if ctx.compute_fixes {
            let return_value_text = ctx.get_span_text(return_value_span).to_string();
            let replacement = format!("await ({return_value_text})");
            let edits = ctx
                .edit_builder()
                .replace(return_value_span, replacement)
                .into_edits();
            let fix = LintFix::suggestion("Add await to returned promise").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
        return;
    }

    // report redundant await where policy forbids it
    if expectation == AwaitExpectation::Forbid
        && let ReturnValueKind::Awaited {
            await_expression_id,
            awaited_value_id,
        } = returned_value_kind
    {
        let await_span = ctx.get_span(await_expression_id);
        let mut diagnostic = LintReport::new(
            RETURN_AWAIT.id,
            RETURN_AWAIT.code,
            RETURN_AWAIT.category,
            severity,
            "returning an awaited promise is not allowed in this context",
            await_span,
        )
        .label("remove await so this return follows configured await policy");
        if ctx.compute_fixes {
            let awaited_span = ctx.get_span(awaited_value_id);
            let awaited_text = ctx.get_span_text(awaited_span).to_string();
            let edits = ctx
                .edit_builder()
                .replace(await_span, awaited_text)
                .into_edits();
            let fix = LintFix::suggestion("Remove redundant await in return").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// Collect all branch expressions that may be returned by one expression.
fn collect_possible_return_values(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    values: &mut Vec<dir::LocalNodeId<dir::Expression>>,
) {
    // normalize one parenthesized wrapper layer
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);

    // split conditional expression branches
    if let dir::Expression::If {
        then_expression,
        else_expression: Some(else_expression_id),
        ..
    } = expression
    {
        collect_possible_return_values(tree, *then_expression, values);
        collect_possible_return_values(tree, *else_expression_id, values);
        return;
    }

    values.push(expression_id);
}

/// Collect concise async callable bodies that behave as implicit returns.
fn async_callable_expression_bodies(tree: &dir::Tree) -> Vec<dir::LocalNodeId<dir::Expression>> {
    let mut bodies = Vec::new();

    // collect function declaration expression bodies
    for declaration_id in tree.iter_node_ids_of_type::<dir::Declaration>() {
        let declaration = tree.get(declaration_id);
        let dir::Declaration::Function(declaration) = declaration else {
            continue;
        };
        let Some(body_id) = declaration.body else {
            continue;
        };
        if declaration.signature.asynchrony != dir::Asynchrony::Async {
            continue;
        }
        if matches!(tree.get(body_id), dir::Expression::Block(..)) {
            continue;
        }

        bodies.push(body_id);
    }

    // collect member method expression bodies
    for member_id in tree.iter_node_ids_of_type::<dir::Member>() {
        let member = tree.get(member_id);
        let dir::Member::Method {
            signature,
            body: Some(body_id),
            ..
        } = member
        else {
            continue;
        };
        if signature.asynchrony != dir::Asynchrony::Async {
            continue;
        }
        if matches!(tree.get(*body_id), dir::Expression::Block(..)) {
            continue;
        }

        bodies.push(*body_id);
    }

    // collect object method expression bodies
    for property_id in tree.iter_node_ids_of_type::<dir::Property>() {
        let property = tree.get(property_id);
        let dir::Property::Method {
            signature,
            body: Some(body_id),
            ..
        } = property
        else {
            continue;
        };
        if signature.asynchrony != dir::Asynchrony::Async {
            continue;
        }
        if matches!(tree.get(*body_id), dir::Expression::Block(..)) {
            continue;
        }

        bodies.push(*body_id);
    }

    bodies
}

/// Return thenable certainty for one expression.
fn expression_thenable_certainty(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    promise_symbol: Option<dir::GlobalSymbolId>,
) -> ThenableCertainty {
    // normalize wrappers once for type checks
    let expression_id = expression_unwrap_parenthesized(ctx.dir.tree(), expression_id);

    // keep known promise-like expressions at highest certainty
    if promise_symbol.is_some_and(|promise_symbol| {
        expression_is_promise_like(
            ctx.module_id(),
            ctx.dir.tree(),
            ctx.types,
            promise_symbol,
            expression_id,
        )
    }) {
        return ThenableCertainty::Always;
    }

    // keep unresolved and any-typed values as maybe
    let has_type = expression_type_or_call_return_type_map(
        ctx.artifacts.as_ref(),
        ctx.profile_id,
        ctx.module_id(),
        ctx.dir.tree(),
        ctx.types,
        expression_id,
        |_types, _type_id| true,
    )
    .unwrap_or(false);
    if !has_type {
        return ThenableCertainty::Maybe;
    }
    if expression_is_any_typed(
        ctx.module_id(),
        ctx.dir.tree(),
        &ctx.symbols,
        ctx.types,
        expression_id,
    ) {
        return ThenableCertainty::Maybe;
    }

    ThenableCertainty::Never
}

/// Return the explicit await expression and its awaited value when present.
fn explicit_await_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(
    dir::LocalNodeId<dir::Expression>,
    dir::LocalNodeId<dir::Expression>,
)> {
    let expression = tree.get(expression_id);

    // direct await expression
    if let dir::Expression::Await { expression } = expression {
        return Some((expression_id, *expression));
    }

    // recurse through parenthesized wrappers
    let dir::Expression::Parenthesized { expression } = expression else {
        return None;
    };

    explicit_await_expression(tree, *expression)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag redundant return-await in async functions.
    #[test]
    fn test_flags_redundant_return_await() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_redundant_return_await.ds",
            r#"
async function load(): Promise<int32> {
    return await fetchValue();
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Safely remove redundant return-await.
    #[test]
    fn test_fix_redundant_return_await() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_fix_redundant_return_await.ds",
            r#"
async function load(): Promise<int32> {
    return await fetchValue();
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("return-await")
            .assert_has_fix("return-await")
            .assert_suggested_fixed(
                r#"
async function load(): Promise<int32> {
    return fetchValue();
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
            );
    }

    /// Allow return-await inside try contexts.
    #[test]
    fn test_allows_return_await_inside_try() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_allows_return_await_inside_try.ds",
            r#"
async function load(): Promise<int32> {
    try {
        return await fetchValue();
    } catch (error) {
        return 0;
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Ignore synchronous functions.
    #[test]
    fn test_ignores_sync_function() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_ignores_sync_function.ds",
            r#"
function load(): int32 {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Flag redundant return-await in async methods.
    #[test]
    fn test_flags_redundant_return_await_in_async_method() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_redundant_return_await_in_async_method.ds",
            r#"
class Loader {
    async load(): Promise<int32> {
        return await fetchValue();
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Flag return-await in finally blocks in default mode.
    #[test]
    fn test_flags_return_await_inside_finally_in_default_mode() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_return_await_inside_finally_in_default_mode.ds",
            r#"
async function load(): Promise<int32> {
    try {
        return 0;
    } finally {
        return await fetchValue();
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Fix redundant return-await when wrapped in parentheses.
    #[test]
    fn test_fix_parenthesized_return_await() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_fix_parenthesized_return_await.ds",
            r#"
async function load(): Promise<int32> {
    return (await fetchValue());
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("return-await")
            .assert_has_fix("return-await")
            .assert_suggested_fixed(
                r#"
async function load(): Promise<int32> {
    return (fetchValue());
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
            );
    }

    /// Keep try-context semantics scoped to each callable boundary.
    #[test]
    fn test_flags_nested_async_return_await_inside_outer_try() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_nested_async_return_await_inside_outer_try.ds",
            r#"
async function outer(): Promise<int32> {
    try {
        async function inner(): Promise<int32> {
            return await fetchValue();
        }

        return await inner();
    } catch (error) {
        return 0;
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("return-await")
            .assert_lint_count("return-await", 1);
    }

    /// Flag redundant return-await in async object methods.
    #[test]
    fn test_flags_redundant_return_await_in_async_object_method() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_redundant_return_await_in_async_object_method.ds",
            r#"
let loader = {
    async load(): Promise<int32> {
        return await fetchValue();
    }
};

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Keep return-await in try blocks for async object methods.
    #[test]
    fn test_allows_return_await_in_async_object_method_try_block() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_allows_return_await_in_async_object_method_try_block.ds",
            r#"
let loader = {
    async load(): Promise<int32> {
        try {
            return await fetchValue();
        } catch (error) {
            return 0;
        }
    }
};

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Require await for Promise returns inside try blocks.
    #[test]
    fn test_requires_await_for_promise_return_inside_try() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_requires_await_for_promise_return_inside_try.ds",
            r#"
async function load(): Promise<int32> {
    try {
        return fetchValue();
    } catch (error) {
        return 0;
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Flag await on non-promise return values.
    #[test]
    fn test_flags_return_await_for_non_promise_value() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_return_await_for_non_promise_value.ds",
            r#"
async function load(): Promise<int32> {
    return await 1;
}
"#,
        );
        test.result(result)
            .assert_lint("return-await")
            .assert_suggested_fixed(
                r#"
async function load(): Promise<int32> {
    return 1;
}
"#,
            );
    }

    /// Allow bare non-promise returns in try blocks.
    #[test]
    fn test_allows_non_promise_return_inside_try_without_await() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_allows_non_promise_return_inside_try_without_await.ds",
            r#"
async function load(): Promise<int32> {
    try {
        return 1;
    } catch (error) {
        return 0;
    }
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Allow Promise returns in catch blocks without finally in default mode.
    #[test]
    fn test_allows_plain_promise_return_in_catch_without_finally() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_allows_plain_promise_return_in_catch_without_finally.ds",
            r#"
async function load(): Promise<int32> {
    try {
        throw 1;
    } catch (error) {
        return fetchValue();
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Require await in catch blocks when finally is present.
    #[test]
    fn test_requires_await_in_catch_with_finally() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_requires_await_in_catch_with_finally.ds",
            r#"
async function load(): Promise<int32> {
    try {
        throw 1;
    } catch (error) {
        return fetchValue();
    } finally {
        return 0;
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Require await in ordinary contexts when mode is always.
    #[test]
    fn test_always_mode_requires_await_in_ordinary_context() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait).with_options(|options| {
            options.correctness.return_await_mode = ReturnAwaitMode::Always;
        });
        let result = test.lint_dir(
            "return_await/test_always_mode_requires_await_in_ordinary_context.ds",
            r#"
async function load(): Promise<int32> {
    return fetchValue();
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Forbid await in all contexts when mode is never.
    #[test]
    fn test_never_mode_forbids_await_in_try_context() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait).with_options(|options| {
            options.correctness.return_await_mode = ReturnAwaitMode::Never;
        });
        let result = test.lint_dir(
            "return_await/test_never_mode_forbids_await_in_try_context.ds",
            r#"
async function load(): Promise<int32> {
    try {
        return await fetchValue();
    } catch (error) {
        return 0;
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Allow both awaited and plain Promise returns outside error contexts in correctness mode.
    #[test]
    fn test_correctness_mode_allows_both_return_forms_outside_try() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait).with_options(|options| {
            options.correctness.return_await_mode = ReturnAwaitMode::ErrorHandlingCorrectnessOnly;
        });
        let result = test.lint_dir(
            "return_await/test_correctness_mode_allows_both_return_forms_outside_try.ds",
            r#"
async function one(): Promise<int32> {
    return await fetchValue();
}

async function two(): Promise<int32> {
    return fetchValue();
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Flag awaited concise bodies in default mode.
    #[test]
    fn test_flags_awaited_async_concise_body_in_default_mode() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_awaited_async_concise_body_in_default_mode.ds",
            r#"
const load = async (): Promise<int32> => await fetchValue();

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Require await in concise async bodies when mode is always.
    #[test]
    fn test_always_mode_requires_await_in_async_concise_body() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait).with_options(|options| {
            options.correctness.return_await_mode = ReturnAwaitMode::Always;
        });
        let result = test.lint_dir(
            "return_await/test_always_mode_requires_await_in_async_concise_body.ds",
            r#"
const load = async (): Promise<int32> => fetchValue();

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Allow return await when explicit resource management is active.
    #[test]
    fn test_allows_return_await_with_prior_using_declaration() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_allows_return_await_with_prior_using_declaration.ds",
            r#"
async function load(): Promise<int32> {
    using resource = makeResource();
    return await fetchValue();
}

function makeResource(): int32 {
    return 1;
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Require await for promise returns when explicit resource management is active.
    #[test]
    fn test_requires_await_with_prior_using_declaration() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_requires_await_with_prior_using_declaration.ds",
            r#"
async function load(): Promise<int32> {
    using resource = makeResource();
    return fetchValue();
}

function makeResource(): int32 {
    return 1;
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }
}
