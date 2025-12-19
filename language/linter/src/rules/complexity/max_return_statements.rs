use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of return statements per function.
    ///
    /// Functions with many return points can be harder to follow and maintain.
    /// Consider restructuring with early returns or extracting logic.
    #[lint(
        id = "max-return-statements",
        code = "LX016",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxReturnStatements,
    "Limit return statements per function"
}

impl LintRule for MaxReturnStatements {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxReturnStatements::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let mut visitor = ReturnStatementVisitor {
            options: NodeVisitorOptions::default(),
            return_counts: Vec::new(),
            max_return_statements: ctx.options.max_return_statements,
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

struct ReturnStatementVisitor {
    options: NodeVisitorOptions,
    return_counts: Vec<usize>,
    max_return_statements: usize,
    severity: LintSeverity,
    file_id: destack_source::FileId,
    diagnostics: Vec<LintDiagnostic>,
}

impl NodeVisitor for ReturnStatementVisitor {
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
            // count return statements for the innermost function
            Expression::Return { .. } => {
                if let Some(count) = self.return_counts.last_mut() {
                    *count += 1;
                }
            }

            // handle function declarations: start fresh count
            Expression::Declaration(declaration_id) => {
                let declaration = tree.get(*declaration_id);
                if let ast::Declaration::Function { body, .. } = declaration {
                    if let Some(body_id) = body {
                        let body_span = tree.get_span(*body_id);

                        // push new counter for this function
                        self.return_counts.push(0);

                        // walk the function body
                        let body_expression = tree.get(*body_id);
                        destack_base::ensure_sufficient_stack(|| {
                            walk_expression(self, tree, *body_id, body_expression)
                        });

                        // pop and check the count
                        let return_count = self.return_counts.pop().unwrap_or(0);
                        if return_count > self.max_return_statements {
                            self.diagnostics.push(
                                LintDiagnostic::new(
                                    MAX_RETURN_STATEMENTS.id,
                                    MAX_RETURN_STATEMENTS.code,
                                    MAX_RETURN_STATEMENTS.category,
                                    self.severity,
                                    format!(
                                        "function has {return_count} return statements (max {})",
                                        self.max_return_statements
                                    ),
                                    self.file_id,
                                    body_span,
                                )
                                .with_label("consider restructuring to reduce return points"),
                            );
                        }
                    }
                    return; // don't walk the declaration again
                }
            }

            _ => {}
        }

        // default recursion for non-function expressions
        destack_base::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_returns() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function tooManyReturns(x: int32): int32 {
    if (x < 0) { return -1; }
    if (x == 0) { return 0; }
    if (x == 1) { return 1; }
    if (x == 2) { return 2; }
    if (x == 3) { return 3; }
    if (x == 4) { return 4; }
    if (x == 5) { return 5; }
    if (x == 6) { return 6; }
    if (x == 7) { return 7; }
    if (x == 8) { return 8; }
    return 10;
}
"#,
        );
        test.result(result).assert_lint("max-return-statements");
    }

    #[test]
    fn test_allows_few_returns() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function fewReturns(x: int32): int32 {
    if (x < 0) { return -1; }
    if (x == 0) { return 0; }
    return x;
}
"#,
        );
        test.result(result).assert_no_lint("max-return-statements");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        // 10 returns is at the limit (default max is 10)
        let result = test.lint_ast(
            "test.ds",
            r#"
function atLimit(x: int32): int32 {
    if (x == 0) { return 0; }
    if (x == 1) { return 1; }
    if (x == 2) { return 2; }
    if (x == 3) { return 3; }
    if (x == 4) { return 4; }
    if (x == 5) { return 5; }
    if (x == 6) { return 6; }
    if (x == 7) { return 7; }
    if (x == 8) { return 8; }
    return 9;
}
"#,
        );
        test.result(result).assert_no_lint("max-return-statements");
    }

    #[test]
    fn test_counts_returns_in_match() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function matchReturns(x: int32): int32 {
    match (x) {
        0 => return 0
        1 => return 1
        2 => return 2
        3 => return 3
        4 => return 4
        5 => return 5
        6 => return 6
        7 => return 7
        8 => return 8
        9 => return 9
        _ => return 10
    }
}
"#,
        );
        test.result(result).assert_lint("max-return-statements");
    }

    #[test]
    fn test_does_not_count_nested_function() {
        let test = TestProgram::for_rule(MaxReturnStatements);
        let result = test.lint_ast(
            "test.ds",
            r#"
function outer(x: int32): int32 {
    function inner(y: int32): int32 {
        if (y == 0) { return 0; }
        if (y == 1) { return 1; }
        if (y == 2) { return 2; }
        if (y == 3) { return 3; }
        if (y == 4) { return 4; }
        if (y == 5) { return 5; }
        if (y == 6) { return 6; }
        if (y == 7) { return 7; }
        if (y == 8) { return 8; }
        if (y == 9) { return 9; }
        return 10;
    }
    return inner(x);
}
"#,
        );
        // outer has 1 return, inner has 11 but is a separate function
        // so we get TWO violations: one for outer (1 return, ok), one for inner (11 returns, not ok)
        test.result(result).assert_lint("max-return-statements");
    }
}
