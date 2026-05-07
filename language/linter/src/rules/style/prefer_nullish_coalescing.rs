use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    assign_pattern_has_equivalent_source_form, expression_reference_path,
    expression_unwrap_parenthesized, expressions_have_equivalent_source_form,
    has_non_nullish_falsy_type, is_maybe_nullish_type, is_strict_boolean_type, span_has_comment,
};
use crate::{
    ConstValue, LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint,
};

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
        let mut visitor = PreferNullishCoalescingVisitor::new(ctx, self.meta());
        visitor.run();
    }
}

/// Node visitor that checks nullish-coalescing preference candidates.
struct PreferNullishCoalescingVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The rule options for this module.
    rule_options: PreferNullishCoalescingOptions,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferNullishCoalescingVisitor<'a, 'b> {
    /// Build a visitor for prefer-nullish-coalescing checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let rule_options = prefer_nullish_coalescing_options(ctx);

        Self {
            ctx,
            meta,
            rule_options,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one `left || right` expression.
    fn check_logical_or(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // skip lowered `||=` shapes handled by assignment checks
        if logical_or_is_or_assign_lowering(self.ctx, expression_id, left_id) {
            return;
        }

        // skip boolean and control-flow contexts
        if should_skip_expression_context(self.ctx, expression_id, self.rule_options) {
            return;
        }

        // keep only semantically safe nullish defaulting candidates
        if !left_side_prefers_nullish(self.ctx, left_id) {
            return;
        }

        // honor per-node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report one nullish-coalescing suggestion
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            PREFER_NULLISH_COALESCING.id,
            PREFER_NULLISH_COALESCING.code,
            PREFER_NULLISH_COALESCING.category,
            severity,
            "prefer nullish coalescing for defaults",
            span,
        )
        .label("use `??` to default only on nullish values");
        if let Some(fix) = make_nullish_fix(self.ctx, expression_id, left_id, right_id) {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Check one `left ||= right` expression.
    fn check_or_assignment(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // keep only semantically safe nullish defaulting candidates
        if !left_side_prefers_nullish(self.ctx, left_id) {
            return;
        }

        // honor per-node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report one nullish-assignment suggestion
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            PREFER_NULLISH_COALESCING.id,
            PREFER_NULLISH_COALESCING.code,
            PREFER_NULLISH_COALESCING.category,
            severity,
            "prefer nullish coalescing assignment for defaults",
            span,
        )
        .label("use `??=` to default only on nullish values");
        if let Some(fix) = make_nullish_assignment_fix(self.ctx, expression_id, left_id, right_id) {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Check one ternary null-check expression.
    fn check_ternary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        condition_id: dir::LocalNodeId<dir::Expression>,
        then_expression_id: dir::LocalNodeId<dir::Expression>,
        else_expression_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // keep ternary checks disabled when configured
        if self.rule_options.ignore_ternary_tests {
            return;
        }

        // keep only semantically safe ternary null checks
        let Some((left_expression_id, fallback_expression_id)) = ternary_nullish_candidate(
            self.ctx,
            condition_id,
            then_expression_id,
            else_expression_id,
        ) else {
            return;
        };
        if !left_side_prefers_nullish(self.ctx, left_expression_id) {
            return;
        }

        // honor per-node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report one ternary nullish suggestion
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            PREFER_NULLISH_COALESCING.id,
            PREFER_NULLISH_COALESCING.code,
            PREFER_NULLISH_COALESCING.category,
            severity,
            "prefer nullish coalescing over ternary null checks",
            span,
        )
        .label("use `??` to default only on nullish values");
        if let Some(fix) = make_nullish_fix(
            self.ctx,
            expression_id,
            left_expression_id,
            fallback_expression_id,
        ) {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

impl NodeVisitor for PreferNullishCoalescingVisitor<'_, '_> {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit one expression node.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check binary logical-or expressions
        if let dir::Expression::Binary {
            left,
            operator: dir::BinaryOperator::Or,
            right,
            ..
        } = expression
        {
            self.check_logical_or(id, *left, *right);
        }

        // check logical-or assignments
        if let dir::Expression::AssignBinary {
            left,
            operator: dir::AssignOperator::OrAssign,
            right,
        } = expression
        {
            self.check_or_assignment(id, *left, *right);
        }

        // check ternary null-check expressions
        if let dir::Expression::If {
            form: dir::IfForm::Ternary,
            condition,
            then_expression,
            else_expression: Some(else_expression),
            ..
        } = expression
            && let dir::IfCondition::Expression {
                condition: condition_expression_id,
            } = condition
        {
            self.check_ternary(
                id,
                *condition_expression_id,
                *then_expression,
                *else_expression,
            );
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Configuration for prefer-nullish-coalescing.
#[derive(Debug, Clone, Copy)]
struct PreferNullishCoalescingOptions {
    /// Ignore condition positions when true.
    ignore_conditional_tests: bool,
    /// Ignore mixed logical chains when true.
    ignore_mixed_logical_expressions: bool,
    /// Ignore ternary tests when true.
    ignore_ternary_tests: bool,
}

/// Resolve rule options from linter configuration.
fn prefer_nullish_coalescing_options(
    ctx: &LintModuleDirContext<'_>,
) -> PreferNullishCoalescingOptions {
    PreferNullishCoalescingOptions {
        ignore_conditional_tests: ctx
            .options
            .style
            .prefer_nullish_coalescing_ignore_conditional_tests,
        ignore_mixed_logical_expressions: ctx
            .options
            .style
            .prefer_nullish_coalescing_ignore_mixed_logical_expressions,
        ignore_ternary_tests: ctx
            .options
            .style
            .prefer_nullish_coalescing_ignore_ternary_tests,
    }
}

/// Extract one `left ?? fallback` candidate from a ternary null check.
fn ternary_nullish_candidate(
    ctx: &mut LintModuleDirContext<'_>,
    condition_id: dir::LocalNodeId<dir::Expression>,
    then_expression_id: dir::LocalNodeId<dir::Expression>,
    else_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(
    dir::LocalNodeId<dir::Expression>,
    dir::LocalNodeId<dir::Expression>,
)> {
    // keep one binary comparison condition
    let condition_id = expression_unwrap_parenthesized(ctx.tree, condition_id);
    let condition = ctx.tree.get(condition_id);
    let dir::Expression::Binary {
        left,
        operator,
        right,
    } = condition
    else {
        return None;
    };

    // require nullish checks against one side
    let (checked_expression_id, is_not_equal_check) =
        nullish_binary_target(ctx, *left, *operator, *right)?;

    // require stable checked references for safe rewrites
    let checked_expression_path = expression_reference_path(ctx, checked_expression_id)?;
    if !checked_expression_path.members.is_empty() {
        return None;
    }

    // normalize branch expressions
    let then_expression_id = expression_unwrap_parenthesized(ctx.tree, then_expression_id);
    let else_expression_id = expression_unwrap_parenthesized(ctx.tree, else_expression_id);

    // map comparison polarity to ternary fallback branch
    if is_not_equal_check
        && expressions_have_equivalent_source_form(ctx, checked_expression_id, then_expression_id)
    {
        return Some((checked_expression_id, else_expression_id));
    }
    if !is_not_equal_check
        && expressions_have_equivalent_source_form(ctx, checked_expression_id, else_expression_id)
    {
        return Some((checked_expression_id, then_expression_id));
    }

    None
}

/// Return the checked expression and polarity for one nullish binary comparison.
fn nullish_binary_target(
    ctx: &mut LintModuleDirContext<'_>,
    left_id: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(dir::LocalNodeId<dir::Expression>, bool)> {
    // keep equality and inequality comparisons only
    let is_not_equal_check = match operator {
        dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict => true,
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => false,
        _ => return None,
    };

    // extract one checked expression against nullish literal
    if expression_is_nullish_literal(ctx, left_id) {
        return Some((right_id, is_not_equal_check));
    }
    if expression_is_nullish_literal(ctx, right_id) {
        return Some((left_id, is_not_equal_check));
    }

    None
}

/// Return true when one expression is a null or undefined literal.
fn expression_is_nullish_literal(
    ctx: &mut LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(ctx.tree, expression_id);
    let Some(const_value) = ctx.const_value(expression_id) else {
        return false;
    };

    matches!(const_value, ConstValue::Null | ConstValue::Undefined)
}

/// Build a safe `||` to `??` fix for one expression.
fn make_nullish_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let expression_span = ctx.get_span(expression_id);
    if span_has_comment(ctx.ast, expression_span) {
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
    if span_has_comment(ctx.ast, expression_span) {
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
    options: PreferNullishCoalescingOptions,
) -> bool {
    // skip mixed logical shapes when configured
    if options.ignore_mixed_logical_expressions
        && expression_is_mixed_logical(ctx.tree, expression_id)
    {
        return true;
    }

    let Some(parent_id) = ctx.tree.get_parent(expression_id.id) else {
        return false;
    };

    // skip mixed logical chains when configured
    if options.ignore_mixed_logical_expressions && parent_id.ty == dir::NodeType::Expression {
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

    // skip condition positions when configured
    if options.ignore_conditional_tests {
        return expression_is_condition(ctx.tree, expression_id, parent_id);
    }

    false
}

/// Return true when one logical expression mixes operators in its operand tree.
fn expression_is_mixed_logical(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = tree.get(expression_id);
    let dir::Expression::Binary {
        left,
        operator,
        right,
    } = expression
    else {
        return false;
    };
    if !matches!(
        operator,
        dir::BinaryOperator::And | dir::BinaryOperator::Or | dir::BinaryOperator::Coalesce
    ) {
        return false;
    }

    logical_operand_has_other_operator(tree, *left, *operator)
        || logical_operand_has_other_operator(tree, *right, *operator)
}

/// Return true when one logical operand tree contains a different logical operator.
fn logical_operand_has_other_operator(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    root_operator: dir::BinaryOperator,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    let dir::Expression::Binary {
        left,
        operator,
        right,
    } = expression
    else {
        return false;
    };
    if !matches!(
        operator,
        dir::BinaryOperator::And | dir::BinaryOperator::Or | dir::BinaryOperator::Coalesce
    ) {
        return false;
    }
    if *operator != root_operator {
        return true;
    }

    logical_operand_has_other_operator(tree, *left, root_operator)
        || logical_operand_has_other_operator(tree, *right, root_operator)
}

/// Return true when an operand must be parenthesized in a `??` expression.
fn expression_needs_parentheses_for_nullish_operand(
    tree: &dir::Tree,
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
    tree: &dir::Tree,
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
    if has_non_nullish_falsy_type(ctx.types, ctx.strings, type_id) {
        return false;
    }

    true
}

/// Return true when this `||` expression is the lowering shape of one `||=` expression.
fn logical_or_is_or_assign_lowering(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    left_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(parent_id) = ctx.tree.get_parent(expression_id.id) else {
        return false;
    };
    if parent_id.ty != dir::NodeType::Expression {
        return false;
    }

    let parent_expression = ctx.tree.get(parent_id.into_typed::<dir::Expression>());

    // skip explicit assign-binary wrappers
    if let dir::Expression::AssignBinary {
        operator: dir::AssignOperator::OrAssign,
        right,
        ..
    } = parent_expression
    {
        return *right == expression_id;
    }

    // skip lowered `a = a || b` wrappers
    let dir::Expression::Assign {
        left: assignment_left,
        right: assignment_right,
    } = parent_expression
    else {
        return false;
    };
    if *assignment_right != expression_id {
        return false;
    }

    assign_pattern_has_equivalent_source_form(ctx, *assignment_left, left_id)
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

    /// Report and fix `!= null` ternary defaults.
    #[test]
    fn test_fix_ternary_not_equal_null_defaulting() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_fix_ternary_not_equal_null_defaulting.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = profile != null ? profile : { id: 1 };
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

    /// Report and fix yoda-style `== null` ternary defaults.
    #[test]
    fn test_fix_ternary_equal_null_defaulting_yoda() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_fix_ternary_equal_null_defaulting_yoda.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = null == profile ? { id: 1 } : profile;
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

    /// Allow ternary null checks with non-stable checked expressions.
    #[test]
    fn test_allows_ternary_null_check_for_non_stable_expression() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_ternary_null_check_for_non_stable_expression.ds",
            r#"
let selected = getProfile() != null ? getProfile() : { id: 1 };
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow ternary null checks when the checked branch does not return the tested expression.
    #[test]
    fn test_allows_ternary_mismatched_checked_branch() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing);
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_ternary_mismatched_checked_branch.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = profile != null ? { id: 2 } : { id: 1 };
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow ternary candidates when ternary checks are disabled.
    #[test]
    fn test_allows_ternary_when_option_ignores_ternary_checks() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing).with_options(
            |options| options.style.prefer_nullish_coalescing_ignore_ternary_tests = true,
        );
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_ternary_when_option_ignores_ternary_checks.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = profile != null ? profile : { id: 1 };
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Allow mixed logical candidates when mixed logical checks are disabled.
    #[test]
    fn test_allows_mixed_logical_when_option_ignores_mixed_logical() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing).with_options(
            |options| {
                options
                    .style
                    .prefer_nullish_coalescing_ignore_mixed_logical_expressions = true
            },
        );
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_allows_mixed_logical_when_option_ignores_mixed_logical.ds",
            r#"
let profile: { id: int32 } | null = null;
let selected = profile || (true && { id: 1 });
"#,
        );

        test.result(diagnostics)
            .assert_no_lint("prefer-nullish-coalescing");
    }

    /// Report condition tests when condition ignores are disabled.
    #[test]
    fn test_flags_condition_context_when_option_disables_condition_ignore() {
        let test = TestProgram::for_rule_without_prelude(PreferNullishCoalescing).with_options(
            |options| {
                options
                    .style
                    .prefer_nullish_coalescing_ignore_conditional_tests = false
            },
        );
        let diagnostics = test.lint_dir(
            "prefer_nullish_coalescing/test_flags_condition_context_when_option_disables_condition_ignore.ds",
            r#"
let profile: { id: int32 } | null = null;
if (profile || { id: 1 }) {
}
"#,
        );

        test.result(diagnostics)
            .assert_lint("prefer-nullish-coalescing")
            .assert_has_fix("prefer-nullish-coalescing")
            .assert_safe_fixed(
                r#"
let profile: { id: int32 } | null = null;
if (profile ?? { id: 1 }) {
}
"#,
            );
    }
}
