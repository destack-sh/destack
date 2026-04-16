use destack_core::StringId;
use destack_dir::{
    self as dir, FunctionMode, IfKind, NodeVisitor, NodeVisitorOptions, Property, WellKnownSymbol,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_is_global_qualified_member, expression_static_property_access,
    expression_target_symbol, expression_unwrap_parenthesized, span_has_comment,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer object spread over `Object.assign()`.
    ///
    /// Object spread is more idiomatic and avoids verbose calls.
    #[lint(
        id = "prefer-object-spread",
        code = "LY048",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Object)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferObjectSpread,
    "Prefer object spread over Object.assign()"
}

impl LintRule for PreferObjectSpread {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferObjectSpread::meta()
    }

    /// Check module DIR nodes for Object.assign calls that should use spread.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferObjectSpreadVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags prefer-object-spread patterns.
struct PreferObjectSpreadVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Object symbol for this module.
    object_symbol: dir::GlobalSymbolId,
    /// The Object member name.
    object_name: StringId,
    /// The string id for the assign method name.
    assign_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferObjectSpreadVisitor<'a, 'b> {
    /// Build a visitor for prefer-object-spread checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let object_symbol = ctx.well_known_symbol(WellKnownSymbol::Object);
        let object_name = ctx.repository.strings.intern("Object");
        let assign_name = ctx.repository.strings.intern("assign");
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            object_symbol,
            object_name,
            assign_name,
            global_qualifiers,
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

    /// Check a call expression for Object.assign usage.
    fn check_assign_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        dynamic_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // ignore non object assign calls
        if !self.is_object_assign_receiver(left) {
            return;
        }

        // require at least one argument
        if dynamic_arguments.is_empty() {
            return;
        }

        // require an object literal as the first argument
        let first_argument_id = dynamic_arguments[0];
        let argument = self.ctx.tree.get(first_argument_id);
        let value_id = argument.value();
        if !self.is_object_literal(value_id) {
            return;
        }

        // ignore calls with spread arguments
        if self.has_spread_argument(dynamic_arguments) {
            return;
        }

        // ignore accessor merges with multiple arguments, spread can change getter and setter timing
        if dynamic_arguments.len() > 1 && self.has_accessor_object_argument(dynamic_arguments) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic and attach fix when safe
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_OBJECT_SPREAD.id,
            PREFER_OBJECT_SPREAD.code,
            PREFER_OBJECT_SPREAD.category,
            severity,
            "prefer object spread over Object.assign()",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use { ...value } instead of Object.assign");
        if let Some(fix) =
            self.object_spread_fix(expression_id, left, generic_arguments, dynamic_arguments)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix from Object.assign({}, ...) to object spread.
    fn object_spread_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        member_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        dynamic_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<LintFix> {
        // skip static call arguments until static argument rendering is supported
        if !generic_arguments.is_empty() {
            return None;
        }

        // skip static member arguments for Object.assign<T>(...)
        if let dir::Expression::Member {
            generic_arguments, ..
        } = self.ctx.tree.get(member_id)
            && !generic_arguments.is_empty()
        {
            return None;
        }

        // require at least one argument
        if dynamic_arguments.is_empty() {
            return None;
        }

        // build literal entries from positional arguments only
        let mut literal_entries = Vec::new();
        for argument_id in dynamic_arguments {
            let argument = self.ctx.tree.get(*argument_id);
            let dir::Argument::Positional { value, .. } = argument else {
                return None;
            };

            if let Some(object_literal_entries) = self.object_literal_entries(*value) {
                literal_entries.extend(object_literal_entries);
                continue;
            }

            let value_id = expression_unwrap_parenthesized(self.ctx.tree, *value);
            let value_span = self.ctx.get_span(value_id);
            let value_text = self.ctx.get_span_text(value_span);
            if self.spread_argument_needs_parentheses(value_id) {
                literal_entries.push(format!("...({value_text})"));
            } else {
                literal_entries.push(format!("...{value_text}"));
            }
        }

        // replace the full call with an object spread literal
        let replacement = if literal_entries.is_empty() {
            "{}".to_owned()
        } else {
            format!("{{ {} }}", literal_entries.join(", "))
        };
        let expression_span = self.ctx.get_span(expression_id);
        if span_has_comment(self.ctx.ast, expression_span) {
            return None;
        }

        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace Object.assign() with object spread").with_edits(edits))
    }

    /// Return the receiver expression for Object.assign calls.
    fn assign_receiver_expression(
        &self,
        member_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        // match member or static index access for assign
        let (receiver_id, property_name) =
            expression_static_property_access(self.ctx.tree, member_id)?;
        if property_name != self.assign_name {
            return None;
        };

        Some(receiver_id)
    }

    /// Return true when the expression is Object.assign or global qualified Object.assign.
    fn is_object_assign_receiver(&self, member_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the member receiver
        let Some(receiver_id) = self.assign_receiver_expression(member_id) else {
            return false;
        };

        // match direct Object symbol references
        if let Some(receiver_symbol) = expression_target_symbol(self.ctx.tree, receiver_id)
            && receiver_symbol == self.object_symbol
        {
            return true;
        }

        // match global qualified Object references
        expression_is_global_qualified_member(
            self.ctx.tree,
            receiver_id,
            &self.global_qualifiers,
            self.object_name,
        )
    }

    /// Return true when the expression is an object literal.
    fn is_object_literal(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // unwrap parenthesized expressions
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);

        // match object literals
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::ObjectExpression { .. } = expression else {
            return false;
        };

        true
    }

    /// Return true when call arguments include spread.
    fn has_spread_argument(&self, arguments: &[dir::LocalNodeId<dir::Argument>]) -> bool {
        arguments.iter().any(|argument_id| {
            let argument = self.ctx.tree.get(*argument_id);
            matches!(argument, dir::Argument::Spread { .. })
        })
    }

    /// Return true when one object argument contains accessors.
    fn has_accessor_object_argument(&self, arguments: &[dir::LocalNodeId<dir::Argument>]) -> bool {
        arguments.iter().any(|argument_id| {
            let argument = self.ctx.tree.get(*argument_id);
            let expression_id = argument.value();
            self.is_object_literal_with_accessors(expression_id)
        })
    }

    /// Return true when one object literal expression contains getter or setter properties.
    fn is_object_literal_with_accessors(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::ObjectExpression { ty: _, properties } = expression else {
            return false;
        };

        properties.iter().any(|property_id| {
            let property = self.ctx.tree.get(*property_id);
            let Property::Method { signature, .. } = property else {
                return false;
            };

            matches!(
                signature.mode,
                Some(FunctionMode::Getter | FunctionMode::Setter)
            )
        })
    }

    /// Return one flattened object literal entry list, or None when the expression is not object literal.
    fn object_literal_entries(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Vec<String>> {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::ObjectExpression { ty: _, properties } = expression else {
            return None;
        };

        let mut entries = Vec::new();
        for property_id in properties {
            let property_span = self.ctx.get_span(*property_id);
            let property_text = self.ctx.get_span_text(property_span);
            entries.push(property_text.to_owned());
        }

        Some(entries)
    }

    /// Return true when one spread argument expression needs parentheses to stay valid.
    fn spread_argument_needs_parentheses(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression = self.ctx.tree.get(expression_id);
        matches!(
            expression,
            dir::Expression::Assign { .. } | dir::Expression::AssignBinary { .. }
        ) || matches!(
            expression,
            dir::Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        )
    }
}

