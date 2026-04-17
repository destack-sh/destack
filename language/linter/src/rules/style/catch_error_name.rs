use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    PromiseCallbackArity, expression_unwrap_transparent, fresh_name_in_symbol_scope_for_rename,
    local_symbol_has_direct_references, parameter_binding_name_and_symbol,
    promise_rejection_callback, rename_local_symbol_fix, symbol_primary_declaration_for,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce a specific name for caught errors.
    ///
    /// Consistent naming of caught errors improves code readability.
    /// Configure the expected name via `catch_error_name` option.
    #[lint(
        id = "catch-error-name",
        code = "LY001",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub CatchErrorName,
    "Enforce consistent catch error naming"
}

impl LintRule for CatchErrorName {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        CatchErrorName::meta()
    }

    /// Check module DIR expressions for catch binding names.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let expected_name = &ctx.options.style.catch_error_name;
        let catch_name = ctx.repository.strings.intern("catch");
        let then_name = ctx.repository.strings.intern("then");

        // inspect expressions for catch bindings and promise rejection callbacks
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);

            // check try catch binding names
            if let dir::Expression::Try {
                try_expression: _,
                catch_pattern: _,
                catch_ty: _,
                catch_expression: _,
                finally_expression: _,
                scope: _,
                symbol: _,
            } = expression
            {
                report_try_catch_binding(ctx, meta, expression, expected_name);
                continue;
            }

            // check promise callback parameter names
            if let dir::Expression::Call {
                left: _,
                generic_arguments: _,
                arguments: _,
            } = expression
            {
                report_promise_rejection_callback(
                    ctx,
                    meta,
                    expression,
                    expected_name,
                    catch_name,
                    then_name,
                );
                continue;
            }
        }
    }
}

