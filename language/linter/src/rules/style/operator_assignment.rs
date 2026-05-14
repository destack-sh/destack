use crate::LintMeta;
use destack_dir::{self as dir, AssignOperator, BinaryOperator, ScalarLiteral};
use destack_workspace::{LintSeverity, OperatorAssignmentMode};

use crate::rules::common::{
    assign_pattern_expression, expression_is_equal, expression_path_segments,
    expression_unwrap_parenthesized_source_form, span_has_comment,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer compound assignment operators.
    ///
    /// Use `x += 1` instead of `x = x + 1` for brevity and clarity.
    #[lint(
        id = "operator-assignment",
        code = "LY028",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub OperatorAssignment,
    "Prefer compound assignment operators"
}

impl LintRule for OperatorAssignment {
    fn meta(&self) -> &'static LintMeta {
        OperatorAssignment::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // scan all assignment expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Assign {
                left,
                operator,
                right,
            } = ctx.dir.get(node_id)
            else {
                continue;
            };

            if ctx.options.style.operator_assignment_mode == OperatorAssignmentMode::Never {
                self.check_disallowed_shorthand_assignment(ctx, meta, node_id, *left, *operator);
                continue;
            }

            // only plain assignments can be replaced with shorthand
            if *operator != AssignOperator::Assign {
                continue;
            }

            // normalize the assignment and binary shapes
            let normalized_right_id =
                expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *right);
            let dir::Expression::Binary {
                left: binary_left_id,
                operator: binary_operator,
                right: binary_right_id,
            } = ctx.dir.get(normalized_right_id)
            else {
                continue;
            };

            // keep only binary operators that have shorthand assignment forms
            let Some(shorthand) = shorthand_assignment_operator(*binary_operator) else {
                continue;
            };

            // resolve normalized operands for structural comparison
            let Some(assignment_left_id) = assign_pattern_expression(ctx.dir.tree(), *left) else {
                continue;
            };
            let normalized_assignment_left_id =
                expression_unwrap_parenthesized_source_form(ctx.dir.tree(), assignment_left_id);
            let normalized_binary_left_id =
                expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *binary_left_id);
            let normalized_binary_right_id =
                expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *binary_right_id);

            // report when assignment target appears on binary left side
            let left_matches_left = expression_is_equal(
                ctx,
                normalized_assignment_left_id,
                normalized_binary_left_id,
            );

            // report commutative right side matches without automatic fixes
            let left_matches_right = shorthand.is_commutative
                && expression_is_equal(
                    ctx,
                    normalized_assignment_left_id,
                    normalized_binary_right_id,
                );
            if !left_matches_left && !left_matches_right {
                continue;
            }

            // skip disabled severities
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                OPERATOR_ASSIGNMENT.id,
                OPERATOR_ASSIGNMENT.code,
                OPERATOR_ASSIGNMENT.category,
                severity,
                format!(
                    "assignment can be simplified with `{}`",
                    shorthand.assignment_text
                ),
                expression_span,
            )
            .label(format!("use `{}` instead", shorthand.assignment_text));

            // add safe fixes only for left side replacement candidates
            if ctx.compute_fixes
                && left_matches_left
                && can_fix_assignment_target(ctx, normalized_assignment_left_id)
                && !span_has_comment(ctx.dir.tree(), expression_span)
            {
                let left_text = ctx.get_span_text(ctx.dir.get_span(*left));
                let right_text = ctx.get_span_text(ctx.dir.get_span(*binary_right_id));
                let replacement = format!("{left_text} {} {right_text}", shorthand.assignment_text);
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe(format!("Replace with `{}`", shorthand.assignment_text))
                    .with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

impl OperatorAssignment {
    /// Report shorthand assignment operators when the mode disallows them.
    fn check_disallowed_shorthand_assignment(
        &self,
        ctx: &mut LintModuleContext<'_>,
        meta: &'static LintMeta,
        node_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::AssignPattern>,
        operator: AssignOperator,
    ) {
        let Some(left_id) = assign_pattern_expression(ctx.dir.tree(), left_id) else {
            return;
        };

        let Some(binary_text) = expanded_assignment_operator_text(operator) else {
            return;
        };

        let severity = ctx.get_effective_severity(meta, node_id);
        if !severity.is_enabled() {
            return;
        }

        let expression_span = ctx.dir.get_span(node_id);
        let left_text = ctx.get_span_text(ctx.dir.get_span(left_id));
        ctx.report(
            LintReport::new(
                OPERATOR_ASSIGNMENT.id,
                OPERATOR_ASSIGNMENT.code,
                OPERATOR_ASSIGNMENT.category,
                severity,
                format!(
                    "unexpected shorthand assignment `{}`",
                    assignment_operator_text(operator)
                ),
                expression_span,
            )
            .label(format!(
                "use `{left_text} = {left_text} {binary_text} …` instead"
            )),
        );
    }
}

/// Describe one shorthand assignment mapping.
struct ShorthandAssignment {
    /// The replacement assignment token text.
    assignment_text: &'static str,
    /// Whether the operator is commutative for right side matching.
    is_commutative: bool,
}

/// Return shorthand mapping for one binary operator.
fn shorthand_assignment_operator(operator: BinaryOperator) -> Option<ShorthandAssignment> {
    let mapping = match operator {
        BinaryOperator::Add => ("+=", false),
        BinaryOperator::Subtract => ("-=", false),
        BinaryOperator::Multiply => ("*=", true),
        BinaryOperator::Exponent => ("**=", false),
        BinaryOperator::Divide => ("/=", false),
        BinaryOperator::Remainder => ("%=", false),
        BinaryOperator::ShiftLeft => ("<<=", false),
        BinaryOperator::ShiftRight => (">>=", false),
        BinaryOperator::UnsignedShiftRight => (">>>=", false),
        BinaryOperator::ElementwiseAnd => ("&=", true),
        BinaryOperator::ElementwiseXor => ("^=", true),
        BinaryOperator::ElementwiseOr => ("|=", true),
        _ => return None,
    };

    Some(ShorthandAssignment {
        assignment_text: mapping.0,
        is_commutative: mapping.1,
    })
}

/// Return the assignment token text for one shorthand assignment operator.
fn assignment_operator_text(operator: AssignOperator) -> &'static str {
    match operator {
        AssignOperator::Assign => "=",
        AssignOperator::MultiplyAssign => "*=",
        AssignOperator::ExponentAssign => "**=",
        AssignOperator::DivideAssign => "/=",
        AssignOperator::RemainderAssign => "%=",
        AssignOperator::AddAssign => "+=",
        AssignOperator::SubtractAssign => "-=",
        AssignOperator::ShiftLeftAssign => "<<=",
        AssignOperator::ShiftRightAssign => ">>=",
        AssignOperator::UnsignedShiftRightAssign => ">>>=",
        AssignOperator::ElementwiseAndAssign => "&=",
        AssignOperator::ElementwiseXorAssign => "^=",
        AssignOperator::ElementwiseOrAssign => "|=",
        AssignOperator::AndAssign => "&&=",
        AssignOperator::OrAssign => "||=",
        AssignOperator::CoalesceAssign => "??=",
    }
}

