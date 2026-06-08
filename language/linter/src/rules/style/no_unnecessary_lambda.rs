use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::{
    block_single_return_value, expression_reference_path, expression_target_symbol,
    expression_unwrap_statement, expression_unwrap_transparent,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow lambdas that only wrap a direct function call.
    ///
    /// When a lambda only forwards its parameters to another callable without
    /// changing order or receiver binding, the lambda is unnecessary and the
    /// callee can be passed directly instead.
    #[lint(
        id = "no-unnecessary-lambda",
        code = "LY024",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUnnecessaryLambda,
    "Disallow lambdas that only wrap a direct function call"
}

impl LintRule for NoUnnecessaryLambda {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnnecessaryLambda::meta()
    }

    /// Check module DIR nodes for direct forwarding lambdas.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect lambda declaration expressions only
        for expression_id in ctx.dir.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.dir.get(expression_id);
            let dir::Expression::Declaration(declaration) = expression else {
                continue;
            };

            let declaration_id = *declaration;
            let declaration = ctx.dir.get(declaration_id);
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };

            // keep sync scalar lambdas only
            if declaration.signature.form != dir::FunctionForm::Lambda
                || declaration.signature.asynchrony != dir::Asynchrony::Sync
                || declaration.signature.is_generator
            {
                continue;
            }

            // keep lambdas with one plain named parameter list
            let Some(parameter_symbols) = lambda_parameter_symbols(ctx, &declaration.signature)
            else {
                continue;
            };
            if parameter_symbols.is_empty() {
                continue;
            }

            // keep lambda bodies that forward directly to one call
            let Some(call_expression_id) = lambda_forwarded_call_body(ctx, declaration.body) else {
                continue;
            };
            let call_expression = ctx.dir.get(call_expression_id);
            let dir::Expression::Call {
                position: _,
                left,
                generic_arguments,
                arguments,
            } = call_expression
            else {
                continue;
            };
            if !generic_arguments.is_empty() || arguments.len() != parameter_symbols.len() {
                continue;
            }

            // keep direct callee references without receiver binding
            if !callee_is_direct_function_reference(ctx, *left, &parameter_symbols) {
                continue;
            }

            // keep one to one positional forwarding only
            if !arguments_forward_parameters(ctx, arguments, &parameter_symbols) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let declaration_span = ctx.get_span(declaration_id);
            let callee_span = ctx.get_span(*left);
            let callee_text = ctx.get_span_text(callee_span).to_string();
            let fix = LintFix::safe("Replace with function reference")
                .replace(declaration_span, callee_text.clone());

            ctx.report(
                LintReport::new(
                    NO_UNNECESSARY_LAMBDA.id,
                    NO_UNNECESSARY_LAMBDA.code,
                    NO_UNNECESSARY_LAMBDA.category,
                    severity,
                    format!("unnecessary lambda wrapping `{callee_text}`"),
                    declaration_span,
                )
                .label(format!("use `{callee_text}` directly"))
                .fix(fix),
            );
        }
    }
}

/// Return plain named parameter symbols for one lambda signature.
fn lambda_parameter_symbols(
    ctx: &LintModuleContext<'_>,
    signature: &dir::FunctionSignature,
) -> Option<Vec<dir::LocalSymbolId>> {
    let mut symbols = Vec::with_capacity(signature.parameters.len());

    // keep plain named parameters without defaults
    for parameter_id in &signature.parameters {
        let parameter = ctx.dir.get(*parameter_id);
        let dir::Parameter::Named { default, .. } = parameter else {
            return None;
        };
        if default.is_some() {
            return None;
        }

        symbols.push(ctx.local_symbol_for_node(*parameter_id)?);
    }

    Some(symbols)
}

/// Return the forwarded call expression for one lambda body.
fn lambda_forwarded_call_body(
    ctx: &LintModuleContext<'_>,
    body_expression_id: Option<dir::LocalNodeId<dir::Expression>>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let body_expression_id = body_expression_id?;
    let body_expression_id = expression_unwrap_statement(ctx.dir.tree(), body_expression_id);
    let body_expression = ctx.dir.get(body_expression_id);

    // keep direct expression bodies first
    if matches!(body_expression, dir::Expression::Call { .. }) {
        return Some(body_expression_id);
    }

    // then allow single return blocks
    let dir::Expression::Block(block) = body_expression else {
        return None;
    };

    let returned_value_id = block_single_return_value(ctx.dir.tree(), *block)?;
    let returned_value_id = expression_unwrap_transparent(ctx.dir.tree(), returned_value_id);
    let returned_value = ctx.dir.get(returned_value_id);
    if !matches!(returned_value, dir::Expression::Call { .. }) {
        return None;
    }

    Some(returned_value_id)
}

