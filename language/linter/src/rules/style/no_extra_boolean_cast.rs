use destack_core::StringId;
use destack_dir::{
    self as dir, NodeVisitor, NodeVisitorOptions, UnaryOperator, WellKnownSymbol, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_is_symbol_or_global_qualified_member, expression_parent_id,
    expression_unwrap_parenthesized, source_text_contains_comment_token,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary boolean casts.
    ///
    /// Boolean casts inside boolean contexts add noise and do not
    /// change behavior.
    #[lint(
        id = "no-extra-boolean-cast",
        code = "LY067",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Boolean)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoExtraBooleanCast,
    "Disallow unnecessary boolean casts"
}

impl LintRule for NoExtraBooleanCast {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoExtraBooleanCast::meta()
    }

    /// Check module DIR nodes for redundant boolean casts.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoExtraBooleanCastVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags redundant boolean casts.
struct NoExtraBooleanCastVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Boolean symbol for this module.
    boolean_symbol: dir::GlobalSymbolId,
    /// The Boolean member name.
    boolean_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoExtraBooleanCastVisitor<'a, 'b> {
    /// Build a visitor for no-extra-boolean-cast checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let boolean_symbol = ctx.well_known_symbol(WellKnownSymbol::Boolean);
        let boolean_name = ctx.string_id("Boolean");
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            boolean_symbol,
            boolean_name,
            global_qualifiers,
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

    /// Report one redundant boolean cast diagnostic.
    fn report(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        replacement_id: dir::LocalNodeId<dir::Expression>,
        message: &'static str,
        label: &'static str,
    ) {
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            NO_EXTRA_BOOLEAN_CAST.id,
            NO_EXTRA_BOOLEAN_CAST.code,
            NO_EXTRA_BOOLEAN_CAST.category,
            severity,
            message,
            self.ctx.module.file_id,
            span,
        )
        .with_label(label);

        if let Some(fix) = build_no_extra_boolean_cast_fix(self.ctx, expression_id, replacement_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return true when the expression resolves to the built in Boolean constructor.
    fn is_boolean_reference(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol_or_global_qualified_member(
            self.ctx.tree,
            expression_id,
            self.boolean_symbol,
            &self.global_qualifiers,
            self.boolean_name,
        )
    }

    /// Check `Boolean(value)` and `new Boolean(value)` calls.
    fn check_boolean_cast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        if !self.is_boolean_reference(callee_id) {
            return;
        }

        if arguments.len() != 1 {
            return;
        }

        if !self.expression_is_in_flagged_context(expression_id) {
            return;
        }

        let argument = self.ctx.tree.get(arguments[0]);
        self.report(
            expression_id,
            argument.value(),
            "redundant Boolean() cast",
            "this cast is redundant in a boolean context",
        );
    }

    /// Check `!!value` expressions for redundant casts.
    fn check_double_negation(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        let inner_expression_id = expression_unwrap_parenthesized(self.ctx.tree, right);
        let inner_expression = self.ctx.tree.get(inner_expression_id);
        let dir::Expression::Unary {
            operator: UnaryOperator::Not,
            right: inner_right,
        } = inner_expression
        else {
            return;
        };

        if !self.expression_is_in_flagged_context(expression_id) {
            return;
        }

        self.report(
            expression_id,
            *inner_right,
            "redundant double negation",
            "this cast is redundant in a boolean context",
        );
    }

    /// Return true when one expression is in a flagged boolean context.
    fn expression_is_in_flagged_context(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        self.expression_is_in_flagged_context_inner(
            expression_id,
            self.ctx
                .options
                .style
                .no_extra_boolean_cast_enforce_for_inner_expressions,
        )
    }

    /// Return true when one expression is in a flagged boolean context.
    fn expression_is_in_flagged_context_inner(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        enforce_inner: bool,
    ) -> bool {
        let Some(parent_id) = expression_parent_id(self.ctx.tree, expression_id) else {
            return false;
        };
        let parent_expression = self.ctx.tree.get(parent_id);

        // parenthesized wrappers
        if matches!(parent_expression, dir::Expression::Parenthesized { .. }) {
            return self.expression_is_in_flagged_context_inner(parent_id, enforce_inner);
        }

        // direct boolean contexts
        match parent_expression {
            dir::Expression::Call {
                left, arguments, ..
            }
            | dir::Expression::New {
                left, arguments, ..
            } => {
                if arguments.first().is_some_and(|argument_id| {
                    self.ctx.tree.get(*argument_id).value() == expression_id
                }) && self.is_boolean_reference(*left)
                {
                    return true;
                }
            }
            dir::Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                if *right == expression_id {
                    return true;
                }
            }
            dir::Expression::If {
                condition: dir::IfCondition::Expression { condition },
                ..
            } => {
                if *condition == expression_id {
                    return true;
                }
            }
            dir::Expression::For { condition, .. } => {
                if *condition == Some(expression_id) {
                    return true;
                }
            }
            dir::Expression::Loop { condition, .. } => {
                if *condition == Some(expression_id) {
                    return true;
                }
            }
            _ => {}
        }

        if !enforce_inner {
            return false;
        }

        // nested inner expressions
        match parent_expression {
            dir::Expression::Binary {
                operator: dir::BinaryOperator::And | dir::BinaryOperator::Or,
                left,
                right,
            } => {
                if *left == expression_id || *right == expression_id {
                    return self.expression_is_in_flagged_context_inner(parent_id, true);
                }
            }
            dir::Expression::Binary {
                operator: dir::BinaryOperator::Coalesce,
                right,
                ..
            } => {
                if *right == expression_id {
                    return self.expression_is_in_flagged_context_inner(parent_id, true);
                }
            }
            dir::Expression::If {
                kind: dir::IfKind::Ternary,
                then_expression,
                else_expression,
                ..
            } => {
                if *then_expression == expression_id || *else_expression == Some(expression_id) {
                    return self.expression_is_in_flagged_context_inner(parent_id, true);
                }
            }
            dir::Expression::SequenceExpression { expressions } => {
                if expressions.last().copied() == Some(expression_id) {
                    return self.expression_is_in_flagged_context_inner(parent_id, true);
                }
            }
            _ => {}
        }

        false
    }
}

