use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_is_standalone_statement, expression_unwrap_parenthesized, is_array_type,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow deleting array elements.
    ///
    /// Deleting array elements creates sparse arrays and keeps the length unchanged.
    #[lint(
        id = "no-array-delete",
        code = "LC004",
        category = Correctness,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoArrayDelete,
    "Disallow delete on array elements"
}

impl LintRule for NoArrayDelete {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoArrayDelete::meta()
    }

    /// Check module DIR nodes for array deletes.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk module expressions
        let mut visitor = ArrayDeleteVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags delete on arrays.
struct ArrayDeleteVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ArrayDeleteVisitor<'a, 'b> {
    /// Build a visitor for array delete checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
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
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a delete target for array element deletion.
    fn check_delete_target(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_id: dir::LocalNodeId<dir::Expression>,
    ) {
        let target_id = expression_unwrap_parenthesized(self.ctx.tree, target_id);

        // only consider index expressions
        let target_expression = self.ctx.tree.get(target_id);
        let dir::Expression::Index { left, .. } = target_expression else {
            return;
        };

        // resolve the array type for the index target
        let Some(type_id) = self.ctx.expression_type_id(*left) else {
            return;
        };
        let is_array = is_array_type(self.ctx.types, type_id, Some(self.array_symbol));
        if !is_array {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            NO_ARRAY_DELETE.id,
            NO_ARRAY_DELETE.code,
            NO_ARRAY_DELETE.category,
            severity,
            "avoid deleting array elements",
            span,
        )
        .label("use a method like `splice` instead");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && let Some(fix) = no_array_delete_fix(self.ctx, expression_id, target_id)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

/// Build an unsafe fix for standalone `delete array[index]` statements.
fn no_array_delete_fix(
    ctx: &LintModuleDirContext<'_>,
    delete_id: dir::LocalNodeId<dir::Expression>,
    target_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    // only rewrite standalone delete expressions
    if !expression_is_standalone_statement(ctx.tree, delete_id) {
        return None;
    }

    // require an indexed target expression
    let target_id = expression_unwrap_parenthesized(ctx.tree, target_id);
    let target_expression = ctx.tree.get(target_id);
    let dir::Expression::Index { left, right } = target_expression else {
        return None;
    };
    let right_id = (*right)?;

    // resolve source text slices
    let left_span = ctx.get_span(*left);
    let right_span = ctx.get_span(right_id);
    let left_text = ctx.get_span_text(left_span);
    let right_text = ctx.get_span_text(right_span);
    if left_text.trim().is_empty() || right_text.trim().is_empty() {
        return None;
    }

    // build replacement call
    let replacement = format!("{left_text}.splice({right_text}, 1)");

    // build rewrite edit
    let delete_span = ctx.get_span(delete_id);
    let edits = ctx
        .edit_builder()
        .replace(delete_span, replacement)
        .into_edits();

    // return unsafe mutation fix
    Some(LintFix::r#unsafe("Replace delete with splice").with_edits(edits))
}

impl NodeVisitor for ArrayDeleteVisitor<'_, '_> {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit an expression node.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check delete expressions
        if let dir::Expression::Delete { value } = expression {
            self.check_delete_target(id, *value);
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
    fn test_flags_delete_array_element() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_flags_delete_array_element.ds",
            r#"
let items = [1, 2, 3];
delete items[0];
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-array-delete")
            .assert_has_fix("no-array-delete");
    }

    #[test]
    fn test_allows_delete_object_property() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_allows_delete_object_property.ds",
            r#"
let item = { value: 1 };
delete item.value;
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-array-delete");
    }

    #[test]
    fn test_fix_rewrites_delete_array_element() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_fix_rewrites_delete_array_element.ds",
            r#"
let items = [1, 2, 3];
delete items[0];
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-array-delete")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
items.splice(0, 1);
"#,
            );
    }

    #[test]
    fn test_no_fix_when_delete_result_is_used() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_no_fix_when_delete_result_is_used.ds",
            r#"
let items = [1, 2, 3];
let removed = delete items[0];
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-array-delete")
            .assert_has_no_fix("no-array-delete");
    }

    #[test]
    fn test_mutation_fix_rewrites_delete_with_dynamic_index() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_mutation_fix_rewrites_delete_with_dynamic_index.ds",
            r#"
let items = [1, 2, 3];
let index = 1;
delete items[index];
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-array-delete")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
let index = 1;
items.splice(index, 1);
"#,
            );
    }

    #[test]
    fn test_flags_parenthesized_delete_array_element() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_flags_parenthesized_delete_array_element.ds",
            r#"
let items = [1, 2, 3];
delete ((items[0]));
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-array-delete");
    }

    #[test]
    fn test_fix_rewrites_parenthesized_delete_array_element() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_fix_rewrites_parenthesized_delete_array_element.ds",
            r#"
let items = [1, 2, 3];
delete ((items[0]));
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-array-delete")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
items.splice(0, 1);
"#,
            );
    }

    #[test]
    fn test_fix_rewrites_parenthesized_delete_expression_statement() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_fix_rewrites_parenthesized_delete_expression_statement.ds",
            r#"
let items = [1, 2, 3];
(delete items[0]);
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-array-delete")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
(items.splice(0, 1));
"#,
            );
    }

    #[test]
    fn test_flags_delete_tuple_element() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_flags_delete_tuple_element.ds",
            r#"
let items: [int32, int32, int32] = [1, 2, 3];
delete items[0];
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-array-delete");
    }

    #[test]
    fn test_fix_rewrites_sequence_index_delete_array_element() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "no_array_delete/test_fix_rewrites_sequence_index_delete_array_element.ds",
            r#"
function compute(): number {
    return 1;
}

let items = [1, 2, 3];
delete items[(compute(), 1)];
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-array-delete")
            .assert_unsafe_fixed(
                r#"
function compute(): number {
    return 1;
}

let items = [1, 2, 3];
items.splice((compute(), 1), 1);
"#,
            );
    }
}
