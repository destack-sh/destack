use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_path_segments;
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Suggest using `match` instead of complex if-else-if chains or switch statements.
    ///
    /// When comparing the same value against multiple possibilities,
    /// a `match` expression is often clearer and ensures exhaustiveness.
    #[lint(
        id = "prefer-match",
        code = "LY044",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub PreferMatch,
    "Prefer match over complex if-else-if or switch statements"
}

// minimum number of else-if branches to trigger the suggestion
const MIN_BRANCHES: usize = 3;

impl LintRule for PreferMatch {
    fn meta(&self) -> &'static LintMeta {
        PreferMatch::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // only check top-level if expressions (not nested else-ifs)
            let ast::Expression::If {
                form: ast::IfForm::If,
                condition,
                else_expression: Some(else_expr),
                ..
            } = expr
            else {
                continue;
            };

            // skip if this is an else-if of a parent
            if is_else_if_of_parent(ctx, node_id) {
                continue;
            }

            let ast::IfCondition::Expression { condition } = condition else {
                continue;
            };

            // collect if-else-if comparisons over one shared subject
            let Some(if_chain) = collect_if_chain_for_match(ctx, node_id, *condition, *else_expr)
            else {
                continue;
            };

            // count if/else-if cases and optional final else
            let mut branch_count = if_chain.case_arms.len();
            if if_chain.else_expression.is_some() {
                branch_count += 1;
            }