/// Report one lint when one try catch binding name does not match the configured name.
fn report_try_catch_binding(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    expression: &dir::Expression,
    expected_name: &str,
) {
    // keep try expressions with one catch binding
    let dir::Expression::Try {
        try_expression: _,
        catch_pattern: Some(pattern_id),
        catch_ty: _,
        catch_expression: _,
        finally_expression: _,
        scope: _,
        symbol: _,
    } = expression
    else {
        return;
    };

    // keep simple catch binding patterns
    let pattern = ctx.tree.get(*pattern_id);
    let dir::Pattern::Binding {
        name: actual_name_id,
        symbol,
        mutability: _,
        pattern: _,
    } = pattern
    else {
        return;
    };
    let actual_name = ctx.repository.strings.get(*actual_name_id).to_string();
    if name_matches_expected(&actual_name, expected_name)
        || name_is_unused_placeholder(ctx, *symbol, &actual_name)
    {
        return;
    }

    // resolve effective severity and skip disabled diagnostics
    let severity = ctx.get_effective_severity(meta, *pattern_id);
    if !severity.is_enabled() {
        return;
    }

    // report the mismatch and offer one symbol-aware rename fix
    let mut diagnostic = LintDiagnostic::new(
        CATCH_ERROR_NAME.id,
        CATCH_ERROR_NAME.code,
        CATCH_ERROR_NAME.category,
        severity,
        format!("catch error should be named `{expected_name}`, not `{actual_name}`"),
        ctx.module.file_id,
        ctx.get_span(*pattern_id),
    )
    .with_label("rename this catch binding to the configured name");
    if ctx.include_fixes
        && let Some(replacement_name) =
            fresh_name_in_symbol_scope_for_rename(ctx, *symbol, expected_name)
        && let Some(fix) = rename_local_symbol_fix(
            ctx,
            *symbol,
            &replacement_name,
            &format!("Rename catch binding `{actual_name}` to `{replacement_name}`"),
        )
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Report one lint when one promise rejection callback parameter name does not match.
fn report_promise_rejection_callback(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    expression: &dir::Expression,
    expected_name: &str,
    catch_name: dir::StringId,
    then_name: dir::StringId,
) {
    // keep call expressions only
    let dir::Expression::Call {
        left,
        generic_arguments: _,
        arguments,
    } = expression
    else {
        return;
    };

    // keep strict promise catch and then callback arities
    let Some(callback) = promise_rejection_callback(
        ctx.tree,
        *left,
        arguments.len(),
        catch_name,
        then_name,
        PromiseCallbackArity::Exact,
    ) else {
        return;
    };

    // keep callback arguments with one named first parameter
    let callback_argument_id = arguments[callback.callback_argument_index];
    let Some((parameter_id, actual_name_id, symbol_id)) =
        callback_parameter_binding_from_argument(ctx, callback_argument_id)
    else {
        return;
    };
    let actual_name = ctx.repository.strings.get(actual_name_id).to_string();
    if name_matches_expected(&actual_name, expected_name)
        || name_is_unused_placeholder(ctx, symbol_id, &actual_name)
    {
        return;
    }

    // resolve effective severity and skip disabled diagnostics
    let severity = ctx.get_effective_severity(meta, parameter_id);
    if !severity.is_enabled() {
        return;
    }

    // report the mismatch and offer one symbol-aware rename fix
    let mut diagnostic = LintDiagnostic::new(
        CATCH_ERROR_NAME.id,
        CATCH_ERROR_NAME.code,
        CATCH_ERROR_NAME.category,
        severity,
        format!(
            "promise rejection parameter should be named `{expected_name}`, not `{actual_name}`"
        ),
        ctx.module.file_id,
        ctx.get_span(parameter_id),
    )
    .with_label("rename this callback parameter to the configured name");
    if ctx.include_fixes
        && let Some(replacement_name) =
            fresh_name_in_symbol_scope_for_rename(ctx, symbol_id, expected_name)
        && let Some(fix) = rename_local_symbol_fix(
            ctx,
            symbol_id,
            &replacement_name,
            &format!("Rename callback parameter `{actual_name}` to `{replacement_name}`"),
        )
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Resolve one callback parameter binding from one call argument.
fn callback_parameter_binding_from_argument(
    ctx: &LintModuleDirContext<'_>,
    argument_id: dir::LocalNodeId<dir::Argument>,
) -> Option<(
    dir::LocalNodeId<dir::Parameter>,
    dir::StringId,
    dir::LocalSymbolId,
)> {
    // resolve the callback argument expression
    let argument = ctx.tree.get(argument_id);
    let callback_expression_id = argument.value();

    callback_parameter_binding(ctx, callback_expression_id)
}

/// Resolve one callback first parameter binding from one callback expression.
fn callback_parameter_binding(
    ctx: &LintModuleDirContext<'_>,
    callback_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(
    dir::LocalNodeId<dir::Parameter>,
    dir::StringId,
    dir::LocalSymbolId,
)> {
    // keep callback declarations that resolve in the current module
    let declaration_id = callback_primary_declaration(ctx, callback_expression_id)?;
    if declaration_id.module_id != ctx.module_id() {
        return None;
    }

    // keep callbacks with one first parameter binding
    let parameter_id = first_callback_parameter_in_declaration(ctx.tree, declaration_id.local_id)?;
    let (name_id, symbol_id) = parameter_binding_name_and_symbol(ctx.tree, parameter_id)?;

    Some((parameter_id, name_id, symbol_id))
}

/// Resolve one callback primary declaration from one callback expression.
fn callback_primary_declaration(
    ctx: &LintModuleDirContext<'_>,
    callback_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalNodeIdAny> {
    // keep inline callback declarations first
    let callback_expression_id = expression_unwrap_transparent(ctx.tree, callback_expression_id);
    let callback_expression = ctx.tree.get(callback_expression_id);
    if let dir::Expression::Declaration(declaration) = callback_expression {
        return Some((*declaration).into_global_any(ctx.module_id()));
    }

    // then resolve referenced callback declarations
    let target_symbol = callback_expression.target_symbol()?;
    symbol_primary_declaration_for(
        &ctx.repository,
        ctx.revision,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        target_symbol,
    )
}

/// Resolve one first callback parameter from one declaration node.
fn first_callback_parameter_in_declaration(
    tree: &dir::NodeTree,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<dir::LocalNodeId<dir::Parameter>> {
    // keep function declarations
    if declaration_id.ty == dir::NodeType::Declaration {
        let declaration = tree.get(declaration_id.into_typed::<dir::Declaration>());
        let dir::Declaration::Function(declaration) = declaration else {
            return None;
        };

        return declaration.signature.parameters.first().copied();
    }

    // keep method declarations
    if declaration_id.ty == dir::NodeType::Member {
        let member = tree.get(declaration_id.into_typed::<dir::Member>());
        let signature = member.signature()?;

        return signature.parameters.first().copied();
    }

    None
}

/// Return true when one underscore-prefixed name is unused in the current module.
fn name_is_unused_placeholder(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::LocalSymbolId,
    actual_name: &str,
) -> bool {
    if !actual_name.starts_with('_') {
        return false;
    }

    !local_symbol_has_direct_references(ctx.module_id(), ctx.tree, symbol_id)
}

/// Return true when one actual name matches configured catch naming conventions.
fn name_matches_expected(actual_name: &str, expected_name: &str) -> bool {
    // allow exact matches first
    if actual_name == expected_name {
        return true;
    }

    // normalize trailing underscores
    let actual_name = actual_name.trim_end_matches('_');
    if actual_name == expected_name {
        return true;
    }

    // allow suffix style names like `myError`
    if actual_name.ends_with(expected_name) {
        return true;
    }

    // allow suffix style names with leading uppercase like `myError` for `error`
    let mut expected_chars = expected_name.chars();
    let Some(first_expected) = expected_chars.next() else {
        return false;
    };
    let expected_uppercase =
        first_expected.to_uppercase().collect::<String>() + expected_chars.as_str();

    actual_name.ends_with(expected_uppercase.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag mismatched catch binding names.
    #[test]
    fn test_detects_wrong_error_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_detects_wrong_error_name.ds",
            r#"
try {
    doSomething()
} catch (e) {
    console.log(e)
}
"#,
        );
        test.result(result)
            .assert_lint("catch-error-name")
            .assert_has_fix("catch-error-name");
    }

    /// Allow configured catch binding names.
    #[test]
    fn test_allows_correct_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_allows_correct_name.ds",
            r#"
try {
    doSomething()
} catch (error) {
    console.log(error)
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    /// Allow try blocks without catch clauses.
    #[test]
    fn test_allows_try_without_catch() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_allows_try_without_catch.ds",
            r#"
try {
    doSomething()
} finally {
    cleanup()
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    /// Flag non-configured catch aliases.
    #[test]
    fn test_detects_err_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_detects_err_name.ds",
            r#"
try {
    fetch()
} catch (err) {
    console.error(err)
}
"#,
        );
        test.result(result).assert_lint("catch-error-name");
    }

    /// Rename catch binding references when no conflicts exist.
    #[test]
    fn test_fix_renames_catch_binding() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_fix_renames_catch_binding.ds",
            r#"
try {
    run()
} catch (err) {
    console.log(err)
}
"#,
        );
        test.result(result)
            .assert_lint("catch-error-name")
            .assert_unsafe_fixed(
                r#"
try {
    run()
} catch (error) {
    console.log(error)
}
"#,
            );
    }

    /// Rename catch binding with underscore suffix when configured name collides.
    #[test]
    fn test_fix_renames_catch_binding_with_collision_suffix() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_fix_renames_catch_binding_with_collision_suffix.ds",
            r#"
let error = 1;

try {
    run()
} catch (err) {
    console.log(err, error)
}
"#,
        );
        test.result(result)
            .assert_lint("catch-error-name")
            .assert_unsafe_fixed(
                r#"
let error = 1;

try {
    run()
} catch (error_) {
    console.log(error_, error)
}
"#,
            );
    }

    /// Allow destructured catch patterns.
    #[test]
    fn test_allows_non_binding_catch_pattern() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_allows_non_binding_catch_pattern.ds",
            r#"
try {
    run()
} catch ({ reason }) {
    console.log(reason)
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    /// Allow suffix naming variants.
    #[test]
    fn test_allows_catch_name_suffix_variants() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_allows_catch_name_suffix_variants.ds",
            r#"
try {
    run()
} catch (networkError_) {
    console.log(networkError_)
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    /// Allow unused underscore-prefixed catch bindings.
    #[test]
    fn test_allows_unused_underscore_prefixed_catch_binding() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_allows_unused_underscore_prefixed_catch_binding.ds",
            r#"
try {
    run()
} catch (_foo) {
    console.log("ignored")
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    /// Flag promise catch callback parameter names.
    #[test]
    fn test_detects_promise_catch_callback_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_detects_promise_catch_callback_name.ds",
            r#"
promise.catch((err) => {
    console.log(err)
})
"#,
        );
        test.result(result).assert_lint("catch-error-name");
    }

    /// Flag promise then rejection callback parameter names.
    #[test]
    fn test_detects_promise_then_rejection_callback_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_detects_promise_then_rejection_callback_name.ds",
            r#"
promise.then(
    (value) => value,
    (err) => console.log(err),
)
"#,
        );
        test.result(result).assert_lint("catch-error-name");
    }

    /// Rename promise callback parameter references when no conflicts exist.
    #[test]
    fn test_fix_renames_promise_callback_parameter() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_fix_renames_promise_callback_parameter.ds",
            r#"