impl NodeVisitor for NoExtraBooleanCastVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // Boolean(value)
        if let dir::Expression::Call {
            left, arguments, ..
        } = expression
        {
            self.check_boolean_cast(id, *left, arguments);
        }

        // new Boolean(value)
        if let dir::Expression::New {
            left, arguments, ..
        } = expression
        {
            self.check_boolean_cast(id, *left, arguments);
        }

        // !!value
        if let dir::Expression::Unary {
            operator: UnaryOperator::Not,
            right,
        } = expression
        {
            self.check_double_negation(id, *right);
        }

        walk_expression(self, tree, id, expression);
    }
}

/// Build a safe fix for one redundant boolean cast.
fn build_no_extra_boolean_cast_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    replacement_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let span = ctx.get_span(expression_id);
    let replacement_span = ctx.get_span(replacement_id);
    if source_text_contains_comment_token(ctx.get_span_text(span))
        || source_text_contains_comment_token(ctx.get_span_text(replacement_span))
    {
        return None;
    }

    let mut replacement_text = ctx.get_span_text(replacement_span).to_string();
    if replacement_needs_parentheses(ctx.tree, expression_id, replacement_id) {
        replacement_text = format!("({replacement_text})");
    }

    let edits = ctx
        .edit_builder()
        .replace(span, replacement_text)
        .into_edits();
    Some(LintFix::safe("Remove redundant boolean cast").with_edits(edits))
}

/// Return true when one replacement expression needs protective parentheses.
fn replacement_needs_parentheses(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    replacement_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let replacement_id = expression_unwrap_parenthesized(tree, replacement_id);
    let replacement_expression = tree.get(replacement_id);
    let Some(parent_id) = expression_parent_id(tree, expression_id) else {
        return false;
    };
    let parent_expression = tree.get(parent_id);

    match parent_expression {
        dir::Expression::Unary {
            operator: UnaryOperator::Not,
            right,
        } if *right == expression_id => matches!(
            replacement_expression,
            dir::Expression::Binary { .. }
                | dir::Expression::Assign { .. }
                | dir::Expression::AssignBinary { .. }
                | dir::Expression::If {
                    kind: dir::IfKind::Ternary,
                    ..
                }
                | dir::Expression::SequenceExpression { .. }
        ),
        dir::Expression::Call { arguments, .. } | dir::Expression::New { arguments, .. }
            if arguments
                .first()
                .is_some_and(|argument_id| tree.get(*argument_id).value() == expression_id) =>
        {
            matches!(
                replacement_expression,
                dir::Expression::SequenceExpression { .. }
            )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Allow Boolean casts in ordinary value contexts.
    #[test]
    fn test_allows_boolean_constructor_assignment() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_boolean_constructor_assignment.ds",
            r#"
let flag = true;
let value = Boolean(flag);
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Report Boolean casts in if conditions.
    #[test]
    fn test_flags_boolean_constructor_in_if_condition() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_boolean_constructor_in_if_condition.ds",
            r#"
let flag = true;
if (Boolean(flag)) {
    let value = flag;
}
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }

    /// Safely remove Boolean casts in if conditions.
    #[test]
    fn test_fix_boolean_constructor_in_if_condition() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_fix_boolean_constructor_in_if_condition.ds",
            r#"
let flag = true;
if (Boolean(flag)) {
    let value = flag;
}
"#,
        );
        test.result(result)
            .assert_lint("no-extra-boolean-cast")
            .assert_has_fix("no-extra-boolean-cast")
            .assert_safe_fixed(
                r#"
let flag = true;
if (flag) {
    let value = flag;
}
"#,
            );
    }

    /// Report new Boolean casts in if conditions.
    #[test]
    fn test_flags_new_boolean_in_if_condition() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_new_boolean_in_if_condition.ds",
            r#"
