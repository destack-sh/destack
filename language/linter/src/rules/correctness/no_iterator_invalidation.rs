use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_target_symbol;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

/// Method names that mutate collections.
const MUTATING_METHODS: &[&str] = &[
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "reverse",
    "sort",
    "fill",
    "copyWithin",
    "set",
    "delete",
    "clear",
    "add",
];

declare_lint! {
    /// Disallow modifying a collection while iterating over it.
    ///
    /// Modifying an array or collection during iteration can cause elements
    /// to be skipped or processed multiple times, leading to bugs.
    #[lint(
        id = "no-iterator-invalidation",
        code = "LC030",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoIteratorInvalidation,
    "Disallow collection mutation during iteration"
}

impl LintRule for NoIteratorInvalidation {
    fn meta(&self) -> &'static LintMeta {
        NoIteratorInvalidation::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = IteratorInvalidationVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags collection mutation during iteration.
struct IteratorInvalidationVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// Stack of iterated collection symbols.
    iterated_symbols: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> IteratorInvalidationVisitor<'a, 'b> {
    /// Build a new visitor.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            iterated_symbols: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module expression roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check if a call mutates an iterated collection.
    fn check_mutation_call(
        &mut self,
        call_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        // nothing to check if we're not iterating
        if self.iterated_symbols.is_empty() {
            return;
        }

        // check if this is a method call (Member expression)
        let expression = self.ctx.tree.get(left);
        let dir::Expression::Member {
            left: receiver,
            name,
            ..
        } = expression
        else {
            return;
        };

        // check if the method is a mutating method
        let method_name = {
            let s = self.ctx.program.strings.get(*name);
            s.as_ref().to_string()
        };
        if !MUTATING_METHODS.contains(&method_name.as_str()) {
            return;
        }

        // check if the receiver is one of our iterated collections
        let Some(receiver_symbol) = expression_target_symbol(self.ctx.tree, *receiver) else {
            return;
        };
        if !self.iterated_symbols.contains(&receiver_symbol) {
            return;
        }

        // check effective severity
        let severity = self.ctx.get_effective_severity(self.meta, call_id);
        if !severity.is_enabled() {
            return;
        }

        // report
        let span = self.ctx.get_span(call_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_ITERATOR_INVALIDATION.id,
                NO_ITERATOR_INVALIDATION.code,
                NO_ITERATOR_INVALIDATION.category,
                severity,
                format!("collection mutated during iteration via '{method_name}'"),
                self.ctx.module.file_id,
                span,
            )
            .with_label("modifying a collection while iterating can cause bugs"),
        );
    }
}

impl NodeVisitor for IteratorInvalidationVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for for-of loops
        if let dir::Expression::ForEach {
            kind: dir::ForEachKind::Of,
            iterator,
            body,
            ..
        } = expression
        {
            // get the iterated collection's symbol
            if let Some(symbol) = expression_target_symbol(tree, *iterator) {
                // push onto stack and walk body
                self.iterated_symbols.push(symbol);

                // walk the body
                let body_block = tree.get(*body);
                for expr_id in &body_block.expressions {
                    let expr = tree.get(*expr_id);
                    self.visit_expression(tree, *expr_id, expr);
                }

                // pop from stack
                self.iterated_symbols.pop();
                return; // don't walk children again
            }
        }

        // check call expressions for mutations
        if let dir::Expression::Call { left, .. } = expression {
            self.check_mutation_call(id, *left);
        }

        // walk children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag push during for-of iteration.
    #[test]
    fn test_flags_push_in_for_of() {
        let test = TestProgram::for_rule_with_prelude(NoIteratorInvalidation);
        let result = test.lint_dir(
            "no_iterator_invalidation/test_flags_push_in_for_of.ds",
            r#"
let arr = [1, 2, 3];
for (const x of arr) {
    arr.push(x * 2);
}
"#,
        );
        test.result(result).assert_lint("no-iterator-invalidation");
    }

    /// Flag pop during for-of iteration.
    #[test]
    fn test_flags_pop_in_for_of() {
        let test = TestProgram::for_rule_with_prelude(NoIteratorInvalidation);
        let result = test.lint_dir(
            "no_iterator_invalidation/test_flags_pop_in_for_of.ds",
            r#"
let arr = [1, 2, 3];
for (const x of arr) {
    arr.pop();
}
"#,
        );
        test.result(result).assert_lint("no-iterator-invalidation");
    }

    /// Flag splice during for-of iteration.
    #[test]
    fn test_flags_splice_in_for_of() {
        let test = TestProgram::for_rule_with_prelude(NoIteratorInvalidation);
        let result = test.lint_dir(
            "no_iterator_invalidation/test_flags_splice_in_for_of.ds",
            r#"
let arr = [1, 2, 3];
for (const x of arr) {
    arr.splice(0, 1);
}
"#,
        );
        test.result(result).assert_lint("no-iterator-invalidation");
    }

    /// Allow reading from the collection during iteration.
    #[test]
    fn test_allows_read_in_for_of() {
        let test = TestProgram::for_rule_with_prelude(NoIteratorInvalidation);
        let result = test.lint_dir(
            "no_iterator_invalidation/test_allows_read_in_for_of.ds",
            r#"
let arr = [1, 2, 3];
let sum = 0;
for (const x of arr) {
    sum = sum + arr.length;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-iterator-invalidation");
    }

    /// Allow mutating a different collection.
    #[test]
    fn test_allows_mutating_different_collection() {
        let test = TestProgram::for_rule_with_prelude(NoIteratorInvalidation);
        let result = test.lint_dir(
            "no_iterator_invalidation/test_allows_mutating_different_collection.ds",
            r#"
let source = [1, 2, 3];
let dest: number[] = [];
for (const x of source) {
    dest.push(x * 2);
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-iterator-invalidation");
    }

    /// Allow mutation outside of loop.
    #[test]
    fn test_allows_mutation_outside_loop() {
        let test = TestProgram::for_rule_with_prelude(NoIteratorInvalidation);
        let result = test.lint_dir(
            "no_iterator_invalidation/test_allows_mutation_outside_loop.ds",
            r#"
let arr = [1, 2, 3];
for (const x of arr) {
    let y = x;
}
arr.push(4);
"#,
        );
        test.result(result)
            .assert_no_lint("no-iterator-invalidation");
    }
}
