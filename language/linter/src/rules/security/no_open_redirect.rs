use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{
    TaintAnalysis, TaintCache, expression_is_global_qualified_member,
    expression_is_symbol_or_global_qualified_member, expression_static_property_access,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow tainted values in browser redirect APIs.
    ///
    /// Open redirects can be exploited for phishing attacks by redirecting
    /// users to malicious sites while appearing to come from a trusted domain.
    #[lint(
        id = "no-open-redirect",
        code = "LS005",
        category = Security,
        level = Dir,
        requires_all = [RequireLibSymbol("location", &["dom"])],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoOpenRedirect,
    "Disallow open redirects"
}

impl LintRule for NoOpenRedirect {
    fn meta(&self) -> &'static LintMeta {
        NoOpenRedirect::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoOpenRedirectVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags open redirect patterns.
struct NoOpenRedirectVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The location lib symbol.
    location_symbol: dir::GlobalSymbolId,
    /// The `location` name.
    location_name: StringId,
    /// The `window` name.
    window_name: StringId,
    /// The `href` property name.
    href_name: StringId,
    /// The `assign` method name.
    assign_name: StringId,
    /// The `replace` method name.
    replace_name: StringId,
    /// Global qualifier symbols for matching `globalThis.location`.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// Cached taint analysis state.
    taint_cache: TaintCache,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoOpenRedirectVisitor<'a, 'b> {
    /// Build a new visitor.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // intern names
        let location_name = ctx.repository.strings.intern("location");
        let window_name = ctx.repository.strings.intern("window");
        let href_name = ctx.repository.strings.intern("href");
        let assign_name = ctx.repository.strings.intern("assign");
        let replace_name = ctx.repository.strings.intern("replace");

        // resolve symbols
        let location_symbol = ctx.declared_library_symbol(location_name);
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            location_symbol,
            location_name,
            window_name,
            href_name,
            assign_name,
            replace_name,
            global_qualifiers,
            taint_cache: TaintCache::default(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module expression roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check an assignment for open redirect patterns.
    fn check_assign(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        // check for location.href or location assignment
        if !self.is_location_target(left) {
            return;
        }

        // check if right side is potentially tainted
        if !self.expression_is_tainted(right) {
            return;
        }

        self.report(expression_id, "assignment to location");
    }

    /// Check a call for open redirect patterns.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // check for location.assign() or location.replace()
        if !self.is_redirect_method(left) {
            return;
        }

        // need at least one argument
        let Some(first_arg) = arguments.first() else {
            return;
        };

        // check if argument is potentially tainted
        let argument = self.ctx.tree.get(*first_arg);
        if !self.expression_is_tainted(argument.value()) {
            return;
        }

        self.report(expression_id, "call to redirect method");
    }

    /// Report an open redirect diagnostic.
    fn report(&mut self, expression_id: dir::LocalNodeId<dir::Expression>, context: &str) {
        // check effective severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_OPEN_REDIRECT.id,
                NO_OPEN_REDIRECT.code,
                NO_OPEN_REDIRECT.category,
                severity,
                format!("potential open redirect via {context}"),
                self.ctx.module.file_id,
                span,
            )
            .with_label("user-controlled URL may redirect to malicious site"),
        );
    }

    /// Return true when the expression is a location target.
    fn is_location_target(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match location directly
        if expression_is_symbol_or_global_qualified_member(
            self.ctx.tree,
            expression_id,
            self.location_symbol,
            &self.global_qualifiers,
            self.location_name,
        ) {
            return true;
        }

        // match location.href and window.location
        if let Some((receiver_id, property_name)) =
            expression_static_property_access(self.ctx.tree, expression_id)
        {
            if property_name == self.href_name && self.is_location_ref(receiver_id) {
                return true;
            }

            // enforce this lint guard
            if property_name == self.location_name && self.is_window_ref(receiver_id) {
                return true;
            }
        }

        false
    }

    /// Return true when the expression is a redirect method.
    fn is_redirect_method(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match location.assign or location.replace
        expression_static_property_access(self.ctx.tree, expression_id).is_some_and(
            |(receiver_id, property_name)| {
                (property_name == self.assign_name || property_name == self.replace_name)
                    && self.is_location_ref(receiver_id)
            },
        )
    }

    /// Return true when the expression references location.
    fn is_location_ref(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match window.location
        if let Some((receiver_id, property_name)) =
            expression_static_property_access(self.ctx.tree, expression_id)
            && property_name == self.location_name
            && self.is_window_ref(receiver_id)
        {
            return true;
        }

        expression_is_symbol_or_global_qualified_member(
            self.ctx.tree,
            expression_id,
            self.location_symbol,
            &self.global_qualifiers,
            self.location_name,
        )
    }

    /// Return true when the expression references window.
    fn is_window_ref(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.window_name,
        )
    }

    /// Return true when an expression is tainted.
    fn expression_is_tainted(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let mut taint = TaintAnalysis::new(
            self.ctx.repository.as_ref(),
            self.ctx.revision,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.tree,
            self.ctx.symbols,
            self.ctx.types,
            &mut self.taint_cache,
            true,
        );
        taint.expression_is_tainted(expression_id)
    }
}

