use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_is_global_qualified_member, expression_target_symbol, global_qualifier_symbols,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow using the Object constructor.
    ///
    /// Object constructors are verbose and can hide intent compared to
    /// object literals.
    #[lint(
        id = "no-object-constructor",
        code = "LY031",
        category = Style,
        level = Dir,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoObjectConstructor,
    "Disallow Object constructor usage"
}

impl LintRule for NoObjectConstructor {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoObjectConstructor::meta()
    }

    /// Check module DIR nodes for Object constructor calls.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for object constructor calls
        let mut visitor = ObjectConstructorVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags Object constructor calls.
struct ObjectConstructorVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Object symbol for this module.
    object_symbol: Option<dir::GlobalSymbolId>,
    /// The Object member name.
    object_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ObjectConstructorVisitor<'a, 'b> {
    /// Build a visitor for object constructor checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // resolve the well known Object symbol for this module
        let object_symbol = ctx.get_well_known_symbol(WellKnownSymbol::Object);
        let object_name = ctx.program.strings.intern("Object");
        let global_qualifiers = global_qualifier_symbols(ctx);

        // prepare visitor state
        Self {
            ctx,
            meta,
            object_symbol,
            object_name,
            global_qualifiers,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // skip when no Object symbol is available
        if self.object_symbol.is_none() {
            return;
        }

        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // walk the module expression tree
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call or constructor for Object usage.
    fn check_object_constructor(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        kind: &'static str,
    ) {
        // ignore non object references
        if !self.is_object_reference(left) {
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
                NO_OBJECT_CONSTRUCTOR.id,
                NO_OBJECT_CONSTRUCTOR.code,
                NO_OBJECT_CONSTRUCTOR.category,
                severity,
                "avoid using the Object constructor",
                self.ctx.module.file_id,
                span,
            )
            .with_label(format!("replace this {kind} with an object literal")),
        );
    }

    /// Return true when the expression is a reference to Object.
    fn is_object_reference(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match direct symbol references
        if let Some(object_symbol) = self.object_symbol
            && expression_target_symbol(self.ctx.tree, expression_id) == Some(object_symbol)
        {
            return true;
        }

        // match global qualified references
        expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.object_name,
        )
    }
}

impl NodeVisitor for ObjectConstructorVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for object constructor calls
        if let Some((kind, left)) = object_constructor_reference(expression) {
            self.check_object_constructor(id, left, kind);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Identify Object constructor call forms.
fn object_constructor_reference(
    expression: &dir::Expression,
) -> Option<(&'static str, dir::LocalNodeId<dir::Expression>)> {
    // match call and constructor expressions
    match expression {
        dir::Expression::Call { left, .. } => Some(("call", *left)),
        dir::Expression::New { left, .. } => Some(("constructor", *left)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LintLevel;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_object_constructor_call() {
        let test = TestProgram::for_rule_with_builtins(NoObjectConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let value = Object();
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("no-object-constructor");
    }

    #[test]
    fn test_flags_object_constructor_new() {
        let test = TestProgram::for_rule_with_builtins(NoObjectConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let value = new Object();
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("no-object-constructor");
    }

    /// Report global Object constructor calls.
    #[test]
    fn test_flags_global_object_constructor() {
        let test = TestProgram::for_rule_with_builtins(NoObjectConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let value = globalThis.Object();
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("no-object-constructor");
    }

    #[test]
    fn test_allows_object_literal() {
        let test = TestProgram::for_rule_with_builtins(NoObjectConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let value = { key: "value" };
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_no_lint("no-object-constructor");
    }
}
