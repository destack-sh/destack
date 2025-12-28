use destack_ast::{self as ast, Argument, Declaration, Expression, FunctionKind};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer arrow functions for callbacks.
    ///
    /// Arrow functions are more concise and don't bind their own `this`.
    /// Use arrow functions for callbacks unless `this` binding is needed.
    #[lint(
        id = "prefer-arrow-callback",
        code = "LY026",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArrowCallback,
    "Prefer arrow functions for callbacks"
}

impl LintRule for PreferArrowCallback {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferArrowCallback::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // iterate over all call expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let Expression::Call {
                dynamic_arguments, ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            for argument_id in dynamic_arguments {
                check_callback_argument(ctx, meta, *argument_id);
            }
        }
    }
}

/// Check if an argument is a function expression that should be an arrow function.
fn check_callback_argument(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    argument_id: ast::LocalNodeId<Argument>,
) {
    let argument = ctx.tree.get(argument_id);

    // get value from the argument (Positional, Named, etc.)
    let value_id = match argument {
        Argument::Positional { value, .. } => *value,
        Argument::Named { value, .. } => *value,
        Argument::Labeled { value, .. } => *value,
        Argument::Spread { .. } => return,
    };

    let value = ctx.tree.get(value_id);

    // check if value is a function declaration (not lambda)
    let Expression::Declaration(declaration_id) = value else {
        return;
    };
    let declaration = ctx.tree.get(*declaration_id);
    let Declaration::Function {
        descriptor,
        signature,
        ..
    } = declaration
    else {
        return;
    };

    // skip named functions
    if descriptor.name.is_some() {
        return;
    }

    // skip lambda functions (they're already arrows)
    if signature.kind == FunctionKind::Lambda {
        return;
    }

    let severity = ctx.get_effective_severity(meta, argument_id);
    if !severity.is_enabled() {
        return;
    }

    let decl_span = ctx.tree.get_span(*declaration_id);
    let decl_text = ctx.get_span_text(decl_span);

    // build fix: convert function(params) { body } to (params) => { body }
    // the declaration text starts with "function", we strip that and insert " =>" before the body
    let replacement = if let Some(rest) = decl_text.strip_prefix("function") {
        // find where the body starts (the opening brace)
        if let Some(brace_pos) = rest.find('{') {
            let params = rest[..brace_pos].trim();
            let body = &rest[brace_pos..];
            format!("{params} => {body}")
        } else {
            // shouldn't happen for normal functions, skip fix
            return;
        }
    } else {
        return;
    };

    let edits = ctx
        .edit_builder()
        .replace(decl_span, replacement)
        .into_edits();
    let fix = LintFix::safe("Convert to arrow function").with_edits(edits);

    ctx.report(
        LintDiagnostic::new(
            PREFER_ARROW_CALLBACK.id,
            PREFER_ARROW_CALLBACK.code,
            PREFER_ARROW_CALLBACK.category,
            severity,
            "prefer arrow function for callback",
            ctx.module.file_id,
            decl_span,
        )
        .with_label("use `() => { ... }` instead of `function() { ... }`")
        .with_fix(fix),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_arrow_callback() {
        let test = TestProgram::for_rule_without_builtins(PreferArrowCallback);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map((x) => x + 1)
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_detects_function_callback() {
        let test = TestProgram::for_rule_without_builtins(PreferArrowCallback);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(function(x) { return x + 1 })
"#,
        );
        test.result(result).assert_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_allows_named_function() {
        let test = TestProgram::for_rule_without_builtins(PreferArrowCallback);
        // named functions are intentional, don't flag
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(function increment(x) { return x + 1 })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_allows_non_callback_function() {
        let test = TestProgram::for_rule_without_builtins(PreferArrowCallback);
        // function declarations not used as callbacks
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    return 42
}
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_fix_function_to_arrow() {
        let test = TestProgram::for_rule_without_builtins(PreferArrowCallback);
        let result = test.lint_ast(
            "test.ds",
            r#"
items.map(function(x) { return x + 1 });
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_safe_fixed(
                r#"
items.map((x) => {
    return x + 1
});
"#,
            );
    }
}
