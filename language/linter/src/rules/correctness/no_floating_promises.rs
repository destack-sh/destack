use destack_dir::{
    self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, StringId, walk_expression,
};
use destack_repository::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{
    expression_is_promise_like, expression_is_standalone_statement,
    expression_unwrap_parenthesized, is_function_type, supports_promise_spread_elements,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

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
        requires_all = [RequireLanguageItem(LanguageItem::Promise)],
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
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = FloatingPromiseVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags floating Promise expressions.
struct FloatingPromiseVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Promise symbol.
    promise_symbol: dir::GlobalSymbolId,
    /// Whether explicit `void` should suppress the lint.
    ignore_void: bool,
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
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let promise_symbol = ctx.language_item(LanguageItem::Promise);
        let ignore_void = ctx.options().correctness.no_floating_promises_ignore_void;
        let then_name = ctx.string_id("then");
        let catch_name = ctx.string_id("catch");
        let finally_name = ctx.string_id("finally");

        Self {
            ctx,
            meta,
            promise_symbol,
            ignore_void,
            then_name,
            catch_name,
            finally_name,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Return true when an expression produces a Promise value.
    fn is_promise_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
        let expression = self.ctx.dir.get(expression_id);

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
            let left_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), *left);
            let left_expression = self.ctx.dir.get(left_id);
            if let dir::Expression::Member {
                left: receiver,
                name,
                ..
            } = left_expression
                && (*name == Some(self.then_name)
                    || *name == Some(self.catch_name)
                    || *name == Some(self.finally_name))
            {
                return self.is_promise_expression(*receiver);
            }
        }

        expression_is_promise_like(self.ctx, self.promise_symbol, expression_id)
    }

    /// Return true when an expression produces an array or tuple of Promises.
    fn is_promise_array_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
        let expression = self.ctx.dir.get(expression_id);

        // preserve Promise array checks for explicit void discards
        if let dir::Expression::Unary {
            operator: dir::UnaryOperator::Void,
            right,
        } = expression
        {
            return self.is_promise_array_expression(*right);
        }

        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        supports_promise_spread_elements(self.ctx, type_id, Some(self.promise_symbol))
    }

    /// Return true when a call is a Promise handler chain.
    fn is_handler_call(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
        let expression = self.ctx.dir.get(expression_id);
        let dir::Expression::Call {
            left, arguments, ..
        } = expression
        else {
            return false;
        };

        // resolve left id
        let left_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), *left);
        let left_expression = self.ctx.dir.get(left_id);
        let dir::Expression::Member { name, .. } = left_expression else {
            return false;
        };

        // `.catch(...)` and `.finally(...)` always handle rejection paths
        if *name == Some(self.catch_name) || *name == Some(self.finally_name) {
            return true;
        }

        // `.then(...)` only handles rejection when a function rejection handler is provided
        if *name != Some(self.then_name) {
            return false;
        }

        // require optional structure
        let Some(second_argument_id) = arguments.get(1) else {
            return false;
        };
        let second_argument = self.ctx.dir.get(*second_argument_id);
        let Some(handler_id) = second_argument.value() else {
            return false;
        };
        let Some(handler_type_id) = self.ctx.expression_type_id(handler_id) else {
            return false;
        };

        is_function_type(self.ctx, handler_type_id)
    }

    /// Return true when the Promise expression is handled.
    fn is_handled_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
        let expression = self.ctx.dir.get(expression_id);

        // branch by expression kind
        match expression {
            dir::Expression::Await { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. } => true,
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Void,
                ..
            } => self.ignore_void,
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
        let is_promise_expression = self.is_promise_expression(inner_id);
        let is_promise_array_expression = self.is_promise_array_expression(inner_id);

        if !is_promise_expression && !is_promise_array_expression {
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
        let mut diagnostic = LintReport::new(
            NO_FLOATING_PROMISES.id,
            NO_FLOATING_PROMISES.code,
            NO_FLOATING_PROMISES.category,
            severity,
            if is_promise_array_expression {
                "array of Promise results is ignored"
            } else {
                "Promise result is ignored"
            },
            span,
        )
        .label(if is_promise_array_expression {
            "await Promise.all(...), return it, or handle the Promises explicitly"
        } else {
            "await, return, or attach a Promise handler"
        });

        // compute fixes only when requested by the runner
        if self.ctx.compute_fixes
            && let Some(fix) = self.no_floating_promises_fix(inner_id)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix by explicitly discarding Promise results with `void`.
    fn no_floating_promises_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintFix> {
        // this fix only applies when explicit void discard is accepted
        if !self.ignore_void {
            return None;
        }

        // do not stack `void` on existing explicit void expressions
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
        let expression = self.ctx.dir.get(expression_id);
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
        let normalized_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
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
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check expression statements
        if expression_is_standalone_statement(tree, id) {
            self.check_statement(id, id);
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
            options.correctness.no_floating_promises_ignore_void = false;
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
    fn test_flags_array_of_promises_statement() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_flags_array_of_promises_statement.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

[load()];
"#,
        );
        test.result(result).assert_lint("no-floating-promises");
    }

    #[test]
    fn test_allows_void_array_of_promises_when_enabled() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_allows_void_array_of_promises_when_enabled.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

void [load()];
"#,
        );
        test.result(result).assert_no_lint("no-floating-promises");
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
    fn test_fix_prefixes_array_of_promises_with_void() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises);
        let result = test.lint_dir(
            "no_floating_promises/test_fix_prefixes_array_of_promises_with_void.ts",
            r#"
async function load(): Promise<number> {
    return 1;
}

[load()];
"#,
        );
        test.result(result)
            .assert_lint("no-floating-promises")
            .assert_safe_fixed(
                r#"
async function load(): Promise<number> {
    return 1;
}

void [load()];
"#,
            );
    }

    #[test]
    fn test_no_fix_when_void_discard_is_disabled() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPromises).with_options(|options| {
            options.correctness.no_floating_promises_ignore_void = false;
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
