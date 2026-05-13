use destack_core::StringId;
use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{
    expression_enters_nested_declaration_scope, expression_method_call, is_array_type,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `includes` or `indexOf` style lookups inside loops over another array.
    ///
    /// Using `includes` or `indexOf` inside a loop results in O(n²) time
    /// complexity. Consider using a Set for O(1) lookups instead.
    #[lint(
        id = "no-nested-array-includes",
        code = "LP007",
        category = Performance,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNestedArrayIncludes,
    "Disallow includes/indexOf style lookups inside loops (O(n²))"
}

impl LintRule for NoNestedArrayIncludes {
    fn meta(&self) -> &'static LintMeta {
        NoNestedArrayIncludes::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoNestedArrayIncludesVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags includes/indexOf inside loops.
struct NoNestedArrayIncludesVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the includes method name.
    includes_name: StringId,
    /// The string id for the indexOf method name.
    index_of_name: StringId,
    /// The string id for the lastIndexOf method name.
    last_index_of_name: StringId,
    /// Whether the current traversal is inside a loop.
    is_in_loop: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoNestedArrayIncludesVisitor<'a, 'b> {
    /// Build a visitor for no-nested-array-includes checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.language_item(LanguageItem::Array);
        let includes_name = ctx.string_id("includes");
        let index_of_name = ctx.string_id("indexOf");
        let last_index_of_name = ctx.string_id("lastIndexOf");

        Self {
            ctx,
            meta,
            array_symbol,
            includes_name,
            index_of_name,
            last_index_of_name,
            is_in_loop: false,
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

    /// Check a call expression for includes/indexOf inside a loop.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // only check inside loops
        if !self.is_in_loop {
            return;
        }

        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };

        // check if this is includes/indexOf/lastIndexOf
        let is_includes = method_call.method_name == self.includes_name;
        let is_index_of = method_call.method_name == self.index_of_name;
        let is_last_index_of = method_call.method_name == self.last_index_of_name;
        if !is_includes && !is_index_of && !is_last_index_of {
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

        // build diagnostic message
        let method_name = if is_includes {
            "includes"
        } else if is_index_of {
            "indexOf"
        } else {
            "lastIndexOf"
        };

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_NESTED_ARRAY_INCLUDES.id,
                NO_NESTED_ARRAY_INCLUDES.code,
                NO_NESTED_ARRAY_INCLUDES.category,
                severity,
                format!("{method_name}() in a loop causes O(n²) performance"),
                span,
            )
            .label("consider using a Set for O(1) lookups"),
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

impl NodeVisitor for NoNestedArrayIncludesVisitor<'_, '_> {
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
            dir::Expression::Loop {
                condition, body, ..
            } => {
                self.visit_loop(tree, *condition, *body);
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

    /// Flag includes inside for-of loop.
    #[test]
    fn test_flags_includes_in_for_of() {
        let test = TestProgram::for_rule_without_prelude(NoNestedArrayIncludes);
        let result = test.lint_dir(
            "no_nested_array_includes/test_flags_includes_in_for_of.ds",
            r#"
let items = [1, 2, 3];
let lookup = [2, 4, 6];
for (const item of items) {
    if (lookup.includes(item)) {
        console.log(item);
    }
}
"#,
        );
        test.result(result).assert_lint("no-nested-array-includes");
    }

    /// Flag indexOf inside for loop.
    #[test]
    fn test_flags_index_of_in_for() {
        let test = TestProgram::for_rule_without_prelude(NoNestedArrayIncludes);
        let result = test.lint_dir(
            "no_nested_array_includes/test_flags_index_of_in_for.ds",
            r#"
let items = [1, 2, 3];
let lookup = [2, 4, 6];
for (let i = 0; i < items.length; i += 1) {
    if (lookup.indexOf(items[i]) !== -1) {
        console.log(items[i]);
    }
}
"#,
        );
        test.result(result).assert_lint("no-nested-array-includes");
    }

    /// Flag lastIndexOf inside for loop.
    #[test]
    fn test_flags_last_index_of_in_for() {
        let test = TestProgram::for_rule_without_prelude(NoNestedArrayIncludes);
        let result = test.lint_dir(
            "no_nested_array_includes/test_flags_last_index_of_in_for.ds",
            r#"
let items = [1, 2, 3];
let lookup = [2, 4, 6];
for (let i = 0; i < items.length; i += 1) {
    if (lookup.lastIndexOf(items[i]) !== -1) {
        console.log(items[i]);
    }
}
"#,
        );
        test.result(result).assert_lint("no-nested-array-includes");
    }

    /// Flag includes inside while loop.
    #[test]
    fn test_flags_includes_in_while() {
        let test = TestProgram::for_rule_without_prelude(NoNestedArrayIncludes);
        let result = test.lint_dir(
            "no_nested_array_includes/test_flags_includes_in_while.ds",
            r#"
let items = [1, 2, 3];
let lookup = [2, 4, 6];
let i = 0;
while (i < items.length) {
    if (lookup.includes(items[i])) {
        console.log(items[i]);
    }
    i += 1;
}
"#,
        );
        test.result(result).assert_lint("no-nested-array-includes");
    }

    /// Allow includes outside loops.
    #[test]
    fn test_allows_includes_outside_loop() {
        let test = TestProgram::for_rule_without_prelude(NoNestedArrayIncludes);
        let result = test.lint_dir(
            "no_nested_array_includes/test_allows_includes_outside_loop.ds",
            r#"
let items = [1, 2, 3];
let hasTwo = items.includes(2);
"#,
        );
        test.result(result)
            .assert_no_lint("no-nested-array-includes");
    }

    /// Allow Set.has inside loops.
    #[test]
    fn test_allows_set_has_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoNestedArrayIncludes);
        let result = test.lint_dir(
            "no_nested_array_includes/test_allows_set_has_in_loop.ds",
            r#"
let items = [1, 2, 3];
let lookup = new Set([2, 4, 6]);
for (const item of items) {
    if (lookup.has(item)) {
        console.log(item);
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-nested-array-includes");
    }

    /// Allow includes inside functions declared within loops.
    #[test]
    fn test_allows_nested_function_inside_loop() {
        let test = TestProgram::for_rule_without_prelude(NoNestedArrayIncludes);
        let result = test.lint_dir(
            "no_nested_array_includes/test_allows_nested_function_inside_loop.ds",
            r#"
for (const item of items) {
    function check(other) {
        return values.includes(other);
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-nested-array-includes");
    }
}
