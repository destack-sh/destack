use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression, walk_match_case};
use destack_workspace::LintSeverity;

use crate::rules::common::{TypeNullishness, TypeTruthiness, type_nullishness, type_truthiness};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow conditions that are always truthy, always falsy, or nullish-fixed.
    ///
    /// This catches conditions that cannot change outcome based on their
    /// static type and nullish coalescing where the left side is known.
    #[lint(
        id = "no-unnecessary-condition",
        code = "LC030",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoUnnecessaryCondition,
    "Disallow conditions that are always truthy, always falsy, or nullish-fixed"
}

impl LintRule for NoUnnecessaryCondition {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnnecessaryCondition::meta()
    }

    /// Check module DIR nodes for always-fixed conditions.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = UnnecessaryConditionVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor for unnecessary condition checks.
struct UnnecessaryConditionVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> UnnecessaryConditionVisitor<'a, 'b> {
    /// Build a visitor for unnecessary condition checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one condition expression.
    fn check_condition(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        context_label: &'static str,
    ) {
        let expression = self.ctx.tree.get(expression_id);

        // preserve condition semantics for explicit negation
        let (diagnostic_id, truthiness) = if let dir::Expression::Unary {
            operator: dir::UnaryOperator::Not,
            right,
        } = expression
        {
            let Some(type_id) = self.ctx.expression_type_id(*right) else {
                return;
            };
            let truthiness = type_truthiness(self.ctx.types, self.ctx.strings, type_id);
            let truthiness = match truthiness {
                TypeTruthiness::AlwaysTruthy => TypeTruthiness::AlwaysFalsy,
                TypeTruthiness::AlwaysFalsy => TypeTruthiness::AlwaysTruthy,
                TypeTruthiness::Unknown => TypeTruthiness::Unknown,
            };
            (expression_id, truthiness)
        } else {
            let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
                return;
            };
            let truthiness = type_truthiness(self.ctx.types, self.ctx.strings, type_id);
            (expression_id, truthiness)
        };

        // resolve values for this check
        let (message, label) = match truthiness {
            TypeTruthiness::AlwaysTruthy => (
                format!("{context_label} is always truthy"),
                "this condition always evaluates to true",
            ),
            TypeTruthiness::AlwaysFalsy => (
                format!("{context_label} is always falsy"),
                "this condition always evaluates to false",
            ),
            TypeTruthiness::Unknown => return,
        };

        // resolve effective lint severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // resolve diagnostic span
        let span = self.ctx.get_span(diagnostic_id);
        self.ctx.report(
            LintReport::new(
                NO_UNNECESSARY_CONDITION.id,
                NO_UNNECESSARY_CONDITION.code,
                NO_UNNECESSARY_CONDITION.category,
                severity,
                message,
                span,
            )
            .label(label),
        );
    }

    /// Check one nullish coalescing expression.
    fn check_coalesce(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
    ) {
        let Some(type_id) = self.ctx.expression_type_id(left_id) else {
            return;
        };
        let nullishness = type_nullishness(self.ctx.types, type_id);

        // resolve values for this check
        let (message, label) = match nullishness {
            TypeNullishness::Never => (
                "left side of ?? is never nullish".to_string(),
                "the right side is unreachable",
            ),
            TypeNullishness::Always => (
                "left side of ?? is always nullish".to_string(),
                "the left side is unreachable",
            ),
            TypeNullishness::Maybe => return,
        };

        // skip disabled diagnostics
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the full expression span
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_UNNECESSARY_CONDITION.id,
                NO_UNNECESSARY_CONDITION.code,
                NO_UNNECESSARY_CONDITION.category,
                severity,
                message,
                span,
            )
            .label(label),
        );
    }

    /// Check one logical short circuit expression.
    fn check_logical(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
    ) {
        let Some(type_id) = self.ctx.expression_type_id(left_id) else {
            return;
        };
        let truthiness = type_truthiness(self.ctx.types, self.ctx.strings, type_id);

        // map operator and truthiness to one unreachable or redundant branch message
        let (message, label) = match (operator, truthiness) {
            (dir::BinaryOperator::And, TypeTruthiness::AlwaysFalsy) => (
                "left side of && is always falsy".to_string(),
                "the right side is unreachable",
            ),
            (dir::BinaryOperator::And, TypeTruthiness::AlwaysTruthy) => (
                "left side of && is always truthy".to_string(),
                "the left side is redundant",
            ),
            (dir::BinaryOperator::Or, TypeTruthiness::AlwaysTruthy) => (
                "left side of || is always truthy".to_string(),
                "the right side is unreachable",
            ),
            (dir::BinaryOperator::Or, TypeTruthiness::AlwaysFalsy) => (
                "left side of || is always falsy".to_string(),
                "the left side is redundant",
            ),
            _ => return,
        };

        // skip disabled diagnostics
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the full expression span
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_UNNECESSARY_CONDITION.id,
                NO_UNNECESSARY_CONDITION.code,
                NO_UNNECESSARY_CONDITION.category,
                severity,
                message,
                span,
            )
            .label(label),
        );
    }

    /// Check one loop condition expression.
    fn check_loop_condition(&mut self, condition_id: dir::LocalNodeId<dir::Expression>) {
        self.check_condition(condition_id, "loop condition");
    }
}

