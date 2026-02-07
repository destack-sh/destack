use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{expression_method_call, is_array_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `for-of` over `Array.forEach()`.
    ///
    /// `for-of` loops are more performant than `forEach` because they avoid
    /// the overhead of function calls, and they support `break`, `continue`,
    /// and `return` statements.
    #[lint(
        id = "no-array-for-each",
        code = "LP003",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoArrayForEach,
    "Prefer for-of over Array.forEach()"
}

impl LintRule for NoArrayForEach {
    fn meta(&self) -> &'static LintMeta {
        NoArrayForEach::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoArrayForEachVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags Array.forEach() calls.
struct NoArrayForEachVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the forEach method name.
    for_each_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoArrayForEachVisitor<'a, 'b> {
    /// Build a visitor for no-array-for-each checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let for_each_name = ctx.program.strings.intern("forEach");

        Self {
            ctx,
            meta,
            array_symbol,
            for_each_name,
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

    /// Check a call expression for forEach usage.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };

        // check if this is forEach
        if method_call.method_name != self.for_each_name {
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
            LintDiagnostic::new(
                NO_ARRAY_FOR_EACH.id,
                NO_ARRAY_FOR_EACH.code,
                NO_ARRAY_FOR_EACH.category,
                severity,
                "prefer for-of over forEach",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use a for-of loop instead"),
        );
    }
}

impl NodeVisitor for NoArrayForEachVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_call(id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag forEach on arrays.
    #[test]
    fn test_flags_array_foreach() {
        let test = TestProgram::for_rule_without_prelude(NoArrayForEach);
        let result = test.lint_dir(
            "no_array_for_each/test_flags_array_foreach.ds",
            r#"
let items = [1, 2, 3];
items.forEach((item) => {
    console.log(item);
});
"#,
        );
        test.result(result).assert_lint("no-array-for-each");
    }

    /// Flag forEach with index parameter.
    #[test]
    fn test_flags_foreach_with_index() {
        let test = TestProgram::for_rule_without_prelude(NoArrayForEach);
        let result = test.lint_dir(
            "no_array_for_each/test_flags_foreach_with_index.ds",
            r#"
let items = ["a", "b", "c"];
items.forEach((item, index) => {
    console.log(index, item);
});
"#,
        );
        test.result(result).assert_lint("no-array-for-each");
    }

    /// Flag forEach on typed arrays.
    #[test]
    fn test_flags_typed_array_foreach() {
        let test = TestProgram::for_rule_without_prelude(NoArrayForEach);
        let result = test.lint_dir(
            "no_array_for_each/test_flags_typed_array_foreach.ds",
            r#"
let items: number[] = [1, 2, 3];
items.forEach((item) => console.log(item));
"#,
        );
        test.result(result).assert_lint("no-array-for-each");
    }

    /// Allow for-of loops.
    #[test]
    fn test_allows_for_of() {
        let test = TestProgram::for_rule_without_prelude(NoArrayForEach);
        let result = test.lint_dir(
            "no_array_for_each/test_allows_for_of.ds",
            r#"
let items = [1, 2, 3];
for (const item of items) {
    console.log(item);
}
"#,
        );
        test.result(result).assert_no_lint("no-array-for-each");
    }

    /// Allow map on arrays.
    #[test]
    fn test_allows_map() {
        let test = TestProgram::for_rule_without_prelude(NoArrayForEach);
        let result = test.lint_dir(
            "no_array_for_each/test_allows_map.ds",
            r#"
let items = [1, 2, 3];
let doubled = items.map((item) => item * 2);
"#,
        );
        test.result(result).assert_no_lint("no-array-for-each");
    }
}
