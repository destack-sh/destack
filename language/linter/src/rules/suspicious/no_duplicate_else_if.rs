use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_is_else_if_branch, expression_is_equal, expression_unwrap_parenthesized_source_form,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate conditions in if-else-if chains.
    ///
    /// Having the same condition in multiple branches of an if-else-if chain
    /// is almost always a bug since the later branch will never be reached.
    #[lint(
        id = "no-duplicate-else-if",
        code = "LU010",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoDuplicateElseIf,
    "Disallow duplicate else-if conditions"
}

impl LintRule for NoDuplicateElseIf {
    fn meta(&self) -> &'static LintMeta {
        NoDuplicateElseIf::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect each if condition against its parent else-if chain
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expr = ctx.dir.get(node_id);
            let dir::Expression::If {
                form: dir::IfForm::If,
                condition,
                ..
            } = expr
            else {
                continue;
            };

            // keep only expression-style conditions
            let dir::IfCondition::Expression {
                condition: condition_id,
            } = condition
            else {
                continue;
            };

            // skip conditions that are not duplicate or already covered
            if !condition_is_duplicate_or_covered(ctx, node_id, *condition_id) {
                continue;
            }

            // resolve lint severity for this test expression
            let severity = ctx.get_effective_severity(meta, *condition_id);
            if !severity.is_enabled() {
                continue;
            }

            // report one covered duplicate diagnostic
            let mut diagnostic = LintReport::new(
                NO_DUPLICATE_ELSE_IF.id,
                NO_DUPLICATE_ELSE_IF.code,
                NO_DUPLICATE_ELSE_IF.category,
                severity,
                "this branch can never execute, condition is duplicate or already covered",
                ctx.dir.get_span(*condition_id),
            )
            .label("this condition is already handled by earlier branch conditions");

