use destack_core::StringId;
use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_repository::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{is_array_type, is_string_array_type};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Require comparison function for array `.sort()`.
    ///
    /// Without a comparison function, `.sort()` converts elements to strings
    /// and sorts them lexicographically. This is almost never correct for
    /// numeric arrays: `[10, 2, 1].sort()` produces `[1, 10, 2]`.
    #[lint(
        id = "require-array-sort-compare",
        code = "LC040",
        category = Correctness,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Array)],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub RequireArraySortCompare,
    "Require comparison function for .sort()"
}

impl LintRule for RequireArraySortCompare {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        RequireArraySortCompare::meta()
    }

    /// Check module DIR nodes for sort calls without comparator.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = ArraySortVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags sort calls without comparison functions.
struct ArraySortVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The sort method name.
    sort_name: StringId,
    /// The toSorted method name.
    to_sorted_name: StringId,
    /// The language item String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ArraySortVisitor<'a, 'b> {
    /// Build a visitor for array sort checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.language_item(LanguageItem::Array);
        let sort_name = ctx.string_id("sort");
        let to_sorted_name = ctx.string_id("toSorted");
        let string_symbol = ctx.language_item(LanguageItem::String);

        Self {
            ctx,
            meta,
            array_symbol,
            sort_name,
            to_sorted_name,
            string_symbol,
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

    /// Check a call expression for array sort without comparator.
    fn check_sort_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // match member access for sort method
        let left_expression = self.ctx.dir.get(left);
        let dir::Expression::Member {
            left: receiver,
            name,
            ..
        } = left_expression
        else {
            return;
        };

        // check method name
        if *name != Some(self.sort_name) && *name != Some(self.to_sorted_name) {
            return;
        }

        // ensure no arguments provided
        if !arguments.is_empty() {
            return;
        }

        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(*receiver) else {
            return;
        };

        // check if the receiver is an array type
        if !is_array_type(self.ctx, type_id, Some(self.array_symbol)) {
            return;
        }

        // ignore string arrays because lexicographic ordering is usually intentional
        if is_string_array_type(
            self.ctx,
            type_id,
            Some(self.array_symbol),
            Some(self.string_symbol),
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
                REQUIRE_ARRAY_SORT_COMPARE.id,
                REQUIRE_ARRAY_SORT_COMPARE.code,
                REQUIRE_ARRAY_SORT_COMPARE.category,
                severity,
                "array sort call requires a comparison function",
                span,
            )
            .label("provide a comparison function for non-string array ordering"),
        );
    }
}

impl NodeVisitor for ArraySortVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for sort without comparator
        if let dir::Expression::Call {
            left, arguments, ..
        } = expression
        {
            self.check_sort_call(id, *left, arguments);
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
    fn test_flags_sort_without_compare() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_flags_sort_without_compare.ds",
            r#"
let items = [3, 1, 2];
items.sort();
"#,
        );
        test.result(result)
            .assert_lint("require-array-sort-compare");
    }

    #[test]
    fn test_flags_sort_on_typed_array() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_flags_sort_on_typed_array.ds",
            r#"
let items: number[] = [3, 1, 2];
items.sort();
"#,
        );
        test.result(result)
            .assert_lint("require-array-sort-compare");
    }

    #[test]
    fn test_allows_sort_with_compare() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_allows_sort_with_compare.ds",
            r#"
let items = [3, 1, 2];
items.sort((a, b) => a - b);
"#,
        );
        test.result(result)
            .assert_no_lint("require-array-sort-compare");
    }

    #[test]
    fn test_allows_sort_on_non_array() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_allows_sort_on_non_array.ds",
            r#"
let custom = { sort: () => {} };
custom.sort();
"#,
        );
        test.result(result)
            .assert_no_lint("require-array-sort-compare");
    }

    #[test]
    fn test_allows_string_array_sort_without_compare() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_allows_string_array_sort_without_compare.ds",
            r#"
let names: string[] = ["c", "a", "b"];
names.sort();
"#,
        );
        test.result(result)
            .assert_no_lint("require-array-sort-compare");
    }

    #[test]
    fn test_flags_to_sorted_without_compare() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_flags_to_sorted_without_compare.ds",
            r#"
let items = [3, 1, 2];
items.toSorted();
"#,
        );
        test.result(result)
            .assert_lint("require-array-sort-compare");
    }

    #[test]
    fn test_allows_to_sorted_on_string_array_without_compare() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_allows_to_sorted_on_string_array_without_compare.ds",
            r#"
let names: string[] = ["c", "a", "b"];
names.toSorted();
"#,
        );
        test.result(result)
            .assert_no_lint("require-array-sort-compare");
    }

    #[test]
    fn test_flags_sort_on_mixed_string_number_array_union() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_flags_sort_on_mixed_string_number_array_union.ds",
            r#"
let values: string[] | number[] = [3, 1, 2];
values.sort();
"#,
        );
        test.result(result)
            .assert_lint("require-array-sort-compare");
    }

    #[test]
    fn test_allows_sort_on_all_string_array_union() {
        let test = TestProgram::for_rule_without_prelude(RequireArraySortCompare);
        let result = test.lint_dir(
            "require_array_sort_compare/test_allows_sort_on_all_string_array_union.ds",
            r#"
let values: string[] | [string, string] = ["b", "a"];
values.sort();
"#,
        );
        test.result(result)
            .assert_no_lint("require-array-sort-compare");
    }
}
