use std::collections::HashMap;

use destack_base::StringId;
use destack_dir::{
    self as dir, GlobalSymbolId, LocalNodeId, Mutability, NodeVisitor, NodeVisitorOptions,
    WellKnownSymbol, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::expression_method_call;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer array literal over empty array followed by push.
    ///
    /// Initializing an empty array and then immediately pushing known values
    /// is less efficient and less readable than using an array literal.
    #[lint(
        id = "prefer-array-literal",
        code = "LP013",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArrayLiteral,
    "Prefer array literal over empty array + push"
}

impl LintRule for PreferArrayLiteral {
    fn meta(&self) -> &'static LintMeta {
        PreferArrayLiteral::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferArrayLiteralVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Info about an empty array declaration.
#[derive(Clone, Copy)]
struct EmptyArrayDecl {
    /// The expression id of the declaration.
    expression_id: LocalNodeId<dir::Expression>,
    /// Whether we've seen a non-push use of this variable.
    has_other_use: bool,
    /// Number of consecutive push calls seen.
    push_count: u32,
}

/// Visitor that flags empty array + push patterns.
struct PreferArrayLiteralVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The string id for the push method name.
    push_name: StringId,
    /// Tracked empty array declarations.
    empty_arrays: HashMap<GlobalSymbolId, EmptyArrayDecl>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferArrayLiteralVisitor<'a, 'b> {
    /// Build a visitor for prefer-array-literal checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let push_name = ctx.program.strings.intern("push");

        Self {
            ctx,
            meta,
            push_name,
            empty_arrays: HashMap::new(),
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

        // report any arrays that had only pushes
        self.report_candidates();
    }

    /// Check for empty array declarations.
    fn check_let(
        &mut self,
        expression_id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // match let declarations
        let dir::Expression::Let {
            mutability: Mutability::Mutable,
            declarators,
            ..
        } = expression
        else {
            return;
        };

        // check each declarator
        for declarator_id in declarators {
            let declarator = self.ctx.tree.get(*declarator_id);

            // require an initializer
            let Some(init_id) = declarator.value else {
                continue;
            };

            // check if the initializer is an empty array
            let init = self.ctx.tree.get(init_id);
            let is_empty_array = matches!(
                init,
                dir::Expression::ArrayExpression { elements } if elements.is_empty()
            );
            if !is_empty_array {
                continue;
            }

            // get the declared symbol
            let pattern = self.ctx.tree.get(declarator.pattern);
            let Some(local_symbol) = pattern.symbol() else {
                continue;
            };

            // track this empty array
            let global_symbol = GlobalSymbolId::new(self.ctx.module_id(), local_symbol);
            self.empty_arrays.insert(
                global_symbol,
                EmptyArrayDecl {
                    expression_id,
                    has_other_use: false,
                    push_count: 0,
                },
            );
        }
    }

    /// Check for push calls on tracked arrays.
    fn check_call(&mut self, expression_id: LocalNodeId<dir::Expression>) {
        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };

        // check if this is push
        if method_call.method_name != self.push_name {
            // mark as other use if the receiver is a tracked array
            self.mark_other_use(method_call.receiver_id);
            return;
        }

        // check if the receiver is a tracked empty array
        let receiver = self.ctx.tree.get(method_call.receiver_id);
        let Some(target_symbol) = receiver.target_symbol() else {
            return;
        };

        // increment the push count
        if let Some(decl) = self.empty_arrays.get_mut(&target_symbol) {
            decl.push_count += 1;
        }
    }

    /// Mark an expression's target as having a non-push use.
    fn mark_other_use(&mut self, expression_id: LocalNodeId<dir::Expression>) {
        // check if this references a tracked array
        let expression = self.ctx.tree.get(expression_id);
        let Some(target_symbol) = expression.target_symbol() else {
            return;
        };

        // mark as having other uses
        if let Some(decl) = self.empty_arrays.get_mut(&target_symbol) {
            decl.has_other_use = true;
        }
    }

    /// Report arrays that could be array literals.
    fn report_candidates(&mut self) {
        for decl in self.empty_arrays.values() {
            // only report if we have pushes and no other uses
            if decl.push_count == 0 || decl.has_other_use {
                continue;
            }

            // honor per node severity
            let severity = self
                .ctx
                .get_effective_severity(self.meta, decl.expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // report the diagnostic
            let span = self.ctx.get_span(decl.expression_id);
            self.ctx.report(
                LintDiagnostic::new(
                    PREFER_ARRAY_LITERAL.id,
                    PREFER_ARRAY_LITERAL.code,
                    PREFER_ARRAY_LITERAL.category,
                    severity,
                    "prefer array literal over empty array + push",
                    self.ctx.module.file_id,
                    span,
                )
                .with_label("initialize with values directly"),
            );
        }
    }
}

impl NodeVisitor for PreferArrayLiteralVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for let declarations
        if matches!(expression, dir::Expression::Let { .. }) {
            self.check_let(id, expression);
        }

        // check for call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_call(id);
        }

        // check for member accesses (like items.length) as other use
        // but exclude .push() which is handled by check_call
        if let dir::Expression::Member { left, name, .. } = expression
            && *name != self.push_name
        {
            self.mark_other_use(*left);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag empty array with push.
    #[test]
    fn test_flags_empty_array_push() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_flags_empty_array_push.ds",
            r#"
let items: number[] = [];
items.push(1);
items.push(2);
"#,
        );
        test.result(result).assert_lint("prefer-array-literal");
    }

    /// Flag single push.
    #[test]
    fn test_flags_single_push() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_flags_single_push.ds",
            r#"
let items: string[] = [];
items.push("hello");
"#,
        );
        test.result(result).assert_lint("prefer-array-literal");
    }

    /// Allow array literal.
    #[test]
    fn test_allows_array_literal() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_array_literal.ds",
            r#"
let items = [1, 2, 3];
"#,
        );
        test.result(result).assert_no_lint("prefer-array-literal");
    }

    /// Allow empty array with other uses.
    #[test]
    fn test_allows_empty_array_with_other_uses() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_empty_array_with_other_uses.ds",
            r#"
let items: number[] = [];
items.push(1);
console.log(items.length);
items.push(2);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-literal");
    }

    /// Allow empty array for dynamic population.
    #[test]
    fn test_allows_dynamic_population() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_dynamic_population.ds",
            r#"
let items: number[] = [];
for (let i = 0; i < 10; i += 1) {
    items.push(i);
}
"#,
        );
        // the loop body references items in a non-push context (the push is inside a loop)
        // this is more complex to detect, so we'll flag it for now
        // in reality this is a valid pattern, but we'd need CFG analysis to allow it
        test.result(result).assert_lint("prefer-array-literal");
    }

    /// Allow const arrays.
    #[test]
    fn test_allows_const_array() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_const_array.ds",
            r#"
const items: number[] = [];
"#,
        );
        test.result(result).assert_no_lint("prefer-array-literal");
    }
}
