use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow increment/decrement whose result is unused.
    ///
    /// When an increment or decrement expression appears as a statement
    /// and the variable is never used afterward, the operation has no effect.
    /// This often indicates dead code or a logic error.
    #[lint(
        id = "no-useless-increment",
        code = "LC052",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
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

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check if an update expression is useless (in return position).
    fn check_useless_in_return(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // match postfix increment/decrement in return
        let dir::Expression::Unary { operator, .. } = expression else {
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

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let op_name = match operator {
            dir::UnaryOperator::PostIncrement => "increment",
            dir::UnaryOperator::PostDecrement => "decrement",
            _ => "update",
        };
        self.ctx.report(
            LintDiagnostic::new(
                NO_USELESS_INCREMENT.id,
                NO_USELESS_INCREMENT.code,
                NO_USELESS_INCREMENT.category,
                severity,
                format!("postfix {op_name} in return has no effect"),
                self.ctx.module.file_id,
                span,
            )
            .with_label("the updated value is discarded"),
        );
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
        // check return statements with postfix increment/decrement
        if let dir::Expression::Return { value: Some(value) } = expression {
            let inner_expr = tree.get(*value);
            self.check_useless_in_return(*value, inner_expr);
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
            "test.ds",
            r#"
function getAndIncrement(): int32 {
    let x = 0;
    return x++;
}
"#,
        );
        test.result(result).assert_lint("no-useless-increment");
    }

    /// Flag postfix decrement in return.
    #[test]
    fn test_flags_postfix_decrement_in_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "test.ds",
            r#"
function getAndDecrement(): int32 {
    let x = 10;
    return x--;
}
"#,
        );
        test.result(result).assert_lint("no-useless-increment");
    }

    /// Allow prefix increment in return.
    #[test]
    fn test_allows_prefix_increment() {
        let test = TestProgram::for_rule_without_prelude(NoUselessIncrement);
        let result = test.lint_dir(
            "test.ds",
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
            "test.ds",
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
}
