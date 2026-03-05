use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    has_non_nullish_falsy_type, is_maybe_nullish_type, is_strict_boolean_type,
    span_has_comment_trivia,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer nullish coalescing over `||` for nullish defaulting.
    ///
    /// This rule reports `a || b` when the left side can be nullish and
    /// cannot produce other falsy values, so `a ?? b` is clearer and preserves intent.
    #[lint(
        id = "prefer-nullish-coalescing",
        code = "LY046",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub PreferNullishCoalescing,
    "Prefer `??` over `||` for nullish defaults"
}

impl LintRule for PreferNullishCoalescing {
    fn meta(&self) -> &'static LintMeta {
        PreferNullishCoalescing::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect binary logical or expressions
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                right,
                ..
            } = expression
            else {
                continue;
            };

            // skip boolean and control-flow contexts
            if should_skip_expression_context(ctx, expression_id) {
                continue;
            }

            // keep only semantically safe nullish defaulting candidates
            if !left_side_prefers_nullish(ctx, *left) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // report one nullish coalescing suggestion
            let span = ctx.get_span(expression_id);
            let mut diagnostic = LintDiagnostic::new(
                PREFER_NULLISH_COALESCING.id,
                PREFER_NULLISH_COALESCING.code,
                PREFER_NULLISH_COALESCING.category,
                severity,
                "prefer nullish coalescing for defaults",
                ctx.module.file_id,
                span,
            )
            .with_label("use `??` to default only on nullish values");
            if let Some(fix) = make_nullish_fix(ctx, expression_id, *left, *right) {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }

        // inspect logical or assignments
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let dir::Expression::AssignBinary {
                left,
                operator: dir::AssignOperator::OrAssign,
                right,
            } = expression
            else {
                continue;
            };

            // keep only semantically safe nullish defaulting candidates
            if !left_side_prefers_nullish(ctx, *left) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // report one nullish assignment suggestion
            let span = ctx.get_span(expression_id);
            let mut diagnostic = LintDiagnostic::new(
                PREFER_NULLISH_COALESCING.id,
                PREFER_NULLISH_COALESCING.code,
                PREFER_NULLISH_COALESCING.category,
                severity,
                "prefer nullish coalescing assignment for defaults",
                ctx.module.file_id,
                span,
            )
            .with_label("use `??=` to default only on nullish values");
            if let Some(fix) = make_nullish_assignment_fix(ctx, expression_id, *left, *right) {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a safe `||` to `??` fix for one expression.
fn make_nullish_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let expression_span = ctx.get_span(expression_id);
    if span_has_comment_trivia(ctx.ast, expression_span) {
        return None;
    }

    let left_text = ctx.get_span_text(ctx.get_span(left_id));
    let right_text = ctx.get_span_text(ctx.get_span(right_id));
    if left_text.trim().is_empty() || right_text.trim().is_empty() {
        return None;
    }

    let left_text = if expression_needs_parentheses_for_nullish_operand(ctx.tree, left_id) {
        format!("({left_text})")
    } else {
        left_text.to_owned()
    };
    let right_text = if expression_needs_parentheses_for_nullish_operand(ctx.tree, right_id) {
        format!("({right_text})")
    } else {
        right_text.to_owned()
    };

    let replacement = format!("{left_text} ?? {right_text}");
    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();
    Some(LintFix::safe("Replace `||` with `??`").with_edits(edits))
}

/// Build a safe `||=` to `??=` fix for one expression.
fn make_nullish_assignment_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let expression_span = ctx.get_span(expression_id);
    if span_has_comment_trivia(ctx.ast, expression_span) {
        return None;
    }

    let left_text = ctx.get_span_text(ctx.get_span(left_id));
    let right_text = ctx.get_span_text(ctx.get_span(right_id));
    if left_text.trim().is_empty() || right_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("{left_text} ??= {right_text}");
    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();

    Some(LintFix::safe("Replace `||=` with `??=`").with_edits(edits))
}

/// Return true when this expression should not be linted.
fn should_skip_expression_context(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(parent_id) = ctx.tree.get_parent(expression_id.id) else {
        return false;
    };

    // skip mixed logical chains
    if parent_id.ty == dir::NodeType::Expression {
        let parent = ctx.tree.get(parent_id.into_typed::<dir::Expression>());
        if matches!(
            parent,
            dir::Expression::Binary {
                operator: dir::BinaryOperator::And
                    | dir::BinaryOperator::Or
                    | dir::BinaryOperator::Coalesce,
                ..
            }
        ) {
            return true;
        }
    }

    // skip condition positions
    expression_is_condition(ctx.tree, expression_id, parent_id)
}

/// Return true when an operand must be parenthesized in a `??` expression.
fn expression_needs_parentheses_for_nullish_operand(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = tree.get(expression_id);
    matches!(
        expression,
        dir::Expression::Binary {
            operator: dir::BinaryOperator::And
                | dir::BinaryOperator::Or
                | dir::BinaryOperator::Coalesce,
            ..
        }
    )
}

