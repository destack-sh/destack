use std::collections::HashSet;

use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression, walk_match_case};
use destack_workspace::LintSeverity;

use crate::rules::common::is_strict_boolean_type;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow non-boolean values in boolean contexts.
    ///
    /// This enforces explicit comparisons instead of relying on truthy and falsy coercion.
    #[lint(
        id = "strict-boolean-expressions",
        code = "LR032",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub StrictBooleanExpressions,
    "Disallow non-boolean values in boolean contexts"
}

impl LintRule for StrictBooleanExpressions {
    fn meta(&self) -> &'static LintMeta {
        StrictBooleanExpressions::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // run the boolean context visitor
        let mut visitor = StrictBooleanVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that checks boolean contexts.
struct StrictBooleanVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The expressions already checked for boolean contexts.
    visited: HashSet<u32>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> StrictBooleanVisitor<'a, 'b> {
    /// Build a visitor for boolean expression checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // prepare initial visitor state
        Self {
            ctx,
            meta,
            visited: HashSet::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // walk the module expression tree
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }
}

impl NodeVisitor for StrictBooleanVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // match boolean contexts
        match expression {
            dir::Expression::If { condition, .. } => {
                if let dir::IfCondition::Expression { condition } = condition {
                    check_is_boolean(
                        self.ctx,
                        self.meta,
                        *condition,
                        "if condition",
                        &mut self.visited,
                    );
                }
            }
            dir::Expression::Loop {
                condition: Some(condition),
                ..
            } => {
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *condition,
                    "loop condition",
                    &mut self.visited,
                );
            }
            dir::Expression::For {
                condition: Some(condition),
                ..
            } => {
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *condition,
                    "for condition",
                    &mut self.visited,
                );
            }
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } => {
                // ensure the negated operand is boolean
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *right,
                    "negation operand",
                    &mut self.visited,
                );
            }
            dir::Expression::Binary {
                operator: dir::BinaryOperator::And | dir::BinaryOperator::Or,
                left,
                ..
            } => {
                // value-producing logical expressions only use the left operand
                // as a boolean gate unless the whole expression is itself in a
                // boolean context handled by a parent visitor case
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *left,
                    "logical operand",
                    &mut self.visited,
                );
            }
            dir::Expression::AssignBinary {
                operator: dir::AssignOperator::AndAssign | dir::AssignOperator::OrAssign,
                left,
                ..
            } => {
                // value-producing logical assignments only use the existing
                // target value as a boolean gate
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *left,
                    "logical assignment target",
                    &mut self.visited,
                );
            }
            _ => {}
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }

    fn visit_match_case(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        // enforce boolean guards in match cases
        let selector = match match_case {
            dir::MatchCase::Expression { selector, .. } => selector,
            dir::MatchCase::Block { selector, .. } => selector,
        };
        if let dir::MatchSelector::Pattern {
            guard: Some(guard), ..
        } = selector
        {
            check_is_boolean(
                self.ctx,
                self.meta,
                *guard,
                "match guard",
                &mut self.visited,
            );
        }

        // walk match case children
        walk_match_case(self, tree, id, match_case);
    }
}

/// Check a boolean context expression and report non-boolean usage.
fn check_is_boolean(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
    context: &str,
    visited: &mut HashSet<u32>,
) {
    // avoid repeating work for shared expressions
    if !visited.insert(expression_id.id) {
        return;
    }

    // expand boolean operators into their operands
    let expression = ctx.tree.get(expression_id);
    match expression {
        dir::Expression::Unary {
            operator: dir::UnaryOperator::Not,
            right,
        } => {
            check_is_boolean(ctx, meta, *right, "negation operand", visited);
            return;
        }
        dir::Expression::Binary {
            operator: dir::BinaryOperator::And | dir::BinaryOperator::Or,
            left,
            right,
        } => {
            check_is_boolean(ctx, meta, *left, "logical operand", visited);
            check_is_boolean(ctx, meta, *right, "logical operand", visited);
            return;
        }
        _ => {}
    }

    // check for strictly boolean types
    let Some(type_id) = ctx.expression_type_id(expression_id) else {
        return;
    };
    if is_strict_boolean_type(ctx.types, type_id) {
        return;
    }

    // honor per node severity
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return;
    }

    // report the diagnostic
    let span = ctx.get_span(expression_id);
    ctx.report(
        LintDiagnostic::new(
            STRICT_BOOLEAN_EXPRESSIONS.id,
            STRICT_BOOLEAN_EXPRESSIONS.code,
            STRICT_BOOLEAN_EXPRESSIONS.category,
            severity,
            format!("non-boolean expression in {context}"),
            ctx.module.file_id,
            span,
        )
        .with_label("use an explicit comparison"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_boolean_condition() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_allows_boolean_condition.ds",
            r#"
let is_ready = true
if (is_ready) {
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("strict-boolean-expressions");
    }

    #[test]
    fn test_flags_non_boolean_condition() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_flags_non_boolean_condition.ds",
            r#"
let count = 1
if (count) {
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }

    #[test]
    fn test_flags_negation_operand() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_flags_negation_operand.ds",
            r#"
let name = "destack"
if (!name) {
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }

    /// Allow comparison results in boolean conditions.
    #[test]
    fn test_allows_comparison_condition() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_allows_comparison_condition.ds",
            r#"
let count = 1
if (count > 0) {
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("strict-boolean-expressions");
    }

    /// Flag non boolean operands in logical operators.
    #[test]
    fn test_flags_logical_operands() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_flags_logical_operands.ds",
            r#"
let count = 1
let is_ready = true
if (count && is_ready) {
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint_count("strict-boolean-expressions", 1);
    }

    /// Allow logical operators when both operands are boolean.
    #[test]
    fn test_allows_logical_expression_values() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_allows_logical_expression_values.ds",
            r#"
let is_ready: boolean = true
let is_valid: boolean = false
let is_ok = is_ready && is_valid
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("strict-boolean-expressions");
    }

    /// Flag only the gating operand in value-producing logical expressions.
    #[test]
    fn test_flags_logical_expression_values() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_flags_logical_expression_values.ds",
            r#"
let count = 1
let name = "destack"
let fallback = count || name
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint_count("strict-boolean-expressions", 1);
    }

    /// Allow logical assignments when operands are boolean.
    #[test]
    fn test_allows_logical_assignment() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_allows_logical_assignment.ds",
            r#"
let is_ready: boolean = true
let is_valid: boolean = false
is_ready &&= is_valid
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("strict-boolean-expressions");
    }

    /// Flag non-boolean logical assignments.
    #[test]
    fn test_flags_logical_assignment() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_flags_logical_assignment.ds",
            r#"
let count = 1
let fallback = 2
count ||= fallback
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint_count("strict-boolean-expressions", 1);
    }

    /// Flag non boolean loop conditions.
    #[test]
    fn test_flags_loop_condition() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_flags_loop_condition.ds",
            r#"
let count = 1
while (count) {
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }

    /// Flag non boolean match guards.
    #[test]
    fn test_flags_match_guard() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_flags_match_guard.ds",
            r#"
let value = 1
match (value) {
    1 if value => "one"
    _ => "other"
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }

    /// Flag non boolean ternary conditions.
    #[test]
    fn test_flags_ternary_condition() {
        let test = TestProgram::for_rule_with_prelude(StrictBooleanExpressions);
        let result = test.lint_dir(
            "strict_boolean_expressions/test_flags_ternary_condition.ds",
            r#"
let count = 1
let value = if (count) { 1 } else { 2 }
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }
}
