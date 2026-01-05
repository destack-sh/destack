use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{
    expression_is_global_qualified_member, expression_target_symbol, global_qualifier_symbols,
    unwrap_parenthesized_expression,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow JSON parse stringify clones.
    ///
    /// JSON cloning is slow, lossy, and fails for many types.
    #[lint(
        id = "no-json-clone",
        code = "LP008",
        category = Performance,
        level = Dir,
        requires_all = [RequireLibSymbol("JSON", &[])],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoJsonClone,
    "Disallow JSON.parse(JSON.stringify(...)) cloning"
}

impl LintRule for NoJsonClone {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoJsonClone::meta()
    }

    /// Check module DIR nodes for JSON clone patterns.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for JSON clone calls
        let mut visitor = NoJsonCloneVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags JSON clone patterns.
struct NoJsonCloneVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The JSON symbol for this module.
    json_symbol: dir::GlobalSymbolId,
    /// The JSON member name.
    json_name: StringId,
    /// The parse member name.
    parse_name: StringId,
    /// The stringify member name.
    stringify_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoJsonCloneVisitor<'a, 'b> {
    /// Build a visitor for no-json-clone checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // intern commonly used names
        let json_name = ctx.program.strings.intern("JSON");
        let parse_name = ctx.program.strings.intern("parse");
        let stringify_name = ctx.program.strings.intern("stringify");

        // resolve lib symbols
        let json_symbol = ctx.lib_item(json_name);
        let global_qualifiers = global_qualifier_symbols(ctx);

        // prepare visitor state
        Self {
            ctx,
            meta,
            json_symbol,
            json_name,
            parse_name,
            stringify_name,
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

    /// Check a call expression for JSON clone usage.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // match JSON.parse calls
        if !self.is_json_member(left, self.parse_name) {
            return;
        }

        // resolve the first argument
        let Some(first_argument) = arguments.first() else {
            return;
        };
        let argument = self.ctx.tree.get(*first_argument);
        let argument_id = unwrap_parenthesized_expression(self.ctx.tree, argument.value());
        if !self.is_json_stringify_call(argument_id) {
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
                NO_JSON_CLONE.id,
                NO_JSON_CLONE.code,
                NO_JSON_CLONE.category,
                severity,
                "JSON clone usage",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use a structured clone or manual copy"),
        );
    }

    /// Return true when the expression is a JSON member access.
    fn is_json_member(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        name: StringId,
    ) -> bool {
        // match member access
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Member {
            left, name: member, ..
        } = expression
        else {
            return false;
        };
        if *member != name {
            return false;
        }

        // match direct JSON references
        if expression_target_symbol(self.ctx.tree, *left) == Some(self.json_symbol) {
            return true;
        }

        // match global qualified JSON references
        expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.json_name,
        )
    }

    /// Return true when the expression is a JSON.stringify call.
    fn is_json_stringify_call(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match call expressions
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call { left, .. } = expression else {
            return false;
        };

        self.is_json_member(*left, self.stringify_name)
    }
}

impl NodeVisitor for NoJsonCloneVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for JSON clone usage
        if let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        {
            self.check_call(id, *left, dynamic_arguments);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report JSON clone usage.
    #[test]
    fn test_flags_json_clone() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "test.ds",
            r#"
const next = JSON.parse(JSON.stringify(value));
"#,
        );
        test.result(result).assert_lint("no-json-clone");
    }

    /// Report global JSON clone usage.
    #[test]
    fn test_flags_global_json_clone() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "test.ds",
            r#"
const next = globalThis.JSON.parse(globalThis.JSON.stringify(value));
"#,
        );
        test.result(result).assert_lint("no-json-clone");
    }

    /// Allow JSON.parse without stringify.
    #[test]
    fn test_allows_json_parse() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "test.ds",
            r#"
const parsed = JSON.parse(text);
"#,
        );
        test.result(result).assert_no_lint("no-json-clone");
    }
}
