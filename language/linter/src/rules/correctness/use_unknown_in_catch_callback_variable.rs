use destack_ast as ast;
use destack_dir::{self as dir, WellKnownSymbol};
use destack_workspace::{LintSeverity, Module, ProfileId};

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_declared_or_inferred_type_id, function_parameter_types_at, is_explicit_any_type,
    is_promise_type, symbol_primary_declaration_for,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Require `unknown` instead of `any` for Promise catch callback variables.
    ///
    /// Catch callback parameters represent unknown error values at runtime.
    /// Using `unknown` preserves type safety and forces explicit narrowing.
    #[lint(
        id = "use-unknown-in-catch-callback-variable",
        code = "LC044",
        category = Correctness,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Promise)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Experimental,
        declarations = Exclude
    )]
    pub UseUnknownInCatchCallbackVariable,
    "Require `unknown` for Promise catch callback variables"
}

impl LintRule for UseUnknownInCatchCallbackVariable {
    fn meta(&self) -> &'static LintMeta {
        UseUnknownInCatchCallbackVariable::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let catch_name = ctx.program.strings.intern("catch");
        let promise_symbol = ctx
            .well_known_symbols()
            .get_type_symbol(WellKnownSymbol::Promise)
            .unwrap_or_else(|| ctx.well_known_symbol(WellKnownSymbol::Promise));

        // inspect call expressions
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let dir::Expression::Call {
                left,
                dynamic_arguments,
                ..
            } = expression
            else {
                continue;
            };
            if dynamic_arguments.is_empty() {
                continue;
            }

            let Some(catch_receiver) = promise_catch_receiver_expression(ctx, *left, catch_name)
            else {
                continue;
            };
            let Some(receiver_type_id) = expression_declared_or_inferred_type_id(
                ctx.module_id(),
                ctx.tree,
                ctx.types,
                catch_receiver,
            )
            .or_else(|| ctx.expression_type_id(catch_receiver)) else {
                continue;
            };
            if !is_promise_type(ctx.types, receiver_type_id, Some(promise_symbol)) {
                continue;
            }

            let callback_argument = ctx.tree.get(dynamic_arguments[0]).value();
            let callback_parameter_id = first_callback_parameter(ctx, callback_argument);
            if !catch_callback_uses_any_parameter(ctx, callback_argument, callback_parameter_id) {
                continue;
            }

            let diagnostic_span = callback_parameter_id
                .map(|parameter_id| ctx.get_span(parameter_id))
                .unwrap_or_else(|| ctx.get_span(callback_argument));
            let severity = if let Some(parameter_id) = callback_parameter_id {
                ctx.get_effective_severity(meta, parameter_id)
            } else {
                ctx.get_effective_severity(meta, expression_id)
            };
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                USE_UNKNOWN_IN_CATCH_CALLBACK_VARIABLE.id,
                USE_UNKNOWN_IN_CATCH_CALLBACK_VARIABLE.code,
                USE_UNKNOWN_IN_CATCH_CALLBACK_VARIABLE.category,
                severity,
                "catch callback parameter should be unknown",
                ctx.module.file_id,
                diagnostic_span,
            )
            .with_label("use `unknown` instead of `any` for catch callback parameters");

            // compute fixes only when requested by the runner
            if ctx.include_fixes
                && let Some(parameter_id) = callback_parameter_id
                && let Some(fix) = catch_callback_unknown_fix(ctx, parameter_id)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a safe fix for one explicit `any` catch callback parameter.
