use destack_core::StringId;
use destack_dir::{
    self as dir, GlobalSymbolId, LocalNodeId, Mutability, NodeVisitor, NodeVisitorOptions,
    WellKnownSymbol, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    assign_pattern_contains_expression, assign_pattern_target_symbol,
    fresh_name_in_expression_scope, is_array_type, strip_dot_member_suffix,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer `for-of` over index-based `for` loops.
    ///
    /// When iterating over an array using only `arr[i]` access patterns,
    /// a `for-of` loop is cleaner and less error-prone.
    #[lint(
        id = "prefer-for-of",
        code = "LP014",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferForOf,
    "Prefer for-of over index-based for loops"
}

/// Extracted loop pattern data for `for (let i = 0; i < arr.length; i += 1)`.
#[derive(Clone, Copy)]
struct ForLoopPattern {
    /// The loop index symbol.
    index_symbol: GlobalSymbolId,
    /// The iterated array symbol.
    array_symbol: GlobalSymbolId,
    /// The array expression used in the condition.
    array_expression_id: LocalNodeId<dir::Expression>,
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
        let length_name = ctx.string_id("length");

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

        // extract the index and array loop pattern
        let Some(pattern) = self.extract_for_loop_pattern(init_id, cond_id) else {
            return;
        };

        // verify the increment is i++ or i += 1
        if !self.is_simple_increment(incr_id, pattern.index_symbol) {
            return;
        }

