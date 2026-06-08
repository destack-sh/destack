use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_repository::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::is_array_like_iteration_type;
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow iterating over arrays with for-in.
    ///
    /// `for-in` iterates over enumerable property names (strings), not values.
    /// This is almost never what you want for arrays. Use `for-of` instead.
    #[lint(
        id = "no-for-in-array",
        code = "LC017",
        category = Correctness,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Array)],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoForInArray,
    "Disallow iterating over arrays with for-in"
}

impl LintRule for NoForInArray {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoForInArray::meta()
    }

    /// Check module DIR nodes for for-in on arrays.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = ForInArrayVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags for-in on arrays.
struct ForInArrayVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ForInArrayVisitor<'a, 'b> {
    /// Build a visitor for for-in array checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.language_item(LanguageItem::Array);
        Self {
            ctx,
            meta,
            array_symbol,
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

    /// Check a for-each expression for array iteration with for-in.
    fn check_for_in_array(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        iterator_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // resolve the iterator type
        let Some(type_id) = self.ctx.expression_type_id(iterator_id) else {
            return;
        };

        // check if the iterator may be array-like
        if !is_array_like_iteration_type(
            self.ctx,
            self.ctx.strings,
            type_id,
            Some(self.array_symbol),
        ) {
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
                NO_FOR_IN_ARRAY.id,
                NO_FOR_IN_ARRAY.code,
                NO_FOR_IN_ARRAY.category,
                severity,
                "do not use for-in with arrays",
                span,
            )
            .label("use for-of to iterate over array values"),
        );
    }
}

impl NodeVisitor for ForInArrayVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for-in expressions on arrays
        if let dir::Expression::ForEach {
            operator: dir::ForEachOperator::In,
            iterator,
            ..
        } = expression
        {
            self.check_for_in_array(id, *iterator);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_for_in_array() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_flags_for_in_array.ds",
            r#"
let items = [1, 2, 3];
for (const key in items) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-for-in-array");
    }

    #[test]
    fn test_flags_for_in_array_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_flags_for_in_array_without_fix.ds",
            r#"
let items = [1, 2, 3];
for (const key in items) {
    let value = key;
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-for-in-array")
            .assert_has_no_fix("no-for-in-array");
    }

    #[test]
    fn test_flags_for_in_typed_array() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_flags_for_in_typed_array.ds",
            r#"
let items: number[] = [1, 2, 3];
for (const key in items) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-for-in-array");
    }

    #[test]
    fn test_flags_typed_for_in_array_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_flags_typed_for_in_array_without_fix.ds",
            r#"
let items: number[] = [1, 2, 3];
for (const key in items) {
    let value = key;
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-for-in-array")
            .assert_has_no_fix("no-for-in-array");
    }

    #[test]
    fn test_flags_for_in_array_with_inline_comments_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_flags_for_in_array_with_inline_comments_without_fix.ds",
            r#"
let items = [1, 2, 3];
for (const key/* left */in/* right */items) {
    let value = key;
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-for-in-array")
            .assert_has_no_fix("no-for-in-array");
    }

    #[test]
    fn test_flags_for_in_union_with_array_branch() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_flags_for_in_union_with_array_branch.ds",
            r#"
let items: number[] | { a: int32 } = [1, 2, 3];
for (const key in items) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-for-in-array");
    }

    #[test]
    fn test_allows_for_of_array() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_allows_for_of_array.ds",
            r#"
let items = [1, 2, 3];
for (const value of items) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-for-in-array");
    }

    #[test]
    fn test_allows_for_in_object() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_allows_for_in_object.ds",
            r#"
let obj = { a: 1, b: 2 };
for (const key in obj) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-for-in-array");
    }
}