            if branch_count >= MIN_BRANCHES {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintReport::new(
                    PREFER_MATCH.id,
                    PREFER_MATCH.code,
                    PREFER_MATCH.category,
                    severity,
                    format!("consider using `match` for this {branch_count}-branch if-else chain"),
                    ctx.tree.get_span(node_id),
                )
                .label("a `match` expression would be clearer here");
                if ctx.compute_fixes
                    && let Some(fix) = prefer_match_fix(ctx, node_id, &if_chain)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// One if-else-if chain normalized for match conversion.
struct IfMatchChain {
    /// The shared subject expression.
    subject_expression: ast::LocalNodeId<ast::Expression>,
    /// Case arms in source order.
    case_arms: Vec<IfMatchCaseArm>,
    /// The final else branch body when present.
    else_expression: Option<ast::LocalNodeId<ast::Expression>>,
}

/// One case arm from an if branch.
struct IfMatchCaseArm {
    /// Pattern expression compared against the shared subject.
    pattern_expression: ast::LocalNodeId<ast::Expression>,
    /// Branch body expression.
    body_expression: ast::LocalNodeId<ast::Expression>,
}

/// Check if this if expression is the else-if of a parent if.
fn is_else_if_of_parent(
    ctx: &LintAstContext<'_>,
    node_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let Some(parent_id) = ctx.parents.get(node_id) else {
        return false;
    };

    if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return false;
    }

    let parent_expr_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent = ctx.tree.get(parent_expr_id);

    matches!(
        parent,
        ast::Expression::If {
            else_expression: Some(else_id),
            ..
        } if *else_id == node_id
    )
}

/// Collect an if-chain when all branch conditions compare one shared subject path.
fn collect_if_chain_for_match(
    ctx: &LintAstContext<'_>,
    root_if_expression_id: ast::LocalNodeId<ast::Expression>,
    root_condition_id: ast::LocalNodeId<ast::Expression>,
    root_else_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<IfMatchChain> {
    let (subject_key, subject_expression, pattern_expression) =
        comparison_subject_and_pattern(ctx, root_condition_id)?;
    let root_if_expression = ctx.tree.get(root_if_expression_id);
    let ast::Expression::If {
        then_expression, ..
    } = root_if_expression
    else {
        return None;
    };
    let mut case_arms = vec![IfMatchCaseArm {
        pattern_expression,
        body_expression: *then_expression,
    }];

    let mut else_expression = None;
    let mut current_else_expression_id = Some(root_else_expression_id);
    while let Some(next_else_expression_id) = current_else_expression_id {
        let next_else_expression = ctx.tree.get(next_else_expression_id);
        if let ast::Expression::If {
            form: ast::IfForm::If,
            condition,
            then_expression,
            else_expression: chained_else_expression,
        } = next_else_expression
        {
            let ast::IfCondition::Expression { condition } = condition else {
                return None;
            };

            let (next_subject_key, _, next_pattern_expression) =
                comparison_subject_and_pattern(ctx, *condition)?;
            if next_subject_key != subject_key {
                return None;
            }

            case_arms.push(IfMatchCaseArm {
                pattern_expression: next_pattern_expression,
                body_expression: *then_expression,
            });
            current_else_expression_id = *chained_else_expression;
            continue;
        }

        else_expression = Some(next_else_expression_id);
        current_else_expression_id = None;
    }

    Some(IfMatchChain {
        subject_expression,
        case_arms,
        else_expression,
    })
}

/// Return `(subject_key, subject_expression, pattern_expression)` for one equality condition.
fn comparison_subject_and_pattern(
    ctx: &LintAstContext<'_>,
    condition_id: ast::LocalNodeId<ast::Expression>,
) -> Option<(
    String,
    ast::LocalNodeId<ast::Expression>,
    ast::LocalNodeId<ast::Expression>,
)> {
    let condition = ctx.tree.get(condition_id);

    // handle parenthesized conditions
    if let ast::Expression::Parenthesized { expression } = condition {
        return comparison_subject_and_pattern(ctx, *expression);
    }

    // look for equality comparisons
    let ast::Expression::Binary {
        left,
        operator,
        right,
    } = condition
    else {
        return None;
    };

    // only consider == and === comparisons
    if !matches!(
        operator,
        ast::BinaryOperator::Equal | ast::BinaryOperator::EqualStrict
    ) {
        return None;
    }

    // keep a path expression on one side and use the other side as the pattern
    if let Some(path_segments) = expression_path_segments(ctx.tree, *left) {
        let key = path_key(ctx, &path_segments);
        return Some((key, *left, *right));
    }

    if let Some(path_segments) = expression_path_segments(ctx.tree, *right) {
        let key = path_key(ctx, &path_segments);
        return Some((key, *right, *left));
    }

    None
}

/// Build a stable key for one path expression.
fn path_key(ctx: &LintAstContext<'_>, path_segments: &[ast::StringId]) -> String {
    path_segments
        .iter()
        .map(|segment| ctx.strings.get(*segment).as_ref().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Build a safe if-chain to match rewrite when the chain has a final else.
fn prefer_match_fix(
    ctx: &LintAstContext<'_>,
    if_expression_id: ast::LocalNodeId<ast::Expression>,
    if_chain: &IfMatchChain,
) -> Option<LintFix> {
    let else_expression = if_chain.else_expression?;

    let subject_text = ctx
        .get_span_text(ctx.tree.get_span(if_chain.subject_expression))
        .to_string();
    if subject_text.is_empty() {
        return None;
    }

    // collect case lines first to avoid partial rewrites
    let mut case_lines = Vec::with_capacity(if_chain.case_arms.len() + 1);
    for case_arm in &if_chain.case_arms {
        let pattern_text = ctx
            .get_span_text(ctx.tree.get_span(case_arm.pattern_expression))
            .to_string();
        let body_text = match_arm_body_text(ctx, case_arm.body_expression)?;
        case_lines.push(format!("{pattern_text} => {body_text}"));
    }
    let else_body_text = match_arm_body_text(ctx, else_expression)?;
    case_lines.push(format!("_ => {else_body_text}"));

    // render a simple newline-delimited match expression
    let mut replacement = format!("match {subject_text} {{\n");
    for line in case_lines {
        replacement.push_str("    ");
        replacement.push_str(&line);
        replacement.push('\n');
    }
    replacement.push('}');

    let edits = ctx
        .edit_builder()
        .replace(ctx.tree.get_span(if_expression_id), replacement)
        .into_edits();
    Some(LintFix::safe("Rewrite if/else-if chain to match").with_edits(edits))
}

/// Return match arm body text for one expression.
fn match_arm_body_text(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<String> {
    let body_text = ctx
        .get_span_text(ctx.tree.get_span(expression_id))
        .to_string();
    if body_text.is_empty() {
        return None;
    }

    Some(body_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_long_if_else_chain() {
        let test = TestProgram::for_rule_without_prelude(PreferMatch);
        let result = test.lint_ast(
            "prefer_match/test_detects_long_if_else_chain.ds",
            r#"
if (x == 1) {
    a()
} else if (x == 2) {
    b()
} else if (x == 3) {
    c()
}
"#,
        );
        test.result(result).assert_lint("prefer-match");
    }

    #[test]
    fn test_detects_with_else_block() {
        let test = TestProgram::for_rule_without_prelude(PreferMatch);
        let result = test.lint_ast(
            "prefer_match/test_detects_with_else_block.ds",
            r#"
if (x == 1) {
    a()
} else if (x == 2) {
    b()
} else {
    c()
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-match")
            .assert_safe_fixed(
                r#"
match (x) {
    1 => {
        a()
    }
    2 => {
        b()
    }
    _ => {
        c()
    }
}
"#,
            );
    }

    #[test]
    fn test_allows_short_if_else() {
        let test = TestProgram::for_rule_without_prelude(PreferMatch);
        let result = test.lint_ast(
            "prefer_match/test_allows_short_if_else.ds",
            r#"
if (x == 1) {
    a()
} else {
    b()
}
"#,
        );
        test.result(result).assert_no_lint("prefer-match");
    }

    #[test]
    fn test_allows_different_variables() {
        let test = TestProgram::for_rule_without_prelude(PreferMatch);
        let result = test.lint_ast(
            "prefer_match/test_allows_different_variables.ds",
            r#"
if (x == 1) {
    a()
} else if (y == 2) {
    b()
} else if (z == 3) {
    c()
}
"#,
        );
        test.result(result).assert_no_lint("prefer-match");
    }

    #[test]
    fn test_allows_non_equality_conditions() {
        let test = TestProgram::for_rule_without_prelude(PreferMatch);
        let result = test.lint_ast(
            "prefer_match/test_allows_non_equality_conditions.ds",
            r#"
if (x > 1) {
    a()
} else if (x > 2) {
    b()
} else if (x > 3) {
    c()
}
"#,
        );
        test.result(result).assert_no_lint("prefer-match");
    }

    #[test]
    fn test_detects_without_else_has_no_fix() {
        let test = TestProgram::for_rule_without_prelude(PreferMatch);
        let result = test.lint_ast(
            "prefer_match/test_detects_without_else_has_no_fix.ds",
            r#"
if (x == 1) {
    a()
} else if (x == 2) {
    b()
} else if (x == 3) {
    c()
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-match")
            .assert_has_no_fix("prefer-match");
    }
}
