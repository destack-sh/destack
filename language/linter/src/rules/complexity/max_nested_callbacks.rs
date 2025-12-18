use destack_ast::{
    self as ast, Argument, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    walk_argument, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the depth of nested callbacks.
    ///
    /// Deeply nested callbacks (callback hell) are hard to read and maintain.
    /// Consider using async/await, promises, or extracting nested callbacks
    /// into named functions.
    #[lint(
        id = "max-nested-callbacks",
        code = "LX005",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxNestedCallbacks,
    "Limit nested callback depth"
}

impl LintRule for MaxNestedCallbacks {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxNestedCallbacks::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let max_callbacks = ctx.options.max_nested_callbacks;

        let mut visitor = CallbackVisitor {
            options: NodeVisitorOptions::default(),
            depth: 0,
            max_callbacks,
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

struct CallbackVisitor {
    options: NodeVisitorOptions,
    depth: usize,
    max_callbacks: usize,
    severity: LintSeverity,
    file_id: destack_source::FileId,
    diagnostics: Vec<LintDiagnostic>,
}

impl CallbackVisitor {
    /// Check if an expression is a function (callback)
    fn is_function(&self, tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
        let expression = tree.get(expression_id);
        match expression {
            Expression::Declaration(declaration_id) => {
                let declaration = tree.get(*declaration_id);
                matches!(declaration, ast::Declaration::Function { .. })
            }
            Expression::Parenthesized { expression } => self.is_function(tree, *expression),
            _ => false,
        }
    }
}

impl NodeVisitor for CallbackVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: LocalNodeId<Argument>, argument: &Argument) {
        // get the value from the argument
        let value_id = match argument {
            Argument::Named { value, .. } => *value,
            Argument::Labeled { value, .. } => *value,
            Argument::Positional { value } => *value,
            Argument::Spread { value } => *value,
        };

        // check if this argument is a function (callback)
        if self.is_function(tree, value_id) {
            self.depth += 1;
            if self.depth > self.max_callbacks {
                self.diagnostics.push(
                    LintDiagnostic::new(
                        MAX_NESTED_CALLBACKS.id,
                        MAX_NESTED_CALLBACKS.code,
                        MAX_NESTED_CALLBACKS.category,
                        self.severity,
                        format!(
                            "callback nesting depth {} exceeds maximum of {}",
                            self.depth, self.max_callbacks
                        ),
                        self.file_id,
                        tree.get_span(value_id),
                    )
                    .with_label("consider using async/await or extracting to a named function"),
                );
            }
            // walk the argument (which will visit the callback body)
            destack_base::ensure_sufficient_stack(|| walk_argument(self, tree, id, argument));
            self.depth -= 1;
        } else {
            // not a callback, just walk normally
            destack_base::ensure_sufficient_stack(|| walk_argument(self, tree, id, argument));
        }
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // for top-level function declarations, reset depth
        if let Expression::Declaration(declaration_id) = expression {
            let declaration = tree.get(*declaration_id);
            if let ast::Declaration::Function { body, .. } = declaration
                && let Some(body_id) = body
            {
                // only reset if we're not already inside a callback
                // (callbacks are tracked via visit_argument)
                if self.depth == 0 {
                    let body_expression = tree.get(*body_id);
                    self.visit_expression(tree, *body_id, body_expression);
                    return;
                }
            }
        }

        // walk children
        destack_base::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_deeply_nested_callbacks() {
        let test = TestProgram::for_rule(MaxNestedCallbacks);
        let result = test.lint_ast(
            "test.ds",
            r#"
foo(() => {
    bar(() => {
        baz(() => {
            qux(() => {
                quux(() => {
                    console.log("too deep");
                });
            });
        });
    });
});
"#,
        );
        test.result(result).assert_lint("max-nested-callbacks");
    }

    #[test]
    fn test_allows_shallow_callbacks() {
        let test = TestProgram::for_rule(MaxNestedCallbacks);
        let result = test.lint_ast(
            "test.ds",
            r#"
foo(() => {
    bar(() => {
        console.log("ok");
    });
});
"#,
        );
        test.result(result).assert_no_lint("max-nested-callbacks");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule(MaxNestedCallbacks);
        let result = test.lint_ast(
            "test.ds",
            r#"
foo(() => {
    bar(() => {
        baz(() => {
            qux(() => {
                console.log("exactly at limit 4");
            });
        });
    });
});
"#,
        );
        test.result(result).assert_no_lint("max-nested-callbacks");
    }

    #[test]
    fn test_non_callback_functions_dont_count() {
        let test = TestProgram::for_rule(MaxNestedCallbacks);
        let result = test.lint_ast(
            "test.ds",
            r#"
function outer() {
    function inner1() {
        function inner2() {
            function inner3() {
                function inner4() {
                    function inner5() {
                        console.log("nested declarations, not callbacks");
                    }
                }
            }
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("max-nested-callbacks");
    }
}
