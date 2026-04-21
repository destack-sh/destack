use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    assign_pattern_expression, expression_numeric_sign, expression_path_segments,
    expression_unwrap_parenthesized_source_form,
};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintMeta, LintRule, declare_lint};

declare_lint! {
    /// Disallow for loops that go in the wrong direction.
    ///
    /// A for loop with a counter moving in the wrong direction relative to its
    /// condition will never terminate or never execute. This is almost always a bug.
    #[lint(
        id = "for-direction",
        code = "LC001",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub ForDirection,
    "Disallow for loops with incorrect direction"
}

/// Direction of loop iteration.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    /// The counter moves upward.
    Increasing,
    /// The counter moves downward.
    Decreasing,
}

/// Expected loop direction for one condition counter.
struct CounterExpectation {
    /// The counter path segments.
    counter_segments: Vec<ast::StringId>,
    /// The expected update direction for this counter.
    expected_direction: Direction,
}

impl LintRule for ForDirection {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        ForDirection::meta()
    }

    /// Check module AST nodes for for-loop direction mismatches.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // walk expression nodes and inspect for loops
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::For {
                condition: Some(condition_id),
                increment: Some(increment_id),
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // collect counter expectations from the loop condition
            let expectations = condition_counter_expectations(ctx, *condition_id);
            if expectations.is_empty() {
                continue;
            }

            // find the first mismatched counter update direction
            let mut mismatch_direction = None;
            for expectation in expectations {
                let increment_direction =
                    update_direction_for_counter(ctx, *increment_id, &expectation.counter_segments);
                let Some(increment_direction) = increment_direction else {
                    continue;
                };

                // enforce this lint guard
                if increment_direction != expectation.expected_direction {
                    mismatch_direction = Some(expectation.expected_direction);
                    break;
                }
            }
            let Some(expected_direction) = mismatch_direction else {
                continue;
            };

            // resolve effective severity for this loop
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build the direction specific diagnostic text
            let direction_word = match expected_direction {
                Direction::Increasing => "increase",
                Direction::Decreasing => "decrease",
            };
            let mut diagnostic = LintDiagnostic::new(
                FOR_DIRECTION.id,
                FOR_DIRECTION.code,
                FOR_DIRECTION.category,
                severity,
                format!("for loop counter should {direction_word} to match condition"),
                ctx.module.file_id,
                ctx.tree.get_span(node_id),
            )
            .with_label("counter moves in wrong direction");

            // attach a direction flip fix when supported
            if ctx.compute_fixes
                && let Some(fix) = build_for_direction_fix(ctx, *increment_id, expected_direction)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            // report the lint
            ctx.report(diagnostic);
        }
    }
}

/// Build an unsafe fix that flips increment direction for one loop update.
fn build_for_direction_fix(
    ctx: &LintAstContext<'_>,
    increment_id: ast::LocalNodeId<ast::Expression>,
    expected_direction: Direction,
) -> Option<LintFix> {
    let replacement = increment_with_expected_direction(ctx, increment_id, expected_direction)?;
    let edits = ctx
        .edit_builder()
        .replace(ctx.tree.get_span(increment_id), replacement)
        .into_edits();

    Some(LintFix::r#unsafe("Flip loop update direction").with_edits(edits))
}

/// Render one increment expression with the expected direction.
fn increment_with_expected_direction(
    ctx: &LintAstContext<'_>,
    increment_id: ast::LocalNodeId<ast::Expression>,
    expected_direction: Direction,
) -> Option<String> {
    // inspect the increment expression shape
    let increment_id = expression_unwrap_parenthesized_source_form(ctx.tree, increment_id);
    let increment = ctx.tree.get(increment_id);

    // render a direction corrected update form
    match increment {
        ast::Expression::Unary { operator, right } => {
            // resolve operand source text
            let operand_text = ctx.get_span_text(ctx.tree.get_span(*right));

            // map unary operator to the opposite direction form
            let replacement = match (operator, expected_direction) {
                (ast::UnaryOperator::PostIncrement, Direction::Decreasing) => {
                    format!("{operand_text}--")
                }
                (ast::UnaryOperator::PostDecrement, Direction::Increasing) => {
                    format!("{operand_text}++")
                }
                (ast::UnaryOperator::PreIncrement, Direction::Decreasing) => {
                    format!("--{operand_text}")
                }
                (ast::UnaryOperator::PreDecrement, Direction::Increasing) => {
                    format!("++{operand_text}")
                }
                _ => return None,
            };

            Some(replacement)
        }
        ast::Expression::Assign {
            operator,
            left,
            right,
        } => {
            let Some(left_expression_id) = assign_pattern_expression(ctx.tree, *left) else {
                return None;
            };

            // map assignment operator to target direction
            let replacement_operator =
                assign_operator_with_expected_direction(*operator, expected_direction)?;

            // preserve original left and right expression text
            let left_text = ctx.get_span_text(ctx.tree.get_span(left_expression_id));
            let right_text = ctx.get_span_text(ctx.tree.get_span(*right));

            Some(format!("{left_text} {replacement_operator} {right_text}"))
        }
        _ => None,
    }
}

/// Return an assignment operator matching the expected direction.
fn assign_operator_with_expected_direction(
    operator: ast::AssignOperator,
    expected_direction: Direction,
) -> Option<&'static str> {
    match (operator, expected_direction) {
        (ast::AssignOperator::AddAssign, Direction::Decreasing) => Some("-="),
        (ast::AssignOperator::WrappingAddAssign, Direction::Decreasing) => Some("-%="),
        (ast::AssignOperator::SaturatingAddAssign, Direction::Decreasing) => Some("-|="),
        (ast::AssignOperator::SubtractAssign, Direction::Increasing) => Some("+="),
        (ast::AssignOperator::WrappingSubtractAssign, Direction::Increasing) => Some("+%="),
        (ast::AssignOperator::SaturatingSubtractAssign, Direction::Increasing) => Some("+|="),
        _ => None,
    }
}

