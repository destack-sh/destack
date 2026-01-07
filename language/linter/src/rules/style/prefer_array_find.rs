use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{const_i64, expression_method_call, is_array_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest `.find()` over `.filter()[0]`.
    ///
    /// `find` stops at the first match and doesn't create an intermediate array,
    /// making it more efficient and expressing intent more clearly.
    #[lint(
        id = "prefer-array-find",
        code = "LY084",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArrayFind,
    "Prefer find() over filter()[0]"
}

/// The kind of first-element access pattern detected.
#[derive(Debug, Clone, Copy)]
enum FirstElementAccess {
    /// Index access: `.filter(...)[0]`
    Index,
    /// Shift method: `.filter(...).shift()`
    Shift,
    /// At method: `.filter(...).at(0)`
    At,
}

impl LintRule for PreferArrayFind {
    fn meta(&self) -> &'static LintMeta {
        PreferArrayFind::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferArrayFindVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags filter()[0] patterns.
struct PreferArrayFindVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The array symbol for this module profile.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the filter method name.
    filter_name: StringId,
    /// The string id for the shift method name.
    shift_name: StringId,
    /// The string id for the at method name.
    at_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferArrayFindVisitor<'a, 'b> {
    /// Build a visitor for prefer-array-find checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let filter_name = ctx.program.strings.intern("filter");
        let shift_name = ctx.program.strings.intern("shift");
        let at_name = ctx.program.strings.intern("at");

        Self {
            ctx,
            meta,
            array_symbol,
            filter_name,
            shift_name,
            at_name,
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

    /// Check if this is a filter()[0] index access pattern.
    fn check_index(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Index { left, right, .. } = expression else {
            return;
        };

        // index expression must be present
        let Some(right) = right else {
            return;
        };

        // check if index is 0
        let Some(const_value) = self.ctx.const_value(*right) else {
            return;
        };
        let Some(index_value) = const_i64(&const_value) else {
            return;
        };
        if index_value != 0 {
            return;
        }

        // check if left is a filter call on an array
        if self.is_array_filter_call(*left) {
            self.report(expression_id, FirstElementAccess::Index);
        }
    }

    /// Check if this is a filter().shift() or filter().at(0) pattern.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        let Some(call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };

        // check for .shift() with no arguments
        if call.method_name == self.shift_name {
            let expression = self.ctx.tree.get(expression_id);
            let dir::Expression::Call {
                dynamic_arguments, ..
            } = expression
            else {
                return;
            };
            if !dynamic_arguments.is_empty() {
                return;
            }

            if self.is_array_filter_call(call.receiver_id) {
                self.report(expression_id, FirstElementAccess::Shift);
            }
            return;
        }

        // check for .at(0)
        if call.method_name == self.at_name {
            if !self.is_at_zero(expression_id) {
                return;
            }

            if self.is_array_filter_call(call.receiver_id) {
                self.report(expression_id, FirstElementAccess::At);
            }
        }
    }

    /// Check if this is a .at(0) call.
    fn is_at_zero(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            dynamic_arguments, ..
        } = expression
        else {
            return false;
        };

        // must have exactly one argument
        if dynamic_arguments.len() != 1 {
            return false;
        }

        // argument must be literal 0
        let argument = self.ctx.tree.get(dynamic_arguments[0]);
        let expression_id = argument.value();
        let Some(const_value) = self.ctx.const_value(expression_id) else {
            return false;
        };
        let Some(index_value) = const_i64(&const_value) else {
            return false;
        };

        index_value == 0
    }

    /// Check if expression is a filter() call on an array.
    fn is_array_filter_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let Some(call) = expression_method_call(self.ctx.tree, expression_id) else {
            return false;
        };

        // check method name is filter
        if call.method_name != self.filter_name {
            return false;
        }

        // check filter has at least one argument
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            dynamic_arguments, ..
        } = expression
        else {
            return false;
        };
        if dynamic_arguments.is_empty() {
            return false;
        }

        // check receiver is an array
        self.is_array_receiver(call.receiver_id)
    }

    /// Return true when the receiver expression is an array type.
    fn is_array_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_array_type(self.ctx.types, type_id, Some(self.array_symbol))
    }

    /// Report a prefer-array-find match.
    fn report(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        access: FirstElementAccess,
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build message based on access pattern
        let pattern = match access {
            FirstElementAccess::Index => "filter()[0]",
            FirstElementAccess::Shift => "filter().shift()",
            FirstElementAccess::At => "filter().at(0)",
        };

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                PREFER_ARRAY_FIND.id,
                PREFER_ARRAY_FIND.code,
                PREFER_ARRAY_FIND.category,
                severity,
                format!("prefer find() over {pattern}"),
                self.ctx.module.file_id,
                span,
            )
            .with_label("use array.find(...) instead"),
        );
    }
}

impl NodeVisitor for PreferArrayFindVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check index access
        if matches!(expression, dir::Expression::Index { .. }) {
            self.check_index(id);
        }

        // check method calls
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

    /// Flag filter()[0] pattern.
    #[test]
    fn test_flags_filter_index_zero() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let first = items.filter(x => x > 1)[0];
"#,
        );
        test.result(result).assert_lint("prefer-array-find");
    }

    /// Flag filter().shift() pattern.
    #[test]
    fn test_flags_filter_shift() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let first = items.filter(x => x > 1).shift();
"#,
        );
        test.result(result).assert_lint("prefer-array-find");
    }

    /// Flag filter().at(0) pattern.
    #[test]
    fn test_flags_filter_at_zero() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let first = items.filter(x => x > 1).at(0);
"#,
        );
        test.result(result).assert_lint("prefer-array-find");
    }

    /// Allow filter()[1] since it's not a first-element access.
    #[test]
    fn test_allows_filter_index_one() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let second = items.filter(x => x > 1)[1];
"#,
        );
        test.result(result).assert_no_lint("prefer-array-find");
    }

    /// Allow direct find() usage.
    #[test]
    fn test_allows_find_directly() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let first = items.find(x => x > 1);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-find");
    }

    /// Allow index on non-filter result.
    #[test]
    fn test_allows_non_filter_index() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let first = items.map(x => x * 2)[0];
"#,
        );
        test.result(result).assert_no_lint("prefer-array-find");
    }

    /// Allow filter().at(1) since it's not a first-element access.
    #[test]
    fn test_allows_filter_at_one() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let second = items.filter(x => x > 0).at(1);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-find");
    }

    /// Allow filter().at(-1) for last element access.
    #[test]
    fn test_allows_filter_at_negative() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let last = items.filter(x => x > 0).at(-1);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-find");
    }
}
