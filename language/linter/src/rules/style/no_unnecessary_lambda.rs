use destack_ast::{self as ast, Argument, Declaration, Expression, FunctionKind, Parameter};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow lambdas that only wrap a direct function call.
    ///
    /// When a lambda only passes its parameters directly to another function
    /// without modification, the lambda is unnecessary and the function reference
    /// can be used directly instead.
    ///
    /// ## Bad
    /// ```
    /// items.map(x => foo(x))
    /// items.filter((a, b) => bar(a, b))
    /// ```
    ///
    /// ## Good
    /// ```
    /// items.map(foo)
    /// items.filter(bar)
    /// ```
    #[lint(
        id = "no-unnecessary-lambda",
        code = "LY064",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUnnecessaryLambda,
    "Disallow lambdas that only wrap a direct function call"
}

impl LintRule for NoUnnecessaryLambda {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnnecessaryLambda::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let Expression::Declaration(declaration_id) = ctx.tree.get(node_id) else {
                continue;
            };

            let declaration = ctx.tree.get(*declaration_id);
            let Declaration::Function {
                signature, body, ..
            } = declaration
            else {
                continue;
            };

            // only check lambda functions
            if signature.kind != FunctionKind::Lambda {
                continue;
            }

            // must have a body
            let Some(body_id) = body else {
                continue;
            };

            // must have at least one parameter
            if signature.dynamic_parameters.is_empty() {
                continue;
            }

            // get parameter names
            let parameter_names: Vec<_> = signature
                .dynamic_parameters
                .iter()
                .filter_map(|param_id| {
                    let param = ctx.tree.get(*param_id);
                    match param {
                        Parameter::Named { name, .. } => Some(*name),
                        _ => None,
                    }
                })
                .collect();

            // all parameters must be named (no patterns/variadic)
            if parameter_names.len() != signature.dynamic_parameters.len() {
                continue;
            }

            // body must be a call expression (possibly wrapped in Statement)
            let call_expression_id = unwrap_statement(ctx, *body_id);
            let body_expression = ctx.tree.get(call_expression_id);
            let Expression::Call {
                left: callee_id,
                dynamic_arguments,
                static_arguments,
                ..
            } = body_expression
            else {
                continue;
            };

            // no static arguments
            if static_arguments.is_some() {
                continue;
            }

            // same number of arguments as parameters
            if dynamic_arguments.len() != parameter_names.len() {
                continue;
            }

            // check if each argument is just the corresponding parameter
            let mut is_unnecessary = true;
            for (i, argument_id) in dynamic_arguments.iter().enumerate() {
                let argument = ctx.tree.get(*argument_id);
                let Argument::Positional { value: value_id } = argument else {
                    is_unnecessary = false;
                    break;
                };

                let value = ctx.tree.get(*value_id);
                let Expression::Path { path, .. } = value else {
                    is_unnecessary = false;
                    break;
                };

                // must be a single-segment path (just the variable name)
                if path.segments.len() != 1 {
                    is_unnecessary = false;
                    break;
                }

                // must match the corresponding parameter
                let argument_name = ctx.strings.get(path.segments[0]);
                let parameter_name = ctx.strings.get(parameter_names[i]);
                if argument_name.as_ref() != parameter_name.as_ref() {
                    is_unnecessary = false;
                    break;
                }
            }
            if !is_unnecessary {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // get the callee text for the fix
            let callee_span = ctx.tree.get_span(*callee_id);
            let callee_text = ctx.get_span_text(callee_span);
            let declaration_span = ctx.tree.get_span(*declaration_id);
            let edits = ctx
                .edit_builder()
                .replace(declaration_span, callee_text.to_string())
                .into_edits();
            let fix = LintFix::safe("Replace with function reference").with_edits(edits);

            ctx.report(
                LintDiagnostic::new(
                    NO_UNNECESSARY_LAMBDA.id,
                    NO_UNNECESSARY_LAMBDA.code,
                    NO_UNNECESSARY_LAMBDA.category,
                    severity,
                    format!("unnecessary lambda wrapping `{callee_text}`"),
                    ctx.module.file_id,
                    declaration_span,
                )
                .with_label(format!("use `{callee_text}` directly"))
                .with_fix(fix),
            );
        }
    }
}

/// Unwrap Statement expressions to get the inner expression.
fn unwrap_statement(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<Expression>,
) -> ast::LocalNodeId<Expression> {
    let expr = ctx.tree.get(expr_id);
    if let Expression::Statement(inner_id) = expr {
        unwrap_statement(ctx, *inner_id)
    } else {
        expr_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_unnecessary_single_param_lambda() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(x => foo(x))
"#,
        );
        test.result(result).assert_lint("no-unnecessary-lambda");
    }

    #[test]
    fn test_detects_unnecessary_multi_param_lambda() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.reduce((a, b) => add(a, b))
"#,
        );
        test.result(result).assert_lint("no-unnecessary-lambda");
    }

    #[test]
    fn test_allows_lambda_with_extra_arg() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(x => foo(x, 1))
"#,
        );
        test.result(result).assert_no_lint("no-unnecessary-lambda");
    }

    #[test]
    fn test_allows_lambda_with_different_order() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.reduce((a, b) => sub(b, a))
"#,
        );
        test.result(result).assert_no_lint("no-unnecessary-lambda");
    }

    #[test]
    fn test_allows_lambda_with_method_call() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(x => x.toString())
"#,
        );
        test.result(result).assert_no_lint("no-unnecessary-lambda");
    }

    #[test]
    fn test_allows_zero_param_lambda() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
defer(() => cleanup())
"#,
        );
        test.result(result).assert_no_lint("no-unnecessary-lambda");
    }

    #[test]
    fn test_allows_lambda_with_expression_body() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(x => x + 1)
"#,
        );
        test.result(result).assert_no_lint("no-unnecessary-lambda");
    }

    #[test]
    fn test_allows_lambda_param_used_twice() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(x => foo(x, x))
"#,
        );
        test.result(result).assert_no_lint("no-unnecessary-lambda");
    }

    #[test]
    fn test_fix_removes_lambda() {
        let test = TestProgram::for_rule(NoUnnecessaryLambda);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(x => foo(x));
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-lambda")
            .assert_safe_fixed(
                r#"
items.map(foo);
"#,
            );
    }
}
