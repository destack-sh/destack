use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow control flow statements in finally blocks.
    ///
    /// Using `return`, `throw`, `break`, or `continue` in a `finally` block can cause
    /// unexpected behavior by overriding the return value or exception from the try/catch.
    #[lint(
        id = "no-unsafe-finally",
        code = "LC011",
        category = Correctness,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUnsafeFinally,
    "Disallow control flow in finally blocks"
}

impl LintRule for NoUnsafeFinally {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnsafeFinally::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Try {
                finally_expression: Some(finally_id),
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // check for unsafe control flow in the finally block
            let mut visitor = FinallyVisitor {
                options: NodeVisitorOptions::default(),
                severity,
                file_id: ctx.module.file_id,
                diagnostics: Vec::new(),
                in_function: false,
            };

            let finally_expression = ctx.tree.get(*finally_id);
            visitor.visit_expression(ctx.tree, *finally_id, finally_expression);

            for diagnostic in visitor.diagnostics {
                ctx.report(diagnostic);
            }
        }
    }
}

struct FinallyVisitor {
    options: NodeVisitorOptions,
    severity: LintSeverity,
    file_id: destack_source::FileId,
    diagnostics: Vec<LintDiagnostic>,
    in_function: bool,
}

impl NodeVisitor for FinallyVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            // don't traverse into nested functions, as their control flow is local
            Expression::Declaration(declaration_id) => {
                let declaration = tree.get(*declaration_id);
                if matches!(declaration, ast::Declaration::Function { .. }) {
                    // skip function bodies
                    return;
                }
            }
            // return in finally is unsafe
            Expression::Return { .. } if !self.in_function => {
                self.diagnostics.push(
                    LintDiagnostic::new(
                        NO_UNSAFE_FINALLY.id,
                        NO_UNSAFE_FINALLY.code,
                        NO_UNSAFE_FINALLY.category,
                        self.severity,
                        "`return` in finally block",
                        self.file_id,
                        tree.get_span(id),
                    )
                    .with_label("this may override a thrown exception"),
                );
            }
            // throw in finally is unsafe
            Expression::Throw { .. } if !self.in_function => {
                self.diagnostics.push(
                    LintDiagnostic::new(
                        NO_UNSAFE_FINALLY.id,
                        NO_UNSAFE_FINALLY.code,
                        NO_UNSAFE_FINALLY.category,
                        self.severity,
                        "`throw` in finally block",
                        self.file_id,
                        tree.get_span(id),
                    )
                    .with_label("this may override a thrown exception"),
                );
            }
            // break in finally is unsafe
            Expression::Break { .. } if !self.in_function => {
                self.diagnostics.push(
                    LintDiagnostic::new(
                        NO_UNSAFE_FINALLY.id,
                        NO_UNSAFE_FINALLY.code,
                        NO_UNSAFE_FINALLY.category,
                        self.severity,
                        "`break` in finally block",
                        self.file_id,
                        tree.get_span(id),
                    )
                    .with_label("this may disrupt expected control flow"),
                );
            }
            // continue in finally is unsafe
            Expression::Continue { .. } if !self.in_function => {
                self.diagnostics.push(
                    LintDiagnostic::new(
                        NO_UNSAFE_FINALLY.id,
                        NO_UNSAFE_FINALLY.code,
                        NO_UNSAFE_FINALLY.category,
                        self.severity,
                        "`continue` in finally block",
                        self.file_id,
                        tree.get_span(id),
                    )
                    .with_label("this may disrupt expected control flow"),
                );
            }
            _ => {}
        }

        destack_base::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_return_in_finally() {
        let test = TestProgram::for_rule_without_builtins(NoUnsafeFinally);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    try {
        throw "error";
    } finally {
        return 1;
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_detects_throw_in_finally() {
        let test = TestProgram::for_rule_without_builtins(NoUnsafeFinally);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    try {
        throw "error1";
    } finally {
        throw "error2";
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_detects_break_in_finally() {
        let test = TestProgram::for_rule_without_builtins(NoUnsafeFinally);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (true) {
    try {
        throw "error";
    } finally {
        break;
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_detects_continue_in_finally() {
        let test = TestProgram::for_rule_without_builtins(NoUnsafeFinally);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (true) {
    try {
        throw "error";
    } finally {
        continue;
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_allows_return_in_try() {
        let test = TestProgram::for_rule_without_builtins(NoUnsafeFinally);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    try {
        return 1;
    } finally {
        console.log("cleanup");
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-finally");
    }

    #[test]
    fn test_allows_return_in_nested_function() {
        let test = TestProgram::for_rule_without_builtins(NoUnsafeFinally);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    try {
        throw "error";
    } finally {
        const inner = () => {
            return 1;
        };
        inner();
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-finally");
    }
}
