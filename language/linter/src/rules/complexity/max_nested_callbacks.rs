use crate::LintMeta;
use destack_ast::{
    self as ast, Argument, Expression, LocalNodeId, NodeVisitor, NodeVisitorOptions, Tree,
    walk_argument,
};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_unwrap_parenthesized_source_form;
use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the depth of nested callbacks.
    ///
    /// Deeply nested callbacks are hard to read and maintain.
    /// Consider using async and await, promises, or extracting nested callbacks into named functions.
    #[lint(
        id = "max-nested-callbacks",
        code = "LX007",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxNestedCallbacks,
    "Limit nested callback depth"
}

impl LintRule for MaxNestedCallbacks {
    fn meta(&self) -> &'static LintMeta {
        MaxNestedCallbacks::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_callbacks = ctx.options.complexity.max_nested_callbacks;

        // traverse module roots and collect callback-depth violations
        let mut visitor = CallbackVisitor {
            options: NodeVisitorOptions::default(),
            parents: ctx.parents,
            callback_depth: 0,
            max_callbacks,
            violations: Vec::new(),
        };

        for root_id in ctx.roots.iter() {
            let root_expression = ctx.tree.get(*root_id);
            visitor.visit_expression(ctx.tree, *root_id, root_expression);
        }

        // report collected violations with effective severity
        for violation in visitor.violations {
            let severity = ctx.get_effective_severity(meta, violation.node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintReport::new(
                    MAX_NESTED_CALLBACKS.id,
                    MAX_NESTED_CALLBACKS.code,
                    MAX_NESTED_CALLBACKS.category,
                    severity,
                    format!(
                        "callback nesting depth {} exceeds maximum of {}",
                        violation.depth, max_callbacks
                    ),
                    violation.span,
                )
                .label("consider using async and await or extracting to a named function"),
            );
        }
    }
}

/// One callback depth violation.
struct CallbackViolation {
    /// Node id of the callback expression.
    node_id: LocalNodeId<Expression>,
    /// Source span of the callback expression.
    span: Span,
    /// Effective callback depth for this callback.
    depth: usize,
}

/// Node visitor for callback-depth tracking.
struct CallbackVisitor<'a> {
    /// Traversal options.
    options: NodeVisitorOptions,
    /// Parent index for call-argument context checks.
    parents: &'a ast::NodeParentIndex,
    /// Current callback nesting depth.
    callback_depth: usize,
    /// Configured maximum callback depth.
    max_callbacks: usize,
    /// Collected callback depth violations.
    violations: Vec<CallbackViolation>,
}

impl CallbackVisitor<'_> {
    /// Return true when one expression is a callback function expression.
    fn expression_is_callback_function(
        &self,
        tree: &Tree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // normalize parenthesized wrappers before shape checks
        let expression_id = expression_unwrap_parenthesized_source_form(tree, expression_id);
        let expression = tree.get(expression_id);

        // require callable function expressions with a body
        let Expression::Declaration(declaration_id) = expression else {
            return false;
        };

        let declaration = tree.get(*declaration_id);
        matches!(
            declaration,
            ast::Declaration::Function(ast::FunctionDeclaration { body: Some(_), .. })
        )
    }
}

impl NodeVisitor for CallbackVisitor<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_argument(
        &mut self,
        tree: &Tree,
        argument_id: LocalNodeId<Argument>,
        argument: &Argument,
    ) {
        // only count callback arguments for call expressions
        if !argument_is_call_argument(tree, self.parents, argument_id) {
            walk_argument(self, tree, argument_id, argument);
            return;
        }

        // resolve argument expression value
        let Some(value_expression_id) = argument_value_expression_id(argument) else {
            walk_argument(self, tree, argument_id, argument);
            return;
        };
        if !self.expression_is_callback_function(tree, value_expression_id) {
            walk_argument(self, tree, argument_id, argument);
            return;
        }

        // push callback depth for this callback argument
        self.callback_depth += 1;
        if self.callback_depth > self.max_callbacks {
            self.violations.push(CallbackViolation {
                node_id: value_expression_id,
                span: tree.get_span(value_expression_id),
                depth: self.callback_depth,
            });
        }

        // recurse into callback body and nested callback arguments
        walk_argument(self, tree, argument_id, argument);

        // pop callback depth after this callback argument
        self.callback_depth = self.callback_depth.saturating_sub(1);
    }
}

/// Return the expression id for one argument value.
fn argument_value_expression_id(argument: &Argument) -> Option<LocalNodeId<Expression>> {
    match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => Some(*value),
        Argument::Error => None,
    }
}

/// Return true when one argument id belongs to call arguments.
fn argument_is_call_argument(
    tree: &Tree,
    parents: &ast::NodeParentIndex,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    // require expression parent
    let Some(parent_id) = parents.get(argument_id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return false;
    }

    // require argument membership on call expressions only
    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = tree.get(parent_expression_id);
    let Expression::Call { arguments, .. } = parent_expression else {
        return false;
    };

    arguments.contains(&argument_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_deeply_nested_callbacks() {
        let test = TestProgram::for_rule_without_prelude(MaxNestedCallbacks);
        let result = test.lint_ast(
            "max_nested_callbacks/test_detects_deeply_nested_callbacks.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxNestedCallbacks);
        let result = test.lint_ast(
            "max_nested_callbacks/test_allows_shallow_callbacks.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxNestedCallbacks);
        let result = test.lint_ast(
            "max_nested_callbacks/test_allows_exactly_at_limit.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxNestedCallbacks);
        let result = test.lint_ast(
            "max_nested_callbacks/test_non_callback_functions_dont_count.ds",
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

    #[test]
    fn test_ignores_new_expression_callback_arguments() {
        let test =
            TestProgram::for_rule_without_prelude(MaxNestedCallbacks).with_options(|options| {
                options.complexity.max_nested_callbacks = 0;
            });
        let result = test.lint_ast(
            "max_nested_callbacks/test_ignores_new_expression_callback_arguments.ds",
            r#"
new Service(() => {
    console.log("not counted by source semantics");
});
"#,
        );
        test.result(result).assert_no_lint("max-nested-callbacks");
    }

    #[test]
    fn test_counts_parenthesized_callback_arguments() {
        let test =
            TestProgram::for_rule_without_prelude(MaxNestedCallbacks).with_options(|options| {
                options.complexity.max_nested_callbacks = 0;
            });
        let result = test.lint_ast(
            "max_nested_callbacks/test_counts_parenthesized_callback_arguments.ds",
            r#"
foo((() => {
    console.log("counted callback");
}));
"#,
        );
        test.result(result).assert_lint("max-nested-callbacks");
    }
}