impl NodeVisitor for PreferObjectSpreadVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check Object.assign calls
        if let dir::Expression::Call {
            left,
            generic_arguments,
            dynamic_arguments,
            ..
        } = expression
        {
            self.check_assign_call(id, *left, generic_arguments.as_slice(), dynamic_arguments);
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
    fn test_flags_object_assign_with_empty_object() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_flags_object_assign_with_empty_object.ds",
            r#"
let base = { a: 1 };
let merged = Object.assign({}, base);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-spread")
            .assert_has_fix("prefer-object-spread")
            .assert_safe_fixed(
                r#"
let base = { a: 1 };
let merged = { ...base };
"#,
            );
    }

    /// Fix Object.assign with one object literal argument.
    #[test]
    fn test_fix_object_assign_single_object_literal() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_fix_object_assign_single_object_literal.ds",
            r#"
let merged = Object.assign({ a: 1, b: 2 });
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-spread")
            .assert_has_fix("prefer-object-spread")
            .assert_safe_fixed(
                r#"
let merged = { a: 1, b: 2 };
"#,
            );
    }

    /// Report global Object.assign with empty object.
    #[test]
    fn test_flags_global_object_assign_with_empty_object() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_flags_global_object_assign_with_empty_object.ds",
            r#"
let base = { a: 1 };
let merged = globalThis.Object.assign({}, base);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-spread")
            .assert_has_fix("prefer-object-spread")
            .assert_safe_fixed(
                r#"
let base = { a: 1 };
let merged = { ...base };
"#,
            );
    }

    /// Fix Object.assign with multiple source values.
    #[test]
    fn test_fix_object_assign_with_multiple_sources() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_fix_object_assign_with_multiple_sources.ds",
            r#"
