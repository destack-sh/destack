use std::collections::{HashMap, HashSet};

use destack_dir::{self as dir, GlobalSymbolId, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    assign_pattern_target_symbol, collect_expression_read_symbol_usage,
    collect_pattern_value_binding_symbols,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

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
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = UselessAssignmentVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags useless assignments.
struct UselessAssignmentVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> UselessAssignmentVisitor<'a, 'b> {
    /// Build a visitor for useless assignment checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a block for consecutive assignments to the same variable.
    fn check_block(&mut self, block_id: dir::LocalNodeId<dir::Block>) {
        // skip try bodies: exceptional control flow can bypass local overwrite ordering
        if block_is_try_body(self.ctx.dir.tree(), block_id) {
            return;
        }

        // resolve block
        let block = self.ctx.dir.get(block_id);

        // track the last assignment expression for each variable
        let mut last_assignments: HashMap<GlobalSymbolId, dir::LocalNodeId<dir::Expression>> =
            HashMap::new();
        let mut to_report: HashSet<u32> = HashSet::new();

        // inspect candidate nodes
        for expr_id in block.iter_expressions() {
            let inner_id = expr_id;
            let inner_expr = self.ctx.dir.get(inner_id);

            // check if this expression reads any of the assigned variables
            let reads = collect_expression_read_symbol_usage(
                self.ctx.module_id(),
                self.ctx.dir.tree(),
                self.ctx.types,
                inner_id,
            );
            for read in &reads {
                last_assignments.remove(read);
            }

            // check if this expression writes assigned variables
            for assigned_symbol in self.assignment_targets(inner_expr) {
                // if there was a previous assignment, it is useless
                if let Some(prev_expr_id) = last_assignments.get(&assigned_symbol) {
                    to_report.insert(prev_expr_id.id);
                }
                last_assignments.insert(assigned_symbol, inner_id);
            }

            // stop at terminating control flow because later statements are unreachable
            if expression_stops_execution(inner_expr) {
                break;
            }
        }

        // report useless assignments
        for expr_id in to_report {
            self.report(dir::LocalNodeId::new(expr_id));
        }
    }

    /// Get write targets for one assignment-like expression.
    fn assignment_targets(&self, expression: &dir::Expression) -> Vec<GlobalSymbolId> {
        let mut targets = Vec::new();

        // branch by expression kind
        match expression {
            // regular assignments like `x = 1`
            dir::Expression::Assign { left, .. } => {
                if let Some(target_symbol) = assign_pattern_target_symbol(self.ctx, *left) {
                    targets.push(target_symbol);
                }
            }

            // let bindings like `let x = 1` and destructuring patterns
            dir::Expression::Let { declarators, .. } => {
                for declarator_id in declarators {
                    let declarator = self.ctx.dir.get(*declarator_id);
                    let mut local_symbols = HashSet::new();
                    collect_pattern_value_binding_symbols(
                        self.ctx.dir.tree(),
                        self.ctx.symbols,
                        declarator.pattern,
                        &mut local_symbols,
                    );
                    for local_symbol in local_symbols {
                        targets.push(local_symbol.into_global(self.ctx.module.id));
                    }
                }
            }

            _ => {}
        }

        targets
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
        let diagnostic = LintReport::new(
            NO_USELESS_ASSIGNMENT.id,
            NO_USELESS_ASSIGNMENT.code,
            NO_USELESS_ASSIGNMENT.category,
            severity,
            "assignment is immediately overwritten",
            span,
        )
        .label("this value is never used");

        self.ctx.report(diagnostic);
    }
}

/// Return true when one block appears in a try body chain.
fn block_is_try_body(tree: &dir::Tree, block_id: dir::LocalNodeId<dir::Block>) -> bool {
    let Some(parent) = tree.get_parent(block_id.id) else {
        return false;
    };
    if parent.ty != dir::NodeType::Expression {
        return false;
    }

    // resolve current expression id
    let mut current_expression_id = parent.into_typed::<dir::Expression>();
    loop {
        let Some(parent) = tree.get_parent(current_expression_id.id) else {
            return false;
        };
        if parent.ty != dir::NodeType::Expression {
            return false;
        }

        // walk up expression parents and check for an enclosing try block
        let parent_expression_id = parent.into_typed::<dir::Expression>();
        let parent_expression = tree.get(parent_expression_id);
        if let dir::Expression::Try { try_expression, .. } = parent_expression
            && *try_expression == current_expression_id
        {
            return true;
        }

        current_expression_id = parent_expression_id;
    }
}

/// Return true when this expression unconditionally stops block execution.
fn expression_stops_execution(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::Return { .. }
            | dir::Expression::Throw { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
    )
}

/// Collect read symbols in one expression subtree.
impl NodeVisitor for UselessAssignmentVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_block(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Block>,
        block: &dir::Block,
    ) {
        // check this block for useless assignments
        self.check_block(id);

        // walk children
        for expr_id in block.iter_expressions() {
            let expression = tree.get(expr_id);
            self.visit_expression(tree, expr_id, expression);
        }
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
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

    /// Remove overwritten assignments.
    #[test]
    fn test_fix_removes_overwritten_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_fix_removes_overwritten_assignment.ds",
            r#"
function test(): void {
    let x: int32;
    x = 1;
    x = 2;
    console.log(x);
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-assignment")
            .assert_has_no_fix("no-useless-assignment");
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

    /// Mutation: remove overwritten let declarations.
    #[test]
    fn test_mutation_fix_removes_overwritten_let_declaration() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_mutation_fix_removes_overwritten_let_declaration.ds",
            r#"
function test(): void {
    let value: int32;
    value = 1;
    value = 2;
    return value;
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-assignment")
            .assert_has_no_fix("no-useless-assignment");
    }

    /// Keep side effecting overwritten assignments diagnostic only.
    #[test]
    fn test_flags_side_effecting_overwritten_assignment_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_flags_side_effecting_overwritten_assignment_without_fix.ds",
            r#"
function read(): int32 {
    return 1;
}

function test(): void {
    let value: int32;
    value = read();
    value = 2;
    return;
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-assignment")
            .assert_has_no_fix("no-useless-assignment");
    }

    /// Allow assignment used in a conditional expression before overwrite.
    #[test]
    fn test_allows_assignment_read_in_condition_before_overwrite() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_allows_assignment_read_in_condition_before_overwrite.ds",
            r#"
function test(): void {
    let value = 1;
    if (value > 0) {
        log(value);
    }
    value = 2;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-assignment");
    }

    /// Allow compound assignments that read the previous value.
    #[test]
    fn test_allows_compound_assignment_reading_previous_value() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_allows_compound_assignment_reading_previous_value.ds",
            r#"
function test(): void {
    let value = 1;
    value += 2;
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-assignment");
    }

    /// Ignore unreachable overwrites after a terminating return.
    #[test]
    fn test_ignores_unreachable_overwrite_after_return() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_ignores_unreachable_overwrite_after_return.ds",
            r#"
function test(value: int32): int32 {
    let current = value;
    return current;
    current = 0;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-assignment");
    }

    /// Ignore overwritten assignments inside try bodies.
    #[test]
    fn test_ignores_try_body_overwrite() {
        let test = TestProgram::for_rule_without_prelude(NoUselessAssignment);
        let result = test.lint_dir(
            "no_useless_assignment/test_ignores_try_body_overwrite.ds",
            r#"
function test(): int32 {
    let value = 0;
    try {
        value = 1;
        value = 2;
    } catch {
    }
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-assignment");
    }
}