/// Return the expanded binary operator text for one shorthand assignment operator.
fn expanded_assignment_operator_text(operator: AssignOperator) -> Option<&'static str> {
    match operator {
        AssignOperator::Assign => None,
        AssignOperator::MultiplyAssign => Some("*"),
        AssignOperator::ExponentAssign => Some("**"),
        AssignOperator::DivideAssign => Some("/"),
        AssignOperator::RemainderAssign => Some("%"),
        AssignOperator::AddAssign => Some("+"),
        AssignOperator::SubtractAssign => Some("-"),
        AssignOperator::ShiftLeftAssign => Some("<<"),
        AssignOperator::ShiftRightAssign => Some(">>"),
        AssignOperator::UnsignedShiftRightAssign => Some(">>>"),
        AssignOperator::ElementwiseAndAssign => Some("&"),
        AssignOperator::ElementwiseXorAssign => Some("^"),
        AssignOperator::ElementwiseOrAssign => Some("|"),
        AssignOperator::AndAssign | AssignOperator::OrAssign | AssignOperator::CoalesceAssign => {
            None
        }
    }
}

/// Return true when one assignment target can be safely auto fixed.
fn can_fix_assignment_target(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);

    match expression {
        // bare references are safe rewrite targets
        dir::Expression::Identifier { .. }
        | dir::Expression::QualifiedReference { .. }
        | dir::Expression::This => true,

        // dot member targets are safe when their receiver is stable
        dir::Expression::Member { left, .. } => {
            let object_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *left);
            expression_path_segments(ctx.dir.tree(), object_id).is_some()
                || matches!(ctx.dir.get(object_id), dir::Expression::This)
        }

        // bracket member targets are safe when receiver and index are stable
        dir::Expression::Index { left, index, .. } => {
            let object_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), *left);
            let object_is_stable = expression_path_segments(ctx.dir.tree(), object_id).is_some()
                || matches!(ctx.dir.get(object_id), dir::Expression::This);

            let index_is_stable_literal = index.is_some_and(|index_id| {
                let index_id =
                    expression_unwrap_parenthesized_source_form(ctx.dir.tree(), index_id);
                matches!(
                    ctx.dir.get(index_id),
                    dir::Expression::ScalarLiteral(ScalarLiteral::String(_))
                        | dir::Expression::ScalarLiteral(ScalarLiteral::Integer(_))
                        | dir::Expression::ScalarLiteral(ScalarLiteral::Float(_))
                        | dir::Expression::ScalarLiteral(ScalarLiteral::Bigint(_))
                )
            });

            object_is_stable && index_is_stable_literal
        }

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_add_assignment() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_detects_add_assignment.ds",
            r#"
