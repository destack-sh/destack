use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::{RequireLibSymbol, RequireWellKnownSymbol};
use crate::rules::common::{
    expression_is_any_symbol_or_global_qualified_member,
    expression_is_symbol_or_global_qualified_member, expression_type_or_call_return_type_map,
    expression_unwrap_parenthesized, is_string_type,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow string execution APIs that act like eval.
    ///
    /// Passing strings to timers or using the Function constructor is unsafe.
    #[lint(
        id = "no-implied-eval",
        code = "LS003",
        category = Security,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Function)],
        requires_any = [
            RequireLibSymbol("setTimeout", &["dom", "node"]),
            RequireLibSymbol("setInterval", &["dom", "node"]),
            RequireLibSymbol("setImmediate", &["dom", "node"]),
        ],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoImpliedEval,
    "Disallow implied eval via strings"
}

impl LintRule for NoImpliedEval {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoImpliedEval::meta()
    }

    /// Check module DIR nodes for implied eval usage.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoImpliedEvalVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags implied eval usage.
struct NoImpliedEvalVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Function constructor symbol.
    function_symbol: dir::GlobalSymbolId,
    /// The String well known symbol when available.
    string_symbol: Option<dir::GlobalSymbolId>,
    /// The Function member name.
    function_name: StringId,
    /// The setTimeout symbol for this module.
    set_timeout_symbol: Option<dir::GlobalSymbolId>,
    /// The setInterval symbol for this module.
    set_interval_symbol: Option<dir::GlobalSymbolId>,
    /// The setImmediate symbol for this module.
    set_immediate_symbol: Option<dir::GlobalSymbolId>,
    /// The setTimeout member name.
    set_timeout_name: StringId,
    /// The setInterval member name.
    set_interval_name: StringId,
    /// The setImmediate member name.
    set_immediate_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoImpliedEvalVisitor<'a, 'b> {
    /// Build a visitor for no-implied-eval checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let function_symbol = ctx.well_known_symbol(WellKnownSymbol::Function);
        let string_symbol = ctx.get_well_known_symbol(WellKnownSymbol::String);
        let function_name = ctx.string_id("Function");
        let set_timeout_name = ctx.string_id("setTimeout");
        let set_interval_name = ctx.string_id("setInterval");
        let set_immediate_name = ctx.string_id("setImmediate");
        let set_timeout_symbol = ctx.get_declared_library_symbol(set_timeout_name);
        let set_interval_symbol = ctx.get_declared_library_symbol(set_interval_name);
        let set_immediate_symbol = ctx.get_declared_library_symbol(set_immediate_name);
        let global_qualifiers = ctx.global_qualifier_symbols();

        // prepare visitor state
        Self {
            ctx,
            meta,
            function_symbol,
            string_symbol,
            function_name,
            set_timeout_symbol,
            set_interval_symbol,
            set_immediate_symbol,
            set_timeout_name,
            set_interval_name,
            set_immediate_name,
            global_qualifiers,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call expression for implied eval usage.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // check Function calls
        if self.is_function_symbol(left) {
            self.report_implied_eval(expression_id);
            return;
        }

        // check timer calls
        if !self.is_timer_symbol(left) {
            return;
        }

        // resolve the first argument
        let Some(first_argument) = arguments.first() else {
            return;
        };
        let argument = self.ctx.tree.get(*first_argument);
        let argument_id = expression_unwrap_parenthesized(self.ctx.tree, argument.value());
        if !self.is_string_like(argument_id) {
            return;
        }

        self.report_implied_eval(expression_id);
    }

    /// Check a constructor expression for implied eval usage.
    fn check_new(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        if self.is_function_symbol(left) {
            self.report_implied_eval(expression_id);
        }
    }

    /// Report an implied eval diagnostic.
    fn report_implied_eval(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_IMPLIED_EVAL.id,
                NO_IMPLIED_EVAL.code,
                NO_IMPLIED_EVAL.category,
                severity,
                "implied eval usage",
                span,
            )
            .label("avoid passing strings to execution APIs"),
        );
    }

    /// Return true when the expression refers to the Function constructor.
    fn is_function_symbol(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol_or_global_qualified_member(
            self.ctx,
            expression_id,
            self.function_symbol,
            &self.global_qualifiers,
            self.function_name,
        )
    }

    /// Return true when the expression refers to a timer function.
    fn is_timer_symbol(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let symbols = [
            self.set_timeout_symbol,
            self.set_interval_symbol,
            self.set_immediate_symbol,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let names = [
            self.set_timeout_name,
            self.set_interval_name,
            self.set_immediate_name,
        ];

        expression_is_any_symbol_or_global_qualified_member(
            self.ctx,
            expression_id,
            &symbols,
            &self.global_qualifiers,
            &names,
        )
    }

    /// Return true when the expression is a string literal or template.
    fn is_string_like(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression = self.ctx.tree.get(expression_id);
        if matches!(
            expression,
            dir::Expression::ScalarLiteral {
                value: dir::ScalarLiteral::String(_),
            } | dir::Expression::TemplateExpression { .. }
        ) {
            return true;
        }

        // treat `left + right` as string-like if either side is string-like
        if let dir::Expression::Binary {
            operator: dir::BinaryOperator::Add,
            left,
            right,
        } = expression
        {
            return self.is_string_like(*left) || self.is_string_like(*right);
        }

        expression_type_or_call_return_type_map(
            self.ctx.artifacts.as_ref(),
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.tree,
            self.ctx.types,
            expression_id,
            |types, type_id| is_string_type(types, type_id, self.string_symbol),
        )
        .unwrap_or(false)
    }
}

