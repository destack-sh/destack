use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_unwrap_transparent, expressions_have_equivalent_syntax};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow increment/decrement whose result is unused.
    ///
    /// When an increment or decrement expression appears as a statement
    /// and the variable is never used afterward, the operation has no effect.
    /// This often indicates dead code or a logic error.
    #[lint(
        id = "no-useless-increment",
        code = "LC039",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessIncrement,
    "Disallow increment/decrement with unused result"
}

impl LintRule for NoUselessIncrement {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUselessIncrement::meta()
    }

    /// Check module DIR nodes for useless increments.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = UselessIncrementVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags useless increment/decrement operations.
struct UselessIncrementVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> UselessIncrementVisitor<'a, 'b> {
    /// Build a visitor for useless increment checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check if a postfix update in return position is useless.
    fn check_useless_in_return(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // match postfix increment/decrement in return
        let dir::Expression::Unary { operator, right } = expression else {
            return;
        };

        // only check postfix operators
        if !matches!(
            operator,
            dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PostDecrement
        ) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        self.report_useless_postfix_update(expression_id, *operator, *right, severity, "in return");
    }

    /// Check one assignment for a useless postfix update on the right side.
    fn check_useless_postfix_assignment(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        right_id: dir::LocalNodeId<dir::Expression>,
    ) {
        let normalized_right_id = expression_unwrap_transparent(self.ctx.tree, right_id);
        let right_expression = self.ctx.tree.get(normalized_right_id);
        let dir::Expression::Unary { operator, right } = right_expression else {
            return;
        };

        // enforce this lint guard
        if !matches!(
            operator,
            dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PostDecrement
        ) {
            return;
        }

        // enforce this lint guard
        if !expressions_have_equivalent_syntax(self.ctx, left_id, *right) {
            return;
        }

        // resolve effective lint severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        self.report_useless_postfix_update(
            normalized_right_id,
            *operator,
            *right,
            severity,
            "in self-assignment",
        );
    }

    /// Report one useless postfix update.
    fn report_useless_postfix_update(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        operand_id: dir::LocalNodeId<dir::Expression>,
        severity: LintSeverity,
        context: &str,
    ) {
        let span = self.ctx.get_span(expression_id);
        let op_name = match operator {
            dir::UnaryOperator::PostIncrement => "increment",
            dir::UnaryOperator::PostDecrement => "decrement",
            _ => "update",
        };
        let mut diagnostic = LintDiagnostic::new(
            NO_USELESS_INCREMENT.id,
            NO_USELESS_INCREMENT.code,
            NO_USELESS_INCREMENT.category,
            severity,
            format!("postfix {op_name} {context} has no effect"),
            self.ctx.module.file_id,
            span,
        )
        .with_label("the updated value is discarded");

        // attach fix when enabled
        if self.ctx.include_fixes
            && let Some(fix) = self.no_useless_increment_fix(expression_id, operator, operand_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build an unsafe fix that rewrites postfix updates to prefix updates.
    fn no_useless_increment_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        operand_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintFix> {
        let operand_span = self.ctx.get_span(operand_id);
        let operand_text = self.ctx.get_span_text(operand_span);
        if operand_text.trim().is_empty() {
            return None;
        }

        // rewrite postfix operators to prefix operators
        let replacement = match operator {
            dir::UnaryOperator::PostIncrement => format!("++{operand_text}"),
            dir::UnaryOperator::PostDecrement => format!("--{operand_text}"),
            _ => return None,
        };

        // replace the full update expression text
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();
        Some(LintFix::r#unsafe("Rewrite postfix update to prefix update").with_edits(edits))
    }
}

impl NodeVisitor for UselessIncrementVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check assignments where postfix right side targets the same reference
        if let dir::Expression::Assign { left, right } = expression {
            self.check_useless_postfix_assignment(id, *left, *right);
        }

        // check return statements with postfix increment/decrement
        if let dir::Expression::Return { value: Some(value) } = expression {
            let normalized_value_id = expression_unwrap_transparent(tree, *value);
            let normalized_value = tree.get(normalized_value_id);
            if matches!(
                normalized_value,
                dir::Expression::Unary {
                    operator: dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PostDecrement,
                    ..
                }
            ) {
                self.check_useless_in_return(normalized_value_id, normalized_value);
            }
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag postfix increment in return.
    #[test]
    fn test_flags_postfix_increment_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_flags_postfix_increment_in_return.ds",
            r#"
function getAndIncrement(): int32 {
    let x = 0;
    return x++;
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-increment")
            .assert_has_fix("no-useless-increment");
    }

    /// Flag postfix decrement in return.
    #[test]
    fn test_flags_postfix_decrement_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_flags_postfix_decrement_in_return.ds",
            r#"
function getAndDecrement(): int32 {
    let x = 10;
    return x--;
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-increment")
            .assert_has_fix("no-useless-increment");
    }

    /// Allow prefix increment in return.
    #[test]
    fn test_allows_prefix_increment() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_allows_prefix_increment.ds",
            r#"
function incrementAndGet(): int32 {
    let x = 0;
    return ++x;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-increment");
    }

    /// Allow postfix increment not in return.
    #[test]
    fn test_allows_postfix_not_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_allows_postfix_not_in_return.ds",
            r#"
function count(): int32 {
    let x = 0;
    x++;
    x++;
    return x;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-increment");
    }

    /// Flag parenthesized postfix updates in return.
    #[test]
    fn test_flags_parenthesized_postfix_increment_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_flags_parenthesized_postfix_increment_in_return.ds",
            r#"
function getAndIncrement(): int32 {
    let x = 0;
    return (x++);
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-increment")
            .assert_has_fix("no-useless-increment");
    }

    /// Unsafely rewrite postfix increment returns to prefix updates.
    #[test]
    fn test_fix_rewrites_postfix_increment_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_fix_rewrites_postfix_increment_in_return.ds",
            r#"
function getAndIncrement(): int32 {
    let x = 0;
    return x++;
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-increment")
            .assert_unsafe_fixed(
                r#"
function getAndIncrement(): int32 {
    let x = 0;
    return ++x;
}
"#,
            );
    }

    /// Unsafely rewrite postfix decrement returns to prefix updates.
    #[test]
    fn test_fix_rewrites_postfix_decrement_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_fix_rewrites_postfix_decrement_in_return.ds",
            r#"
function getAndDecrement(): int32 {
    let x = 10;
    return x--;
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-increment")
            .assert_unsafe_fixed(
                r#"
function getAndDecrement(): int32 {
    let x = 10;
    return --x;
}
"#,
            );
    }

    /// Mutation: rewrite postfix member updates in return expressions.
    #[test]
    fn test_mutation_fix_rewrites_member_postfix_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_mutation_fix_rewrites_member_postfix_return.ds",
            r#"
function next(items: int32[], index: int32): int32 {
    return items[index]++;
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-increment")
            .assert_unsafe_fixed(
                r#"
function next(items: int32[], index: int32): int32 {
    return ++items[index];
}
"#,
            );
    }

    /// Flag useless postfix updates in self assignments.
    #[test]
    fn test_flags_postfix_update_in_self_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "no_useless_increment/test_flags_postfix_update_in_self_assignment.ds",
            r#"
function keep(value: int32): int32 {
    value = value++;
    return value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-increment")
            .assert_has_fix("no-useless-increment");
    }
}
