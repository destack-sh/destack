use crate::LintMeta;
use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeVisitor, NodeVisitorOptions, Tree, walk_expression,
    walk_member,
};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_is_else_if_branch;
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

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
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxDepth,
    "Limit block nesting depth"
}

impl LintRule for MaxDepth {
    fn meta(&self) -> &'static LintMeta {
        MaxDepth::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let max_depth = ctx.options.complexity.max_depth;

        // collect violations
        let mut visitor = DepthNodeVisitor {
            options: NodeVisitorOptions::default(),
            parents: ctx.parents,
            depth_stack: vec![0],
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
struct DepthNodeVisitor<'a> {
    /// Visitor options.
    options: NodeVisitorOptions,
    /// Parent index for else-if normalization.
    parents: &'a ast::NodeParentIndex,
    /// Depth stack per callable boundary.
    depth_stack: Vec<usize>,
    /// Configured maximum nesting depth.
    max_depth: usize,
    /// Collected depth violations.
    violations: Vec<DepthViolation>,
}

impl DepthNodeVisitor<'_> {
    /// Increase depth in the current callable and return the new value.
    fn increment_depth(&mut self) -> usize {
        if let Some(depth) = self.depth_stack.last_mut() {
            *depth += 1;
            return *depth;
        }

        0
    }

    /// Decrease depth in the current callable.
    fn decrement_depth(&mut self) {
        if let Some(depth) = self.depth_stack.last_mut() {
            *depth = depth.saturating_sub(1);
        }
    }

    /// Push one new callable boundary depth frame.
    fn push_callable_depth(&mut self) {
        self.depth_stack.push(0);
    }

    /// Pop one callable boundary depth frame.
    fn pop_callable_depth(&mut self) {
        self.depth_stack.pop();
        if self.depth_stack.is_empty() {
            self.depth_stack.push(0);
        }
    }
}

impl NodeVisitor for DepthNodeVisitor<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // keep callable boundaries isolated
        if let Expression::Declaration(declaration_id) = expression {
            let declaration = tree.get(*declaration_id);
            if let ast::Declaration::Function(declaration) = declaration
                && let Some(body_id) = declaration.body
            {
                self.push_callable_depth();

                let body_expression = tree.get(body_id);
                self.visit_expression(tree, body_id, body_expression);

                self.pop_callable_depth();
                return;
            }
        }

        // check whether this expression increases the current depth
        let increases_depth = expression_increases_depth(tree, self.parents, id, expression);
        if increases_depth {
            let depth = self.increment_depth();
            if depth > self.max_depth {
                self.violations.push(DepthViolation {
                    node_id: id,
                    span: tree.get_span(id),
                    depth,
                });
            }
        }

        // recurse into expression children
        walk_expression(self, tree, id, expression);

        // leave this depth frame when needed
        if increases_depth {
            self.decrement_depth();
        }
    }

    fn visit_member(&mut self, tree: &Tree, id: LocalNodeId<ast::Member>, member: &ast::Member) {
        // keep method and static-block callable scopes isolated from outer depth
        let body_id = match member {
            ast::Member::Method {
                body: Some(body), ..
            } => Some(*body),
            ast::Member::StaticBlock { body, .. } => Some(*body),
            _ => None,
        };

        let Some(body_id) = body_id else {
            walk_member(self, tree, id, member);
            return;
        };

        self.push_callable_depth();

        let body_expression = tree.get(body_id);
        self.visit_expression(tree, body_id, body_expression);

        self.pop_callable_depth();
    }
}

/// Return true when one expression increases nesting depth.
fn expression_increases_depth(
    tree: &Tree,
    parents: &ast::NodeParentIndex,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    match expression {
        Expression::If { .. } => !expression_is_else_if_branch(tree, parents, expression_id),
        Expression::While { .. }
        | Expression::For { .. }
        | Expression::ForEach { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Match { .. } => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_deep_nesting() {
        let test = TestProgram::for_rule_without_prelude(MaxDepth);
        let result = test.lint_ast(
            "max_depth/test_detects_deep_nesting.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxDepth);
        let result = test.lint_ast(
            "max_depth/test_allows_shallow_nesting.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxDepth);
        let result = test.lint_ast(
            "max_depth/test_else_if_does_not_increase_depth.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxDepth);
        let result = test.lint_ast(
            "max_depth/test_counts_loop_nesting.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxDepth);
        let result = test.lint_ast(
            "max_depth/test_function_resets_depth.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxDepth);
        let result = test.lint_ast(
            "max_depth/test_at_exact_limit.ds",
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

    #[test]
    fn test_static_block_depth_resets_from_outer_if() {
        let test = TestProgram::for_rule_without_prelude(MaxDepth)
            .with_options(|options| options.complexity.max_depth = 2);
        let result = test.lint_ast(
            "max_depth/test_static_block_depth_resets_from_outer_if.ds",
            r#"
if (a) {
    class C {
        static {
            if (b) {
                if (c) {
                    value()
                }
            }
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("max-depth");
    }

    #[test]
    fn test_reports_static_block_depth_over_limit() {
        let test = TestProgram::for_rule_without_prelude(MaxDepth)
            .with_options(|options| options.complexity.max_depth = 2);
        let result = test.lint_ast(
            "max_depth/test_reports_static_block_depth_over_limit.ds",
            r#"
class C {
    static {
        if (a) {
            if (b) {
                if (c) {
                    value()
                }
            }
        }
    }
}
"#,
        );
        test.result(result).assert_lint("max-depth");
    }
}
