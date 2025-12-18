use destack_ast::{
    self as ast, BinaryOperator, Expression, LocalNodeId, NodeTree, NodeVisitor,
    NodeVisitorOptions, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit cognitive complexity of functions.
    ///
    /// Cognitive complexity measures how difficult code is to understand,
    /// not just how many paths exist. It penalizes nested structures more
    /// heavily than flat ones.
    ///
    /// Complexity is incremented for:
    /// - Control flow: `if`, `else if`, `for`, `while`, `loop`, `match`
    /// - Logical operators: `&&`, `||`, `??` (sequences of same operator count as 1)
    /// - Catch clauses and ternary expressions
    ///
    /// Additionally, nesting increases the penalty for each structure.
    #[lint(
        id = "cognitive-complexity",
        code = "LX008",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CognitiveComplexity,
    "Limit cognitive complexity"
}

impl LintRule for CognitiveComplexity {
    fn meta(&self) -> &'static crate::LintMeta {
        CognitiveComplexity::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let max_complexity = ctx.options.max_cognitive_complexity;

        // check each function declaration
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let ast::Declaration::Function { body, .. } = declaration else {
                continue;
            };

            let Some(body_id) = body else {
                continue;
            };

            // calculate cognitive complexity for this function
            let mut visitor = CognitiveVisitor {
                options: NodeVisitorOptions::default(),
                complexity: 0,
                nesting: 0,
                last_operator: None,
            };

            let body_expression = ctx.tree.get(*body_id);
            visitor.visit_expression(ctx.tree, *body_id, body_expression);

            if visitor.complexity > max_complexity {
                ctx.report(
                    LintDiagnostic::new(
                        COGNITIVE_COMPLEXITY.id,
                        COGNITIVE_COMPLEXITY.code,
                        COGNITIVE_COMPLEXITY.category,
                        severity,
                        format!(
                            "cognitive complexity {} exceeds maximum of {}",
                            visitor.complexity, max_complexity
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(*body_id),
                    )
                    .with_label("consider simplifying or extracting logic"),
                );
            }
        }
    }
}

/// Logical operators that contribute to cognitive complexity.
/// Sequences of the same operator only count once.
#[derive(Clone, Copy, PartialEq)]
enum LogicalOperator {
    /// Logical AND (`&&`).
    And,
    /// Logical OR (`||`).
    Or,
    /// Nullish coalescing (`??`).
    Coalesce,
}

/// Visitor that calculates cognitive complexity for a function body.
struct CognitiveVisitor {
    /// Node visitor options.
    options: NodeVisitorOptions,
    /// Accumulated complexity score.
    complexity: usize,
    /// Current nesting depth for control structures.
    nesting: usize,
    /// Last logical operator seen, for sequence deduplication.
    last_operator: Option<LogicalOperator>,
}

impl CognitiveVisitor {
    /// Add complexity with a nesting bonus.
    fn add_complexity(&mut self, base: usize) {
        self.complexity += base + self.nesting;
    }

    /// Add complexity without a nesting bonus (for logical operators).
    fn add_flat_complexity(&mut self, amount: usize) {
        self.complexity += amount;
    }
}

