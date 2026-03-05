use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_method_call, is_array_type, statement_expression_ancestor, strip_dot_member_suffix,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `for-of` over `Array.forEach()`.
    ///
    /// `for-of` loops are more performant than `forEach` because they avoid
    /// the overhead of function calls, and they support `break`, `continue`,
    /// and `return` statements.
    #[lint(
        id = "no-array-for-each",
        code = "LP002",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
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
        let mut diagnostic = LintDiagnostic::new(
            NO_ARRAY_FOR_EACH.id,
            NO_ARRAY_FOR_EACH.code,
            NO_ARRAY_FOR_EACH.category,
            severity,
            "prefer for-of over forEach",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use a for-of loop instead");
        if self.ctx.include_fixes
            && let Some(fix) = self.no_array_for_each_fix(expression_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build an unsafe forEach to for-of rewrite for simple inline callbacks.
    fn no_array_for_each_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintFix> {
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            left,
            static_arguments,
            dynamic_arguments,
        } = expression
        else {
            return None;
        };

        // require no static args and exactly one dynamic callback arg
        if static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty())
            || dynamic_arguments.len() != 1
        {
            return None;
        }

        // keep statement-level calls only
        let statement_id = statement_expression_ancestor(self.ctx.tree, expression_id)?;

        // require a direct `.forEach` member access
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member {
            left: _receiver_id,
            name,
            ..
        } = member_expression
        else {
            return None;
        };
        if *name != self.for_each_name {
            return None;
        }

        // require an inline non-async function callback
        let callback_argument = self.ctx.tree.get(dynamic_arguments[0]);
        let dir::Argument::Positional {
            value: callback_id, ..
        } = callback_argument
        else {
            return None;
        };
        let callback_expression = self.ctx.tree.get(*callback_id);
        let dir::Expression::Declaration { declaration } = callback_expression else {
            return None;
        };
        let callback_declaration = self.ctx.tree.get(*declaration);
        let dir::Declaration::Function {
            signature,
            body: Some(body_id),
            ..
        } = callback_declaration
        else {
            return None;
        };
        if signature.asynchrony != dir::Asynchrony::Sync || signature.dynamic_parameters.len() != 1
        {
            return None;
        }

        // require a single named parameter without modifiers/default
        let parameter_id = signature.dynamic_parameters[0];
        let parameter = self.ctx.tree.get(parameter_id);
        let dir::Parameter::Named {
            modifiers: None,
            name,
            default: None,
            ..
        } = parameter
        else {
            return None;
        };

        // require a block body so we can preserve statements exactly
        let body_expression = self.ctx.tree.get(*body_id);
        if !matches!(body_expression, dir::Expression::Block { .. }) {
            return None;
        }

        // rewrite to a simple for-of loop
        let parameter_name = self.ctx.program.strings.get(*name).to_string();
        let member_text = self.ctx.get_span_text(self.ctx.get_span(*left));
        let receiver_text = strip_dot_member_suffix(member_text.as_ref(), "forEach")?;
        let body_text = self.ctx.get_span_text(self.ctx.get_span(*body_id));
        let replacement = format!("for (const {parameter_name} of {receiver_text}) {body_text}");
        let edits = self
            .ctx
            .edit_builder()
            .replace(self.ctx.get_span(statement_id), replacement)
            .into_edits();
        Some(LintFix::r#unsafe("Rewrite forEach callback as for-of loop").with_edits(edits))
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
        test.result(result)
            .assert_lint("no-array-for-each")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
for (const item of items) {
    console.log(item);
}
"#,
            );
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

    /// Keep no fix for callbacks that depend on index parameter.
    #[test]
    fn test_no_fix_for_index_callback() {
        let test = TestProgram::for_rule_without_prelude(NoArrayForEach);
        let result = test.lint_dir(
            "no_array_for_each/test_no_fix_for_index_callback.ds",
            r#"
let items = ["a", "b", "c"];
items.forEach((item, index) => {
    console.log(index, item);
});
"#,
        );
        test.result(result)
            .assert_lint("no-array-for-each")
            .assert_has_no_fix("no-array-for-each");
    }

    /// Keep no fix for non-inline callbacks.
    #[test]
    fn test_no_fix_for_non_inline_callback() {
        let test = TestProgram::for_rule_without_prelude(NoArrayForEach);
        let result = test.lint_dir(
            "no_array_for_each/test_no_fix_for_non_inline_callback.ds",
            r#"
let items = [1, 2, 3];
const logItem = (item) => {
    console.log(item);
};
items.forEach(logItem);
"#,
        );
        test.result(result)
            .assert_lint("no-array-for-each")
            .assert_has_no_fix("no-array-for-each");
    }
}