promise.catch((err) => {
    console.log(err)
})
"#,
        );
        test.result(result)
            .assert_lint("catch-error-name")
            .assert_unsafe_fixed(
                r#"
promise.catch((error) => {
    console.log(error)
})
"#,
            );
    }

    /// Flag promise catch function callback parameter names.
    #[test]
    fn test_detects_promise_catch_function_callback_name() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_detects_promise_catch_function_callback_name.ds",
            r#"
promise.catch(function (err) {
    console.log(err)
})
"#,
        );
        test.result(result).assert_lint("catch-error-name");
    }

    /// Rename promise catch function callback parameter references when no conflicts exist.
    #[test]
    fn test_fix_renames_promise_catch_function_callback_parameter() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_fix_renames_promise_catch_function_callback_parameter.ds",
            r#"
promise.catch(function (err) {
    console.log(err)
})
"#,
        );
        test.result(result)
            .assert_lint("catch-error-name")
            .assert_unsafe_fixed(
                r#"
promise.catch(function (error) {
    console.log(error)
})
"#,
            );
    }

    /// Ignore promise rejection callbacks when the method arity exceeds the supported shape.
    #[test]
    fn test_ignores_promise_callbacks_with_extra_arguments() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_ignores_promise_callbacks_with_extra_arguments.ds",
            r#"