        // collect array index rewrites and reject unsupported index uses
        let Some(index_accesses) = self.collect_index_accesses_for_rewrite(
            body,
            pattern.index_symbol,
            pattern.array_symbol,
        ) else {
            return;
        };

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            PREFER_FOR_OF.id,
            PREFER_FOR_OF.code,
            PREFER_FOR_OF.category,
            severity,
            "prefer for-of over index-based for loop",
            span,
        )
        .label("use for-of to iterate directly over elements");
        if self.ctx.include_fixes
            && let Some(fix) = self.prefer_for_of_fix(expression_id, body, pattern, &index_accesses)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Extract the index variable and array from a for loop pattern.
    fn extract_for_loop_pattern(
        &self,
        init_id: LocalNodeId<dir::Expression>,
        cond_id: LocalNodeId<dir::Expression>,
    ) -> Option<ForLoopPattern> {
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
        if self.ctx.expression_target_symbol(*left) != Some(index_symbol) {
            return None;
        }

        // match right as arr.length
        let right_expr = self.ctx.tree.get(*right);
        let dir::Expression::Member { left, name, .. } = right_expr else {
            return None;
        };
        if *name != Some(self.length_name) {
            return None;
        }

        // get the array symbol
        let array_symbol = self.ctx.expression_target_symbol(*left)?;

        // verify it's an array type
        let type_id = self.ctx.expression_type_id(*left)?;
        if !is_array_type(self.ctx.types, type_id, Some(self.array_symbol)) {
            return None;
        }

        Some(ForLoopPattern {
            index_symbol,
            array_symbol,
            array_expression_id: *left,
        })
    }

    /// Check if the increment is a simple ++i, i++, or i += 1.
    fn is_simple_increment(
        &self,
        incr_id: LocalNodeId<dir::Expression>,
        index_symbol: GlobalSymbolId,
    ) -> bool {
        let incr = self.ctx.tree.get(incr_id);

        // match ++i and i++
        if let dir::Expression::Unary { operator, right } = incr
            && matches!(
                operator,
                dir::UnaryOperator::PreIncrement | dir::UnaryOperator::PostIncrement
            )
        {
            return self.ctx.expression_target_symbol(*right) == Some(index_symbol);
        }

        // match i = i + 1 (desugared from i += 1)
        if let dir::Expression::Assign { left, right } = incr {
            if assign_pattern_target_symbol(self.ctx, *left) != Some(index_symbol) {
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
                if self.ctx.expression_target_symbol(*bin_left) != Some(index_symbol) {
                    if self.ctx.expression_target_symbol(*bin_right) != Some(index_symbol) {
                        return false;
                    }

                    // check that bin_left is 1
                    let bin_left_expr = self.ctx.tree.get(*bin_left);
                    return matches!(
                        bin_left_expr,
                        dir::Expression::ScalarLiteral {
                            value: dir::ScalarLiteral::Integer(1)
                        }
                    );
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

    /// Collect all `arr[i]` index accesses used for a safe for-of rewrite.
    fn collect_index_accesses_for_rewrite(
        &self,
        body_id: LocalNodeId<dir::Block>,
        index_symbol: GlobalSymbolId,
        array_symbol: GlobalSymbolId,
    ) -> Option<Vec<LocalNodeId<dir::Expression>>> {
        // collect all uses of the index variable
        let mut collector = IndexUseCollector {
            module_id: self.ctx.module_id(),
            types: self.ctx.types,
            index_symbol,
            array_symbol,
            all_uses_are_indexing: true,
            found_any_use: false,
            index_accesses: Vec::new(),
            options: NodeVisitorOptions::default(),
        };

        let body = self.ctx.tree.get(body_id);
        collector.visit_block(self.ctx.tree, body_id, body);

        // require at least one use and all uses to be indexing
        if !collector.found_any_use || !collector.all_uses_are_indexing {
            return None;
        }

        Some(collector.index_accesses)
    }

    /// Build an unsafe `for` to `for-of` rewrite for index-only loops.
    fn prefer_for_of_fix(
        &self,
        expression_id: LocalNodeId<dir::Expression>,
        body_id: LocalNodeId<dir::Block>,
        pattern: ForLoopPattern,
        index_accesses: &[LocalNodeId<dir::Expression>],
    ) -> Option<LintFix> {
        // choose a fresh loop binding name
        let body_span = self.ctx.get_span(body_id);
        let body_text = self.ctx.get_span_text(body_span).to_string();
        let binding_name =
            fresh_name_in_expression_scope(self.ctx, expression_id, "item", "Element")?;

        // replace each `arr[i]` with the loop binding inside body text
        let mut rewritten_body = body_text;
        let mut spans = index_accesses
            .iter()
            .map(|expression_id| self.ctx.get_span(*expression_id))
            .collect::<Vec<_>>();
        spans.sort_by(|left, right| right.start.cmp(&left.start));
        for span in spans {
            if span.start < body_span.start || span.end > body_span.end {
                return None;
            }

            let start = (span.start - body_span.start) as usize;
            let end = (span.end - body_span.start) as usize;
            rewritten_body.replace_range(start..end, &binding_name);
        }

        // build a for-of loop replacement
        let mut array_text = self
            .ctx
            .get_span_text(self.ctx.get_span(pattern.array_expression_id))
            .to_string();
        if let Some(stripped) = strip_dot_member_suffix(&array_text, "length") {
            array_text = stripped.to_string();
        }
        let replacement = format!("for (const {binding_name} of {array_text}) {rewritten_body}",);
        let edits = self
            .ctx
            .edit_builder()
            .replace(self.ctx.get_span(expression_id), replacement)
            .into_edits();
        Some(LintFix::r#unsafe("Rewrite index loop as for-of loop").with_edits(edits))
    }
}

/// Collector to check how the index variable is used.
struct IndexUseCollector<'a> {
    module_id: destack_source::ModuleId,
    types: &'a dir::TypeTable,
    index_symbol: GlobalSymbolId,
    array_symbol: GlobalSymbolId,
    all_uses_are_indexing: bool,
    found_any_use: bool,
    index_accesses: Vec<LocalNodeId<dir::Expression>>,
    options: NodeVisitorOptions,
}

impl IndexUseCollector<'_> {
    fn expression_target_symbol(
        &self,
        expression_id: LocalNodeId<dir::Expression>,
    ) -> Option<GlobalSymbolId> {
        self.types
            .symbol_resolution(expression_id.into_global_any(self.module_id))
            .and_then(|resolution| match resolution {
                dir::SymbolResolution::Target(symbol) => Some(*symbol),
                dir::SymbolResolution::Candidates(_) => None,
            })
    }
}

impl NodeVisitor for IndexUseCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for index expressions arr[i]
        if let dir::Expression::Index { left, right } = expression {
            // check if right side exists (it's optional in Index)
            if let Some(right_id) = right {
                // if this is arr[i], mark as valid use
                if self.expression_target_symbol(*left) == Some(self.array_symbol)
                    && self.expression_target_symbol(*right_id) == Some(self.index_symbol)
                {
                    if index_expression_is_write_target(tree, id) {
                        self.all_uses_are_indexing = false;
                        self.found_any_use = true;
                        return;
                    }

                    self.found_any_use = true;
                    self.index_accesses.push(id);
                    // don't descend into children, we've handled this
                    return;
                }
            }
        }

        // check for any other reference to the index variable
        if let Some(target) = self.expression_target_symbol(id)
            && target == self.index_symbol
        {
            // this is a use outside of arr[i] pattern
            self.all_uses_are_indexing = false;
        }

        // walk children
        walk_expression(self, tree, id, expression);
    }
}