/// Determine counter expectations from the loop condition.
fn condition_counter_expectations(
    ctx: &LintAstContext<'_>,
    condition_id: ast::LocalNodeId<ast::Expression>,
) -> Vec<CounterExpectation> {
    let condition_id = expression_unwrap_parenthesized_source_form(ctx.tree, condition_id);
    let condition = ctx.tree.get(condition_id);

    // require a binary comparison condition
    let ast::Expression::Binary {
        operator,
        left,
        right,
    } = condition
    else {
        return Vec::new();
    };

    // resolve expected direction for each comparison side
    let (left_direction, right_direction) = match operator {
        ast::BinaryOperator::LessThan | ast::BinaryOperator::LessThanOrEqual => {
            (Direction::Increasing, Direction::Decreasing)
        }
        ast::BinaryOperator::GreaterThan | ast::BinaryOperator::GreaterThanOrEqual => {
            (Direction::Decreasing, Direction::Increasing)
        }
        _ => return Vec::new(),
    };

    // collect expectations for counter paths found on either side
    let mut expectations = Vec::new();
    if let Some(counter_segments) = expression_path_segments(ctx.tree, *left) {
        expectations.push(CounterExpectation {
            counter_segments,
            expected_direction: left_direction,
        });
    }
    if let Some(counter_segments) = expression_path_segments(ctx.tree, *right) {
        expectations.push(CounterExpectation {
            counter_segments,
            expected_direction: right_direction,
        });
    }

    expectations
}

/// Determine update direction for a specific counter.
fn update_direction_for_counter(
    ctx: &mut LintAstContext<'_>,
    increment_id: ast::LocalNodeId<ast::Expression>,
    counter_segments: &[ast::StringId],
) -> Option<Direction> {
    // normalize increment expression shape
    let increment_id = expression_unwrap_parenthesized_source_form(ctx.tree, increment_id);
    let increment = ctx.tree.get(increment_id);

    // resolve update direction only for matching counter updates
    match increment {
        ast::Expression::Unary { operator, right } => {
            // require unary update target to match the counter path
            if !expression_matches_counter(ctx, *right, counter_segments) {
                return None;
            }

            // map unary increment operators to direction
            match operator {
                ast::UnaryOperator::PostIncrement | ast::UnaryOperator::PreIncrement => {
                    Some(Direction::Increasing)
                }
                ast::UnaryOperator::PostDecrement | ast::UnaryOperator::PreDecrement => {
                    Some(Direction::Decreasing)
                }
                _ => None,
            }
        }
        ast::Expression::Assign {
            operator,
            left,
            right,
        } => {
            let Some(left_expression_id) = assign_pattern_expression(ctx.tree, *left) else {
                return None;
            };

            // require assignment target to match the counter path
            if !expression_matches_counter(ctx, left_expression_id, counter_segments) {
                return None;
            }

            // resolve direction from operator and right side sign
            assignment_direction(ctx, *operator, *right)
        }
        _ => None,
    }
}

