use destack_dir::{self as dir, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_statement_ancestor;
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `++` and `--` operators.
    ///
    /// The increment and decrement operators can lead to confusion with
    /// their prefix vs postfix semantics. Use `+= 1` or `-= 1` instead.
    #[lint(
        id = "no-plusplus",
        code = "LR022",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable
    )]
    pub NoPlusplus,
    "Disallow `++` and `--` operators"
}

impl LintRule for NoPlusplus {
    fn meta(&self) -> &'static LintMeta {
        NoPlusplus::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let dir::Expression::Unary { operator, .. } = expression else {
                continue;
            };
            if !matches!(
                operator,
                UnaryOperator::PreIncrement
                    | UnaryOperator::PostIncrement
                    | UnaryOperator::PreDecrement
                    | UnaryOperator::PostDecrement
            ) {
                continue;
            }
            if ctx
                .options()
                .restriction
                .allow_plusplus_for_loop_afterthoughts
                && expression_is_for_loop_afterthought(ctx, node_id)
            {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // add a safe fix when the increment value is not used
            let mut diagnostic = LintReport::new(
                NO_PLUSPLUS.id,
                NO_PLUSPLUS.code,
                NO_PLUSPLUS.category,
                severity,
                "`++` and `--` operators are not allowed",
                ctx.dir.get_span(node_id),
            )
            .label("use `+= 1` or `-= 1` instead");

            // attach fix when enabled
            if ctx.compute_fixes && is_discarded_update_expression(ctx, node_id) {
                let operand_id = unary_operand_id(expression);
                let Some(operand_id) = operand_id else {
                    ctx.report(diagnostic);
                    continue;
                };
                let operand_span = ctx.dir.get_span(operand_id);
                let operand_text = ctx.get_span_text(operand_span);
                let assign_operator = match operator {
                    UnaryOperator::PreIncrement | UnaryOperator::PostIncrement => "+=",
                    UnaryOperator::PreDecrement | UnaryOperator::PostDecrement => "-=",
                    _ => unreachable!(),
                };
                let replacement = format!("{operand_text} {assign_operator} 1");
                let expression_span = ctx.dir.get_span(node_id);
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Replace increment or decrement with assignment")
                    .with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when an update expression result is discarded.
fn is_discarded_update_expression(
    ctx: &LintModuleContext<'_>,
    node_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // standalone statement position
    if expression_statement_ancestor(ctx.dir.tree(), node_id).is_some() {
        return true;
    }

    // for loop afterthought position
    expression_is_for_loop_afterthought(ctx, node_id)
}

/// Return true when one expression is in a for-loop afterthought chain.
fn expression_is_for_loop_afterthought(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // start from the update expression
    let mut current_id = expression_id;

    // walk through parenthesized and sequence wrappers up to one for increment
    loop {
        let Some(parent_id) = ctx.dir.get_parent_id(current_id.id) else {
            return false;
        };
        if ctx.dir.get_node_type(parent_id) != dir::NodeType::Expression {
            return false;
        }

        // resolve parent expression id
        let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
        let parent_expression = ctx.dir.get(parent_expression_id);
        match parent_expression {
            dir::Expression::Parenthesized { expression } if *expression == current_id => {
                current_id = parent_expression_id;
            }
            dir::Expression::SequenceExpression { expressions }
                if expressions.contains(&current_id) =>
            {
                current_id = parent_expression_id;
            }
            dir::Expression::For {
                increment: Some(increment_id),
                ..
            } => {
                return *increment_id == current_id;
            }
            _ => return false,
        }
    }
}

/// Return the operand id from a unary update expression.
fn unary_operand_id(expression: &dir::Expression) -> Option<dir::LocalNodeId<dir::Expression>> {
    let dir::Expression::Unary { right, .. } = expression else {
        return None;
    };

    Some(*right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_post_increment() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint("no_plusplus/test_detects_post_increment.ts", "x++;");
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_detects_pre_increment() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint("no_plusplus/test_detects_pre_increment.ts", "++x;");
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_detects_post_decrement() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint("no_plusplus/test_detects_post_decrement.ts", "x--;");
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_detects_pre_decrement() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint("no_plusplus/test_detects_pre_decrement.ts", "--x;");
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_allows_plus_equals() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint("no_plusplus/test_allows_plus_equals.ts", "x += 1;");
        test.result(result).assert_no_lint("no-plusplus");
    }

    #[test]
    fn test_allows_for_loop_afterthought_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus).with_options(|options| {
            options.restriction.allow_plusplus_for_loop_afterthoughts = true;
        });
        let result = test.lint(
            "no_plusplus/test_allows_for_loop_afterthought_when_enabled.ts",
            r#"
for (let i = 0; i < 3; i++) {
    run(i);
}
"#,
        );
        test.result(result).assert_no_lint("no-plusplus");
    }

    #[test]
    fn test_allows_sequence_for_loop_afterthought_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus).with_options(|options| {
            options.restriction.allow_plusplus_for_loop_afterthoughts = true;
        });
        let result = test.lint(
            "no_plusplus/test_allows_sequence_for_loop_afterthought_when_enabled.ts",
            r#"
for (let i = 0; i < 3; log(i), i++) {
    run(i);
}
"#,
        );
        test.result(result).assert_no_lint("no-plusplus");
    }

    #[test]
    fn test_still_detects_loop_body_increment_when_afterthoughts_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus).with_options(|options| {
            options.restriction.allow_plusplus_for_loop_afterthoughts = true;
        });
        let result = test.lint(
            "no_plusplus/test_still_detects_loop_body_increment_when_afterthoughts_enabled.ts",
            r#"
for (let i = 0; i < 3; i++) {
    total++;
}
"#,
        );
        test.result(result).assert_lint("no-plusplus");
    }

    #[test]
    fn test_fix_post_increment_statement() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_fix_post_increment_statement.ts",
            r#"
