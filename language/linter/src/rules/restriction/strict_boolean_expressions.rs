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
        code = "LR035",
        category = Restriction,
        level = Dir,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub StrictBooleanExpressions,
    "Disallow non-boolean values in boolean contexts"
}

impl LintRule for StrictBooleanExpressions {
    fn meta(&self) -> &'static crate::LintMeta {
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
    visited: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> StrictBooleanVisitor<'a, 'b> {
    /// Build a visitor for boolean expression checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a crate::LintMeta) -> Self {
        // prepare initial visitor state
        Self {
            ctx,
            meta,
            visited: Vec::new(),
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
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // match boolean contexts
        match expression {
            dir::Expression::If { condition, .. } => {
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *condition,
                    "if condition",
                    &mut self.visited,
                );
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
                right,
                ..
            } => {
                // enforce boolean operands for logical expressions
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *left,
                    "logical operand",
                    &mut self.visited,
                );
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *right,
                    "logical operand",
                    &mut self.visited,
                );
            }
            dir::Expression::AssignBinary {
                operator: dir::AssignOperator::AndAssign | dir::AssignOperator::OrAssign,
                left,
                right,
            } => {
                // enforce boolean operands for logical assignments
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *left,
                    "logical assignment target",
                    &mut self.visited,
                );
                check_is_boolean(
                    self.ctx,
                    self.meta,
                    *right,
                    "logical assignment value",
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
        tree: &dir::NodeTree,
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
    meta: &crate::LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
    context: &str,
    visited: &mut Vec<dir::LocalNodeId<dir::Expression>>,
) {
    // avoid repeating work for shared expressions
    if visited
        .iter()
        .any(|visited_id| visited_id.id == expression_id.id)
    {
        return;
    }
    visited.push(expression_id);

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
    use crate::LintLevel;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_boolean_condition() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let is_ready = true
if (is_ready) {
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("strict-boolean-expressions");
    }

    #[test]
    fn test_flags_non_boolean_condition() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let count = 1
if (count) {
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }

    #[test]
    fn test_flags_negation_operand() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let name = "destack"
if (!name) {
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }

    /// Allow comparison results in boolean conditions.
    #[test]
    fn test_allows_comparison_condition() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let count = 1
if (count > 0) {
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("strict-boolean-expressions");
    }

    /// Flag non boolean operands in logical operators.
    #[test]
    fn test_flags_logical_operands() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let count = 1
let is_ready = true
if (count && is_ready) {
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint_count("strict-boolean-expressions", 1);
    }

    /// Allow logical operators when both operands are boolean.
    #[test]
    fn test_allows_logical_expression_values() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let is_ready: boolean = true
let is_valid: boolean = false
let is_ok = is_ready && is_valid
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("strict-boolean-expressions");
    }

    /// Flag non-boolean logical expressions even outside conditions.
    #[test]
    fn test_flags_logical_expression_values() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let count = 1
let name = "destack"
let fallback = count || name
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint_count("strict-boolean-expressions", 2);
    }

    /// Allow logical assignments when operands are boolean.
    #[test]
    fn test_allows_logical_assignment() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let is_ready: boolean = true
let is_valid: boolean = false
is_ready &&= is_valid
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("strict-boolean-expressions");
    }

    /// Flag non-boolean logical assignments.
    #[test]
    fn test_flags_logical_assignment() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let count = 1
let is_ready: boolean = true
is_ready ||= count
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }

    /// Flag non boolean loop conditions.
    #[test]
    fn test_flags_loop_condition() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let count = 1
while (count) {
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }

    /// Flag non boolean match guards.
    #[test]
    fn test_flags_match_guard() {
        let test = TestProgram::for_rule_with_builtins(StrictBooleanExpressions);
        let result = test.lint(
            "test.ds",
            r#"
let value = 1
match (value) {
    1 if value => "one"
    _ => "other"
}
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("strict-boolean-expressions");
    }
}