            // attach one focused fix only for exact duplicate else-if branches
            if ctx.compute_fixes
                && has_exact_duplicate_in_ancestor_chain(ctx, node_id, *condition_id)
                && let Some(fix) = no_duplicate_else_if_fix(ctx, node_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return all expression conditions from parent else-if chain order.
fn ancestor_else_if_conditions(
    ctx: &LintModuleContext<'_>,
    mut expression_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<dir::LocalNodeId<dir::Expression>> {
    // collect parent tests from nearest parent outward
    let mut parent_conditions = Vec::new();

    // walk parent chain while this node is an alternate branch
    while let Some(parent_id) = ctx.dir.get_parent_id(expression_id.id) {
        if ctx.dir.get_node_type(parent_id) != dir::NodeType::Expression {
            break;
        }

        let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
        let parent_expression = ctx.dir.get(parent_expression_id);
        let dir::Expression::If {
            condition,
            else_expression: Some(else_id),
            ..
        } = parent_expression
        else {
            break;
        };
        if *else_id != expression_id {
            break;
        }

        // keep one expression condition from this parent
        if let dir::IfCondition::Expression { condition } = condition {
            parent_conditions.push(*condition);
        }

        // continue with next parent in the chain
        expression_id = parent_expression_id;
    }

    parent_conditions
}

/// Return true when this if test is duplicate or covered by parent else-if chain tests.
fn condition_is_duplicate_or_covered(
    ctx: &LintModuleContext<'_>,
    if_expression_id: dir::LocalNodeId<dir::Expression>,
    test_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // apply this rule only to else-if branches
    if !expression_is_else_if_branch(ctx.dir.tree(), if_expression_id) {
        return false;
    }

    // build the initial condition list to check
    let mut list_to_check = if condition_is_and(ctx, test_expression_id) {
        let mut conditions = vec![test_expression_id];
        conditions.extend(split_by_and(ctx, test_expression_id));
        conditions
            .into_iter()
            .map(|condition_id| {
                split_by_or(ctx, condition_id)
                    .into_iter()
                    .map(|id| split_by_and(ctx, id))
                    .collect()
            })
            .collect::<Vec<Vec<Vec<dir::LocalNodeId<dir::Expression>>>>>()
    } else {
        vec![
            split_by_or(ctx, test_expression_id)
                .into_iter()
                .map(|id| split_by_and(ctx, id))
                .collect(),
        ]
    };

    // remove covered branches for each parent chain condition
    for parent_condition_id in ancestor_else_if_conditions(ctx, if_expression_id) {
        let parent_or_operands = split_by_or(ctx, parent_condition_id)
            .into_iter()
            .map(|id| split_by_and(ctx, id))
            .collect::<Vec<Vec<dir::LocalNodeId<dir::Expression>>>>();

        list_to_check = list_to_check
            .into_iter()
            .map(|or_operands| {
                or_operands
                    .into_iter()
                    .filter(|or_operand| {
                        !parent_or_operands.iter().any(|parent_or_operand| {
                            conditions_are_subset(ctx, parent_or_operand, or_operand)
                        })
                    })
                    .collect()
            })
            .collect();

        if list_to_check
            .iter()
            .any(|or_operands: &Vec<Vec<dir::LocalNodeId<dir::Expression>>>| or_operands.is_empty())
        {
            return true;
        }
    }

    false
}

/// Return true when left condition list is a subset of right condition list.
fn conditions_are_subset(
    ctx: &LintModuleContext<'_>,
    left_conditions: &[dir::LocalNodeId<dir::Expression>],
    right_conditions: &[dir::LocalNodeId<dir::Expression>],
) -> bool {
    left_conditions.iter().all(|left_id| {
        right_conditions
            .iter()
            .any(|right_id| condition_is_equal(ctx, *left_id, *right_id))
    })
}

/// Return true when two condition expressions are equivalent for duplicate checks.
fn condition_is_equal(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // unwrap parenthesized expression wrappers
    let left_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), left_id);
    let right_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), right_id);
    let left_expression = ctx.dir.get(left_id);
    let right_expression = ctx.dir.get(right_id);

    // treat && and || as commutative binary operators
    if let (
        dir::Expression::Binary {
            left: left_left,
            operator: left_operator,
            right: left_right,
        },
        dir::Expression::Binary {
            left: right_left,
            operator: right_operator,
            right: right_right,
        },
    ) = (left_expression, right_expression)
        && left_operator == right_operator
        && matches!(
            left_operator,
            dir::BinaryOperator::And | dir::BinaryOperator::Or
        )
    {
        let same_order = condition_is_equal(ctx, *left_left, *right_left)
            && condition_is_equal(ctx, *left_right, *right_right);
        if same_order {
            return true;
        }

        let swapped_order = condition_is_equal(ctx, *left_left, *right_right)
            && condition_is_equal(ctx, *left_right, *right_left);
        return swapped_order;
    }

    expression_is_equal(ctx, left_id, right_id)
}

/// Return true when one expression is a logical and condition.
fn condition_is_and(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);
    matches!(
        expression,
        dir::Expression::Binary {
            operator: dir::BinaryOperator::And,
            ..
        }
    )
}