/// Return true when the expression is used as a condition.
fn expression_is_condition(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    parent_id: dir::LocalNodeIdAny,
) -> bool {
    if parent_id.ty == dir::NodeType::Expression {
        let parent = tree.get(parent_id.into_typed::<dir::Expression>());
        match parent {
            dir::Expression::If { condition, .. } => {
                if let dir::IfCondition::Expression { condition } = condition {
                    return *condition == expression_id;
                }
            }
            dir::Expression::Loop {
                condition: Some(condition),
                ..
            } => {
                return *condition == expression_id;
            }
            dir::Expression::For {
                condition: Some(condition),
                ..
            } => {
                return *condition == expression_id;
            }
            _ => {}
        }
    }

    if parent_id.ty == dir::NodeType::MatchCase {
        let match_case = tree.get(parent_id.into_typed::<dir::MatchCase>());
        let selector = match match_case {
            dir::MatchCase::Expression { selector, .. }
            | dir::MatchCase::Block { selector, .. } => selector,
        };
        if let dir::MatchSelector::Pattern {
            guard: Some(guard), ..
        } = selector
        {
            return *guard == expression_id;
        }
    }

    false
}

/// Return true when the left side is a safe candidate for `??`.
fn left_side_prefers_nullish(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(type_id) = ctx.expression_type_id(expression_id) else {
        return false;
    };
    if is_strict_boolean_type(ctx.types, type_id) {
        return false;
    }

    // require maybe-nullish values
    if !is_maybe_nullish_type(ctx.types, type_id) {
        return false;
    }

    // reject candidates where non-nullish falsy values are possible
    if has_non_nullish_falsy_type(ctx.types, &ctx.program.strings, type_id) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report `||` defaulting when the left side is object or null.
    #[test]
    fn test_flags_object_or_null_defaulting() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_flags_object_or_null_defaulting.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = profile || { id: 1 };
"#,
        );

        test.result(diagnostics)
            .assert_lint("prefer-nullish-coalescing")
            .assert_has_fix("prefer-nullish-coalescing")
            .assert_safe_fixed(
                r#"
let profile: { id: int32 } | null = null;
let selected = profile ?? { id: 1 };
"#,
            );
    }

    /// Allow `||` when non-nullish falsy string values are possible.
    #[test]
    fn test_allows_string_or_null_defaulting() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_string_or_null_defaulting.ds",
            r#"
let title: string | null = "";
let selected = title || "fallback";
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow `||` when non-nullish falsy numeric values are possible.
    #[test]
    fn test_allows_number_or_null_defaulting() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_number_or_null_defaulting.ds",
            r#"
let count: int32 | null = 0;
let selected = count || 1;
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow `||` in condition positions.
    #[test]
    fn test_allows_condition_context() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_condition_context.ds",
            r#"
let condition: { ok: boolean } | null = null;
if (condition || { ok: true }) {
}
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow `||` when the left side is never nullish.
    #[test]
    fn test_allows_never_nullish_left_side() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_never_nullish_left_side.ds",
            r#"
let profile: { id: int32 } = { id: 1 };
let selected = profile || { id: 2 };
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Wrap logical operands when converting to `??`.
    #[test]
    fn test_fix_wraps_logical_operands() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_fix_wraps_logical_operands.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = profile || (true && { id: 1 });
"#,
        );

        test.result(diagnostics)
            .assert_lint("prefer-nullish-coalescing")
            .assert_has_fix("prefer-nullish-coalescing")
            .assert_safe_fixed(
                r#"
let profile: { id: int32 } | null = null;
let selected = profile ?? (true && { id: 1 });
"#,
            );
    }

    /// Keep lint without fix when expression contains comments.
    #[test]
    fn test_no_fix_when_or_contains_comments() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_no_fix_when_or_contains_comments.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = profile || /* fallback object */ { id: 1 };
"#,
        );

        test.result(diagnostics)
            .assert_lint("prefer-nullish-coalescing")
            .assert_has_no_fix("prefer-nullish-coalescing");
    }

    /// Report and fix `||=` when the left side is nullable object.
    #[test]
    fn test_fix_or_assign_for_nullable_object() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_fix_or_assign_for_nullable_object.ds",
            r#"
let profile: { id: int32 } | null = null;
profile ||= { id: 1 };
"#,
        );

        test.result(diagnostics)
            .assert_lint("prefer-nullish-coalescing")
            .assert_has_fix("prefer-nullish-coalescing")
            .assert_safe_fixed(
                r#"
let profile: { id: int32 } | null = null;
profile ??= { id: 1 };
"#,
            );
    }

    /// Allow `||=` when non-nullish falsy values are possible.
    #[test]
    fn test_allows_or_assign_for_nullable_string() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_or_assign_for_nullable_string.ds",
            r#"
let title: string | null = "";
title ||= "fallback";
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }
}
