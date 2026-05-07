use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    CallLikeExpressionInfo, expression_call_like, expression_is_standalone_statement,
    expression_is_symbol_or_global_qualified_member,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow using the Object constructor.
    ///
    /// Object constructors are verbose and can hide intent compared to
    /// object literals.
    #[lint(
        id = "no-object-constructor",
        code = "LY023",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Object)],
        requires_any = [],
        fixable = Sometimes,
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
        let meta = self.meta();
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
    object_symbol: dir::GlobalSymbolId,
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
        let object_symbol = ctx.well_known_symbol(WellKnownSymbol::Object);
        let object_name = ctx.string_id("Object");
        let global_qualifiers = ctx.global_qualifier_symbols();

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
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call or constructor for Object usage.
    fn check_object_constructor(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        call_like: CallLikeExpressionInfo<'_>,
    ) {
        // ignore non object references
        if !self.is_object_reference(call_like.left) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build diagnostic and attach fix when safe
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            NO_OBJECT_CONSTRUCTOR.id,
            NO_OBJECT_CONSTRUCTOR.code,
            NO_OBJECT_CONSTRUCTOR.category,
            severity,
            "avoid using the Object constructor",
            span,
        )
        .label(format!(
            "replace this {} with an object literal",
            if call_like.is_new {
                "constructor"
            } else {
                "call"
            }
        ));
        if let Some(fix) = self.object_constructor_fix(expression_id, call_like) {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix from a no argument Object constructor call.
    fn object_constructor_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        call_like: CallLikeExpressionInfo<'_>,
    ) -> Option<LintFix> {
        // skip static arguments until we support rendering them
        if !call_like.generic_arguments.is_empty() {
            return None;
        }

        // only fix no argument calls: `Object(value)` has different semantics
        if !call_like.arguments.is_empty() {
            return None;
        }

        // select a context-safe literal replacement form
        let replacement = if expression_is_standalone_statement(self.ctx.tree, expression_id) {
            "{}"
        } else {
            "({})"
        };

        // replace the full constructor expression
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace Object constructor with object literal").with_edits(edits))
    }

    /// Return true when the expression is a reference to Object.
    fn is_object_reference(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol_or_global_qualified_member(
            self.ctx,
            expression_id,
            self.object_symbol,
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
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for object constructor calls
        if let Some(call_like) = expression_call_like(expression) {
            self.check_object_constructor(id, call_like);
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
    fn test_flags_object_constructor_call() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_flags_object_constructor_call.ds",
            r#"
let value = Object();
"#,
        );
        test.result(result).assert_lint("no-object-constructor");
    }

    #[test]
    fn test_flags_object_constructor_new() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_flags_object_constructor_new.ds",
            r#"
let value = new Object();
"#,
        );
        test.result(result).assert_lint("no-object-constructor");
    }

    /// Report global Object constructor calls.
    #[test]
    fn test_flags_global_object_constructor() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_flags_global_object_constructor.ds",
            r#"
let value = globalThis.Object();
"#,
        );
        test.result(result).assert_lint("no-object-constructor");
    }

    #[test]
    fn test_allows_object_literal() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_allows_object_literal.ds",
            r#"
let value = { key: "value" };
"#,
        );
        test.result(result).assert_no_lint("no-object-constructor");
    }

    #[test]
    fn test_fix_object_constructor_call() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_fix_object_constructor_call.ds",
            r#"
let value = Object();
"#,
        );
        test.result(result)
            .assert_lint("no-object-constructor")
            .assert_has_fix("no-object-constructor")
            .assert_safe_fixed(
                r#"
let value = ({});
"#,
            );
    }

    #[test]
    fn test_fix_object_constructor_new() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_fix_object_constructor_new.ds",
            r#"
let value = new Object();
"#,
        );
        test.result(result)
            .assert_lint("no-object-constructor")
            .assert_has_fix("no-object-constructor")
            .assert_safe_fixed(
                r#"
let value = ({});
"#,
            );
    }

    #[test]
    fn test_fix_object_constructor_statement_prefers_block_literal_form() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_fix_object_constructor_statement_prefers_block_literal_form.ds",
            r#"
Object();
"#,
        );
        test.result(result)
            .assert_lint("no-object-constructor")
            .assert_has_fix("no-object-constructor")
            .assert_safe_fixed(
                r#"
{};
"#,
            );
    }

    #[test]
    fn test_fix_object_constructor_in_arrow_keeps_expression_literal() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_fix_object_constructor_in_arrow_keeps_expression_literal.ds",
            r#"
let make = () => Object();
"#,
        );
        test.result(result)
            .assert_lint("no-object-constructor")
            .assert_has_fix("no-object-constructor")
            .assert_safe_fixed(
                r#"
let make = () => ({});
"#,
            );
    }

    #[test]
    fn test_no_fix_object_constructor_with_argument() {
        let test = TestProgram::for_rule_with_prelude(NoObjectConstructor);
        let result = test.lint_dir(
            "no_object_constructor/test_no_fix_object_constructor_with_argument.ds",
            r#"
let value = Object(input);
"#,
        );
        test.result(result)
            .assert_lint("no-object-constructor")
            .assert_has_no_fix("no-object-constructor");
    }
}
