use destack_base::StringId;
use destack_dir::{
    self as dir, GlobalSymbolId, LocalNodeId, Mutability, NodeVisitor, NodeVisitorOptions,
    WellKnownSymbol, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::is_array_type;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `for-of` over index-based `for` loops.
    ///
    /// When iterating over an array using only `arr[i]` access patterns,
    /// a `for-of` loop is cleaner and less error-prone.
    #[lint(
        id = "prefer-for-of",
        code = "LP018",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferForOf,
    "Prefer for-of over index-based for loops"
}

impl LintRule for PreferForOf {
    fn meta(&self) -> &'static LintMeta {
        PreferForOf::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferForOfVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags index-based for loops.
struct PreferForOfVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the length property.
    length_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferForOfVisitor<'a, 'b> {
    /// Build a visitor for prefer-for-of checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let length_name = ctx.program.strings.intern("length");

        Self {
            ctx,
            meta,
            array_symbol,
            length_name,
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

    /// Check a for loop expression for prefer-for-of pattern.
    fn check_for(
        &mut self,
        expression_id: LocalNodeId<dir::Expression>,
        initialization: Option<LocalNodeId<dir::Expression>>,
        condition: Option<LocalNodeId<dir::Expression>>,
        increment: Option<LocalNodeId<dir::Expression>>,
        body: LocalNodeId<dir::Block>,
    ) {
        // require all parts of a classic for loop
        let Some(init_id) = initialization else {
            return;
        };
        let Some(cond_id) = condition else {
            return;
        };
        let Some(incr_id) = increment else {
            return;
        };

        // extract the index variable from initialization
        let Some((index_symbol, array_symbol)) = self.extract_for_loop_pattern(init_id, cond_id)
        else {
            return;
        };

        // verify the increment is i++ or i += 1
        if !self.is_simple_increment(incr_id, index_symbol) {
            return;
        }

        // scan the body to see if the index is only used for array indexing
        if !self.index_only_used_for_indexing(body, index_symbol, array_symbol) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                PREFER_FOR_OF.id,
                PREFER_FOR_OF.code,
                PREFER_FOR_OF.category,
                severity,
                "prefer for-of over index-based for loop",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use for-of to iterate directly over elements"),
        );
    }

    /// Extract the index variable and array from a for loop pattern.
    fn extract_for_loop_pattern(
        &self,
        init_id: LocalNodeId<dir::Expression>,
        cond_id: LocalNodeId<dir::Expression>,
    ) -> Option<(GlobalSymbolId, GlobalSymbolId)> {
        // match initialization: let i = 0
        let init = self.ctx.tree.get(init_id);
        let dir::Expression::Let {
            mutability: Mutability::Mutable,
            declarators,
            ..
        } = init
        else {
            return None;
        };
        if declarators.len() != 1 {
            return None;
        }

        let declarator = self.ctx.tree.get(declarators[0]);
        let value_id = declarator.value?;

        // check the initializer is 0
        let value = self.ctx.tree.get(value_id);
        let is_zero = matches!(
            value,
            dir::Expression::ScalarLiteral {
                value: dir::ScalarLiteral::Integer(0)
            }
        );
        if !is_zero {
            return None;
        }

        // get the index symbol
        let pattern = self.ctx.tree.get(declarator.pattern);
        let index_local = pattern.symbol()?;
        let index_symbol = GlobalSymbolId::new(self.ctx.module_id(), index_local);

        // match condition: i < arr.length
        let cond = self.ctx.tree.get(cond_id);
        let dir::Expression::Binary {
            left,
            operator: dir::BinaryOperator::LessThan,
            right,
        } = cond
        else {
            return None;
        };

        // verify left is the index variable
        let left_expr = self.ctx.tree.get(*left);
        if left_expr.target_symbol() != Some(index_symbol) {
            return None;
        }

        // match right as arr.length
        let right_expr = self.ctx.tree.get(*right);
        let dir::Expression::Member { left, name, .. } = right_expr else {
            return None;
        };
        if *name != self.length_name {
            return None;
        }

        // get the array symbol
        let array_expr = self.ctx.tree.get(*left);
        let array_symbol = array_expr.target_symbol()?;

        // verify it's an array type
        let type_id = self.ctx.expression_type_id(*left)?;
        if !is_array_type(self.ctx.types, type_id, Some(self.array_symbol)) {
            return None;
        }

        Some((index_symbol, array_symbol))
    }

    /// Check if the increment is a simple i++ or i += 1.
    fn is_simple_increment(
        &self,
        incr_id: LocalNodeId<dir::Expression>,
        index_symbol: GlobalSymbolId,
    ) -> bool {
        let incr = self.ctx.tree.get(incr_id);

        // match i++
        if let dir::Expression::Unary {
            operator: dir::UnaryOperator::PostIncrement,
            right,
        } = incr
        {
            let right_expr = self.ctx.tree.get(*right);
            return right_expr.target_symbol() == Some(index_symbol);
        }

        // match i = i + 1 (desugared from i += 1)
        if let dir::Expression::Assign { left, right } = incr {
            let left_expr = self.ctx.tree.get(*left);
            if left_expr.target_symbol() != Some(index_symbol) {
                return false;
            }

            let right_expr = self.ctx.tree.get(*right);
            if let dir::Expression::Binary {
                left: bin_left,
                operator: dir::BinaryOperator::Add,
                right: bin_right,
            } = right_expr
            {
                // check that bin_left is i
                let bin_left_expr = self.ctx.tree.get(*bin_left);
                if bin_left_expr.target_symbol() != Some(index_symbol) {
                    return false;
                }

                // check that bin_right is 1
                let bin_right_expr = self.ctx.tree.get(*bin_right);
                return matches!(
                    bin_right_expr,
                    dir::Expression::ScalarLiteral {
                        value: dir::ScalarLiteral::Integer(1)
                    }
                );
            }
        }

        false
    }

    /// Check if the index variable is only used for array indexing in the body.
    fn index_only_used_for_indexing(
        &self,
        body_id: LocalNodeId<dir::Block>,
        index_symbol: GlobalSymbolId,
        array_symbol: GlobalSymbolId,
    ) -> bool {
        // collect all uses of the index variable
        let mut collector = IndexUseCollector {
            index_symbol,
            array_symbol,
            all_uses_are_indexing: true,
            found_any_use: false,
            options: NodeVisitorOptions::default(),
        };

        let body = self.ctx.tree.get(body_id);
        collector.visit_block(self.ctx.tree, body_id, body);

        // require at least one use and all uses to be indexing
        collector.found_any_use && collector.all_uses_are_indexing
    }
}

/// Collector to check how the index variable is used.
struct IndexUseCollector {
    index_symbol: GlobalSymbolId,
    array_symbol: GlobalSymbolId,
    all_uses_are_indexing: bool,
    found_any_use: bool,
    options: NodeVisitorOptions,
}

impl NodeVisitor for IndexUseCollector {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for index expressions arr[i]
        if let dir::Expression::Index { left, right } = expression {
            let left_expr = tree.get(*left);

            // check if right side exists (it's optional in Index)
            if let Some(right_id) = right {
                let index_expr = tree.get(*right_id);

                // if this is arr[i], mark as valid use
                if left_expr.target_symbol() == Some(self.array_symbol)
                    && index_expr.target_symbol() == Some(self.index_symbol)
                {
                    self.found_any_use = true;
                    // don't descend into children, we've handled this
                    return;
                }
            }
        }