fn catch_callback_unknown_fix(
    ctx: &LintModuleDirContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<LintFix> {
    let type_expression_id =
        ast_parameter_type_expression_id(ctx.module, ctx.profile_id, parameter_id)?;
    if !ast_type_expression_is_explicit_any(ctx.ast, type_expression_id) {
        return None;
    }

    let type_span = ctx.ast.get_span(type_expression_id);
    let edits = ctx
        .edit_builder()
        .replace(type_span, "unknown")
        .into_edits();
    Some(LintFix::safe("Replace `any` with `unknown`").with_edits(edits))
}

/// Resolve the receiver expression for `promise.catch(...)`.
fn promise_catch_receiver_expression(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    catch_name: dir::StringId,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let expression = ctx.tree.get(expression_id);
    let dir::Expression::Member { left, name, .. } = expression else {
        return None;
    };
    (*name == catch_name).then_some(*left)
}

/// Return true when one catch callback takes an `any` typed first parameter.
fn catch_callback_uses_any_parameter(
    ctx: &LintModuleDirContext<'_>,
    callback_expression_id: dir::LocalNodeId<dir::Expression>,
    callback_parameter_id: Option<dir::LocalNodeId<dir::Parameter>>,
) -> bool {
    // prefer direct parameter analysis for local callback declarations
    if let Some(parameter_id) = callback_parameter_id {
        return parameter_uses_explicit_any(
            ctx.module,
            ctx.profile_id,
            ctx.tree,
            ctx.types,
            ctx.module_id(),
            parameter_id,
        );
    }

    // inspect primary callback declarations across module boundaries
    if let Some(declaration_id) = callback_primary_declaration(ctx, callback_expression_id)
        && callback_declaration_uses_any_parameter(ctx, declaration_id)
    {
        return true;
    }

    // for unresolved callback declarations, rely on callback expression types
    callback_type_uses_any_parameter(ctx, callback_expression_id)
}

/// Return true when one callback expression type includes `any` at parameter index zero.
fn callback_type_uses_any_parameter(
    ctx: &LintModuleDirContext<'_>,
    callback_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(callback_type_id) = expression_declared_or_inferred_type_id(
        ctx.module_id(),
        ctx.tree,
        ctx.types,
        callback_expression_id,
    )
    .or_else(|| ctx.expression_type_id(callback_expression_id)) else {
        return false;
    };

    let parameter_types = function_parameter_types_at(ctx.types, callback_type_id, 0);
    if parameter_types.is_empty() {
        return false;
    }

    parameter_types
        .into_iter()
        .any(|parameter_type_id| is_explicit_any_type(ctx.types, parameter_type_id))
}

/// Resolve the primary declaration for one callback expression.
fn callback_primary_declaration(
    ctx: &LintModuleDirContext<'_>,
    callback_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalNodeIdAny> {
    let callback_expression = ctx.tree.get(callback_expression_id);
    if let dir::Expression::Declaration { declaration } = callback_expression {
        return Some((*declaration).into_global_any(ctx.module_id()));
    }

    let target_symbol = callback_expression.target_symbol()?;
    symbol_primary_declaration_for(
        &ctx.program,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        target_symbol,
    )
}

/// Return true when one callback declaration has an `any` typed first parameter.
fn callback_declaration_uses_any_parameter(
    ctx: &LintModuleDirContext<'_>,
    declaration_id: dir::GlobalNodeIdAny,
) -> bool {
    let module = ctx.program.modules.get(declaration_id.module_id);
    let module = module.read();
    let Some(module_dir) = module.dir_maybe(ctx.profile_id) else {
        return false;
    };

    let tree = module_dir.tree.read();
    let Some(parameter_id) =
        first_callback_parameter_in_declaration(&tree, declaration_id.local_id)
    else {
        return false;
    };

    let types = module_dir.types.read();
    parameter_uses_explicit_any(
        &module,
        ctx.profile_id,
        &tree,
        &types,
        declaration_id.module_id,
        parameter_id,
    )
}

/// Return the first callback parameter for one declaration node.
fn first_callback_parameter_in_declaration(
    tree: &dir::NodeTree,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<dir::LocalNodeId<dir::Parameter>> {
    if declaration_id.ty == dir::NodeType::Declaration {
        let declaration = tree.get(declaration_id.into_typed::<dir::Declaration>());
        let dir::Declaration::Function { signature, .. } = declaration else {
            return None;
        };
        return signature.dynamic_parameters.first().copied();
    }

    if declaration_id.ty == dir::NodeType::Member {
        let member = tree.get(declaration_id.into_typed::<dir::Member>());
        let dir::Member::Method { signature, .. } = member else {
            return None;
        };
        return signature.dynamic_parameters.first().copied();
    }

    None
}

/// Return the first callback parameter for one callback expression.
fn first_callback_parameter(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Parameter>> {
    let declaration_id = callback_primary_declaration(ctx, expression_id)?;
    if declaration_id.module_id != ctx.module_id() {
        return None;
    }

    first_callback_parameter_in_declaration(ctx.tree, declaration_id.local_id)
}

/// Return true when one callback parameter is typed as explicit `any`.
fn parameter_uses_explicit_any(
    module: &Module,
    profile_id: ProfileId,
    tree: &dir::NodeTree,
    types: &dir::TypeTable,
    module_id: destack_source::ModuleId,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> bool {
    // source declared `any` should be detected directly from AST structure
    if parameter_declares_explicit_any_in_ast(module, profile_id, parameter_id) {
        return true;
    }

    let global_parameter_id = parameter_id.into_global_any(module_id);

    // declared type takes precedence when available
    if let Some(declared_type_id) = types.get_declared_type_id(global_parameter_id) {
        return is_explicit_any_type(types, declared_type_id);
    }

    let parameter = tree.get(parameter_id);
    let symbol_id = parameter.symbol().into_global(module_id);
    let Some(value_type_id) = types.get_value_type_id(symbol_id) else {
        return false;
    };
    is_explicit_any_type(types, value_type_id)
}

/// Return true when one parameter declaration is explicitly `any` in source AST.
fn parameter_declares_explicit_any_in_ast(
    module: &Module,
    profile_id: ProfileId,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> bool {
    let Some(type_expression_id) =
        ast_parameter_type_expression_id(module, profile_id, parameter_id)
    else {
        return false;
    };

    let ast = module.ast();
    ast_type_expression_is_explicit_any(&ast.tree, type_expression_id)
}

/// Resolve the AST type expression for one DIR parameter.
fn ast_parameter_type_expression_id(
    module: &Module,
    profile_id: ProfileId,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let module_dir = module.dir_maybe(profile_id)?;
    let source_id = {
        let tree = module_dir.tree.read();
        tree.get_source(parameter_id.id)
    };

    let ast = module.ast_maybe()?;
    if ast.tree.get_node_type(source_id) != ast::NodeType::Parameter {
        return None;
    }

    let ast_parameter_id = ast::LocalNodeId::<ast::Parameter>::new(source_id);
    let ast_parameter = ast.tree.get(ast_parameter_id);
    match ast_parameter {
        ast::Parameter::Named { ty, .. }
        | ast::Parameter::Pattern { ty, .. }
        | ast::Parameter::VariadicNamed { ty, .. }
        | ast::Parameter::VariadicPattern { ty, .. } => *ty,
    }
}

/// Return true when one AST type expression is an explicit `any`.
fn ast_type_expression_is_explicit_any(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = tree.get(expression_id);
    match expression {
        ast::Expression::TypeLiteral(ast::TypeLiteral::Any) => true,
        ast::Expression::Parenthesized { expression } => {
            ast_type_expression_is_explicit_any(tree, *expression)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Flag `any` catch callback parameters on Promise chains.
    #[test]
    fn test_flags_any_catch_callback_parameter() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_flags_any_catch_callback_parameter.ds",
            r#"
Promise.reject("failed").catch((error: any) => {
    return error;
});
"#,
        );
        test.result(result)
            .assert_lint("use-unknown-in-catch-callback-variable")
            .assert_has_fix("use-unknown-in-catch-callback-variable");
    }

    /// Allow unknown-typed catch callback parameters.
    #[test]
    fn test_allows_unknown_catch_callback_parameter() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_allows_unknown_catch_callback_parameter.ds",
            r#"
Promise.reject("failed").catch((error: unknown) => {
    return error;
});
"#,
        );
        test.result(result)
            .assert_no_lint("use-unknown-in-catch-callback-variable");
    }

    /// Ignore non-Promise catch methods.
    #[test]
    fn test_ignores_non_promise_catch_method() {
        let test = TestProgram::for_rule_without_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_ignores_non_promise_catch_method.ds",
            r#"
class Stream {
    catch(handler: (error: any) => void): void {
        handler("failed");
    }
}

const stream = new Stream();
stream.catch((error: any) => {
    return error;
});
"#,
        );
        test.result(result)
            .assert_no_lint("use-unknown-in-catch-callback-variable");
    }

    /// Ignore Promise catch callbacks without `any`.
    #[test]
    fn test_ignores_untyped_catch_callback_parameter() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_ignores_untyped_catch_callback_parameter.ds",
            r#"
Promise.reject("failed").catch((error) => {
    return error;
});
"#,
        );
        test.result(result)
            .assert_no_lint("use-unknown-in-catch-callback-variable");
    }

    /// Flag named callback references with explicit any.
    #[test]
    fn test_flags_named_catch_callback_with_any_parameter() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_flags_named_catch_callback_with_any_parameter.ds",
            r#"
function onError(error: any) {
    return error;
}

Promise.reject("failed").catch(onError);
"#,
        );
        test.result(result)
            .assert_lint("use-unknown-in-catch-callback-variable");
    }

    /// Allow named callback references with unknown.
    #[test]
    fn test_allows_named_catch_callback_with_unknown_parameter() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_allows_named_catch_callback_with_unknown_parameter.ds",
            r#"
