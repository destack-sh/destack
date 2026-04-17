use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_is_symbol_or_global_qualified_member, expression_static_property_access,
    expression_target_symbol, expression_unwrap_parenthesized, expression_unwrap_statement,
    expressions_have_equivalent_source_form, pattern_binding_name_and_symbol,
    positional_argument_value,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Require guard in for-in loops.
    ///
    /// For-in loops iterate over all enumerable properties including inherited ones.
    /// Use hasOwnProperty or Object.hasOwn to guard against inherited properties.
    #[lint(
        id = "guard-for-in",
        code = "LU001",
        category = Suspicious,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Object)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub GuardForIn,
    "Require guard in for-in loops"
}

impl LintRule for GuardForIn {
    fn meta(&self) -> &'static LintMeta {
        GuardForIn::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = GuardForInVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that checks for-in loops for real own-property guards.
struct GuardForInVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Object symbol.
    object_symbol: dir::GlobalSymbolId,
    /// Global qualifier symbols like `globalThis`.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The `Object` member name.
    object_name: StringId,
    /// The `hasOwn` member name.
    has_own_name: StringId,
    /// The `prototype` member name.
    prototype_name: StringId,
    /// The `hasOwnProperty` member name.
    has_own_property_name: StringId,
    /// The `call` member name.
    call_name: StringId,
    /// Visitor options.
    options: NodeVisitorOptions,
}

/// The guard polarity used by the first loop condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuardPolarity {
    /// `if (Object.hasOwn(obj, key)) { ... }`
    Positive,
    /// `if (!Object.hasOwn(obj, key)) continue;`
    Negative,
}

impl<'a, 'b> GuardForInVisitor<'a, 'b> {
    /// Build a visitor for guard-for-in checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let object_symbol = ctx.well_known_symbol(WellKnownSymbol::Object);
        let global_qualifiers = ctx.global_qualifier_symbols();
        let object_name = ctx.repository.strings.intern("Object");
        let has_own_name = ctx.repository.strings.intern("hasOwn");
        let prototype_name = ctx.repository.strings.intern("prototype");
        let has_own_property_name = ctx.repository.strings.intern("hasOwnProperty");
        let call_name = ctx.repository.strings.intern("call");