let base = { a: 1 };
let extra = { b: 2 };
let merged = Object.assign({}, base, extra);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-spread")
            .assert_has_fix("prefer-object-spread")
            .assert_safe_fixed(
                r#"
let base = { a: 1 };
let extra = { b: 2 };
let merged = { ...base, ...extra };
"#,
            );
    }

    /// Ignore Object.assign calls with spread arguments.
    #[test]
    fn test_allows_spread_argument_call() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_allows_spread_argument_call.ds",
            r#"
let base = { a: 1 };
let sources = [base];
let merged = Object.assign({}, ...sources);
"#,
        );
        test.result(result).assert_no_lint("prefer-object-spread");
    }

    /// Ignore Object.assign with accessor object arguments.
    #[test]
    fn test_allows_object_assign_with_accessor_argument() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_allows_object_assign_with_accessor_argument.ds",
            r#"
let merged = Object.assign({}, {
  get value() {
    return 1;
  },
});
"#,
        );
        test.result(result).assert_no_lint("prefer-object-spread");
    }

    /// Fix Object.assign with non empty target and one source.
    #[test]
    fn test_fix_object_assign_with_non_empty_target() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_fix_object_assign_with_non_empty_target.ds",
            r#"
let base = { a: 1 };
let merged = Object.assign({ b: 2 }, base);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-spread")
            .assert_has_fix("prefer-object-spread")
            .assert_safe_fixed(
                r#"
let base = { a: 1 };
let merged = { b: 2, ...base };
"#,
            );
    }

    /// Fix computed Object.assign property access.
    #[test]
    fn test_fix_computed_object_assign() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_fix_computed_object_assign.ds",
            r#"
let base = { a: 1 };
let merged = Object["assign"]({}, base);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-spread")
            .assert_has_fix("prefer-object-spread")
            .assert_safe_fixed(
                r#"
let base = { a: 1 };
let merged = { ...base };
"#,
            );
    }

    /// Keep lint without fix when Object.assign call contains comments.
    #[test]
    fn test_no_fix_when_object_assign_contains_comments() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_no_fix_when_object_assign_contains_comments.ds",
            r#"
let base = { a: 1 };
let merged = Object.assign(
  {},
  /* spread source */ base,
);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-spread")
            .assert_has_no_fix("prefer-object-spread");
    }

    /// Parenthesize assignment sources when converting to spread entries.
    #[test]
    fn test_fix_wraps_assignment_source_in_spread() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectSpread);
        let result = test.lint_dir(
            "prefer_object_spread/test_fix_wraps_assignment_source_in_spread.ds",
            r#"
let base = { a: 1 };
let other = { b: 2 };
let merged = Object.assign({}, base = other);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-spread")
            .assert_has_fix("prefer-object-spread")
            .assert_safe_fixed(
                r#"
let base = { a: 1 };
let other = { b: 2 };
let merged = { ...(base = other) };
"#,
            );
    }
}
