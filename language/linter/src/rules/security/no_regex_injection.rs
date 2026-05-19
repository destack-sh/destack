use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{TaintAnalysis, TaintCache, expression_is_symbol};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow tainted values in dynamic regular expression patterns.
    ///
    /// Using user-controlled input in `new RegExp()` can lead to:
    /// - ReDoS (Regular Expression Denial of Service) attacks
    /// - Regex injection where attackers craft malicious patterns
    #[lint(
        id = "no-regex-injection",
        code = "LS007",
        category = Security,
        level = Dir,
        requires_all = [RequireLibSymbol("RegExp", &[])],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoRegexInjection,
    "Disallow regex injection"
}

impl LintRule for NoRegexInjection {
    fn meta(&self) -> &'static LintMeta {
        NoRegexInjection::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoRegexInjectionVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags regex injection patterns.
struct NoRegexInjectionVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The RegExp symbol for this module.
    regexp_symbol: dir::GlobalSymbolId,
    /// Cached taint analysis state.
    taint_cache: TaintCache,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoRegexInjectionVisitor<'a, 'b> {
    /// Build a new visitor.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let regexp_name = ctx.string_id("RegExp");
        let regexp_symbol = ctx.declared_library_symbol(regexp_name);

        Self {
            ctx,
            meta,
            regexp_symbol,
            taint_cache: TaintCache::default(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module expression roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a new expression for RegExp injection.
    fn check_new(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // check if this is new RegExp()
        if !self.is_regexp_constructor(left) {
            return;
        }

        // must have at least one argument (the pattern)
        let Some(first_arg) = arguments.first() else {
            return;
        };

        // check if the pattern argument is potentially tainted
        let argument = self.ctx.dir.get(*first_arg);
        let Some(value) = argument.value() else {
            return;
        };
        if !self.expression_is_tainted(value) {
            return;
        }

        self.report(expression_id);
    }

    /// Check a call expression for RegExp injection.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // RegExp can also be called without `new`
        if !self.is_regexp_constructor(left) {
            return;
        }

        // must have at least one argument
        let Some(first_arg) = arguments.first() else {
            return;
        };

        // check if the pattern argument is potentially tainted
        let argument = self.ctx.dir.get(*first_arg);
        let Some(value) = argument.value() else {
            return;
        };
        if !self.expression_is_tainted(value) {
            return;
        }

        self.report(expression_id);
    }

    /// Return true when the expression is the RegExp constructor.
    fn is_regexp_constructor(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol(self.ctx, expression_id, self.regexp_symbol)
    }

    /// Report a regex injection diagnostic.
    fn report(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // check effective severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_REGEX_INJECTION.id,
                NO_REGEX_INJECTION.code,
                NO_REGEX_INJECTION.category,
                severity,
                "potential regex injection",
                span,
            )
            .label("user-controlled input in RegExp may cause ReDoS"),
        );
    }

    /// Return true when an expression is tainted.
    fn expression_is_tainted(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let mut taint = TaintAnalysis::new(
            self.ctx.artifacts.as_ref(),
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.dir.tree(),
            self.ctx.strings,
            &self.ctx.symbols,
            self.ctx.types,
            self.ctx.resolutions,
            &mut self.taint_cache,
            true,
        );
        taint.expression_is_tainted(expression_id)
    }
}

impl NodeVisitor for NoRegexInjectionVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check new RegExp()
        if let dir::Expression::New {
            left, arguments, ..
        } = expression
        {
            self.check_new(id, *left, arguments);
        }

        // check RegExp() call without new
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

    /// Flag new RegExp with a variable pattern.
    #[test]
    fn test_flags_new_regexp_with_variable() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInjection);
        let result = test.lint_dir(
            "no_regex_injection/test_flags_new_regexp_with_variable.ds",
            r#"
let pattern = getUserInput();
let regex = new RegExp(pattern);
"#,
        );
        test.result(result).assert_lint("no-regex-injection");
    }

    /// Flag RegExp call with a variable pattern.
    #[test]
    fn test_flags_regexp_call_with_variable() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInjection);
        let result = test.lint_dir(
            "no_regex_injection/test_flags_regexp_call_with_variable.ds",
            r#"
let pattern = getUserInput();
let regex = RegExp(pattern);
"#,
        );
        test.result(result).assert_lint("no-regex-injection");
    }

    /// Flag global qualified RegExp constructor calls.
    #[test]
    fn test_flags_global_regexp_call_with_variable() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInjection);
        let result = test.lint_dir(
            "no_regex_injection/test_flags_global_regexp_call_with_variable.ds",
            r#"
let pattern = getUserInput();
let regex = globalThis.RegExp(pattern);
"#,
        );
        test.result(result).assert_lint("no-regex-injection");
    }

    /// Flag computed global qualified RegExp constructor calls.
    #[test]
    fn test_flags_computed_global_regexp_call_with_variable() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInjection);
        let result = test.lint_dir(
            "no_regex_injection/test_flags_computed_global_regexp_call_with_variable.ds",
            r#"
let pattern = getUserInput();
let regex = globalThis["RegExp"](pattern);
"#,
        );
        test.result(result).assert_lint("no-regex-injection");
    }

    /// Flag new RegExp with string concatenation.
    #[test]
    fn test_flags_new_regexp_with_concatenation() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInjection);
        let result = test.lint_dir(
            "no_regex_injection/test_flags_new_regexp_with_concatenation.ds",
            r#"
let input = "user";
let regex = new RegExp("^" + input + "$");
"#,
        );
        test.result(result).assert_lint("no-regex-injection");
    }

    /// Allow new RegExp with a string literal pattern.
    #[test]
    fn test_allows_literal_pattern() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInjection);
        let result = test.lint_dir(
            "no_regex_injection/test_allows_literal_pattern.ds",
            r#"
let regex = new RegExp("^[a-z]+$");
"#,
        );
        test.result(result).assert_no_lint("no-regex-injection");
    }

    /// Allow RegExp call with a string literal pattern.
    #[test]
    fn test_allows_literal_pattern_call() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInjection);
        let result = test.lint_dir(
            "no_regex_injection/test_allows_literal_pattern_call.ds",
            r#"
let regex = RegExp("\\d+");
"#,
        );
        test.result(result).assert_no_lint("no-regex-injection");
    }

    /// Flag regex patterns from explicit decorator taint sources.
    #[test]
    fn test_flags_decorator_taint_source() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInjection);
        let result = test.lint_dir(
            "no_regex_injection/test_flags_decorator_taint_source.ds",
            r#"
@taint("regex")
function userPattern(): string {
    return "a+"
}

let regex = new RegExp(userPattern());
"#,
        );
        test.result(result).assert_lint("no-regex-injection");
    }
}
