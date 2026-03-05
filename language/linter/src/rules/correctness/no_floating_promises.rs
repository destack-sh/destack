use destack_ast::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_is_promise_like, expression_unwrap_parenthesized, is_function_type,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Require Promise results to be handled.
    ///
    /// Promise values should be awaited, returned, or explicitly handled
    /// with `.then()`, `.catch()`, or `.finally()`.
    #[lint(
        id = "no-floating-promises",
        code = "LC016",
        category = Correctness,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Promise)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoFloatingPromises,
    "Require Promise results to be handled"
}

impl LintRule for NoFloatingPromises {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoFloatingPromises::meta()
    }

    /// Check module DIR nodes for floating Promise expressions.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = FloatingPromiseVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags floating Promise expressions.
struct FloatingPromiseVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Promise symbol.
    promise_symbol: dir::GlobalSymbolId,
    /// Whether explicit `void` should suppress the lint.
    allow_void_discard: bool,
    /// The member name `then`.
    then_name: StringId,
    /// The member name `catch`.
    catch_name: StringId,
    /// The member name `finally`.
    finally_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> FloatingPromiseVisitor<'a, 'b> {
    /// Build a visitor for floating Promise checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let promise_symbol = ctx
            .well_known_symbols()
            .get_type_symbol(WellKnownSymbol::Promise)
            .unwrap_or_else(|| ctx.well_known_symbol(WellKnownSymbol::Promise));
        let allow_void_discard = ctx.options.allow_void_discard;
        let then_name = ctx.program.strings.intern("then");
        let catch_name = ctx.program.strings.intern("catch");
        let finally_name = ctx.program.strings.intern("finally");

        Self {
            ctx,
            meta,
            promise_symbol,
            allow_void_discard,
            then_name,
            catch_name,
            finally_name,
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

    /// Return true when an expression produces a Promise value.
    fn is_promise_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);

        // preserve Promise checks for explicit void discards
        if let dir::Expression::Unary {
            operator: dir::UnaryOperator::Void,
            right,
        } = expression
        {
            return self.is_promise_expression(*right);
        }

        // follow Promise handler chains through member receivers
        if let dir::Expression::Call { left, .. } = expression {
            let left_id = expression_unwrap_parenthesized(self.ctx.tree, *left);
            let left_expression = self.ctx.tree.get(left_id);
            if let dir::Expression::Member {
                left: receiver,
                name,
                ..
            } = left_expression
                && (*name == self.then_name
                    || *name == self.catch_name
                    || *name == self.finally_name)
            {
                return self.is_promise_expression(*receiver);
            }
        }

        expression_is_promise_like(
            self.ctx.module_id(),
            self.ctx.tree,
            self.ctx.types,
            self.promise_symbol,
            expression_id,
        )
    }

    /// Return true when a call is a Promise handler chain.
    fn is_handler_call(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        else {
            return false;
        };

        // resolve left id
        let left_id = expression_unwrap_parenthesized(self.ctx.tree, *left);
        let left_expression = self.ctx.tree.get(left_id);
        let dir::Expression::Member { name, .. } = left_expression else {
            return false;
        };

        // `.catch(...)` and `.finally(...)` always handle rejection paths
        if *name == self.catch_name || *name == self.finally_name {
            return true;
        }

        // `.then(...)` only handles rejection when a function rejection handler is provided
        if *name != self.then_name {
            return false;
        }

        // require optional structure
        let Some(second_argument_id) = dynamic_arguments.get(1) else {
            return false;
        };
        let second_argument = self.ctx.tree.get(*second_argument_id);
        let handler_id = second_argument.value();
        let Some(handler_type_id) = self.ctx.expression_type_id(handler_id) else {
            return false;
        };

        is_function_type(self.ctx.types, handler_type_id)
    }

    /// Return true when the Promise expression is handled.
    fn is_handled_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);

        // branch by expression kind
        match expression {
            dir::Expression::Await { .. } | dir::Expression::AwaitMaybe { .. } => true,
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Void,
                ..
            } => self.allow_void_discard,
            dir::Expression::Call { .. } => self.is_handler_call(expression_id),
            _ => false,
        }
    }

    /// Check a statement expression for floating Promise results.
    fn check_statement(
        &mut self,
        statement_id: dir::LocalNodeId<dir::Expression>,
        inner_id: dir::LocalNodeId<dir::Expression>,
    ) {
        if !self.is_promise_expression(inner_id) {
            return;
        }
        if self.is_handled_expression(inner_id) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, statement_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(statement_id);
        let mut diagnostic = LintDiagnostic::new(
            NO_FLOATING_PROMISES.id,
            NO_FLOATING_PROMISES.code,
            NO_FLOATING_PROMISES.category,
            severity,
            "Promise result is ignored",
            self.ctx.module.file_id,
            span,
        )
        .with_label("await, return, or attach a Promise handler");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && let Some(fix) = self.no_floating_promises_fix(inner_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix by explicitly discarding Promise results with `void`.
    fn no_floating_promises_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintFix> {
        // this fix only applies when explicit void discard is accepted
        if !self.allow_void_discard {
            return None;
        }

        // do not stack `void` on existing explicit void expressions
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);
        if matches!(
            expression,
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Void,
                ..
            }
        ) {
            return None;
        }

        // preserve replacement span, but normalize redundant parentheses
        let replacement_span = self.ctx.get_span(expression_id);
        let normalized_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);
        let normalized_span = self.ctx.get_span(normalized_id);
        let expression_text = self.ctx.get_span_text(normalized_span);
        if expression_text.trim().is_empty() {
            return None;
        }

        // build replacement text
        let replacement = format!("void {expression_text}");
        let edits = self
            .ctx
            .edit_builder()
            .replace(replacement_span, replacement)
            .into_edits();
        Some(LintFix::safe("Explicitly discard the Promise with void").with_edits(edits))
    }
}

