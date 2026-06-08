use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_repository::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{
    expression_is_symbol, expression_unwrap_parenthesized, expression_unwrap_transparent,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow JSON parse stringify clones.
    ///
    /// JSON cloning is slow, lossy, and fails for many types.
    #[lint(
        id = "no-json-clone",
        code = "LP006",
        category = Performance,
        level = Dir,
        requires_all = [RequireLibSymbol("JSON", &[])],
        requires_any = [],
        fixable = Sometimes,
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
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
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
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The JSON symbol for this module.
    json_symbol: dir::GlobalSymbolId,
    /// The parse member name.
    parse_name: StringId,
    /// The stringify member name.
    stringify_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoJsonCloneVisitor<'a, 'b> {
    /// Build a visitor for no-json-clone checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let json_name = ctx.string_id("JSON");
        let parse_name = ctx.string_id("parse");
        let stringify_name = ctx.string_id("stringify");
        let json_symbol = ctx.declared_library_symbol(json_name);

        Self {
            ctx,
            meta,
            json_symbol,
            parse_name,
            stringify_name,
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
        let argument = self.ctx.dir.get(*first_argument);
        let Some(argument_id) = argument.value() else {
            return;
        };
        let argument_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), argument_id);
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
        let mut diagnostic = LintReport::new(
            NO_JSON_CLONE.id,
            NO_JSON_CLONE.code,
            NO_JSON_CLONE.category,
            severity,
            "JSON clone usage",
            span,
        )
        .label("use a structured clone or manual copy");

        // compute fixes only when requested by the runner
        if self.ctx.compute_fixes
            && let Some(stringify_argument_id) = self.json_stringify_argument(argument_id)
            && let Some(fix) =
                self.no_json_clone_fix(expression_id, arguments, stringify_argument_id)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return true when the expression is a JSON member access.
    fn is_json_member(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        name: StringId,
    ) -> bool {
        // normalize transparent wrappers on the member expression
        let expression_id = expression_unwrap_transparent(self.ctx.dir.tree(), expression_id);

        // match member access
        let expression = self.ctx.dir.get(expression_id);
        let dir::Expression::Member {
            left, name: member, ..
        } = expression
        else {
            return false;
        };
        if *member != Some(name) {
            return false;
        }

        // match direct and global qualified JSON references
        expression_is_symbol(self.ctx, *left, self.json_symbol)
    }

    /// Return true when the expression is a JSON.stringify call.
    fn is_json_stringify_call(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // normalize transparent wrappers on the call expression
        let expression_id = expression_unwrap_transparent(self.ctx.dir.tree(), expression_id);

        // match call expressions
        let expression = self.ctx.dir.get(expression_id);
        let dir::Expression::Call { left, .. } = expression else {
            return false;
        };

        self.is_json_member(*left, self.stringify_name)
    }

    /// Resolve the argument passed to a JSON.stringify call.
    fn json_stringify_argument(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        // normalize transparent wrappers on the call expression
        let expression_id = expression_unwrap_transparent(self.ctx.dir.tree(), expression_id);

        // match call expressions
        let expression = self.ctx.dir.get(expression_id);
        let dir::Expression::Call {
            left, arguments, ..
        } = expression
        else {
            return None;
        };
        if arguments.len() != 1 {
            return None;
        }

        if !self.is_json_member(*left, self.stringify_name) {
            return None;
        }

        let argument = self.ctx.dir.get(arguments[0]);
        let argument = argument.value()?;
        Some(expression_unwrap_parenthesized(
            self.ctx.dir.tree(),
            argument,
        ))
    }

    /// Build an unsafe fix that rewrites JSON parse stringify clones.
    fn no_json_clone_fix(
        &self,
        parse_call_id: dir::LocalNodeId<dir::Expression>,
        parse_arguments: &[dir::LocalNodeId<dir::Argument>],
        stringify_argument_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintFix> {
        // only rewrite canonical single argument parse calls
        if parse_arguments.len() != 1 {
            return None;
        }

        let argument_span = self.ctx.get_span(stringify_argument_id);
        let argument_text = self.ctx.get_span_text(argument_span);
        if argument_text.trim().is_empty() {
            return None;
        }

        let replacement = format!("structuredClone({argument_text})");
        let parse_call_span = self.ctx.get_span(parse_call_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(parse_call_span, replacement)
            .into_edits();
        Some(LintFix::r#unsafe("Replace JSON clone with structuredClone").with_edits(edits))
    }
}

impl NodeVisitor for NoJsonCloneVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for JSON clone usage
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

    /// Report JSON clone usage.
    #[test]
    fn test_flags_json_clone() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_flags_json_clone.ds",
            r#"
const next = JSON.parse(JSON.stringify(value));
"#,
        );
        test.result(result)
            .assert_lint("no-json-clone")
            .assert_has_fix("no-json-clone");
    }

    /// Report global JSON clone usage.
    #[test]
    fn test_flags_global_json_clone() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_flags_global_json_clone.ds",
            r#"
const next = globalThis.JSON.parse(globalThis.JSON.stringify(value));
"#,
        );
        test.result(result)
            .assert_lint("no-json-clone")
            .assert_has_fix("no-json-clone");
    }

    /// Allow JSON.parse without stringify.
    #[test]
    fn test_allows_json_parse() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_allows_json_parse.ds",
            r#"
