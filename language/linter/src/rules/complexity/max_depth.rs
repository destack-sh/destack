use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    walk_expression,
};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the depth of nested blocks.
    ///
    /// Deeply nested code is harder to read and understand.
    /// Consider extracting logic into separate functions or simplifying control flow.
    #[lint(
        id = "max-depth",
        code = "LX004",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxDepth,
    "Limit block nesting depth"
}

impl LintRule for MaxDepth {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxDepth::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_depth = ctx.options.max_depth;

        // collect violations
        let mut visitor = DepthNodeVisitor {
            options: NodeVisitorOptions::default(),
            depth: 0,
            max_depth,
            violations: Vec::new(),
        };

        for root_id in ctx.roots.iter() {
            let expression = ctx.tree.get(*root_id);
            visitor.visit_expression(ctx.tree, *root_id, expression);
        }

        // create diagnostics with effective severity
        for violation in visitor.violations {
            let severity = ctx.get_effective_severity(meta, violation.node_id);
            if !severity.is_enabled() {
                continue;
            }
            ctx.report(
                LintDiagnostic::new(
                    MAX_DEPTH.id,
                    MAX_DEPTH.code,
                    MAX_DEPTH.category,
                    severity,
                    format!(
                        "nesting depth {} exceeds maximum of {}",
                        violation.depth, max_depth
                    ),
                    ctx.module.file_id,
                    violation.span,
                )
                .with_label("consider extracting into a function"),
            );
        }
    }
}

/// A violation found during depth checking.
struct DepthViolation {
    node_id: LocalNodeId<Expression>,
    span: Span,
    depth: usize,
}

/// NodeVisitor for checking the depth of nested blocks.
struct DepthNodeVisitor {
    options: NodeVisitorOptions,
    depth: usize,
    max_depth: usize,
    violations: Vec<DepthViolation>,
}

impl NodeVisitor for DepthNodeVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // check if this expression increases nesting depth
        let increases_depth = matches!(
            expression,
            Expression::If { .. }
                | Expression::While { .. }
                | Expression::For { .. }
                | Expression::ForEach { .. }
                | Expression::Loop { .. }
                | Expression::Try { .. }
                | Expression::Match { .. }
        );

        if increases_depth {
            self.depth += 1;
            if self.depth > self.max_depth {
                self.violations.push(DepthViolation {
                    node_id: id,
                    span: tree.get_span(id),
                    depth: self.depth,
                });
            }
        }

        // for function declarations, reset depth for the body
        if let Expression::Declaration(declaration_id) = expression {
            let declaration = tree.get(*declaration_id);
            if let ast::Declaration::Function { body, .. } = declaration
                && let Some(body_id) = body
            {
                let saved_depth = self.depth;
                self.depth = 0;
                let body_expression = tree.get(*body_id);
                self.visit_expression(tree, *body_id, body_expression);
                self.depth = saved_depth;
                if increases_depth {
                    self.depth -= 1;
                }
                return;
            }
        }

        // walk children (this handles the recursion with stack protection)
        destack_base::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));

        if increases_depth {
            self.depth -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_deep_nesting() {
        let test = TestProgram::for_rule_without_builtins(MaxDepth);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    if (a) {
        if (b) {
            if (c) {
                if (d) {
                    if (e) {
                        console.log("too deep");
                    }
                }
            }
        }
    }
}
"#,
        );
        test.result(result).assert_lint("max-depth");
    }

    #[test]
    fn test_allows_shallow_nesting() {
        let test = TestProgram::for_rule_without_builtins(MaxDepth);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    if (a) {
        if (b) {
            console.log("ok");
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("max-depth");
    }

    #[test]
    fn test_else_if_does_not_increase_depth() {
        let test = TestProgram::for_rule_without_builtins(MaxDepth);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    if (a) {
        console.log("a");
    } else if (b) {
        console.log("b");
    } else if (c) {
        console.log("c");
    } else if (d) {
        console.log("d");
    } else {
        console.log("else");
    }
}
"#,
        );
        test.result(result).assert_no_lint("max-depth");
    }

    #[test]
    fn test_counts_loop_nesting() {
        let test = TestProgram::for_rule_without_builtins(MaxDepth);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    for (let i = 0; i < 10; i++) {
        while (true) {
            for (const x of items) {
                loop {
                    if (done) {
                        break;
                    }
                }
            }
        }
    }
}
"#,
        );
        test.result(result).assert_lint("max-depth");
    }

    #[test]
    fn test_function_resets_depth() {
        let test = TestProgram::for_rule_without_builtins(MaxDepth);
        let result = test.lint_ast(
            "test.ds",
            r#"
function outer() {
    if (a) {
        if (b) {
            if (c) {
                if (d) {
                    if (e) {
                        function inner() {
                            if (f) {
                                console.log("ok, depth reset");
                            }
                        }
                    }
                }
            }
        }
    }
}
"#,
        );
        // the outer function triggers at depth 5, but inner doesn't (resets to 1)
        test.result(result).assert_lint("max-depth");
    }

    #[test]
    fn test_at_exact_limit() {
        let test = TestProgram::for_rule_without_builtins(MaxDepth);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    if (a) {
        if (b) {
            if (c) {
                if (d) {
                    console.log("exactly at limit 4");
                }
            }
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("max-depth");
    }
}