impl NodeVisitor for FloatingPromiseVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check expression statements
        if let dir::Expression::Statement { statement } = expression {
            self.check_statement(id, *statement);
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
    fn test_flags_floating_promise_statement() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_flags_floating_promise_statement.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

load();
"#,
        );
        test.result(result)
            .assert_lint("no-floating-promises")
            .assert_has_fix("no-floating-promises");
    }

    #[test]
    fn test_allows_awaited_promise() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_allows_awaited_promise.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

async function run(): Promise<void> {
    await load();
}
"#,
        );
        test.result(result).assert_no_lint("no-floating-promises");
    }

    #[test]
    fn test_allows_handler_chain() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_allows_handler_chain.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

load().catch(() => {});
"#,
        );
        test.result(result).assert_no_lint("no-floating-promises");
    }

    #[test]
    fn test_flags_then_without_rejection_handler() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_flags_then_without_rejection_handler.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

load().then(() => {});
"#,
        );
        test.result(result).assert_lint("no-floating-promises");
    }

    #[test]
    fn test_allows_then_with_rejection_handler() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_allows_then_with_rejection_handler.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

load().then(
    () => {},
    () => {}
);
"#,
        );
        test.result(result).assert_no_lint("no-floating-promises");
    }

    #[test]
    fn test_flags_then_with_non_function_rejection_handler() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_flags_then_with_non_function_rejection_handler.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

load().then(
    () => {},
    123
);
"#,
        );
        test.result(result).assert_lint("no-floating-promises");
    }

    #[test]
    fn test_allows_void_discard_by_default() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_allows_void_discard_by_default.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

void load();
"#,
        );
        test.result(result).assert_no_lint("no-floating-promises");
    }

    #[test]
    fn test_flags_void_discard_when_disabled() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises).with_options(|options| {
            options.allow_void_discard = false;
        });
        let result = test.lint_dir(
            "no_floating_promises/test_flags_void_discard_when_disabled.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

void load();
"#,
        );
        test.result(result).assert_lint("no-floating-promises");
    }

    #[test]
    fn test_flags_new_promise_statement() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_flags_new_promise_statement.ts",
            r#"
new Promise((resolve) => {
    resolve(1);
});
"#,
        );
        test.result(result).assert_lint("no-floating-promises");
    }

    #[test]
    fn test_flags_async_iife_statement() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_flags_async_iife_statement.ts",
            r#"
(async () => 1)();
"#,
        );
        test.result(result).assert_lint("no-floating-promises");
    }

    #[test]
    fn test_fix_prefixes_floating_promise_with_void() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_fix_prefixes_floating_promise_with_void.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

load();
"#,
        );
        test.result(result)
            .assert_lint("no-floating-promises")
            .assert_safe_fixed(
                r#"
async function load(): Promise<number> {
    return 1;
}

void load();
"#,
            );
    }

    #[test]
    fn test_no_fix_when_void_discard_is_disabled() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises).with_options(|options| {
            options.allow_void_discard = false;
        });
        let result = test.lint_dir(
            "no_floating_promises/test_no_fix_when_void_discard_is_disabled.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

load();
"#,
        );
        test.result(result)
            .assert_lint("no-floating-promises")
            .assert_has_no_fix("no-floating-promises");
    }

    #[test]
    fn test_mutation_fix_prefixes_async_iife_with_void() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_mutation_fix_prefixes_async_iife_with_void.ts",
            r#"
(async () => 1)();
"#,
        );
        test.result(result)
            .assert_lint("no-floating-promises")
            .assert_safe_fixed(
                r#"
void (async () => 1)();
"#,
            );
    }

    #[test]
    fn test_allows_non_promise_statement() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_allows_non_promise_statement.ts",
            r#"
function compute(): number {
    return 1;
}

compute();
"#,
        );
        test.result(result).assert_no_lint("no-floating-promises");
    }

    #[test]
    fn test_allows_returned_promise() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_allows_returned_promise.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

function wrap(): Promise<number> {
    return load();
}
"#,
        );
        test.result(result).assert_no_lint("no-floating-promises");
    }

    #[test]
    fn test_allows_finally_handler_chain() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_allows_finally_handler_chain.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

load().finally(() => {});
"#,
        );
        test.result(result).assert_no_lint("no-floating-promises");
    }
}
