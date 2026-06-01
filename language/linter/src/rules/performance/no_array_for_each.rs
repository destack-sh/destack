use destack_core::StringId;
use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{
    collect_local_symbol_direct_reference_expression_ids, expression_method_call, is_array_type,
    is_simple_identifier, statement_expression_ancestor, strip_dot_member_suffix,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

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
        requires_all = [RequireLanguageItem(LanguageItem::Array)],
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoArrayForEachVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags Array.forEach() calls.
struct NoArrayForEachVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the forEach method name.
    for_each_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoArrayForEachVisitor<'a, 'b> {
    /// Build a visitor for no-array-for-each checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.language_item(LanguageItem::Array);
        let for_each_name = ctx.string_id("forEach");

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
        let tree = self.ctx.dir.tree();

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call expression for forEach usage.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.dir.tree(), expression_id) else {
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
        if !is_array_type(self.ctx, type_id, Some(self.array_symbol)) {
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
            NO_ARRAY_FOR_EACH.id,
            NO_ARRAY_FOR_EACH.code,
            NO_ARRAY_FOR_EACH.category,
            severity,
            "prefer for-of over forEach",
            span,
        )
        .label("use a for-of loop instead");
        if self.ctx.compute_fixes
            && let Some(fix) = self.no_array_for_each_fix(expression_id)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build an unsafe forEach to for-of rewrite for simple inline callbacks.
    fn no_array_for_each_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintFix> {
        let method_call = expression_method_call(self.ctx.dir.tree(), expression_id)?;

        // require no static args and exactly one dynamic callback arg
        if !method_call.generic_arguments.is_empty() || method_call.arguments.len() != 1 {
            return None;
        }

        // keep statement-level calls only
        let statement_id = statement_expression_ancestor(self.ctx.dir.tree(), expression_id)?;

        // require a direct `.forEach` member access
        if method_call.method_name != self.for_each_name {
            return None;
        }

        // require an inline non-async function callback
        let callback_argument = self.ctx.dir.get(method_call.arguments[0]);
        let dir::Argument::Positional {
            value: callback_id, ..
        } = callback_argument
        else {
            return None;
        };
        let callback_expression = self.ctx.dir.get(*callback_id);
        let dir::Expression::Declaration(declaration) = callback_expression else {
            return None;
        };
        let callback_declaration = self.ctx.dir.get(*declaration);
        let dir::Declaration::Function(declaration) = callback_declaration else {
            return None;
        };
        let body_id = declaration.body?;
        if declaration.signature.asynchrony != dir::Asynchrony::Sync
            || declaration.signature.parameters.len() != 1
        {
            return None;
        }

        // require a single named parameter without modifiers/default
        let parameter_id = declaration.signature.parameters[0];
        let parameter = self.ctx.dir.get(parameter_id);
        let dir::Parameter::Named {
            is_optional: false,
            name,
            default: None,
            ..
        } = parameter
        else {
            return None;
        };
        let parameter_symbol = self.ctx.local_symbol_for_node(parameter_id)?;

        // require a block body so we can preserve statements exactly
        let body_expression = self.ctx.dir.get(body_id);
        if !matches!(body_expression, dir::Expression::Block(..)) {
            return None;
        }

        // choose one loop binding name from the callback parameter symbol
        let parameter_name = self.ctx.strings.get(*name).to_string();
        let binding_name = self.for_of_binding_name(statement_id, &parameter_name)?;

        // rewrite callback parameter references inside the body
        let member_text = self
            .ctx
            .get_span_text(self.ctx.get_span(method_call.callee_id));
        let receiver_text = strip_dot_member_suffix(member_text, "forEach")?;
        let rewritten_body =
            self.rewrite_callback_body(body_id, parameter_symbol, &binding_name)?;
        let replacement = format!("for (const {binding_name} of {receiver_text}) {rewritten_body}");
        let edits = self
            .ctx
            .edit_builder()
            .replace(self.ctx.get_span(statement_id), replacement)
            .into_edits();
        Some(LintFix::r#unsafe("Rewrite forEach callback as for-of loop").with_edits(edits))
    }

    /// Choose one loop binding name that stays valid after callback flattening.
    fn for_of_binding_name(
        &self,
        statement_id: dir::LocalNodeId<dir::Expression>,
        parameter_name: &str,
    ) -> Option<String> {
        if !is_simple_identifier(parameter_name) {
            return None;
        }

        // resolve the lexical scope where the new for-of binding will land
        let scope_cursor = self.ctx.scope_for_node(statement_id)?;
        let scope = self.ctx.symbols.get_scope(scope_cursor);
        let is_name_taken = |candidate: &str| {
            let candidate_id = self.ctx.string_id(candidate);
            let candidate_key = dir::StaticKey::Name(candidate_id);
            scope
                .find_symbol_up_to(candidate_key, scope_cursor.mark)
                .is_some()
        };

        // keep the original callback parameter name when it stays valid
        if !is_name_taken(parameter_name) {
            return Some(parameter_name.to_string());
        }

        // otherwise add an element suffix with deterministic numbering
        let mut candidate = format!("{parameter_name}Element");
        let mut suffix_index = 2usize;
        while is_name_taken(&candidate) {
            candidate = format!("{parameter_name}Element{suffix_index}");
            suffix_index += 1;
            if suffix_index > 1024 {
                return None;
            }
        }

        Some(candidate)
    }

    /// Rewrite one callback body by renaming direct references to the callback parameter.
    fn rewrite_callback_body(
        &self,
        body_id: dir::LocalNodeId<dir::Expression>,
        parameter_symbol: dir::LocalSymbolId,
        binding_name: &str,
    ) -> Option<String> {
        let body_span = self.ctx.get_span(body_id);
        let mut body_text = self.ctx.get_span_text(body_span).to_string();
        let mut spans = collect_local_symbol_direct_reference_expression_ids(
            self.ctx.module_id(),
            self.ctx.dir.tree(),
            self.ctx.resolutions,
            parameter_symbol,
        )
        .into_iter()
        .map(|expression_id| self.ctx.get_span(expression_id))
        .filter(|span| span.start >= body_span.start && span.end <= body_span.end)
        .collect::<Vec<_>>();
        spans.sort_by_key(|span| std::cmp::Reverse(span.start));

        for span in spans {
            let start = (span.start - body_span.start) as usize;
            let end = (span.end - body_span.start) as usize;
            body_text.replace_range(start..end, binding_name);
        }

        Some(body_text)
    }
}

impl NodeVisitor for NoArrayForEachVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
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

    /// Keep the fix valid when the callback parameter name already exists in scope.
    #[test]
    fn test_fix_uses_fresh_binding_name_on_collision() {
        let test = TestProgram::for_rule_without_prelude(NoArrayForEach);
        let result = test.lint_dir(
            "no_array_for_each/test_fix_uses_fresh_binding_name_on_collision.ds",
            r#"
let item = 0;
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
let item = 0;
let items = [1, 2, 3];
for (const itemElement of items) {
    console.log(itemElement);
}
"#,
            );
    }
}
