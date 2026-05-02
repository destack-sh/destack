use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    const_i64, expression_method_call, is_array_type, member_receiver_text,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Suggest `.find()` over `.filter()[0]`.
    ///
    /// `find` stops at the first match and doesn't create an intermediate array,
    /// making it more efficient and expressing intent more clearly.
    #[lint(
        id = "prefer-array-find",
        code = "LY030",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
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
    /// Pop method: `.filter(...).pop()`
    Pop,
    /// At method with negative index: `.filter(...).at(-1)`
    AtNegativeOne,
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
    /// The string id for the pop method name.
    pop_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferArrayFindVisitor<'a, 'b> {
    /// Build a visitor for prefer-array-find checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let filter_name = ctx.string_id("filter");
        let shift_name = ctx.string_id("shift");
        let at_name = ctx.string_id("at");
        let pop_name = ctx.string_id("pop");

        Self {
            ctx,
            meta,
            array_symbol,
            filter_name,
            shift_name,
            at_name,
            pop_name,
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
            self.report(expression_id, *left, FirstElementAccess::Index);
        }
    }

    /// Check if this is a filter().shift() or filter().at(0) pattern.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        let Some(call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };

        // check for .shift() with no arguments
        if call.method_name == self.shift_name {
            if !call.arguments.is_empty() {
                return;
            }

            if self.is_array_filter_call(call.receiver_id) {
                self.report(expression_id, call.receiver_id, FirstElementAccess::Shift);
            }
            return;
        }

        // check for .at(0) and .at(-1)
        if call.method_name == self.at_name {
            let Some(index_value) = self.at_index_value(expression_id) else {
                return;
            };

            if self.is_array_filter_call(call.receiver_id) {
                if index_value == 0 {
                    self.report(expression_id, call.receiver_id, FirstElementAccess::At);
                    return;
                }

                if index_value == -1 {
                    self.report(
                        expression_id,
                        call.receiver_id,
                        FirstElementAccess::AtNegativeOne,
                    );
                }
            }

            return;
        }

        // check for .pop() with no arguments
        if call.method_name == self.pop_name {
            if !call.arguments.is_empty() {
                return;
            }

            if self.is_array_filter_call(call.receiver_id) {
                self.report(expression_id, call.receiver_id, FirstElementAccess::Pop);
            }
        }
    }

    /// Build a find replacement for a filter call.
    fn build_find_replacement(
        &self,
        filter_call_id: dir::LocalNodeId<dir::Expression>,
        method_name: &str,
    ) -> Option<String> {
        let filter_call_expression = self.ctx.tree.get(filter_call_id);
        let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
        } = filter_call_expression
        else {
            return None;
        };
        if !generic_arguments.is_empty() {
            return None;
        }
        if arguments.is_empty() || arguments.len() > 2 {
            return None;
        }

        for argument_id in arguments {
            let argument = self.ctx.tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return None;
            }
        }

        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member {
            left: receiver_expression_id,
            name,
            ..
        } = member_expression
        else {
            return None;
        };
        if *name != Some(self.filter_name) {
            return None;
        }

        // derive receiver text from member expression text
        let member_span = self.ctx.get_span(*left);
        let member_text = self.ctx.get_span_text(member_span);
        let receiver_text = member_receiver_text(
            self.ctx,
            *receiver_expression_id,
            member_text,
            (*name)?,
            false,
        )?;

        // preserve callback and optional this-arg source range
        let first_argument_id = *arguments.first()?;
        let last_argument_id = *arguments.last()?;
        let first_span = self.ctx.get_span(first_argument_id);
        let last_span = self.ctx.get_span(last_argument_id);
        let arguments_span = Span::new(first_span.file, first_span.start, last_span.end);
        let arguments_text = self.ctx.get_span_text(arguments_span);

        Some(format!("{receiver_text}.{method_name}({arguments_text})"))
    }

    /// Return the literal numeric index for one `.at(index)` call.
    fn at_index_value(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> Option<i64> {
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call { arguments, .. } = expression else {
            return None;
        };

        // must have exactly one argument
        if arguments.len() != 1 {
            return None;
        }

        // argument must be literal 0
        let argument = self.ctx.tree.get(arguments[0]);
        let expression_id = argument.value();
        let const_value = self.ctx.const_value(expression_id)?;
        let index_value = const_i64(&const_value)?;

        Some(index_value)
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
        let dir::Expression::Call { arguments, .. } = expression else {
            return false;
        };
        if arguments.is_empty() {
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
        filter_call_id: dir::LocalNodeId<dir::Expression>,
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
            FirstElementAccess::Pop => "filter().pop()",
            FirstElementAccess::AtNegativeOne => "filter().at(-1)",
        };
        let span = self.ctx.get_span(expression_id);
        let preferred_method = match access {
            FirstElementAccess::Index | FirstElementAccess::Shift | FirstElementAccess::At => {
                "find"
            }
            FirstElementAccess::Pop | FirstElementAccess::AtNegativeOne => "findLast",
        };

        // attach a fix when the replacement is well-formed
        let mut diagnostic = LintReport::new(
            PREFER_ARRAY_FIND.id,
            PREFER_ARRAY_FIND.code,
            PREFER_ARRAY_FIND.category,
            severity,
            format!("prefer {preferred_method}() over {pattern}"),
            span,
        )
        .label(format!("use array.{preferred_method}(...) instead"));
        if self.ctx.include_fixes
            && let Some(replacement) = self.build_find_replacement(filter_call_id, preferred_method)
        {
            let edits = self
                .ctx
                .edit_builder()
                .replace(span, replacement)
                .into_edits();
            let fix =
                LintFix::safe("Replace filter first-element access with find").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

impl NodeVisitor for PreferArrayFindVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
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
            "prefer_array_find/test_flags_filter_index_zero.ds",
            r#"
let items = [1, 2, 3];
let first = items.filter(x => x > 1)[0];
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-find")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let first = items.find((x) => x > 1);
"#,
            );
    }

    /// Flag filter().shift() pattern.
    #[test]
    fn test_flags_filter_shift() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "prefer_array_find/test_flags_filter_shift.ds",
            r#"
let items = [1, 2, 3];
let first = items.filter(x => x > 1).shift();
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-find")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let first = items.find((x) => x > 1);
"#,
            );
    }

    /// Flag filter().at(0) pattern.
    #[test]
    fn test_flags_filter_at_zero() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "prefer_array_find/test_flags_filter_at_zero.ds",
            r#"