function onError(error: unknown) {
    return error;
}

Promise.reject("failed").catch(onError);
"#,
        );
        test.result(result)
            .assert_no_lint("use-unknown-in-catch-callback-variable");
    }

    /// Flag callback references imported from another module.
    #[test]
    fn test_flags_cross_module_named_callback_with_any_parameter() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_module_dir_with_modules(
            test_modules! {
                "use_unknown_in_catch_callback_variable/cross_module_handler.ds" => r#"
export function onError(error: any) {
    return error;
}
"#,
                "use_unknown_in_catch_callback_variable/cross_module_main.ds" => r#"
import { onError } from "./cross_module_handler.ds";

Promise.reject("failed").catch(onError);
"#,
            },
            "use_unknown_in_catch_callback_variable/cross_module_main.ds",
        );

        test.result(result)
            .assert_lint("use-unknown-in-catch-callback-variable");
    }

    /// Allow callback references imported from another module when unknown is used.
    #[test]
    fn test_allows_cross_module_named_callback_with_unknown_parameter() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_module_dir_with_modules(
            test_modules! {
                "use_unknown_in_catch_callback_variable/cross_module_unknown_handler.ds" => r#"
export function onError(error: unknown) {
    return error;
}
"#,
                "use_unknown_in_catch_callback_variable/cross_module_unknown_main.ds" => r#"
import { onError } from "./cross_module_unknown_handler.ds";

Promise.reject("failed").catch(onError);
"#,
            },
            "use_unknown_in_catch_callback_variable/cross_module_unknown_main.ds",
        );

        test.result(result)
            .assert_no_lint("use-unknown-in-catch-callback-variable");
    }

    /// Ignore callback aliases that do not expose parameter annotations at the use site.
    #[test]
    fn test_ignores_alias_typed_callback_parameter_any() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_ignores_alias_typed_callback_parameter_any.ds",
            r#"