impl NodeVisitor for UnnecessaryConditionVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::If { condition, .. } => {
                if let dir::IfCondition::Expression { condition } = condition {
                    self.check_condition(*condition, "if condition");
                }
            }
            dir::Expression::Loop {
                condition: Some(condition),
                ..
            } => {
                self.check_loop_condition(*condition);
            }
            dir::Expression::For {
                condition: Some(condition),
                ..
            } => {
                self.check_loop_condition(*condition);
            }
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Coalesce,
                ..
            } => {
                self.check_coalesce(id, *left);
            }
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::And,
                ..
            } => {
                self.check_logical(id, *left, dir::BinaryOperator::And);
            }
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                ..
            } => {
                self.check_logical(id, *left, dir::BinaryOperator::Or);
            }
            _ => {}
        }

        walk_expression(self, tree, id, expression);
    }

    fn visit_match_case(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        let selector = match match_case {
            dir::MatchCase::Expression { selector, .. } => selector,
            dir::MatchCase::Block { selector, .. } => selector,
        };
        if let dir::MatchSelector::Pattern {
            guard: Some(guard), ..
        } = selector
        {
            self.check_condition(*guard, "match guard");
        }

        walk_match_case(self, tree, id, match_case);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag always truthy object conditions.
    #[test]
    fn test_flags_always_truthy_object_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_always_truthy_object_condition.ts",
            r#"
let value: { x: number } = { x: 1 };
if (value) {
}
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag always falsy null conditions.
    #[test]
    fn test_flags_always_falsy_null_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_always_falsy_null_condition.ts",
            r#"
let value: null = null;
if (value) {
}
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow normal boolean conditions.
    #[test]
    fn test_allows_boolean_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_boolean_condition.ts",
            r#"
let value: boolean = true;
if (value) {
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Flag negated conditions with fixed truthiness.
    #[test]
    fn test_flags_negated_fixed_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_negated_fixed_condition.ts",
            r#"
let value: { x: number } = { x: 1 };
if (!value) {
}
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow maybe-nullish union conditions.
    #[test]
    fn test_allows_maybe_nullish_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_maybe_nullish_condition.ts",
            r#"
let value: string | null = "ready";
if (value) {
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Allow negated conditions with unknown truthiness.
    #[test]
    fn test_allows_negated_unknown_condition() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_negated_unknown_condition.ts",
            r#"
let value: boolean = true;
if (!value) {
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Flag nullish coalescing with never-nullish left side.
    #[test]
    fn test_flags_coalesce_left_never_nullish() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_coalesce_left_never_nullish.ts",
            r#"
let value: string = "ready";
let output = value ?? "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag nullish coalescing with always-nullish left side.
    #[test]
    fn test_flags_coalesce_left_always_nullish() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_coalesce_left_always_nullish.ts",
            r#"
let value: null = null;
let output = value ?? "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow nullish coalescing with maybe-nullish left side.
    #[test]
    fn test_allows_coalesce_left_maybe_nullish() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_coalesce_left_maybe_nullish.ts",
            r#"
let value: string | null = null;
let output = value ?? "fallback";
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Flag coalescing with never-nullish array values.
    #[test]
    fn test_flags_coalesce_left_array_never_nullish() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_coalesce_left_array_never_nullish.ts",
            r#"
let value: string[] = [];
let output = value ?? ["fallback"];
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow coalescing when left side is unknown.
    #[test]
    fn test_allows_coalesce_left_unknown() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_coalesce_left_unknown.ts",
            r#"
let value: unknown = null;
let output = value ?? "fallback";
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Flag coalescing when left side is void.
    #[test]
    fn test_flags_coalesce_left_void() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_coalesce_left_void.ts",
            r#"
declare function fetchNothing(): void;
let value = fetchNothing();
let output = value ?? "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag && with always truthy left side.
    #[test]
    fn test_flags_and_left_always_truthy() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_and_left_always_truthy.ts",
            r#"
let value: { ready: boolean } = { ready: true };
let output = value && "ok";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag && with always falsy left side.
    #[test]
    fn test_flags_and_left_always_falsy() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_and_left_always_falsy.ts",
            r#"
let value: null = null;
let output = value && "ok";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag || with always truthy left side.
    #[test]
    fn test_flags_or_left_always_truthy() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_or_left_always_truthy.ts",
            r#"
let value: { ready: boolean } = { ready: true };
let output = value || "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag || with always falsy left side.
    #[test]
    fn test_flags_or_left_always_falsy() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_or_left_always_falsy.ts",
            r#"
let value: null = null;
let output = value || "fallback";
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow logical operators with normal boolean left side.
    #[test]
    fn test_allows_logical_left_boolean() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_logical_left_boolean.ts",
            r#"
let value: boolean = true;
let a = value && "ok";
let b = value || "fallback";
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }

    /// Flag explicit `while (true)` loop conditions.
    #[test]
    fn test_flags_loop_condition_true_literal() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_loop_condition_true_literal.ts",
            r#"
while (true) {
    break;
}
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Flag explicit `while (false)` loop conditions.
    #[test]
    fn test_flags_loop_condition_false_literal() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_flags_loop_condition_false_literal.ts",
            r#"
while (false) {
    break;
}
"#,
        );
        test.result(result).assert_lint("no-unnecessary-condition");
    }

    /// Allow maybe truthy match guards.
    #[test]
    fn test_allows_maybe_truthy_match_guard() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryCondition);
        let result = test.lint_dir(
            "no_unnecessary_condition/test_allows_maybe_truthy_match_guard.ts",
            r#"
let value: string | null = null;
match value {
    _ if value => {}
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-condition");
    }
}