/// Return true when one `arr[i]` expression is used as a write target.
fn index_expression_is_write_target(
    tree: &dir::Tree,
    expression_id: LocalNodeId<dir::Expression>,
) -> bool {
    let Some(parent_id) = tree.get_parent_id(expression_id.id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != dir::NodeType::Expression {
        return false;
    }
    let parent_expression_id = LocalNodeId::<dir::Expression>::new(parent_id);
    let parent_expression = tree.get(parent_expression_id);

    if let dir::Expression::Assign { left, .. } = parent_expression {
        return assign_pattern_contains_expression(tree, *left, expression_id);
    }

    if let dir::Expression::AssignBinary { left, .. } = parent_expression {
        return *left == expression_id;
    }

    if let dir::Expression::Unary { operator, right } = parent_expression
        && matches!(
            operator,
            dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PreDecrement
                | dir::UnaryOperator::PostDecrement
        )
    {
        return *right == expression_id;
    }

    false
}

impl NodeVisitor for PreferForOfVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
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
            "prefer_for_of/test_flags_simple_index_loop.ds",
            r#"
let items = [1, 2, 3];
for (let i = 0; i < items.length; i += 1) {
    console.log(items[i]);
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-for-of")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
for (const itemElement of items) {
    console.log(itemElement);
}
"#,
            );
    }

    /// Keep the fix stable when `item` is already used in scope.
    #[test]
    fn test_fix_uses_fresh_binding_name_on_collision() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "prefer_for_of/test_fix_uses_fresh_binding_name_on_collision.ds",
            r#"
let itemElement = 0;
let items = [1, 2, 3];
for (let i = 0; i < items.length; i++) {
    console.log(items[i]);
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-for-of")
            .assert_unsafe_fixed(
                r#"
let itemElement = 0;
let items = [1, 2, 3];
for (const itemElement2 of items) {
    console.log(itemElement2);
}
"#,
            );
    }

    /// Flag with i++ increment.
    #[test]
    fn test_flags_with_postincrement() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "prefer_for_of/test_flags_with_postincrement.ds",
            r#"
let items = ["a", "b", "c"];
for (let i = 0; i < items.length; i++) {
    console.log(items[i]);
}
"#,
        );
        test.result(result).assert_lint("prefer-for-of");
    }

    /// Flag with ++i increment.
    #[test]
    fn test_flags_with_preincrement() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "prefer_for_of/test_flags_with_preincrement.ds",
            r#"
let items = ["a", "b", "c"];
for (let i = 0; i < items.length; ++i) {
    console.log(items[i]);
}
"#,
        );
        test.result(result).assert_lint("prefer-for-of");
    }

    /// Flag with increment written as i = 1 + i.
    #[test]
    fn test_flags_with_reversed_increment_assignment() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "prefer_for_of/test_flags_with_reversed_increment_assignment.ds",
            r#"
let items = ["a", "b", "c"];
for (let i = 0; i < items.length; i = 1 + i) {
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
            "prefer_for_of/test_allows_index_used_otherwise.ds",
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
            "prefer_for_of/test_allows_index_arithmetic.ds",
            r#"
let items = [1, 2, 3];
for (let i = 0; i < items.length; i += 1) {
    console.log(items[i], items[i + 1]);
}
"#,
        );
        test.result(result).assert_no_lint("prefer-for-of");
    }

    /// Allow when array element access is used as an assignment target.
    #[test]
    fn test_allows_index_assignment_target() {
        let test = TestProgram::for_rule_with_prelude(PreferForOf);
        let result = test.lint_dir(
            "prefer_for_of/test_allows_index_assignment_target.ds",
            r#"
let items = [1, 2, 3];
for (let i = 0; i < items.length; i += 1) {
    items[i] = items[i] + 1;
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
            "prefer_for_of/test_allows_for_of.ds",
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
            "prefer_for_of/test_allows_non_zero_start.ds",
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
