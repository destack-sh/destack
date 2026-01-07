use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::expression_target_symbol;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `new` on Symbol and BigInt.
    ///
    /// Symbol and BigInt are not constructors and should be called as functions.
    #[lint(
        id = "no-new-native-nonconstructor",
        code = "LC034",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [
            RequireWellKnownSymbol(WellKnownSymbol::Symbol),
            RequireWellKnownSymbol(WellKnownSymbol::BigInt)
        ],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoNewNativeNonconstructor,
    "Disallow new on Symbol and BigInt"
}

impl LintRule for NoNewNativeNonconstructor {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoNewNativeNonconstructor::meta()
    }

    /// Check module DIR nodes for new on non-constructors.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NewNonconstructorVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Non-constructor symbols that should not be used with `new`.
const NON_CONSTRUCTORS: &[WellKnownSymbol] = &[WellKnownSymbol::Symbol, WellKnownSymbol::BigInt];

/// Node visitor that flags new on Symbol and BigInt.
struct NewNonconstructorVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// Resolved non-constructor symbols for this module.
    non_constructors: Vec<(WellKnownSymbol, dir::GlobalSymbolId)>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NewNonconstructorVisitor<'a, 'b> {
    /// Build a visitor for new nonconstructor checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let non_constructors = NON_CONSTRUCTORS
            .iter()
            .filter_map(|&wks| ctx.get_well_known_symbol(wks).map(|id| (wks, id)))
            .collect();

        Self {
            ctx,
            meta,
            non_constructors,
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

    /// Check a new expression for Symbol or BigInt usage.
    fn check_new_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        // resolve the target symbol
        let Some(target) = expression_target_symbol(self.ctx.tree, left) else {
            return;
        };

        // check if target is a non-constructor
        let Some(wks) = self
            .non_constructors
            .iter()
            .find(|(_, id)| *id == target)
            .map(|(wks, _)| wks)
        else {
            return;
        };

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let name = wks.export_name();
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_NEW_NATIVE_NONCONSTRUCTOR.id,
                NO_NEW_NATIVE_NONCONSTRUCTOR.code,
                NO_NEW_NATIVE_NONCONSTRUCTOR.category,
                severity,
                format!("{name} is not a constructor"),
                self.ctx.module.file_id,
                span,
            )
            .with_label(format!("call {name}() without new")),
        );
    }
}

impl NodeVisitor for NewNonconstructorVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check new expressions
        if let dir::Expression::New { left, .. } = expression {
            self.check_new_expression(id, *left);
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
    fn test_flags_new_symbol() {
        let test = TestProgram::for_rule_with_prelude(NoNewNativeNonconstructor);
        let result = test.lint_dir(
            "test.ds",
            r#"
let s = new Symbol("description");
"#,
        );
        test.result(result)
            .assert_lint("no-new-native-nonconstructor");
    }

    #[test]
    fn test_flags_new_bigint() {
        let test = TestProgram::for_rule_with_prelude(NoNewNativeNonconstructor);
        let result = test.lint_dir(
            "test.ds",
            r#"
let n = new BigInt(42);
"#,
        );
        test.result(result)
            .assert_lint("no-new-native-nonconstructor");
    }

    #[test]
    fn test_allows_symbol_call() {
        let test = TestProgram::for_rule_with_prelude(NoNewNativeNonconstructor);
        let result = test.lint_dir(
            "test.ds",
            r#"
let s = Symbol("description");
"#,
        );
        test.result(result)
            .assert_no_lint("no-new-native-nonconstructor");
    }

    #[test]
    fn test_allows_bigint_call() {
        let test = TestProgram::for_rule_with_prelude(NoNewNativeNonconstructor);
        let result = test.lint_dir(
            "test.ds",
            r#"
let n = BigInt(42);
"#,
        );
        test.result(result)
            .assert_no_lint("no-new-native-nonconstructor");
    }

    #[test]
    fn test_allows_new_array() {
        let test = TestProgram::for_rule_with_prelude(NoNewNativeNonconstructor);
        let result = test.lint_dir(
            "test.ds",
            r#"
let arr = new Array(5);
"#,
        );
        test.result(result)
            .assert_no_lint("no-new-native-nonconstructor");
    }
}
