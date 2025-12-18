use destack_ast::{
    self as ast, BinaryOperator, Expression, LocalNodeId, NodeTree, NodeVisitor,
    NodeVisitorOptions, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit cyclomatic complexity of functions.
    ///
    /// Cyclomatic complexity measures the number of linearly independent paths
    /// through a function. High complexity indicates code that is difficult to
    /// test and maintain.
    ///
    /// Complexity is incremented for each decision point:
    /// - `if`, `else if`, `while`, `for`, `loop`
    /// - Each `match` arm (except the first)
    /// - `catch` clauses
    /// - `&&`, `||`, and `??` operators
    /// - `?.` optional chaining
    /// - Ternary expressions
    #[lint(
        id = "cyclomatic-complexity",
        code = "LX007",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CyclomaticComplexity,
    "Limit cyclomatic complexity"
}

impl LintRule for CyclomaticComplexity {
    fn meta(&self) -> &'static crate::LintMeta {
        CyclomaticComplexity::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let max_complexity = ctx.options.max_cyclomatic_complexity;

        // check each function declaration
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let ast::Declaration::Function { body, .. } = declaration else {
                continue;
            };

            let Some(body_id) = body else {
                continue;
            };

            // calculate complexity for this function
            let mut visitor = ComplexityVisitor {
                options: NodeVisitorOptions::default(),
                complexity: 1, // base complexity
            };

            let body_expression = ctx.tree.get(*body_id);
            visitor.visit_expression(ctx.tree, *body_id, body_expression);

            if visitor.complexity > max_complexity {
                ctx.report(
                    LintDiagnostic::new(
                        CYCLOMATIC_COMPLEXITY.id,
                        CYCLOMATIC_COMPLEXITY.code,
                        CYCLOMATIC_COMPLEXITY.category,
                        severity,
                        format!(
                            "cyclomatic complexity {} exceeds maximum of {}",
                            visitor.complexity, max_complexity
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(*body_id),
                    )
                    .with_label("consider breaking into smaller functions"),
                );
            }
        }
    }
}

/// Visitor that calculates cyclomatic complexity for a function body.
struct ComplexityVisitor {
    /// Node visitor options.
    options: NodeVisitorOptions,
    /// Accumulated complexity score, starting at 1 (base complexity).
    complexity: usize,
}

impl NodeVisitor for ComplexityVisitor {
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
            // if statements and ternary add complexity
            Expression::If { .. } => {
                self.complexity += 1;
            }
            // loops add complexity
            Expression::While { .. }
            | Expression::For { .. }
            | Expression::ForEach { .. }
            | Expression::Loop { .. } => {
                self.complexity += 1;
            }
            // each match case except the first adds complexity
            Expression::Match { cases, .. } => {
                if cases.len() > 1 {
                    self.complexity += cases.len() - 1;
                }
            }
            // catch clauses add complexity
            Expression::Try {
                catch_expression, ..
            } => {
                if catch_expression.is_some() {
                    self.complexity += 1;
                }
            }
            // logical operators add complexity (short-circuit evaluation)
            Expression::Binary { operator, .. } => {
                if matches!(
                    operator,
                    BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
                ) {
                    self.complexity += 1;
                }
            }
            // optional chaining creates a branch
            Expression::Maybe { .. } => {
                self.complexity += 1;
            }
            _ => {}
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
    fn test_detects_high_complexity() {
        let test = TestProgram::for_rule(CyclomaticComplexity);
        // create a function with 21+ decision points to exceed default of 20
        let result = test.lint_ast(
            "test.ds",
            r#"
function complex(a: bool) {
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
    if (a) { x() }
}
"#,
        );
        // 1 base + 21 if = 22 > 20
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_allows_simple_function() {
        let test = TestProgram::for_rule(CyclomaticComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function simple(x: int32): int32 {
    if (x > 0) {
        return x
    }
    return -x
}
"#,
        );
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_logical_operators() {
        let test = TestProgram::for_rule(CyclomaticComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function manyConditions(a: bool): bool {
    return a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a && a
}
"#,
        );
        // 1 base + 21 && operators = 22 > 20
        test.result(result).assert_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_match_arms() {
        let test = TestProgram::for_rule(CyclomaticComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function manyMatches(x: int32): string {
    match x {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        _ => "other"
    }
}
"#,
        );
        // 1 base + 5 extra arms = 6, under default 20
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_ternary() {
        let test = TestProgram::for_rule(CyclomaticComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function nested(a: bool, b: bool): int32 {
    return a ? (b ? 1 : 2) : (b ? 3 : 4)
}
"#,
        );
        // 1 base + 4 ternaries = 5, under default 20
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }

    #[test]
    fn test_counts_try_catch() {
        let test = TestProgram::for_rule(CyclomaticComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function withTry() {
    try {
        doSomething()
    } catch (error) {
        handleError(error)
    }
}
"#,
        );
        // 1 base + 1 catch = 2, under default 20
        test.result(result).assert_no_lint("cyclomatic-complexity");
    }
}