const parsed = JSON.parse(text);
"#,
        );
        test.result(result).assert_no_lint("no-json-clone");
    }

    /// Unsafely rewrite JSON clone calls to structuredClone.
    #[test]
    fn test_fix_rewrites_json_clone() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_fix_rewrites_json_clone.ds",
            r#"
const next = JSON.parse(JSON.stringify(value));
"#,
        );
        test.result(result)
            .assert_lint("no-json-clone")
            .assert_unsafe_fixed(
                r#"
const next = structuredClone(value);
"#,
            );
    }

    /// Do not auto-fix parse calls that use a reviver.
    #[test]
    fn test_no_fix_when_parse_has_reviver() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_no_fix_when_parse_has_reviver.ds",
            r#"
const next = JSON.parse(JSON.stringify(value), reviver);
"#,
        );
        test.result(result)
            .assert_lint("no-json-clone")
            .assert_has_no_fix("no-json-clone");
    }

    /// Do not auto-fix stringify calls that use replacers.
    #[test]
    fn test_no_fix_when_stringify_has_replacer() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_no_fix_when_stringify_has_replacer.ds",
            r#"
const next = JSON.parse(JSON.stringify(value, replacer));
"#,
        );
        test.result(result)
            .assert_lint("no-json-clone")
            .assert_has_no_fix("no-json-clone");
    }

    /// Mutation: rewrite global JSON clone references.
    #[test]
    fn test_mutation_fix_rewrites_global_json_clone() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_mutation_fix_rewrites_global_json_clone.ds",
            r#"
const next = globalThis.JSON.parse(globalThis.JSON.stringify(source));
"#,
        );
        test.result(result)
            .assert_lint("no-json-clone")
            .assert_unsafe_fixed(
                r#"
const next = structuredClone(source);
"#,
            );
    }

    /// Report parenthesized JSON.parse and JSON.stringify clone usage.
    #[test]
    fn test_flags_parenthesized_json_clone_callees() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_flags_parenthesized_json_clone_callees.ds",
            r#"
const next = ((JSON.parse))(((JSON.stringify))(value));
"#,
        );
        test.result(result)
            .assert_lint("no-json-clone")
            .assert_has_fix("no-json-clone");
    }

    /// Unsafely rewrite parenthesized JSON clone calls to structuredClone.
    #[test]
    fn test_fix_rewrites_parenthesized_json_clone_callees() {
        let test = TestProgram::for_rule_with_prelude(NoJsonClone);
        let result = test.lint_dir(
            "no_json_clone/test_fix_rewrites_parenthesized_json_clone_callees.ds",
            r#"
const next = ((JSON.parse))(((JSON.stringify))(source));
"#,
        );
        test.result(result)
            .assert_lint("no-json-clone")
            .assert_unsafe_fixed(
                r#"
const next = structuredClone(source);
"#,
            );
    }
}
