use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_is_symbol_or_global_qualified_member, expression_unwrap_parenthesized,
    positional_argument_value,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `Object.hasOwn()` over prototype `hasOwnProperty` chains.
    ///
    /// `Object.hasOwn(value, key)` is clearer and avoids brittle call chains.
    #[lint(
        id = "prefer-object-has-own",
        code = "LY069",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Object)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferObjectHasOwn,
    "Prefer Object.hasOwn over Object.prototype.hasOwnProperty.call"
}

impl LintRule for PreferObjectHasOwn {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferObjectHasOwn::meta()
    }

    /// Check module DIR nodes for hasOwnProperty call chains.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferObjectHasOwnVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags prototype hasOwnProperty call chains.
struct PreferObjectHasOwnVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Object symbol for this module.
    object_symbol: dir::GlobalSymbolId,
    /// The Object member name.
    object_name: StringId,
    /// The prototype member name.
    prototype_name: StringId,
    /// The hasOwnProperty member name.
    has_own_property_name: StringId,
    /// The call member name.
    call_name: StringId,
    /// The apply member name.
    apply_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

/// The invocation form used on `hasOwnProperty`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HasOwnInvocationKind {
    /// `Object.prototype.hasOwnProperty.call(...)`.
    Call,
    /// `Object.prototype.hasOwnProperty.apply(...)`.
    Apply,
}

impl<'a, 'b> PreferObjectHasOwnVisitor<'a, 'b> {
    /// Build a visitor for prefer-object-has-own checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let object_symbol = ctx.well_known_symbol(WellKnownSymbol::Object);
        let object_name = ctx.string_id("Object");
        let prototype_name = ctx.string_id("prototype");
        let has_own_property_name = ctx.string_id("hasOwnProperty");
        let call_name = ctx.string_id("call");
        let apply_name = ctx.string_id("apply");
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            object_symbol,
            object_name,
            prototype_name,
            has_own_property_name,
            call_name,
            apply_name,
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

    /// Return true when the expression resolves to the built in Object symbol.
    fn is_object_reference(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol_or_global_qualified_member(
            self.ctx.tree,
            expression_id,
            self.object_symbol,
            &self.global_qualifiers,
            self.object_name,
        )
    }

    /// Return invocation kind when `callee_id` matches
    /// `Object.prototype.hasOwnProperty.call` or `.apply`.
    fn prototype_has_own_invocation_kind(
        &self,
        callee_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<HasOwnInvocationKind> {
        let callee_id = expression_unwrap_parenthesized(self.ctx.tree, callee_id);

        // resolve `.call` or `.apply`
        let call_expression = self.ctx.tree.get(callee_id);
        let dir::Expression::Member {
            left: has_own_property_id,
            name: call_kind_name,
            ..
        } = call_expression
        else {
            return None;
        };
        let invocation_kind = if *call_kind_name == Some(self.call_name) {
            HasOwnInvocationKind::Call
        } else if *call_kind_name == Some(self.apply_name) {
            HasOwnInvocationKind::Apply
        } else {
            return None;
        };

        // resolve `.hasOwnProperty`
        let has_own_property_id =
            expression_unwrap_parenthesized(self.ctx.tree, *has_own_property_id);
        let has_own_property_expression = self.ctx.tree.get(has_own_property_id);
        let dir::Expression::Member {
            left: prototype_id,
            name: has_own_property_name,
            ..
        } = has_own_property_expression
        else {
            return None;
        };
        if *has_own_property_name != Some(self.has_own_property_name) {
            return None;
        }

        // resolve `.prototype`
        let prototype_id = expression_unwrap_parenthesized(self.ctx.tree, *prototype_id);
        let prototype_expression = self.ctx.tree.get(prototype_id);
        let dir::Expression::Member {
            left: object_id,
            name: prototype_name,
            ..
        } = prototype_expression
        else {
            return None;
        };
        if *prototype_name != Some(self.prototype_name) {
            return None;
        }

        self.is_object_reference(*object_id)
            .then_some(invocation_kind)
    }

    /// Check one call expression for prototype hasOwnProperty chains.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        let Some(invocation_kind) = self.prototype_has_own_invocation_kind(callee_id) else {
            return;
        };

        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_OBJECT_HAS_OWN.id,
            PREFER_OBJECT_HAS_OWN.code,
            PREFER_OBJECT_HAS_OWN.category,
            severity,
            "prefer Object.hasOwn()",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use Object.hasOwn(value, key) instead of prototype hasOwnProperty call");