        Self {
            ctx,
            meta,
            object_symbol,
            global_qualifiers,
            object_name,
            has_own_name,
            prototype_name,
            has_own_property_name,
            call_name,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module DIR roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one for-in loop.
    fn check_for_in(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        binding: &dir::ForEachBinding,
        iterator_id: dir::LocalNodeId<dir::Expression>,
        body_id: dir::LocalNodeId<dir::Block>,
    ) {
        let body = self.ctx.tree.get(body_id);

        // empty loop bodies are allowed
        if body.is_empty() {
            return;
        }

        // only simple key bindings can participate in real guard checks or fixes
        let binding_symbol = for_in_binding_symbol(self.ctx.tree, binding)
            .map(|symbol| symbol.into_global(self.ctx.module_id()));
        let has_guard = binding_symbol.is_some_and(|binding_symbol| {
            self.starts_with_own_property_guard(
                iterator_id,
                binding_symbol,
                body.first_expression().unwrap(),
            )
        });
        if has_guard {
            return;
        }

        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let mut diagnostic = LintDiagnostic::new(
            GUARD_FOR_IN.id,
            GUARD_FOR_IN.code,
            GUARD_FOR_IN.category,
            severity,
            "for-in loop should have a real own-property guard",
            self.ctx.module.file_id,
            self.ctx.get_span(expression_id),
        )
        .with_label("add an own-property guard like Object.hasOwn(value, key)");

        if self.ctx.include_fixes
            && let Some(fix) = guard_for_in_fix(self.ctx, binding, iterator_id, body_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return true when the first loop statement is a real own-property guard.
    fn starts_with_own_property_guard(
        &mut self,
        iterator_id: dir::LocalNodeId<dir::Expression>,
        binding_symbol: dir::GlobalSymbolId,
        first_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let first_expression_id = expression_unwrap_statement(self.ctx.tree, first_expression_id);
        let dir::Expression::If {
            condition: dir::IfCondition::Expression { condition },
            then_expression,
            else_expression: _,
            ..
        } = self.ctx.tree.get(first_expression_id)
        else {
            return false;
        };

        match self.guard_condition_polarity(*condition, iterator_id, binding_symbol) {
            Some(GuardPolarity::Positive) => true,
            Some(GuardPolarity::Negative) => self.then_expression_is_continue(*then_expression),
            None => false,
        }
    }

    /// Return the own-property guard polarity for one condition when present.
    fn guard_condition_polarity(
        &mut self,
        condition_id: dir::LocalNodeId<dir::Expression>,
        iterator_id: dir::LocalNodeId<dir::Expression>,
        binding_symbol: dir::GlobalSymbolId,
    ) -> Option<GuardPolarity> {
        let condition_id = expression_unwrap_parenthesized(self.ctx.tree, condition_id);
        let condition = self.ctx.tree.get(condition_id);

        // positive guard
        if self.is_own_property_guard_call(condition_id, iterator_id, binding_symbol) {
            return Some(GuardPolarity::Positive);
        }

        // negated guard
        let dir::Expression::Unary {
            operator: dir::UnaryOperator::Not,
            right,
        } = condition
        else {
            return None;
        };

        self.is_own_property_guard_call(*right, iterator_id, binding_symbol)
            .then_some(GuardPolarity::Negative)
    }

    /// Return true when one call expression is a supported own-property guard form.
    fn is_own_property_guard_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        iterator_id: dir::LocalNodeId<dir::Expression>,
        binding_symbol: dir::GlobalSymbolId,
    ) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let dir::Expression::Call {
            left, arguments, ..
        } = self.ctx.tree.get(expression_id)
        else {
            return false;
        };

        self.is_object_has_own_call(*left, arguments, iterator_id, binding_symbol)
            || self.is_receiver_has_own_property_call(*left, arguments, iterator_id, binding_symbol)
            || self.is_object_prototype_has_own_property_call(
                *left,
                arguments,
                iterator_id,
                binding_symbol,
            )
    }

    /// Return true when one expression resolves to the built in Object value.
    fn is_object_reference(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol_or_global_qualified_member(
            self.ctx.tree,
            expression_id,
            self.object_symbol,
            &self.global_qualifiers,
            self.object_name,
        )
    }

    /// Return true when `callee(arguments...)` matches `Object.hasOwn(iterator, key)`.
    fn is_object_has_own_call(
        &mut self,
        callee_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        iterator_id: dir::LocalNodeId<dir::Expression>,
        binding_symbol: dir::GlobalSymbolId,
    ) -> bool {
        let Some((receiver_id, property_name)) =
            expression_static_property_access(self.ctx.tree, callee_id)
        else {
            return false;
        };
        if property_name != self.has_own_name || !self.is_object_reference(receiver_id) {
            return false;
        }

        let Some(object_argument_id) = positional_argument_value(self.ctx.tree, arguments, 0)
        else {
            return false;
        };
        let Some(key_argument_id) = positional_argument_value(self.ctx.tree, arguments, 1) else {
            return false;
        };

        self.iterator_matches(object_argument_id, iterator_id)
            && self.binding_matches(key_argument_id, binding_symbol)
    }

    /// Return true when `callee(arguments...)` matches `iterator.hasOwnProperty(key)`.
    fn is_receiver_has_own_property_call(
        &mut self,
        callee_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        iterator_id: dir::LocalNodeId<dir::Expression>,
        binding_symbol: dir::GlobalSymbolId,
    ) -> bool {
        let Some((receiver_id, property_name)) =
            expression_static_property_access(self.ctx.tree, callee_id)
        else {
            return false;
        };
        if property_name != self.has_own_property_name {
            return false;
        }

        let Some(key_argument_id) = positional_argument_value(self.ctx.tree, arguments, 0) else {
            return false;
        };

        self.iterator_matches(receiver_id, iterator_id)
            && self.binding_matches(key_argument_id, binding_symbol)
    }

    /// Return true when `callee(arguments...)` matches
    /// `Object.prototype.hasOwnProperty.call(iterator, key)`.
    fn is_object_prototype_has_own_property_call(
        &mut self,
        callee_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        iterator_id: dir::LocalNodeId<dir::Expression>,
        binding_symbol: dir::GlobalSymbolId,
    ) -> bool {
        let Some((has_own_property_call_id, property_name)) =
            expression_static_property_access(self.ctx.tree, callee_id)
        else {
            return false;
        };
        if property_name != self.call_name {
            return false;
        }

        let Some((prototype_id, has_own_property_name)) =
            expression_static_property_access(self.ctx.tree, has_own_property_call_id)
        else {
            return false;
        };
        if has_own_property_name != self.has_own_property_name {
            return false;
        }

        let Some((object_id, prototype_name)) =
            expression_static_property_access(self.ctx.tree, prototype_id)
        else {
            return false;
        };
        if prototype_name != self.prototype_name || !self.is_object_reference(object_id) {
            return false;
        }

        let Some(object_argument_id) = positional_argument_value(self.ctx.tree, arguments, 0)
        else {
            return false;
        };
        let Some(key_argument_id) = positional_argument_value(self.ctx.tree, arguments, 1) else {
            return false;
        };

        self.iterator_matches(object_argument_id, iterator_id)
            && self.binding_matches(key_argument_id, binding_symbol)
    }

    /// Return true when one expression matches the loop iterator.
    fn iterator_matches(
        &mut self,
        candidate_id: dir::LocalNodeId<dir::Expression>,
        iterator_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        expressions_have_equivalent_source_form(self.ctx, candidate_id, iterator_id)
    }

    /// Return true when one expression resolves to the loop binding symbol.
    fn binding_matches(
        &self,
        candidate_id: dir::LocalNodeId<dir::Expression>,
        binding_symbol: dir::GlobalSymbolId,
    ) -> bool {
        expression_target_symbol(self.ctx.tree, candidate_id) == Some(binding_symbol)
    }

    /// Return true when the if consequent is just `continue`.
    fn then_expression_is_continue(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression_id = expression_unwrap_statement(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);

        if matches!(expression, dir::Expression::Continue { .. }) {
            return true;
        }

        let dir::Expression::Block(block) = expression else {
            return false;
        };
        let block = self.ctx.tree.get(*block);
        if block.len() != 1 {
            return false;
        }

        let continue_id =
            expression_unwrap_statement(self.ctx.tree, block.first_expression().unwrap());
        matches!(
            self.ctx.tree.get(continue_id),
            dir::Expression::Continue { .. }
        )
    }
}

impl NodeVisitor for GuardForInVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if let dir::Expression::ForEach {
            kind: dir::ForEachKind::In,
            binding,
            iterator,
            body,
            ..
        } = expression
        {
            self.check_for_in(id, binding, *iterator, *body);
        }

