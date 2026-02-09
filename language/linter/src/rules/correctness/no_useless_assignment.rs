use std::collections::{HashMap, HashSet};

use destack_dir::{self as dir, GlobalSymbolId, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow assignments that are immediately overwritten.
    ///
    /// Assigning a value to a variable and then immediately reassigning
    /// it without using the first value is likely a bug or dead code.
    #[lint(
        id = "no-useless-assignment",
        code = "LC038",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessAssignment,
    "Disallow assignments immediately overwritten"
}

impl LintRule for NoUselessAssignment {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUselessAssignment::meta()
    }

    /// Check module DIR nodes for useless assignments.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = UselessAssignmentVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags useless assignments.
struct UselessAssignmentVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> UselessAssignmentVisitor<'a, 'b> {
    /// Build a visitor for useless assignment checks.
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

    /// Check a block for consecutive assignments to the same variable.
    fn check_block(&mut self, block_id: dir::LocalNodeId<dir::Block>) {
        let block = self.ctx.tree.get(block_id);

        // track the last assignment expression for each variable
        let mut last_assignments: HashMap<GlobalSymbolId, dir::LocalNodeId<dir::Expression>> =
            HashMap::new();
        let mut to_report: Vec<dir::LocalNodeId<dir::Expression>> = Vec::new();

        for expr_id in &block.expressions {
            let expression = self.ctx.tree.get(*expr_id);

            // unwrap statement wrapper if present
            let inner_id = if let dir::Expression::Statement { statement } = expression {
                *statement
            } else {
                *expr_id
            };
            let inner_expr = self.ctx.tree.get(inner_id);

            // check if this expression reads any of the assigned variables
            let reads = self.collect_reads_expr(inner_id);
            for read in &reads {
                last_assignments.remove(read);
            }

            // check if this expression is an assignment
            if let Some(assigned_symbol) = self.get_assignment_target(inner_expr) {
                // if there was a previous assignment, it's useless
                if let Some(prev_expr_id) = last_assignments.get(&assigned_symbol) {
                    to_report.push(*prev_expr_id);
                }
                last_assignments.insert(assigned_symbol, inner_id);
            }
        }

        // report useless assignments
        for expr_id in to_report {
            self.report(expr_id);
        }
    }

    /// Get the assignment target symbol if this is an assignment expression.
    fn get_assignment_target(&self, expression: &dir::Expression) -> Option<GlobalSymbolId> {
        match expression {
            // regular assignment like `x = 1`
            dir::Expression::Assign { left, .. } => {
                let left_expr = self.ctx.tree.get(*left);
                left_expr.target_symbol()
            }
            // let binding like `let x = 1`
            dir::Expression::Let { declarators, .. } => {
                // only handle single declarator for simplicity
                if declarators.len() == 1 {
                    let declarator = self.ctx.tree.get(declarators[0]);
                    let pattern = self.ctx.tree.get(declarator.pattern);
                    if let dir::Pattern::Binding { symbol, .. } = pattern {
                        return Some(symbol.into_global(self.ctx.module.id));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Collect all symbol reads in an expression.
    fn collect_reads_expr(
        &self,
        expr_id: dir::LocalNodeId<dir::Expression>,
    ) -> HashSet<GlobalSymbolId> {
        let mut reads = HashSet::new();
        self.collect_reads_recursive(expr_id, &mut reads);
        reads
    }

    /// Recursively collect reads from an expression.
    fn collect_reads_recursive(
        &self,
        expr_id: dir::LocalNodeId<dir::Expression>,
        reads: &mut HashSet<GlobalSymbolId>,
    ) {
        let expr = self.ctx.tree.get(expr_id);

        // skip the left side of assignments (only collect from right side)
        if let dir::Expression::Assign { right, .. } = expr {
            self.collect_reads_recursive(*right, reads);
            return;
        }

        // skip let bindings (only collect from initializer)
        if let dir::Expression::Let { declarators, .. } = expr {
            for decl_id in declarators {
                let decl = self.ctx.tree.get(*decl_id);
                if let Some(value) = decl.value {
                    self.collect_reads_recursive(value, reads);
                }
            }
            return;
        }

        // collect reference reads
        if let Some(symbol) = expr.target_symbol() {
            reads.insert(symbol);
        }

        // walk children based on expression type
        match expr {
            dir::Expression::Binary { left, right, .. } => {
                self.collect_reads_recursive(*left, reads);
                self.collect_reads_recursive(*right, reads);
            }
            dir::Expression::Unary { right, .. } => {
                self.collect_reads_recursive(*right, reads);
            }
            dir::Expression::Call {
                left,
                dynamic_arguments,
                ..
            } => {
                self.collect_reads_recursive(*left, reads);
                for arg_id in dynamic_arguments {
                    let arg = self.ctx.tree.get(*arg_id);
                    self.collect_reads_recursive(arg.value(), reads);
                }
            }
            dir::Expression::Member { left, .. } => {
                self.collect_reads_recursive(*left, reads);
            }
            dir::Expression::Index { left, right } => {
                self.collect_reads_recursive(*left, reads);
                if let Some(right) = right {
                    self.collect_reads_recursive(*right, reads);
                }
            }
            _ => {}
        }
    }

    /// Report a useless assignment.
    fn report(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_USELESS_ASSIGNMENT.id,
                NO_USELESS_ASSIGNMENT.code,
                NO_USELESS_ASSIGNMENT.category,
                severity,
                "assignment is immediately overwritten",
                self.ctx.module.file_id,
                span,
            )
            .with_label("this value is never used"),
        );
    }
}

impl NodeVisitor for UselessAssignmentVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_block(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) {
        // check this block for useless assignments
        self.check_block(id);

        // walk children
        for expr_id in &block.expressions {
            let expression = tree.get(*expr_id);
            self.visit_expression(tree, *expr_id, expression);
        }
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag immediately overwritten assignment.
    #[test]
    fn test_flags_overwritten_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_flags_overwritten_assignment.ds",
            r#"
function test(): void {
    let x = 1;
    x = 2;
    console.log(x);
}
"#,
        );
        test.result(result).assert_lint("no-useless-assignment");
    }

    /// Allow assignment followed by use.
    #[test]
    fn test_allows_used_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_allows_used_assignment.ds",
            r#"
function test(): void {
    let x = 1;
    console.log(x);
    x = 2;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-assignment");
    }

    /// Allow different variables.
    #[test]
    fn test_allows_different_variables() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_allows_different_variables.ds",
            r#"
function test(): void {
    let x = 1;
    let y = 2;
    let z = x + y;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-assignment");
    }
}