        if let Some(fix) = self.prototype_has_own_fix(expression_id, invocation_kind, arguments) {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix to rewrite a prototype hasOwnProperty chain.
    fn prototype_has_own_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        invocation_kind: HasOwnInvocationKind,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<LintFix> {
        // resolve the object and key arguments from the invocation style
        let (object_value_id, key_value_id) = match invocation_kind {
            HasOwnInvocationKind::Call => {
                let object_id = positional_argument_value(self.ctx.tree, arguments, 0)?;
                let key_id = positional_argument_value(self.ctx.tree, arguments, 1)?;
                (object_id, key_id)
            }
            HasOwnInvocationKind::Apply => {
                let object_id = positional_argument_value(self.ctx.tree, arguments, 0)?;
                let args_array_id = positional_argument_value(self.ctx.tree, arguments, 1)?;
                let key_id = array_first_argument_value(self.ctx.tree, args_array_id)?;
                (object_id, key_id)
            }
        };

        // build `Object.hasOwn(object, key)` replacement text
        let expression_span = self.ctx.get_span(expression_id);
        let object_span = self.ctx.get_span(object_value_id);
        let object_text = self.ctx.get_span_text(object_span);
        let key_span = self.ctx.get_span(key_value_id);
        let key_text = self.ctx.get_span_text(key_span);
        let replacement = format!("Object.hasOwn({object_text}, {key_text})");
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Use Object.hasOwn()").with_edits(edits))
    }
}

/// Return the first positional value from an array expression argument list.
fn array_first_argument_value(
    tree: &dir::Tree,
    array_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let array_expression = tree.get(array_expression_id);
    let dir::Expression::ArrayExpression { elements } = array_expression else {
        return None;
    };
    let first_element_id = elements.first()?;
    let first_element = tree.get(*first_element_id);
    let dir::Argument::Positional { value, .. } = first_element else {
        return None;
    };

    Some(*value)
}

impl NodeVisitor for PreferObjectHasOwnVisitor<'_, '_> {
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
        if let dir::Expression::Call {
            left, arguments, ..
        } = expression
        {
            self.check_call(id, *left, arguments);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report Object.prototype.hasOwnProperty.call chains.
    #[test]
    fn test_flags_object_prototype_call_chain() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_flags_object_prototype_call_chain.ds",
            r#"
let value = { key: 1 };
let has_key = Object.prototype.hasOwnProperty.call(value, "key");
"#,
        );
        test.result(result).assert_lint("prefer-object-has-own");
    }

    /// Safely rewrite Object.prototype.hasOwnProperty.call chains.
    #[test]
    fn test_fix_object_prototype_call_chain() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_fix_object_prototype_call_chain.ds",
            r#"
let value = { key: 1 };
let has_key = Object.prototype.hasOwnProperty.call(value, "key");
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-has-own")
            .assert_has_fix("prefer-object-has-own")
            .assert_safe_fixed(
                r#"
let value = { key: 1 };
let has_key = Object.hasOwn(value, "key");
"#,
            );
    }

    /// Report Object.prototype.hasOwnProperty.apply chains.
    #[test]
    fn test_flags_object_prototype_apply_chain() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_flags_object_prototype_apply_chain.ds",
            r#"
let value = { key: 1 };
let has_key = Object.prototype.hasOwnProperty.apply(value, ["key"]);
"#,
        );
        test.result(result).assert_lint("prefer-object-has-own");
    }

    /// Safely rewrite Object.prototype.hasOwnProperty.apply chains with literal arrays.
    #[test]
    fn test_fix_object_prototype_apply_chain_with_literal_array() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_fix_object_prototype_apply_chain_with_literal_array.ds",
            r#"