impl NodeVisitor for NoOpenRedirectVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check assignments
        if let dir::Expression::Assign { left, right } = expression {
            self.check_assign(id, *left, *right);
        }

        // check calls
        if let dir::Expression::Call {
            left, arguments, ..
        } = expression
        {
            self.check_call(id, *left, arguments);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag location.href assignment with user input.
    #[test]
    fn test_flags_location_href_with_variable() {
        let test = TestProgram::for_rule_with_prelude(NoOpenRedirect);
        let result = test.lint_dir(
            "no_open_redirect/test_flags_location_href_with_variable.ds",
            r#"
let url = location.search;
location.href = url;
"#,
        );
        test.result(result).assert_lint("no-open-redirect");
    }

    /// Flag location.assign with user input.
    #[test]
    fn test_flags_location_assign_with_variable() {
        let test = TestProgram::for_rule_with_prelude(NoOpenRedirect);
        let result = test.lint_dir(
            "no_open_redirect/test_flags_location_assign_with_variable.ds",
            r#"
let url = location.search;
location.assign(url);
"#,
        );
        test.result(result).assert_lint("no-open-redirect");
    }

    /// Flag location.replace with user input.
    #[test]
    fn test_flags_location_replace_with_variable() {
        let test = TestProgram::for_rule_with_prelude(NoOpenRedirect);
        let result = test.lint_dir(
            "no_open_redirect/test_flags_location_replace_with_variable.ds",
            r#"
let url = location.hash;
location.replace(url);
"#,
        );
        test.result(result).assert_lint("no-open-redirect");
    }

    /// Allow literal URL assignment.
    #[test]
    fn test_allows_literal_url() {
        let test = TestProgram::for_rule_with_prelude(NoOpenRedirect);
        let result = test.lint_dir(
            "no_open_redirect/test_allows_literal_url.ds",
            r#"
location.href = "https://example.com";
"#,
        );
        test.result(result).assert_no_lint("no-open-redirect");
    }

    /// Allow literal URL in assign call.
    #[test]
    fn test_allows_literal_in_assign() {
        let test = TestProgram::for_rule_with_prelude(NoOpenRedirect);
        let result = test.lint_dir(
            "no_open_redirect/test_allows_literal_in_assign.ds",
            r#"
location.assign("/dashboard");
"#,
        );
        test.result(result).assert_no_lint("no-open-redirect");
    }

    /// Flag redirects from explicit decorator taint sources.
    #[test]
    fn test_flags_decorator_taint_source() {
        let test = TestProgram::for_rule_with_prelude(NoOpenRedirect);
        let result = test.lint_dir(
            "no_open_redirect/test_flags_decorator_taint_source.ds",
            r#"
@taint("url")
function readRedirect(): string {
    return "/next"
}

location.assign(readRedirect());
"#,
        );
        test.result(result).assert_lint("no-open-redirect");
    }

    /// Flag computed location href assignment with user input.
    #[test]
    fn test_flags_computed_location_href_assignment() {
        let test = TestProgram::for_rule_with_prelude(NoOpenRedirect);
        let result = test.lint_dir(
            "no_open_redirect/test_flags_computed_location_href_assignment.ds",
            r#"
let url = location.search;
location["href"] = url;
"#,
        );
        test.result(result).assert_lint("no-open-redirect");
    }

    /// Flag computed location assign calls with user input.
    #[test]
    fn test_flags_computed_location_assign_call() {
        let test = TestProgram::for_rule_with_prelude(NoOpenRedirect);
        let result = test.lint_dir(
            "no_open_redirect/test_flags_computed_location_assign_call.ds",
            r#"
let url = location.search;
location["assign"](url);
"#,
        );
        test.result(result).assert_lint("no-open-redirect");
    }
}
