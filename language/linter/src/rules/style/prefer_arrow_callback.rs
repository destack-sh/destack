use destack_ast::{
    Argument, Declaration, Expression, FunctionKind, LocalNodeId, NodeTree, NodeVisitor,
    NodeVisitorOptions, walk_expression,
};
use destack_source::FileId;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        fixable = No,
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

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let mut visitor = CallbackVisitor {
            options: NodeVisitorOptions::default(),
            severity,
            file_id: ctx.module.file_id,
            diagnostics: Vec::new(),
        };

        for root_id in ctx.roots.iter() {
            let expression = ctx.tree.get(*root_id);
            visitor.visit_expression(ctx.tree, *root_id, expression);
        }

        for diagnostic in visitor.diagnostics {
            ctx.report(diagnostic);
        }
    }
}

/// Visitor to find function callbacks.
struct CallbackVisitor {
    options: NodeVisitorOptions,
    severity: LintSeverity,
    file_id: FileId,
    diagnostics: Vec<LintDiagnostic>,
}

impl NodeVisitor for CallbackVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // look for call expressions
        if let Expression::Call {
            dynamic_arguments, ..
        } = expression
        {
            for argument_id in dynamic_arguments {
                self.check_callback_argument(tree, *argument_id);
            }
        }

        // continue walking
        destack_base::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }
}

impl CallbackVisitor {
    /// Check if an argument is a function expression that should be an arrow function.
    fn check_callback_argument(&mut self, tree: &NodeTree, argument_id: LocalNodeId<Argument>) {
        let argument = tree.get(argument_id);

        // get value from the argument (Positional, Named, etc.)
        let value_id = match argument {
            Argument::Positional { value } => *value,
            Argument::Named { value, .. } => *value,
            Argument::Labeled { value, .. } => *value,
            Argument::Spread { .. } => return,
        };

        let value = tree.get(value_id);

        // check if value is a function declaration (not lambda)
        let Expression::Declaration(declaration_id) = value else {
            return;
        };
        let declaration = tree.get(*declaration_id);
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

        // flag traditional function expressions used as callbacks
        self.diagnostics.push(
            LintDiagnostic::new(
                PREFER_ARROW_CALLBACK.id,
                PREFER_ARROW_CALLBACK.code,
                PREFER_ARROW_CALLBACK.category,
                self.severity,
                "prefer arrow function for callback",
                self.file_id,
                tree.get_span(*declaration_id),
            )
            .with_label("use `() => { ... }` instead of `function() { ... }`"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_arrow_callback() {
        let test = TestProgram::for_rule(PreferArrowCallback);
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
        let test = TestProgram::for_rule(PreferArrowCallback);
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
        let test = TestProgram::for_rule(PreferArrowCallback);
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
        let test = TestProgram::for_rule(PreferArrowCallback);
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
}