let x = 1
x = x + 1
"#,
        );
        test.result(result)
            .assert_lint("operator-assignment")
            .assert_safe_fixed(
                r#"
let x = 1;
x += 1;
"#,
            );
    }

    #[test]
    fn test_detects_member_assignment() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_detects_member_assignment.ds",
            r#"
foo.bar = foo.bar + baz
"#,
        );
        test.result(result)
            .assert_lint("operator-assignment")
            .assert_safe_fixed(
                r#"
foo.bar += baz;
"#,
            );
    }

    #[test]
    fn test_detects_unsigned_shift_assignment() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_detects_unsigned_shift_assignment.ds",
            r#"
x = x >>> y
"#,
        );
        test.result(result)
            .assert_lint("operator-assignment")
            .assert_safe_fixed(
                r#"
x >>>= y;
"#,
            );
    }

    #[test]
    fn test_detects_commutative_right_side_without_fix() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_detects_commutative_right_side_without_fix.ds",
            r#"
x = y * x
"#,
        );
        test.result(result)
            .assert_lint("operator-assignment")
            .assert_has_no_fix("operator-assignment");
    }

    #[test]
    fn test_allows_non_commutative_right_side_match() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_allows_non_commutative_right_side_match.ds",
            r#"
x = y - x
"#,
        );
        test.result(result).assert_no_lint("operator-assignment");
    }

    #[test]
    fn test_allows_logical_expression_assignment() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_allows_logical_expression_assignment.ds",
            r#"
x = x && y
"#,
        );
        test.result(result).assert_no_lint("operator-assignment");
    }

    #[test]
    fn test_detects_parenthesized_binary_expression() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_detects_parenthesized_binary_expression.ds",
            r#"
x = (x + y)
"#,
        );
        test.result(result)
            .assert_lint("operator-assignment")
            .assert_safe_fixed(
                r#"
x += y;
"#,
            );
    }

    #[test]
    fn test_reports_dynamic_member_assignment_without_fix() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_reports_dynamic_member_assignment_without_fix.ds",
            r#"
foo[bar].baz = foo[bar].baz + qux
"#,
        );
        test.result(result)
            .assert_lint("operator-assignment")
            .assert_has_no_fix("operator-assignment");
    }

    #[test]
    fn test_skips_fix_when_comments_are_present() {
        let test = TestProgram::for_rule_without_prelude(OperatorAssignment);
        let result = test.lint(
            "operator_assignment/test_skips_fix_when_comments_are_present.ds",
            r#"
x = x /* keep */ + y
"#,
        );
        test.result(result)
            .assert_lint("operator-assignment")
            .assert_has_no_fix("operator-assignment");
    }

    #[test]
    fn test_reports_shorthand_assignment_when_mode_is_never() {
        let test =
            TestProgram::for_rule_without_prelude(OperatorAssignment).with_options(|options| {
                options.style.operator_assignment_mode = OperatorAssignmentMode::Never
            });
        let result = test.lint(
            "operator_assignment/test_reports_shorthand_assignment_when_mode_is_never.ds",
            r#"
x += y
"#,
        );
        test.result(result)
            .assert_lint("operator-assignment")
            .assert_has_no_fix("operator-assignment");
    }

    #[test]
    fn test_ignores_logical_assignment_when_mode_is_never() {
        let test =
            TestProgram::for_rule_without_prelude(OperatorAssignment).with_options(|options| {
                options.style.operator_assignment_mode = OperatorAssignmentMode::Never
            });
        let result = test.lint(
            "operator_assignment/test_ignores_logical_assignment_when_mode_is_never.ds",
            r#"
x &&= y
"#,
        );
        test.result(result).assert_no_lint("operator-assignment");
    }
}