/// Return true when one callee is a direct function reference.
fn callee_is_direct_function_reference(
    ctx: &LintModuleContext<'_>,
    callee_expression_id: dir::LocalNodeId<dir::Expression>,
    parameter_symbols: &[dir::LocalSymbolId],
) -> bool {
    let callee_expression_id = expression_unwrap_transparent(ctx.dir.tree(), callee_expression_id);

    // keep direct references only, not member access with receiver binding
    let Some(reference_path) = expression_reference_path(ctx, callee_expression_id) else {
        return false;
    };
    if !reference_path.members.is_empty() {
        return false;
    }

    let Some(callee_symbol) = expression_target_symbol(ctx, callee_expression_id) else {
        return false;
    };
    if parameter_symbols
        .iter()
        .any(|symbol_id| callee_symbol == symbol_id.into_global(ctx.module_id()))
    {
        return false;
    }

    true
}

/// Return true when call arguments forward the lambda parameters in order.
fn arguments_forward_parameters(
    ctx: &LintModuleContext<'_>,
    argument_ids: &[dir::LocalNodeId<dir::Argument>],
    parameter_symbols: &[dir::LocalSymbolId],
) -> bool {
    for (argument_id, parameter_symbol_id) in argument_ids.iter().zip(parameter_symbols) {
        let argument = ctx.dir.get(*argument_id);
        let dir::Argument::Positional { value, .. } = argument else {
            return false;
        };

        let value_expression_id = expression_unwrap_transparent(ctx.dir.tree(), *value);
        let value_symbol = expression_target_symbol(ctx, value_expression_id);
        if value_symbol != Some(parameter_symbol_id.into_global(ctx.module_id())) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag one single parameter forwarding lambda.
    #[test]
    fn test_detects_unnecessary_single_parameter_lambda() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryLambda);
        let diagnostics = test.lint_dir(
            "no_unnecessary_lambda/test_detects_unnecessary_single_parameter_lambda.ds",
            r#"
function foo(value) {
    return value;
}

items.map((value) => foo(value));
"#,
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-lambda")
            .assert_safe_fixed(
                r#"
function foo(value) {
    return value;
}

items.map(foo);
"#,
            );
    }

    /// Flag one multi parameter forwarding lambda.
    #[test]
    fn test_detects_unnecessary_multiple_parameter_lambda() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryLambda);
        let diagnostics = test.lint_dir(
            "no_unnecessary_lambda/test_detects_unnecessary_multiple_parameter_lambda.ds",
            r#"
function add(left, right) {
    return left + right;
}

items.reduce((left, right) => add(left, right));
"#,
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-lambda");
    }

    /// Flag one block body that only returns a forwarded call.
    #[test]
    fn test_detects_unnecessary_block_body_lambda() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryLambda);
        let diagnostics = test.lint_dir(
            "no_unnecessary_lambda/test_detects_unnecessary_block_body_lambda.ds",
            r#"
function foo(value) {
    return value;
}

items.map((value) => {
    return foo(value);
});
"#,
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-lambda")
            .assert_safe_fixed(
                r#"
function foo(value) {
    return value;
}

items.map(foo);
"#,
            );
    }

    /// Allow lambdas that add extra arguments.
    #[test]
    fn test_allows_lambda_with_extra_argument() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryLambda);
        let diagnostics = test.lint_dir(
            "no_unnecessary_lambda/test_allows_lambda_with_extra_argument.ds",
            r#"
items.map((value) => foo(value, 1));
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("no-unnecessary-lambda");
    }

    /// Allow lambdas that reorder parameters.
    #[test]
    fn test_allows_lambda_with_reordered_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryLambda);
        let diagnostics = test.lint_dir(
            "no_unnecessary_lambda/test_allows_lambda_with_reordered_arguments.ds",
            r#"
items.reduce((left, right) => sub(right, left));
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("no-unnecessary-lambda");
    }

    /// Allow member calls that depend on receiver binding.
    #[test]
    fn test_allows_lambda_with_member_call() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryLambda);
        let diagnostics = test.lint_dir(
            "no_unnecessary_lambda/test_allows_lambda_with_member_call.ds",
            r#"
items.map((value) => formatter.format(value));
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("no-unnecessary-lambda");
    }

    /// Allow lambdas that call a parameter instead of a stable function reference.
    #[test]
    fn test_allows_lambda_that_calls_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryLambda);
        let diagnostics = test.lint_dir(
            "no_unnecessary_lambda/test_allows_lambda_that_calls_parameter.ds",
            r#"
const handler = (callback) => callback();
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("no-unnecessary-lambda");
    }
}