/// Split one condition expression by logical or operators.
fn split_by_or(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<dir::LocalNodeId<dir::Expression>> {
    split_by_logical_operator(ctx, expression_id, dir::BinaryOperator::Or)
}

/// Split one condition expression by logical and operators.
fn split_by_and(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<dir::LocalNodeId<dir::Expression>> {
    split_by_logical_operator(ctx, expression_id, dir::BinaryOperator::And)
}

/// Split one condition expression by one logical operator.
fn split_by_logical_operator(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
) -> Vec<dir::LocalNodeId<dir::Expression>> {
    // normalize parenthesized wrappers
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);

    // recursively flatten matching logical operators
    if let dir::Expression::Binary {
        left,
        operator: expression_operator,
        right,
    } = expression
        && *expression_operator == operator
    {
        let mut split_conditions = split_by_logical_operator(ctx, *left, operator);
        split_conditions.extend(split_by_logical_operator(ctx, *right, operator));
        return split_conditions;
    }

    vec![expression_id]
}

/// Return true when this else-if condition exactly duplicates one parent chain condition.
fn has_exact_duplicate_in_ancestor_chain(
    ctx: &LintModuleContext<'_>,
    if_expression_id: dir::LocalNodeId<dir::Expression>,
    test_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    ancestor_else_if_conditions(ctx, if_expression_id)
        .into_iter()
        .any(|parent_condition_id| {
            expression_is_equal(ctx, parent_condition_id, test_expression_id)
        })
}

/// Build an unsafe fix for one duplicate else-if by replacing it with its fallback branch.
fn no_duplicate_else_if_fix(
    ctx: &LintModuleContext<'_>,
    if_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    // keep fixes for nested else-if expressions only
    if !expression_is_else_if_branch(ctx.dir.tree(), if_expression_id) {
        return None;
    }

    let expression = ctx.dir.get(if_expression_id);
    let dir::Expression::If {
        else_expression: Some(else_expression_id),
        ..
    } = expression
    else {
        return None;
    };

    let replacement = ctx.get_span_text(ctx.dir.get_span(*else_expression_id));
    let edits = ctx
        .edit_builder()
        .replace(ctx.dir.get_span(if_expression_id), replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Remove duplicate else-if branch").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_else_if() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_detects_duplicate_else_if.ds",
            r#"
if (x > 0) {
    a()
} else if (x > 0) {
    b()
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-else-if");
    }

    #[test]
    fn test_detects_duplicate_in_longer_chain() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_detects_duplicate_in_longer_chain.ds",
            r#"
if (x > 0) {
    a()
} else if (x < 0) {
    b()
} else if (x > 0) {
    c()
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-else-if");
    }

    #[test]
    fn test_fix_rewrites_duplicate_else_if_to_fallback_branch() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_fix_rewrites_duplicate_else_if_to_fallback_branch.ds",
            r#"
if (x > 0) {
    a()
} else if (x > 0) {
    b()
} else {
    c()
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-else-if")
            .assert_unsafe_fixed(
                r#"
if (x > 0) {
    a()
} else {
    c()
}
"#,
            );
    }

    #[test]
    fn test_allows_different_conditions() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_allows_different_conditions.ds",
            r#"
if (x > 0) {
    a()
} else if (x < 0) {
    b()
} else if (x == 0) {
    c()
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-else-if");
    }

    #[test]
    fn test_allows_simple_if_else() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_allows_simple_if_else.ds",
            r#"
if (x > 0) {
    a()
} else {
    b()
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-else-if");
    }

    #[test]
    fn test_no_fix_for_duplicate_terminal_else_if_without_fallback() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_no_fix_for_duplicate_terminal_else_if_without_fallback.ds",
            r#"
if (x > 0) {
    a()
} else if (x > 0) {
    b()
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-else-if")
            .assert_has_no_fix("no-duplicate-else-if");
    }

    #[test]
    fn test_detects_covered_else_if_condition_from_or_parent() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_detects_covered_else_if_condition_from_or_parent.ds",
            r#"
if (a || b) {
    first()
} else if (a) {
    second()
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-else-if")
            .assert_has_no_fix("no-duplicate-else-if");
    }

    #[test]
    fn test_detects_commutative_logical_duplicate() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_detects_commutative_logical_duplicate.ds",
            r#"
if (a || b) {
    first()
} else if (b || a) {
    second()
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-else-if")
            .assert_has_no_fix("no-duplicate-else-if");
    }

    #[test]
    fn test_allows_non_covered_else_if_condition_from_and_parent() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateElseIf);
        let result = test.lint(
            "no_duplicate_else_if/test_allows_non_covered_else_if_condition_from_and_parent.ds",
            r#"
if (a && b) {
    first()
} else if (a) {
    second()
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-else-if");
    }
}