promise.catch((err) => {
    console.log(err)
}, extra)

promise.then(
    (value) => value,
    (err) => console.log(err),
    extra,
)
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    /// Allow unused underscore-prefixed promise callback parameters.
    #[test]
    fn test_allows_unused_underscore_prefixed_promise_callback_parameter() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_allows_unused_underscore_prefixed_promise_callback_parameter.ds",
            r#"
promise.catch((_foo) => {
    console.log("ignored")
})
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    /// Flag used underscore-prefixed promise callback parameters.
    #[test]
    fn test_detects_used_underscore_prefixed_promise_callback_parameter() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_detects_used_underscore_prefixed_promise_callback_parameter.ds",
            r#"
promise.catch((_foo) => {
    console.log(_foo)
})
"#,
        );
        test.result(result).assert_lint("catch-error-name");
    }

    /// Rename promise callback parameter with underscore suffix when configured name collides.
    #[test]
    fn test_fix_renames_promise_callback_parameter_with_collision_suffix() {
        let test = TestProgram::for_rule_without_prelude(CatchErrorName);
        let result = test.lint_dir(
            "catch_error_name/test_fix_renames_promise_callback_parameter_with_collision_suffix.ds",
            r#"
let error = 1;

promise.catch((err) => {
    console.log(err, error)
})
"#,
        );
        test.result(result)
            .assert_lint("catch-error-name")
            .assert_unsafe_fixed(
                r#"
let error = 1;

promise.catch((error_) => {
    console.log(error_, error)
})
"#,
            );
    }
}