        // check for any other reference to the index variable
        if let Some(target) = expression.target_symbol()
            && target == self.index_symbol
        {
            // this is a use outside of arr[i] pattern
            self.all_uses_are_indexing = false;
        }

        // walk children
        walk_expression(self, tree, id, expression);
    }
}

impl NodeVisitor for PreferForOfVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for loops
        if let dir::Expression::For {
            initialization,
            condition,
            increment,
            body,
            ..
        } = expression
        {
            self.check_for(id, *initialization, *condition, *increment, *body);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag simple index-based for loop.
    #[test]
    fn test_flags_simple_index_loop() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
for (let i = 0; i < items.length; i += 1) {
    console.log(items[i]);
}
"#,
        );
        test.result(result).assert_lint("prefer-for-of");
    }

    /// Flag with i++ increment.
    #[test]
    fn test_flags_with_postincrement() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = ["a", "b", "c"];
for (let i = 0; i < items.length; i++) {
    console.log(items[i]);
}
"#,
        );
        test.result(result).assert_lint("prefer-for-of");
    }

    /// Allow when index is used for more than indexing.
    #[test]
    fn test_allows_index_used_otherwise() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
for (let i = 0; i < items.length; i += 1) {
    console.log(i, items[i]);
}
"#,
        );
        test.result(result).assert_no_lint("prefer-for-of");
    }

    /// Allow when index is used in arithmetic.
    #[test]
    fn test_allows_index_arithmetic() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
for (let i = 0; i < items.length; i += 1) {
    console.log(items[i], items[i + 1]);
}
"#,
        );
        test.result(result).assert_no_lint("prefer-for-of");
    }

    /// Allow for-of loops.
    #[test]
    fn test_allows_for_of() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
for (const item of items) {
    console.log(item);
}
"#,
        );
        test.result(result).assert_no_lint("prefer-for-of");
    }

    /// Allow when not starting from 0.
    #[test]
    fn test_allows_non_zero_start() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
for (let i = 1; i < items.length; i += 1) {
    console.log(items[i]);
}
"#,
        );
        test.result(result).assert_no_lint("prefer-for-of");
    }
}