let flag = true;
if (new Boolean(flag)) {
    let value = flag;
}
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }

    /// Report double negation in if conditions.
    #[test]
    fn test_flags_double_negation_in_if_condition() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_double_negation_in_if_condition.ds",
            r#"
let flag = true;
if (!!flag) {
    let value = flag;
}
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }

    /// Safely remove double negation in if conditions.
    #[test]
    fn test_fix_double_negation_in_if_condition() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_fix_double_negation_in_if_condition.ds",
            r#"
let flag = true;
if (!!flag) {
    let value = flag;
}
"#,
        );
        test.result(result)
            .assert_lint("no-extra-boolean-cast")
            .assert_has_fix("no-extra-boolean-cast")
            .assert_safe_fixed(
                r#"
let flag = true;
if (flag) {
    let value = flag;
}
"#,
            );
    }

    /// Allow double negation in ordinary value contexts.
    #[test]
    fn test_allows_double_negation_assignment() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_double_negation_assignment.ds",
            r#"
let flag = true;
let value = !!flag;
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Report nested logical operands only when configured.
    #[test]
    fn test_flags_logical_operand_when_enforced() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast).with_options(|options| {
            options
                .style
                .no_extra_boolean_cast_enforce_for_inner_expressions = true;
        });
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_logical_operand_when_enforced.ds",
            r#"
let value = false;
let flag = true;
if (value || !!flag) {
    let seen = flag;
}
"#,
        );
        test.result(result).assert_lint("no-extra-boolean-cast");
    }

    /// Allow nested logical operands by default.
    #[test]
    fn test_allows_logical_operand_by_default() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_logical_operand_by_default.ds",
            r#"
let value = false;
let flag = true;
if (value || !!flag) {
    let seen = flag;
}
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Report nested ternary branches when enforced.
    #[test]
    fn test_flags_ternary_branch_when_enforced() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast).with_options(|options| {
            options
                .style
                .no_extra_boolean_cast_enforce_for_inner_expressions = true;
        });
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_flags_ternary_branch_when_enforced.ds",
            r#"
let ready = true;
let left = true;
let right = false;
if (ready ? !!left : !!right) {
    let seen = left;
}
"#,
        );
        test.result(result)
            .assert_lint_count("no-extra-boolean-cast", 2);
    }

    /// Keep shadowed Boolean references out of the lint path.
    #[test]
    fn test_allows_shadowed_boolean_symbol() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_allows_shadowed_boolean_symbol.ds",
            r#"
function Boolean(value: boolean): boolean {
    return value;
}

let flag = true;
if (Boolean(flag)) {
    let value = flag;
}
"#,
        );
        test.result(result).assert_no_lint("no-extra-boolean-cast");
    }

    /// Preserve grouping under outer unary negation.
    #[test]
    fn test_fix_boolean_constructor_under_not() {
        let test = TestProgram::for_rule_with_prelude(NoExtraBooleanCast);
        let result = test.lint_dir(
            "no_extra_boolean_cast/test_fix_boolean_constructor_under_not.ds",
            r#"
let left = true;
let right = false;
if (!Boolean(left && right)) {
    let seen = left;
}
"#,
        );
        test.result(result)
            .assert_lint("no-extra-boolean-cast")
            .assert_has_fix("no-extra-boolean-cast")
            .assert_safe_fixed(
                r#"
let left = true;
let right = false;
if (!(left && right)) {
    let seen = left;
}
"#,
            );
    }
}