impl NodeVisitor for CognitiveVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // reset logical operator tracking for non-binary expressions
        let is_binary = matches!(expression, Expression::Binary { .. });
        if !is_binary {
            self.last_operator = None;
        }

        match expression {
            // if/else chains: if adds complexity with nesting penalty
            Expression::If { .. } => {
                // the if itself adds complexity (with nesting bonus)
                self.add_complexity(1);

                // increase nesting for children
                self.nesting += 1;

                // walk condition and then branch normally
                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.nesting -= 1;

                // don't walk again
                return;
            }
            // loops add complexity with nesting
            Expression::While { .. }
            | Expression::For { .. }
            | Expression::ForEach { .. }
            | Expression::Loop { .. } => {
                self.add_complexity(1);
                self.nesting += 1;

                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.nesting -= 1;
                return;
            }
            // match adds complexity with nesting
            Expression::Match { .. } => {
                self.add_complexity(1);
                self.nesting += 1;

                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.nesting -= 1;
                return;
            }
            // try/catch: catch adds complexity
            Expression::Try {
                catch_expression, ..
            } => {
                // try block increases nesting
                self.nesting += 1;

                // walk try expression
                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.nesting -= 1;

                // catch adds complexity if present
                if catch_expression.is_some() {
                    self.add_complexity(1);
                }

                return;
            }
            // logical operators: sequences of same operator count as 1
            Expression::Binary { operator, .. } => {
                let logical_operator = match operator {
                    BinaryOperator::And => Some(LogicalOperator::And),
                    BinaryOperator::Or => Some(LogicalOperator::Or),
                    BinaryOperator::Coalesce => Some(LogicalOperator::Coalesce),
                    _ => None,
                };

                if let Some(logical_operator) = logical_operator {
                    // only add complexity if this is a different operator or first in sequence
                    if self.last_operator != Some(logical_operator) {
                        self.add_flat_complexity(1);
                        self.last_operator = Some(logical_operator);
                    }
                }
            }
            _ => {}
        }

        // default: walk children
        destack_base::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_high_cognitive_complexity() {
        // nocheckin: set explicit complexity for cognitive/cyclomatic complexity tests
        let test = TestProgram::for_rule(CognitiveComplexity);
        // nested structures have higher cognitive complexity
        let result = test.lint_ast(
            "test.ds",
            r#"
function complex(a: bool, b: bool, c: bool) {
    if (a) {
        if (b) {
            if (c) {
                if (a) {
                    if (b) {
                        doSomething()
                    }
                }
            }
        }
    }
    if (a) {
        if (b) {
            if (c) {
                doMore()
            }
        }
    }
}
"#,
        );
        // deeply nested ifs accumulate: 1+0, 1+1, 1+2, 1+3, 1+4 = 15 for first block
        // then: 1+0, 1+1, 1+2 = 6 for second block
        // total = 21 > 15
        test.result(result).assert_lint("cognitive-complexity");
    }

    #[test]
    fn test_allows_simple_function() {
        let test = TestProgram::for_rule(CognitiveComplexity);
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
        // just 1 if = 1 complexity
        test.result(result).assert_no_lint("cognitive-complexity");
    }

    #[test]
    fn test_nesting_increases_complexity() {
        let test = TestProgram::for_rule(CognitiveComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function nested(a: bool) {
    if (a) {
        if (a) {
            if (a) {
                if (a) {
                    if (a) {
                        x()
                    }
                }
            }
        }
    }
}
"#,
        );
        // 1+0 + 1+1 + 1+2 + 1+3 + 1+4 = 1+2+3+4+5 = 15, exactly at limit
        test.result(result).assert_no_lint("cognitive-complexity");
    }

    #[test]
    fn test_flat_ifs_lower_complexity() {
        let test = TestProgram::for_rule(CognitiveComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function flat(a: bool) {
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
        // 10 flat ifs = 10 complexity < 15
        test.result(result).assert_no_lint("cognitive-complexity");
    }

    #[test]
    fn test_logical_operator_sequences() {
        let test = TestProgram::for_rule(CognitiveComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function logical(a: bool): bool {
    return a && a && a && a && a
}
"#,
        );
        // sequence of same operator counts as 1
        test.result(result).assert_no_lint("cognitive-complexity");
    }

    #[test]
    fn test_mixed_logical_operators() {
        let test = TestProgram::for_rule(CognitiveComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function mixed(a: bool): bool {
    return a && a || a && a || a && a || a
}
"#,
        );
        // alternating && and || each count: && || && || && || = 6
        test.result(result).assert_no_lint("cognitive-complexity");
    }

    #[test]
    fn test_loops_add_complexity() {
        let test = TestProgram::for_rule(CognitiveComplexity);
        let result = test.lint_ast(
            "test.ds",
            r#"
function loops() {
    while (true) {
        for (let i = 0; i < 10; i++) {
            for (const x of items) {
                loop {
                    if (done) { break }
                }
            }
        }
    }
}
"#,
        );
        // while: 1+0, for: 1+1, foreach: 1+2, loop: 1+3, if: 1+4 = 1+2+3+4+5 = 15
        test.result(result).assert_no_lint("cognitive-complexity");
    }
}