        walk_expression(self, tree, id, expression);
    }
}

/// Return one global loop binding symbol when the for-in binding is a simple identifier.
fn for_in_binding_symbol(
    tree: &dir::NodeTree,
    binding: &dir::ForEachBinding,
) -> Option<dir::LocalSymbolId> {
    let pattern_id = match binding {
        dir::ForEachBinding::Pattern { pattern, .. } => *pattern,
        dir::ForEachBinding::Using { pattern, .. } => *pattern,
    };

    let (_, symbol_id) = pattern_binding_name_and_symbol(tree, pattern_id)?;
    Some(symbol_id)
}

/// Return the bound key name when the for-in binding is a simple identifier.
fn for_in_binding_name(
    ctx: &LintModuleDirContext<'_>,
    binding: &dir::ForEachBinding,
) -> Option<String> {
    let pattern_id = match binding {
        dir::ForEachBinding::Pattern { pattern, .. } => *pattern,
        dir::ForEachBinding::Using { pattern, .. } => *pattern,
    };

    let (name_id, _) = pattern_binding_name_and_symbol(ctx.tree, pattern_id)?;
    Some(ctx.repository.strings.get(name_id).to_string())
}

/// Build an unsafe fix that wraps the body with Object.hasOwn guard.
fn guard_for_in_fix(
    ctx: &LintModuleDirContext<'_>,
    binding: &dir::ForEachBinding,
    iterator_expression_id: dir::LocalNodeId<dir::Expression>,
    body_id: dir::LocalNodeId<dir::Block>,
) -> Option<LintFix> {
    let key_name = for_in_binding_name(ctx, binding)?;
    let iterator_text = side_effect_free_iterator_text(ctx, iterator_expression_id)?;
    let body = ctx.tree.get(body_id);
    let inner_text = block_inner_text(ctx, body)?;
    let replacement =
        format!("{{ if (Object.hasOwn({iterator_text}, {key_name})) {{\n{inner_text}\n}} }}");

    let body_span = ctx.get_span(body_id);
    let edits = ctx
        .edit_builder()
        .replace(body_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Wrap for-in body with Object.hasOwn guard").with_edits(edits))
}

/// Return iterator text only when it is side effect free.
fn side_effect_free_iterator_text(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<String> {
    let expression_id = expression_unwrap_parenthesized(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);
    if !matches!(
        expression,
        dir::Expression::LocalReference {
            generic_arguments,
            ..
        } | dir::Expression::ModuleReference {
            generic_arguments,
            ..
        } | dir::Expression::GlobalReference {
            generic_arguments,
            ..
        } if generic_arguments.is_empty()
    ) {
        return None;
    }

    Some(ctx.get_span_text(ctx.get_span(expression_id)).to_string())
}

/// Return a block's inner source text for fix rewriting.
fn block_inner_text(ctx: &LintModuleDirContext<'_>, block: &dir::Block) -> Option<String> {
    let first_expression_id = block.first_expression()?;
    let last_expression_id = block.last_expression()?;
    let first_span = ctx.get_span(first_expression_id);
    let last_span = ctx.get_span(last_expression_id);
    let inner_span = destack_source::Span::new(first_span.file, first_span.start, last_span.end);
    Some(ctx.get_span_text(inner_span).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_unguarded_for_in_detected() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_unguarded_for_in_detected.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {
        console.log(key)
    }
}
"#,
        );
        test.result(result)
            .assert_lint("guard-for-in")
            .assert_unsafe_fixed(
                r#"
function foo(obj: object) {
    for (const key in obj) {
        if (Object.hasOwn(obj, key)) {
            console.log(key)
        }
    }
}
"#,
            );
    }

    #[test]
    fn test_guarded_for_in_allowed() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_guarded_for_in_allowed.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {
        if (obj.hasOwnProperty(key)) {
            console.log(key)
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_object_has_own_guard_allowed() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_object_has_own_guard_allowed.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {
        if (Object.hasOwn(obj, key)) {
            console.log(key)
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_prototype_has_own_call_guard_allowed() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_prototype_has_own_call_guard_allowed.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {
        if (Object.prototype.hasOwnProperty.call(obj, key)) {
            console.log(key)
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_negative_continue_guard_allowed() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_negative_continue_guard_allowed.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {
        if (!Object.hasOwn(obj, key)) continue
        console.log(key)
    }
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_shadowed_object_has_own_does_not_count() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_shadowed_object_has_own_does_not_count.ds",
            r#"
function foo(obj: object, Object: { hasOwn: (value: object, key: string) => boolean }) {
    for (const key in obj) {
        if (Object.hasOwn(obj, key)) {
            console.log(key)
        }
    }
}
"#,
        );
        test.result(result).assert_lint("guard-for-in");
    }

    #[test]
    fn test_unrelated_if_does_not_count_as_guard() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_unrelated_if_does_not_count_as_guard.ds",
            r#"
function foo(obj: object, ready: boolean) {
    for (const key in obj) {
        if (ready) {
            console.log(key)
        }
    }
}
"#,
        );
        test.result(result).assert_lint("guard-for-in");
    }

    #[test]
    fn test_for_of_not_affected() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_for_of_not_affected.ds",
            r#"
function foo(arr: int32[]) {
    for (const item of arr) {
        console.log(item)
    }
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_empty_for_in_allowed() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_empty_for_in_allowed.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {}
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_no_fix_when_iterator_has_side_effects() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_dir(
            "guard_for_in/test_no_fix_when_iterator_has_side_effects.ds",
            r#"
function foo() {
    for (const key in getObject()) {
        console.log(key)
    }
}
"#,
        );
        test.result(result)
            .assert_lint("guard-for-in")
            .assert_has_no_fix("guard-for-in");
    }
}
