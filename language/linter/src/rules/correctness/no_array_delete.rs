use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::is_array_type;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow deleting array elements.
    ///
    /// Deleting array elements creates sparse arrays and keeps the length unchanged.
    #[lint(
        id = "no-array-delete",
        code = "LC006",
        category = Correctness,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
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
        let meta = self.meta();
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
        self.ctx.report(
            LintDiagnostic::new(
                NO_ARRAY_DELETE.id,
                NO_ARRAY_DELETE.code,
                NO_ARRAY_DELETE.category,
                severity,
                "avoid deleting array elements",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use a method like `splice` instead"),
        );
    }
}

impl NodeVisitor for ArrayDeleteVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
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
            "test.ds",
            r#"
let items = [1, 2, 3];
delete items[0];
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-array-delete");
    }

    #[test]
    fn test_allows_delete_object_property() {
        let test = TestProgram::for_rule_without_prelude(NoArrayDelete);
        let result = test.lint_dir(
            "test.ds",
            r#"
let item = { value: 1 };
delete item.value;
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-array-delete");
    }
}
