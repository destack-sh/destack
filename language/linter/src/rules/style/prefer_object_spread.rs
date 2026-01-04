use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_is_global_qualified_member, expression_target_symbol, global_qualifier_symbols,
    unwrap_parenthesized_expression,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer object spread over `Object.assign()`.
    ///
    /// Object spread is more idiomatic and avoids verbose calls.
    #[lint(
        id = "prefer-object-spread",
        code = "LY055",
        category = Style,
        level = Dir,
        fixable = No,
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
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for object spread opportunities
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
    object_symbol: Option<dir::GlobalSymbolId>,
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
        // resolve the well known Object symbol for this module
        let object_symbol = ctx.get_well_known_symbol(WellKnownSymbol::Object);

        // intern commonly used names
        let object_name = ctx.program.strings.intern("Object");
        let assign_name = ctx.program.strings.intern("assign");
        let global_qualifiers = global_qualifier_symbols(ctx);

        // prepare visitor state
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
        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // walk the module expression tree
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
        dynamic_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // ignore non object assign calls
        if !self.is_object_assign_receiver(left) {
            return;
        }

        // require at least two arguments
        if dynamic_arguments.len() < 2 {
            return;
        }

        // require an empty object literal as the first argument
        let first_argument_id = dynamic_arguments[0];
        let argument = self.ctx.tree.get(first_argument_id);
        let value_id = argument.value();
        if !self.is_empty_object_literal(value_id) {
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
                PREFER_OBJECT_SPREAD.id,
                PREFER_OBJECT_SPREAD.code,
                PREFER_OBJECT_SPREAD.category,
                severity,
                "prefer object spread over Object.assign()",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use { ...value } instead of Object.assign"),
        );
    }

    /// Return the receiver expression for Object.assign calls.
    fn assign_receiver_expression(
        &self,
        member_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        // match member access for assign
        let member_expression = self.ctx.tree.get(member_id);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return None;
        };
        if *name != self.assign_name {
            return None;
        }

        Some(*left)
    }

    /// Return true when the expression is Object.assign or global qualified Object.assign.
    fn is_object_assign_receiver(&self, member_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the member receiver
        let Some(receiver_id) = self.assign_receiver_expression(member_id) else {
            return false;
        };

        // match direct Object symbol references
        if let Some(receiver_symbol) = expression_target_symbol(self.ctx.tree, receiver_id)
            && self
                .object_symbol
                .is_some_and(|object_symbol| object_symbol == receiver_symbol)
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

    /// Return true when the expression is an empty object literal.
    fn is_empty_object_literal(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // unwrap parenthesized expressions
        let expression_id = unwrap_parenthesized_expression(self.ctx.tree, expression_id);

        // match empty object literals
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::ObjectExpression { properties } = expression else {
            return false;
        };

        properties.is_empty()
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
            dynamic_arguments,
            ..
        } = expression
        {
            self.check_assign_call(id, *left, dynamic_arguments);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LintLevel;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_object_assign_with_empty_object() {
        let test = TestProgram::for_rule_with_builtins(PreferObjectSpread);
        let result = test.lint(
            "test.ds",
            r#"
let base = { a: 1 };
let merged = Object.assign({}, base);
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("prefer-object-spread");
    }

    /// Report global Object.assign with empty object.
    #[test]
    fn test_flags_global_object_assign_with_empty_object() {
        let test = TestProgram::for_rule_with_builtins(PreferObjectSpread);
        let result = test.lint(
            "test.ds",
            r#"
let base = { a: 1 };
let merged = globalThis.Object.assign({}, base);
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("prefer-object-spread");
    }

    #[test]
    fn test_allows_object_assign_with_non_empty_target() {
        let test = TestProgram::for_rule_with_builtins(PreferObjectSpread);
        let result = test.lint(
            "test.ds",
            r#"
let base = { a: 1 };
let merged = Object.assign({ b: 2 }, base);
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_no_lint("prefer-object-spread");
    }
}