impl NodeVisitor for NoImpliedEvalVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check Function constructor calls
        if let dir::Expression::New { left, .. } = expression {
            self.check_new(id, *left);
            walk_expression(self, tree, id, expression);
            return;
        }

        // check timer calls
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

    /// Report setTimeout with string arguments.
    #[test]
    fn test_flags_string_set_timeout() {
        let test = TestProgram::for_rule_with_prelude(NoImpliedEval);
        let result = test.lint_dir(
            "no_implied_eval/test_flags_string_set_timeout.ds",
            r#"
setTimeout("doThing()", 10);
"#,
        );
        test.result(result).assert_lint("no-implied-eval");
    }

    /// Report setInterval with string arguments.
    #[test]
    fn test_flags_string_set_interval() {
        let test = TestProgram::for_rule_with_prelude(NoImpliedEval);
        let result = test.lint_dir(
            "no_implied_eval/test_flags_string_set_interval.ds",
            r#"
setInterval("doThing()", 10);
"#,
        );
        test.result(result).assert_lint("no-implied-eval");
    }

    /// Report global setImmediate with string arguments.
    #[test]
    fn test_flags_global_set_immediate() {
        let test = TestProgram::for_rule_with_prelude(NoImpliedEval);
        let result = test.lint_dir(
            "no_implied_eval/test_flags_global_set_immediate.ds",
            r#"
globalThis.setImmediate("doThing()");
"#,
        );
        test.result(result).assert_lint("no-implied-eval");
    }

    /// Report Function constructor calls.
    #[test]
    fn test_flags_function_constructor() {
        let test = TestProgram::for_rule_with_prelude(NoImpliedEval);
        let result = test.lint_dir(
            "no_implied_eval/test_flags_function_constructor.ds",
            r#"
const fn = Function("return 1;");
"#,
        );
        test.result(result).assert_lint("no-implied-eval");
    }

    /// Allow function callbacks in timers.
    #[test]
    fn test_allows_function_timer() {
        let test = TestProgram::for_rule_with_prelude(NoImpliedEval);
        let result = test.lint_dir(
            "no_implied_eval/test_allows_function_timer.ds",
            r#"
setTimeout(() => work(), 10);
"#,
        );
        test.result(result).assert_no_lint("no-implied-eval");
    }

    /// Report timer calls with string concatenation.
    #[test]
    fn test_flags_string_concatenation_timer() {
        let test = TestProgram::for_rule_with_prelude(NoImpliedEval);
        let result = test.lint_dir(
            "no_implied_eval/test_flags_string_concatenation_timer.ds",
            r#"
let expression = "work()";
setTimeout("return " + expression, 10);
"#,
        );
        test.result(result).assert_lint("no-implied-eval");
    }

    /// Report timer calls with string typed variables.
    #[test]
    fn test_flags_string_typed_timer_argument() {
        let test = TestProgram::for_rule_with_prelude(NoImpliedEval);
        let result = test.lint_dir(
            "no_implied_eval/test_flags_string_typed_timer_argument.ds",
            r#"
let expression: string = "work()";
setTimeout(expression, 10);
"#,
        );
        test.result(result).assert_lint("no-implied-eval");
    }
}