type ErrorHandler = (error: any) => void;

const onError: ErrorHandler = (error) => {
    return error;
};

Promise.reject("failed").catch(onError);
"#,
        );
        test.result(result)
            .assert_no_lint("use-unknown-in-catch-callback-variable");
    }

    /// Ignore non-callable catch arguments.
    #[test]
    fn test_ignores_non_callable_catch_argument() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_ignores_non_callable_catch_argument.ds",
            r#"
const notAHandler = 1;
Promise.reject("failed").catch(notAHandler as any);
"#,
        );
        test.result(result)
            .assert_no_lint("use-unknown-in-catch-callback-variable");
    }

    /// Safely rewrite inline catch callback `any` annotation.
    #[test]
    fn test_fix_inline_catch_callback_parameter_any() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_fix_inline_catch_callback_parameter_any.ds",
            r#"
Promise.reject("failed").catch((error: any) => {
    return error;
});
"#,
        );
        test.result(result)
            .assert_lint("use-unknown-in-catch-callback-variable")
            .assert_safe_fixed(
                r#"
Promise.reject("failed")
    .catch((error: unknown) => {
        return error;
    });
"#,
            );
    }

    /// Safely rewrite named catch callback parameter annotations.
    #[test]
    fn test_fix_named_catch_callback_parameter_any() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_fix_named_catch_callback_parameter_any.ds",
            r#"
function onError(error: any) {
    return error;
}

Promise.reject("failed").catch(onError);
"#,
        );
        test.result(result)
            .assert_lint("use-unknown-in-catch-callback-variable")
            .assert_safe_fixed(
                r#"
function onError(error: unknown) {
    return error;
}

Promise.reject("failed").catch(onError);
"#,
            );
    }

    /// Do not auto-fix cross-module callback declarations.
    #[test]
    fn test_no_fix_for_cross_module_callback_parameter() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_module_dir_with_modules(
            test_modules! {
                "use_unknown_in_catch_callback_variable/no_fix_cross_module_handler.ds" => r#"
export function onError(error: any) {
    return error;
}
"#,
                "use_unknown_in_catch_callback_variable/no_fix_cross_module_main.ds" => r#"
import { onError } from "./no_fix_cross_module_handler.ds";

Promise.reject("failed").catch(onError);
"#,
            },
            "use_unknown_in_catch_callback_variable/no_fix_cross_module_main.ds",
        );

        test.result(result)
            .assert_lint("use-unknown-in-catch-callback-variable")
            .assert_has_no_fix("use-unknown-in-catch-callback-variable");
    }

    /// Mutation: detect explicit any in inline function callbacks.
    #[test]
    fn test_mutation_flags_inline_function_callback_parameter_any() {
        let test = TestProgram::for_rule_with_prelude(UseUnknownInCatchCallbackVariable);
        let result = test.lint_dir(
            "use_unknown_in_catch_callback_variable/test_mutation_flags_inline_function_callback_parameter_any.ds",
            r#"
Promise.reject("failed").catch(function onError(error: any) {
    return error;
});
"#,
        );
        test.result(result)
            .assert_lint("use-unknown-in-catch-callback-variable")
            .assert_safe_fixed(
                r#"
Promise.reject("failed")
    .catch(function onError(error: unknown) {
        return error;
    });
"#,
            );
    }
}