let items = [1, 2, 3];
let first = items.filter(x => x > 1).at(0);
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-find")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let first = items.find((x) => x > 1);
"#,
            );
    }

    /// Allow filter()[1] since it's not a first-element access.
    #[test]
    fn test_allows_filter_index_one() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "prefer_array_find/test_allows_filter_index_one.ds",
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
            "prefer_array_find/test_allows_find_directly.ds",
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
            "prefer_array_find/test_allows_non_filter_index.ds",
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
            "prefer_array_find/test_allows_filter_at_one.ds",
            r#"
let items = [1, 2, 3];
let second = items.filter(x => x > 0).at(1);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-find");
    }

    /// Flag filter().at(-1) and rewrite to findLast.
    #[test]
    fn test_flags_filter_at_negative_one() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "prefer_array_find/test_flags_filter_at_negative_one.ds",
            r#"
let items = [1, 2, 3];
let last = items.filter(x => x > 0).at(-1);
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-find")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let last = items.findLast((x) => x > 0);
"#,
            );
    }

    /// Flag filter().pop() and rewrite to findLast.
    #[test]
    fn test_flags_filter_pop() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "prefer_array_find/test_flags_filter_pop.ds",
            r#"
let items = [1, 2, 3];
let last = items.filter(x => x > 0).pop();
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-find")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let last = items.findLast((x) => x > 0);
"#,
            );
    }

    /// Allow filter().at(-2) since it is not a first or last shorthand pattern.
    #[test]
    fn test_allows_filter_at_negative_two() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "prefer_array_find/test_allows_filter_at_negative_two.ds",
            r#"
let items = [1, 2, 3];
let value = items.filter(x => x > 0).at(-2);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-find");
    }

    /// Keep thisArg arguments when rewriting filter to find.
    #[test]
    fn test_fix_preserves_filter_this_arg() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFind);
        let result = test.lint_dir(
            "prefer_array_find/test_fix_preserves_filter_this_arg.ds",
            r#"
let items = [1, 2, 3];
let first = items.filter(predicate, context)[0];
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-find")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let first = items.find(predicate, context);
"#,
            );
    }
}
