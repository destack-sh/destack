use destack_core::StringId;
use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{
    expression_enters_nested_declaration_scope, expression_method_call, is_array_type,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `unshift` in loops.
    ///
    /// Calling `unshift` on an array inside a loop causes O(n²) time complexity
    /// because each `unshift` shifts all existing elements.
    #[lint(
        id = "no-array-unshift-loop",
        code = "LP003",
        category = Performance,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoArrayUnshiftLoop,
    "Disallow unshift in loops (O(n²))"
}

impl LintRule for NoArrayUnshiftLoop {
    fn meta(&self) -> &'static LintMeta {
        NoArrayUnshiftLoop::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoArrayUnshiftLoopVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags unshift calls inside loops.
struct NoArrayUnshiftLoopVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the unshift method name.
    unshift_name: StringId,
    /// Whether the current traversal is inside a loop.
    is_in_loop: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoArrayUnshiftLoopVisitor<'a, 'b> {
    /// Build a visitor for no-array-unshift-loop checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.language_item(LanguageItem::Array);
        let unshift_name = ctx.string_id("unshift");

        Self {
            ctx,
            meta,
            array_symbol,
            unshift_name,
            is_in_loop: false,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call expression for unshift usage inside a loop.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // only check inside loops
        if !self.is_in_loop {
            return;
        }

        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.dir.tree(), expression_id) else {
            return;
        };

        // check if this is unshift
        if method_call.method_name != self.unshift_name {
            return;
        }

        // check if the receiver is an array
        let Some(type_id) = self.ctx.expression_type_id(method_call.receiver_id) else {
            return;
        };
        if !is_array_type(self.ctx.types, type_id, Some(self.array_symbol)) {
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
            LintReport::new(
                NO_ARRAY_UNSHIFT_LOOP.id,
                NO_ARRAY_UNSHIFT_LOOP.code,
                NO_ARRAY_UNSHIFT_LOOP.category,
                severity,
                "unshift in a loop causes O(n²) performance",
                span,
            )
            .label("each unshift shifts all elements"),
        );
    }

    /// Visit a loop expression with loop context.
    fn visit_loop(
        &mut self,
        tree: &dir::Tree,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit loop condition inside loop context
        if let Some(condition_id) = condition {
            self.is_in_loop = true;
            let condition_expression = tree.get(condition_id);
            self.visit_expression(tree, condition_id, condition_expression);
        }

        // visit loop body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }

    /// Visit a for each loop expression with loop context.
    fn visit_for_each(
        &mut self,
        tree: &dir::Tree,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit binding inside loop context
        self.is_in_loop = true;
        match binding {
            dir::ForEachBinding::Pattern { pattern, .. } => {
                let pattern_node = tree.get(*pattern);
                self.visit_pattern(tree, *pattern, pattern_node);
            }
            dir::ForEachBinding::Using {
                asynchrony: _,
                pattern,
            } => {
                let pattern_node = tree.get(*pattern);
                self.visit_pattern(tree, *pattern, pattern_node);
            }
        }

        // visit iterator outside loop context
        self.is_in_loop = was_in_loop;
        let iterator_expression = tree.get(iterator);
        self.visit_expression(tree, iterator, iterator_expression);

        // visit body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }

    /// Visit a for loop expression with loop context.
    fn visit_for(
        &mut self,
        tree: &dir::Tree,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit initialization outside loop context
        if let Some(initialization_id) = initialization {
            self.is_in_loop = was_in_loop;
            let initialization_expression = tree.get(initialization_id);
            self.visit_expression(tree, initialization_id, initialization_expression);
        }

        // visit condition inside loop context
        if let Some(condition_id) = condition {
            self.is_in_loop = true;
            let condition_expression = tree.get(condition_id);
            self.visit_expression(tree, condition_id, condition_expression);
        }

        // visit increment inside loop context
        if let Some(increment_id) = increment {
            self.is_in_loop = true;
            let increment_expression = tree.get(increment_id);
            self.visit_expression(tree, increment_id, increment_expression);
        }

        // visit body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }
}

impl NodeVisitor for NoArrayUnshiftLoopVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // avoid leaking loop context into nested declarations
        if self.is_in_loop && expression_enters_nested_declaration_scope(tree, expression) {
            return;
        }

        // check call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_call(id);
        }

        // handle loop expressions with custom traversal
        match expression {
            dir::Expression::Loop { body } => {
                self.visit_loop(tree, None, *body);
                return;
            }
            dir::Expression::ForEach {
                binding,
                iterator,
                body,
                ..
            } => {
                self.visit_for_each(tree, binding, *iterator, *body);
                return;
            }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
                ..
            } => {
                self.visit_for(tree, *initialization, *condition, *increment, *body);
                return;
            }
            _ => {}
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag unshift inside for-of loop.
    #[test]
    fn test_flags_unshift_in_for_of() {
        let test = TestProgram::for_rule_without_prelude(NoArrayUnshiftLoop);
        let result = test.lint_dir(
            "no_array_unshift_loop/test_flags_unshift_in_for_of.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
for (const item of items) {
    result.unshift(item);
}
"#,
        );
        test.result(result).assert_lint("no-array-unshift-loop");
    }

    /// Flag unshift inside for loop.
    #[test]
    fn test_flags_unshift_in_for() {
        let test = TestProgram::for_rule_without_prelude(NoArrayUnshiftLoop);
        let result = test.lint_dir(
            "no_array_unshift_loop/test_flags_unshift_in_for.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
for (let i = 0; i < items.length; i += 1) {
    result.unshift(items[i]);
}
"#,
        );
        test.result(result).assert_lint("no-array-unshift-loop");
    }

    /// Flag unshift inside while loop.
    #[test]
    fn test_flags_unshift_in_while() {
        let test = TestProgram::for_rule_without_prelude(NoArrayUnshiftLoop);
        let result = test.lint_dir(
            "no_array_unshift_loop/test_flags_unshift_in_while.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
let i = 0;
while (i < items.length) {
    result.unshift(items[i]);
    i += 1;
}
"#,
        );
        test.result(result).assert_lint("no-array-unshift-loop");
    }

    /// Allow unshift outside loops.
    #[test]
    fn test_allows_unshift_outside_loop() {
        let test = TestProgram::for_rule_without_prelude(NoArrayUnshiftLoop);
        let result = test.lint_dir(
            "no_array_unshift_loop/test_allows_unshift_outside_loop.ds",
            r#"
let items: number[] = [1, 2, 3];
items.unshift(0);
"#,
        );
        test.result(result).assert_no_lint("no-array-unshift-loop");
    }

    /// Allow push inside loops.
    #[test]
    fn test_allows_push_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoArrayUnshiftLoop);
        let result = test.lint_dir(
            "no_array_unshift_loop/test_allows_push_in_loop.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
for (const item of items) {
    result.push(item);
}
"#,
        );
        test.result(result).assert_no_lint("no-array-unshift-loop");
    }

    /// Allow nested declaration calls inside loops.
    #[test]
    fn test_allows_nested_declaration_unshift_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoArrayUnshiftLoop);
        let result = test.lint_dir(
            "no_array_unshift_loop/test_allows_nested_declaration_unshift_in_loop.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
for (const item of items) {
    const push_front = () => {
        result.unshift(item);
    };
    push_front();
}
"#,
        );
        test.result(result).assert_no_lint("no-array-unshift-loop");
    }
}