let value = { key: 1 };
let has_key = Object.prototype.hasOwnProperty.apply(value, ["key"]);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-has-own")
            .assert_has_fix("prefer-object-has-own")
            .assert_safe_fixed(
                r#"
let value = { key: 1 };
let has_key = Object.hasOwn(value, "key");
"#,
            );
    }

    /// Report global Object prototype call chains.
    #[test]
    fn test_flags_global_object_prototype_call_chain() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_flags_global_object_prototype_call_chain.ds",
            r#"
let value = { key: 1 };
let has_key = globalThis.Object.prototype.hasOwnProperty.call(value, "key");
"#,
        );
        test.result(result).assert_lint("prefer-object-has-own");
    }

    /// Allow Object.hasOwn calls.
    #[test]
    fn test_allows_object_has_own() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_allows_object_has_own.ds",
            r#"
let value = { key: 1 };
let has_key = Object.hasOwn(value, "key");
"#,
        );
        test.result(result).assert_no_lint("prefer-object-has-own");
    }

    /// Allow unrelated call chains.
    #[test]
    fn test_allows_other_member_call_chains() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_allows_other_member_call_chains.ds",
            r#"
let value = { key: 1 };
let has_key = value.hasOwnProperty("key");
"#,
        );
        test.result(result).assert_no_lint("prefer-object-has-own");
    }

    /// Allow local Object symbols that shadow the global Object constructor.
    #[test]
    fn test_allows_shadowed_object_symbol() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_allows_shadowed_object_symbol.ds",
            r#"
let Object = {
    prototype: {
        hasOwnProperty: {
            call: (value: unknown, key: string): boolean => true,
        },
    },
};

let has_key = Object.prototype.hasOwnProperty.call({}, "key");
"#,
        );
        test.result(result).assert_no_lint("prefer-object-has-own");
    }

    /// Report parenthesized Object prototype call chains.
    #[test]
    fn test_flags_parenthesized_object_prototype_call_chain() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_flags_parenthesized_object_prototype_call_chain.ds",
            r#"
let value = { key: 1 };
let has_key = (Object.prototype.hasOwnProperty).call(value, "key");
"#,
        );
        test.result(result).assert_lint("prefer-object-has-own");
    }

    /// Allow hasOwnProperty call chain references without invocation.
    #[test]
    fn test_allows_has_own_property_chain_reference() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_allows_has_own_property_chain_reference.ds",
            r#"
let call_member = Object.prototype.hasOwnProperty.call;
call_member;
"#,
        );
        test.result(result).assert_no_lint("prefer-object-has-own");
    }

    /// Allow shadowed globalThis chains.
    #[test]
    fn test_allows_shadowed_global_this_object_chain() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_allows_shadowed_global_this_object_chain.ds",
            r#"
let globalThis = {
    Object: {
        prototype: {
            hasOwnProperty: {
                call: (value: unknown, key: string): boolean => true,
            },
        },
    },
};

let has_key = globalThis.Object.prototype.hasOwnProperty.call({}, "key");
"#,
        );
        test.result(result).assert_no_lint("prefer-object-has-own");
    }

    /// Allow non call and apply hasOwnProperty member access.
    #[test]
    fn test_allows_non_call_apply_chain() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_allows_non_call_apply_chain.ds",
            r#"
let member = Object.prototype.hasOwnProperty;
member;
"#,
        );
        test.result(result).assert_no_lint("prefer-object-has-own");
    }

    /// Do not auto fix apply chains with non literal argument arrays.
    #[test]
    fn test_no_fix_for_apply_chain_with_dynamic_array() {
        let test = TestProgram::for_rule_with_prelude(PreferObjectHasOwn);
        let result = test.lint_dir(
            "prefer_object_has_own/test_no_fix_for_apply_chain_with_dynamic_array.ds",
            r#"
let value = { key: 1 };
let args = ["key"];
let has_key = Object.prototype.hasOwnProperty.apply(value, args);
"#,
        );
        test.result(result)
            .assert_lint("prefer-object-has-own")
            .assert_has_no_fix("prefer-object-has-own");
    }
}
