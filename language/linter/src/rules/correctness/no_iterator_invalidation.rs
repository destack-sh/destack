use std::collections::HashSet;

use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{collect_pattern_value_binding_symbols, expression_target_symbol};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

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
        code = "LC021",
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
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoIteratorInvalidation::meta()
    }

    /// Check module DIR nodes for iterator invalidation mutations.
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
    /// Stack of iterated collection alias scopes.
    iterated_symbol_scopes: Vec<HashSet<dir::GlobalSymbolId>>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> IteratorInvalidationVisitor<'a, 'b> {
    /// Build a new visitor.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            iterated_symbol_scopes: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module expression roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
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
        if self.iterated_symbol_scopes.is_empty() {
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
            let Some(name) = *name else {
                return;
            };
            let s = self.ctx.strings.get(name);
            s.as_ref().to_string()
        };
        if !MUTATING_METHODS.contains(&method_name.as_str()) {
            return;
        }

        // check if the receiver is one of our iterated collections
        let Some(receiver_symbol) = expression_target_symbol(self.ctx, *receiver) else {
            return;
        };
        if !self.is_iterated_symbol(receiver_symbol) {
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
            LintReport::new(
                NO_ITERATOR_INVALIDATION.id,
                NO_ITERATOR_INVALIDATION.code,
                NO_ITERATOR_INVALIDATION.category,
                severity,
                format!("collection mutated during iteration via '{method_name}'"),
                span,
            )
            .label("modifying a collection while iterating can cause bugs"),
        );
    }

    /// Return true when one symbol belongs to any active iterated scope.
    fn is_iterated_symbol(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        self.iterated_symbol_scopes
            .iter()
            .rev()
            .any(|symbols| symbols.contains(&symbol_id))
    }

    /// Register local aliases for active iterated symbols from one declarator.
    fn register_iterated_aliases_from_declarator(
        &mut self,
        declarator_id: dir::LocalNodeId<dir::Declarator>,
    ) {
        // require an active iterated scope and value initializer
        let Some(active_symbols) = self.iterated_symbol_scopes.last_mut() else {
            return;
        };

        // resolve declarator
        let declarator = self.ctx.tree.get(declarator_id);
        let Some(value_id) = declarator.value else {
            return;
        };

        // require initializer to reference an iterated symbol
        let Some(source_symbol) = expression_target_symbol(self.ctx, value_id) else {
            return;
        };
        if !active_symbols.contains(&source_symbol) {
            return;
        }

        // collect all bound symbols from the declarator pattern
        let mut local_symbols = HashSet::new();
        collect_pattern_value_binding_symbols(
            self.ctx.tree,
            self.ctx.symbols,
            declarator.pattern,
            &mut local_symbols,
        );

        // register aliases in the current iterated scope
        for local_symbol in local_symbols {
            active_symbols.insert(local_symbol.into_global(self.ctx.module.id));
        }
    }
}

impl NodeVisitor for IteratorInvalidationVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for for of loops
        if let dir::Expression::ForEach {
            operator: dir::ForEachOperator::Of,
            iterator,
            body,
            ..
        } = expression
        {
            // get the iterated collection's symbol
            if let Some(symbol) = self.ctx.expression_target_symbol(*iterator) {
                // push one scope for this loop and walk body
                self.iterated_symbol_scopes.push(HashSet::from([symbol]));

                // walk the body
                let body_block = tree.get(*body);
                for expr_id in body_block.iter_expressions() {
                    let expr = tree.get(expr_id);
                    self.visit_expression(tree, expr_id, expr);
                }

                // pop from stack
                self.iterated_symbol_scopes.pop();
                return; // don't walk children again
            }
        }

        // collect aliases declared inside active loop scopes
        if let dir::Expression::Let { declarators, .. }
        | dir::Expression::Using { declarators, .. } = expression
        {
            for declarator_id in declarators {
                self.register_iterated_aliases_from_declarator(*declarator_id);
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

    /// Flag push during for of iteration.
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

    /// Flag pop during for of iteration.
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

    /// Flag splice during for of iteration.
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

    /// Flag mutation through one simple loop alias.
    #[test]
    fn test_flags_alias_mutation_in_for_of() {
        let test = TestProgram::for_rule_with_prelude(NoIteratorInvalidation);
        let result = test.lint_dir(
            "no_iterator_invalidation/test_flags_alias_mutation_in_for_of.ds",
            r#"
let arr = [1, 2, 3];
for (const x of arr) {
    let alias = arr;
    alias.push(x);
}
"#,
        );
        test.result(result).assert_lint("no-iterator-invalidation");
    }

    /// Allow mutation through non-iterated aliases.
    #[test]
    fn test_allows_non_iterated_alias_mutation() {
        let test = TestProgram::for_rule_with_prelude(NoIteratorInvalidation);
        let result = test.lint_dir(
            "no_iterator_invalidation/test_allows_non_iterated_alias_mutation.ds",
            r#"
let source = [1, 2, 3];
let destination = [4, 5, 6];
for (const x of source) {
    let alias = destination;
    alias.push(x);
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-iterator-invalidation");
    }
}