/// Determine assignment update direction using operator and step sign.
fn assignment_direction(
    ctx: &mut LintAstContext<'_>,
    operator: ast::AssignOperator,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Direction> {
    // resolve the signed step value
    let step_sign = expression_numeric_sign(ctx, right_id)?;

    // map assignment operator and sign to update direction
    match operator {
        ast::AssignOperator::AddAssign
        | ast::AssignOperator::WrappingAddAssign
        | ast::AssignOperator::SaturatingAddAssign => direction_from_sign(step_sign),
        ast::AssignOperator::SubtractAssign
        | ast::AssignOperator::WrappingSubtractAssign
        | ast::AssignOperator::SaturatingSubtractAssign => direction_from_sign(-step_sign),
        _ => None,
    }
}

/// Return update direction for a signed step.
fn direction_from_sign(sign: i8) -> Option<Direction> {
    // resolve positive steps
    if sign > 0 {
        return Some(Direction::Increasing);
    }

    // resolve negative steps
    if sign < 0 {
        return Some(Direction::Decreasing);
    }

    None
}

/// Return true when an expression resolves to the target counter path.
fn expression_matches_counter(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    counter_segments: &[ast::StringId],
) -> bool {
    // resolve expression path segments
    let Some(path_segments) = expression_path_segments(ctx.tree, expression_id) else {
        return false;
    };

    // compare path segments directly
    path_segments == counter_segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_wrong_direction_increment() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_increment.ds",
            r#"
for (let i = 0; i < 10; i--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_fix_flips_post_decrement_to_increment() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_fix_flips_post_decrement_to_increment.ds",
            r#"
for (let i = 0; i < 10; i--) {
    console.log(i);
}
"#,
        );
        test.result(result)
            .assert_lint("for-direction")
            .assert_has_fix("for-direction")
            .assert_unsafe_fixed(
                r#"
for (let i = 0; i < 10; i++) {
    console.log(i);
}
"#,
            );
    }

    #[test]
    fn test_detects_wrong_direction_decrement() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_decrement.ds",
            r#"
for (let i = 10; i > 0; i++) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_mutation_fix_flips_add_assign_to_subtract_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_mutation_fix_flips_add_assign_to_subtract_assign.ds",
            r#"
for (let i = 10; i > 0; i += 2) {
    console.log(i);
}
"#,
        );
        test.result(result)
            .assert_lint("for-direction")
            .assert_has_fix("for-direction")
            .assert_unsafe_fixed(
                r#"
for (let i = 10; i > 0; i -= 2) {
    console.log(i);
}
"#,
            );
    }

    #[test]
    fn test_detects_wrong_direction_less_equal() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_less_equal.ds",
            r#"
for (let i = 0; i <= 10; i--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_detects_wrong_direction_greater_equal() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_greater_equal.ds",
            r#"
for (let i = 10; i >= 0; i++) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_correct_direction_increment() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_correct_direction_increment.ds",
            r#"
for (let i = 0; i < 10; i++) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_correct_direction_decrement() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_correct_direction_decrement.ds",
            r#"
for (let i = 10; i > 0; i--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_correct_direction_add_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_correct_direction_add_assign.ds",
            r#"
for (let i = 0; i < 10; i += 2) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_correct_direction_subtract_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_correct_direction_subtract_assign.ds",
            r#"
for (let i = 10; i > 0; i -= 2) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_no_condition() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_no_condition.ds",
            r#"
for (;;) {
    break;
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_allows_update_on_other_variable() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_allows_update_on_other_variable.ds",
            r#"
let j = 10;
for (let i = 0; i < 10; j--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_detects_wrong_direction_with_counter_on_right() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_with_counter_on_right.ds",
            r#"
for (let i = 0; 10 > i; i--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_allows_correct_direction_with_counter_on_right() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_allows_correct_direction_with_counter_on_right.ds",
            r#"
for (let i = 0; 10 > i; i++) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_detects_negative_step_add_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_negative_step_add_assign.ds",
            r#"
for (let i = 0; i < 10; i += -1) {
    console.log(i);
}
"#,
        );
        test.result(result)
            .assert_lint("for-direction")
            .assert_has_no_fix("for-direction");
    }

    #[test]
    fn test_detects_folded_negative_step_add_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_folded_negative_step_add_assign.ds",
            r#"
for (let i = 0; i < 10; i += 2 - 3) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_fix_drops_parenthesized_update_wrapper() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_fix_drops_parenthesized_update_wrapper.ds",
            r#"
for (let i = 0; i < 10; (i--)) {
    console.log(i);
}
"#,
        );
        test.result(result)
            .assert_lint("for-direction")
            .assert_has_fix("for-direction")
            .assert_unsafe_fixed(
                r#"
for (let i = 0; i < 10; i++) {
    console.log(i);
}
"#,
            );
    }

    #[test]
    fn test_allows_unknown_step_add_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_allows_unknown_step_add_assign.ds",
            r#"
let step = -1;
for (let i = 0; i < 10; i += step) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }
}