x++;
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_fix("no-plusplus")
            .assert_safe_fixed(
                r#"
x += 1;
"#,
            );
    }

    #[test]
    fn test_fix_pre_increment_statement() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_fix_pre_increment_statement.ts",
            r#"
++count;
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_fix("no-plusplus")
            .assert_safe_fixed(
                r#"
count += 1;
"#,
            );
    }

    #[test]
    fn test_fix_post_decrement_statement() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_fix_post_decrement_statement.ts",
            r#"
count--;
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_fix("no-plusplus")
            .assert_safe_fixed(
                r#"
count -= 1;
"#,
            );
    }

    #[test]
    fn test_fix_member_post_increment_statement() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_fix_member_post_increment_statement.ts",
            r#"
item.count++;
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_fix("no-plusplus")
            .assert_safe_fixed(
                r#"
item.count += 1;
"#,
            );
    }

    #[test]
    fn test_fix_pre_decrement_in_for_increment() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_fix_pre_decrement_in_for_increment.ts",
            r#"
for (; keepGoing(); --index) {}
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_fix("no-plusplus")
            .assert_safe_fixed(
                r#"
for (; keepGoing(); index -= 1) {}
"#,
            );
    }

    #[test]
    fn test_fix_post_increment_in_for_afterthought_sequence() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_fix_post_increment_in_for_afterthought_sequence.ts",
            r#"
for (; keepGoing(); (touch(), index++)) {}
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_fix("no-plusplus")
            .assert_safe_fixed(
                r#"
for (; keepGoing(); (touch(), index += 1)) {}
"#,
            );
    }

    #[test]
    fn test_fix_parenthesized_post_increment_statement() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_fix_parenthesized_post_increment_statement.ts",
            r#"
(count++);
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_fix("no-plusplus")
            .assert_safe_fixed(
                r#"
(count += 1);
"#,
            );
    }

    #[test]
    fn test_no_fix_when_update_value_is_used() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_no_fix_when_update_value_is_used.ts",
            r#"
let value = count++;
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_no_fix("no-plusplus");
    }

    #[test]
    fn test_no_fix_when_update_value_is_returned() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_no_fix_when_update_value_is_returned.ts",
            r#"
function next() {
    return count++;
}
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_no_fix("no-plusplus");
    }

    #[test]
    fn test_no_fix_when_update_value_is_condition() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_no_fix_when_update_value_is_condition.ts",
            r#"
while (count++) {
    process();
}
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_no_fix("no-plusplus");
    }

    #[test]
    fn test_no_fix_when_update_value_is_call_argument() {
        let test = TestProgram::for_rule_without_prelude(NoPlusplus);
        let result = test.lint(
            "no_plusplus/test_no_fix_when_update_value_is_call_argument.ts",
            r#"
consume(count++);
"#,
        );
        test.result(result)
            .assert_lint("no-plusplus")
            .assert_has_no_fix("no-plusplus");
    }
}
