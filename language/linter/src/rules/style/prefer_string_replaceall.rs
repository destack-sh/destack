use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{is_string_type, unwrap_parenthesized_expression};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `replaceAll()` over `replace()` with a global regex.
    ///
    /// `replaceAll()` avoids regex overhead and better communicates intent.
    #[lint(
        id = "prefer-string-replaceall",
        code = "LY067",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::String)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferStringReplaceAll,
    "Prefer string.replaceAll() over replace() with a global regex"
}

impl LintRule for PreferStringReplaceAll {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferStringReplaceAll::meta()
    }

    /// Check module DIR nodes for replace calls that should use replaceAll().
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferStringReplaceAllVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags prefer-string-replaceall patterns.
struct PreferStringReplaceAllVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// The string id for the replace method name.
    replace_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferStringReplaceAllVisitor<'a, 'b> {
    /// Build a visitor for prefer-string-replaceall checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let string_symbol = ctx.well_known_symbol(WellKnownSymbol::String);
        let replace_name = ctx.program.strings.intern("replace");

        Self {
            ctx,
            meta,
            string_symbol,
            replace_name,
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

    /// Check a replace call expression for replaceAll usage.
    fn check_replace_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        dynamic_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // match member access for replace
        let member_expression = self.ctx.tree.get(left);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return;
        };
        if *name != self.replace_name {
            return;
        }

        // ensure the receiver is a string
        if !self.is_string_receiver(*left) {
            return;
        }

        // require at least one argument
        let Some(first_argument_id) = dynamic_arguments.first() else {
            return;
        };
        let argument = self.ctx.tree.get(*first_argument_id);
        let argument_id = argument.value();
        if !self.is_global_regex(argument_id) {
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
                PREFER_STRING_REPLACE_ALL.id,
                PREFER_STRING_REPLACE_ALL.code,
                PREFER_STRING_REPLACE_ALL.category,
                severity,
                "prefer replaceAll() over replace() with a global regex",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use replaceAll() for global replacements"),
        );
    }

    /// Return true when the receiver expression is a string type.
    fn is_string_receiver(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_string_type(self.ctx.types, type_id, Some(self.string_symbol))
    }

    /// Return true when the expression is a global regex literal.
    fn is_global_regex(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // unwrap parenthesized expressions
        let expression_id = unwrap_parenthesized_expression(self.ctx.tree, expression_id);

        // match regex literals
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::ScalarLiteral {
            value:
                dir::ScalarLiteral::RegexString {
                    flags: Some(flags), ..
                },
        } = expression
        else {
            return false;
        };

        // inspect regex flags
        let flags = self.ctx.program.strings.get(*flags);
        flags.as_ref().contains('g')
    }
}

impl NodeVisitor for PreferStringReplaceAllVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check replace calls
        if let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        {
            self.check_replace_call(id, *left, dynamic_arguments);
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
    fn test_flags_global_regex_replace() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "test.ds",
            r#"
let text = "hello";
let next = text.replace(/l/g, "x");
"#);
        test.result(result).assert_lint("prefer-string-replaceall");
    }

    #[test]
    fn test_allows_non_global_regex_replace() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "test.ds",
            r#"
let text = "hello";
let next = text.replace(/l/, "x");
"#);
        test.result(result)
            .assert_no_lint("prefer-string-replaceall");
    }
}
