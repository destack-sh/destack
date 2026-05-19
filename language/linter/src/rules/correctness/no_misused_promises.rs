use destack_dir::{
    self as dir, LanguageItem, MatchSelector, NodeVisitor, NodeVisitorOptions, walk_expression,
    walk_match_case,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{
    expression_is_promise_like, expression_type_id, function_parameter_types_at,
    function_return_type, is_any_type, is_async_function_type, is_function_type,
    is_promise_or_any_type, supports_promise_spread_elements,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow Promise values in contexts that do not handle them.
    ///
    /// This catches Promise values used in conditionals and callback
    /// positions that expect synchronous values.
    #[lint(
        id = "no-misused-promises",
        code = "LC023",
        category = Correctness,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Promise)],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoMisusedPromises,
    "Disallow Promise values in contexts that do not handle them"
}

impl LintRule for NoMisusedPromises {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoMisusedPromises::meta()
    }

    /// Check module DIR nodes for Promise misuse in sync-only contexts.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = MisusedPromiseVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags misused Promise values.
struct MisusedPromiseVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Promise symbol.
    promise_symbol: dir::GlobalSymbolId,
    /// Whether to check conditionals.
    check_conditionals: bool,
    /// Whether to check callback positions.
    check_callbacks: bool,
    /// Whether to check spread positions.
    check_spreads: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> MisusedPromiseVisitor<'a, 'b> {
    /// Build a visitor for misused Promise checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let promise_symbol = ctx.language_item(LanguageItem::Promise);
        let check_conditionals = ctx
            .options
            .correctness
            .no_misused_promises_check_conditionals;
        let check_callbacks = ctx.options.correctness.no_misused_promises_check_callbacks;
        let check_spreads = ctx.options.correctness.no_misused_promises_check_spreads;

        Self {
            ctx,
            meta,
            promise_symbol,
            check_conditionals,
            check_callbacks,
            check_spreads,
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

    /// Return true when a type allows Promise values.
    fn type_allows_promise(&self, type_id: dir::LocalTypeId) -> bool {
        is_promise_or_any_type(self.ctx.types, type_id, Some(self.promise_symbol))
    }

    /// Return true when an expression type is Promise.
    fn is_promise_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_promise_like(
            self.ctx.module_id(),
            self.ctx.dir.tree(),
            self.ctx.types,
            self.promise_symbol,
            expression_id,
        )
    }

    /// Report a misused Promise diagnostic for a node.
    fn report<T: dir::Node>(
        &mut self,
        node_id: dir::LocalNodeId<T>,
        message: &'static str,
        label: &'static str,
    ) {
        let node_id_raw = node_id.id;

        // honor per node severity
        let severity = self
            .ctx
            .get_effective_severity(self.meta, dir::LocalNodeId::<T>::new(node_id_raw));
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(dir::LocalNodeId::<T>::new(node_id_raw));
        self.ctx.report(
            LintReport::new(
                NO_MISUSED_PROMISES.id,
                NO_MISUSED_PROMISES.code,
                NO_MISUSED_PROMISES.category,
                severity,
                message,
                span,
            )
            .label(label),
        );
    }

    /// Check a conditional expression for Promise misuse.
    fn check_conditional_expression<T: dir::Node>(
        &mut self,
        node_id: dir::LocalNodeId<T>,
        condition_id: dir::LocalNodeId<dir::Expression>,
    ) {
        if !self.check_conditionals {
            return;
        }
        if !self.is_promise_expression(condition_id) {
            return;
        }

        self.report(
            node_id,
            "Promise used in conditional expression",
            "await the Promise before using it in a conditional",
        );
    }

    /// Check call arguments for Promise misuse.
    fn check_call_arguments(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // resolve callee type once
        let Some(callee_type_id) = expression_type_id(
            self.ctx.module_id(),
            self.ctx.dir.tree(),
            self.ctx.types,
            callee_id,
        ) else {
            return;
        };

        // inspect indexed candidates
        for (index, argument_id) in arguments.iter().enumerate() {
            let argument = self.ctx.dir.get(*argument_id);
            let is_spread = matches!(argument, dir::Argument::Spread { .. });
            let Some(value_id) = argument.value() else {
                continue;
            };

            // require optional structure
            let Some(argument_type_id) = expression_type_id(
                self.ctx.module_id(),
                self.ctx.dir.tree(),
                self.ctx.types,
                value_id,
            ) else {
                continue;
            };
            let parameter_type_ids =
                function_parameter_types_at(self.ctx.types, callee_type_id, index);
            if parameter_type_ids.is_empty() {
                continue;
            }

            // spread Promise elements passed where Promise elements are not accepted
            if is_spread {
                if !self.check_spreads {
                    continue;
                }

                if supports_promise_spread_elements(
                    self.ctx.types,
                    argument_type_id,
                    Some(self.promise_symbol),
                ) && !parameter_type_ids.iter().any(|parameter_type_id| {
                    supports_promise_spread_elements(
                        self.ctx.types,
                        *parameter_type_id,
                        Some(self.promise_symbol),
                    )
                }) {
                    self.report(
                        *argument_id,
                        "Promise passed to a non-Promise parameter",
                        "await the Promise before passing it",
                    );
                }
                continue;
            }

            // skip non spread checks when callback analysis is disabled
            if !self.check_callbacks {
                continue;
            }

            // promise passed where Promise is not accepted
            if expression_is_promise_like(
                self.ctx.module_id(),
                self.ctx.dir.tree(),
                self.ctx.types,
                self.promise_symbol,
                value_id,
            ) && !parameter_type_ids
                .iter()
                .any(|parameter_type_id| self.type_allows_promise(*parameter_type_id))
            {
                self.report(
                    *argument_id,
                    "Promise passed to a non-Promise parameter",
                    "await the Promise before passing it",
                );
                continue;
            }

            // async callback passed where sync callback is expected
            if !is_async_function_type(self.ctx.types, argument_type_id) {
                continue;
            }
            let mut has_synchronous_callback_expectation = false;
            let mut allows_async_callback = false;

            // inspect candidate nodes
            for parameter_type_id in parameter_type_ids {
                if !is_function_type(self.ctx.types, parameter_type_id)
                    || is_any_type(self.ctx.types, parameter_type_id)
                {
                    continue;
                }

                has_synchronous_callback_expectation = true;
                let expected_return_type = function_return_type(self.ctx.types, parameter_type_id);
                if expected_return_type
                    .is_some_and(|return_type| self.type_allows_promise(return_type))
                {
                    allows_async_callback = true;
                    break;
                }
            }

            // enforce this lint guard
            if has_synchronous_callback_expectation && !allows_async_callback {
                self.report(
                    expression_id,
                    "async callback passed to a synchronous callback position",
                    "use a sync callback or handle Promises explicitly",
                );
            }
        }
    }
}

impl NodeVisitor for MisusedPromiseVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check Promise misuse in conditionals and callback positions
        match expression {
            dir::Expression::If {
                condition: dir::IfCondition::Expression { condition },
                ..
            } => {
                self.check_conditional_expression(id, *condition);
            }
            dir::Expression::For {
                condition: Some(condition),
                ..
            } => {
                self.check_conditional_expression(id, *condition);
            }
            dir::Expression::Call {
                left, arguments, ..
            }
            | dir::Expression::New {
                left, arguments, ..
            } => {
                self.check_call_arguments(id, *left, arguments);
            }
            _ => {}
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }

    fn visit_match_case(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::MatchCase>,
        match_case: &dir::MatchCase,
    ) {
        // check Promise misuse in match guards
        if self.check_conditionals {
            let selector = match match_case {
                dir::MatchCase::Expression { selector, .. } => selector,
                dir::MatchCase::Block { selector, .. } => selector,
            };
            if let MatchSelector::Pattern {
                guard: Some(guard), ..
            } = selector
            {
                self.check_conditional_expression(id, *guard);
            }
        }

        // walk match case children
        walk_match_case(self, tree, id, match_case);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_promise_in_conditional() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_flags_promise_in_conditional.ts",
            r#"
async function ready(): Promise<boolean> {
    return true;
}

if (ready()) {
    console.log("ready");
}
"#,
        );
        test.result(result).assert_lint("no-misused-promises");
    }

    #[test]
    fn test_flags_promise_value_argument() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_flags_promise_value_argument.ts",
            r#"
function takesNumber(value: number): void {}
async function load(): Promise<number> {
    return 1;
}

takesNumber(load());
"#,
        );
        test.result(result).assert_lint("no-misused-promises");
    }

    #[test]
    fn test_flags_promise_value_argument_for_rest_parameter() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_flags_promise_value_argument_for_rest_parameter.ts",
            r#"
function takesNumbers(...values: number[]): void {}
async function load(): Promise<number> {
    return 1;
}

takesNumbers(load());
"#,
        );
        test.result(result).assert_lint("no-misused-promises");
    }

    #[test]
    fn test_flags_promise_spread_argument_into_sync_parameters() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_flags_promise_spread_argument_into_sync_parameters.ts",
            r#"
function takesNumbers(...values: number[]): void {}
async function load(): Promise<number> {
    return 1;
}

const promisedNumbers = [load()];
takesNumbers(...promisedNumbers);
"#,
        );
        test.result(result).assert_lint("no-misused-promises");
    }

    #[test]
    fn test_allows_promise_spread_argument_into_promise_parameters() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_allows_promise_spread_argument_into_promise_parameters.ts",
            r#"
function takesPromises(...values: Promise<number>[]): void {}
async function load(): Promise<number> {
    return 1;
}

const promisedNumbers = [load()];
takesPromises(...promisedNumbers);
"#,
        );
        test.result(result).assert_no_lint("no-misused-promises");
    }

    #[test]
    fn test_allows_promise_spread_when_disabled() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises).with_options(|options| {
            options.correctness.no_misused_promises_check_spreads = false;
        });
        let result = test.lint_dir(
            "no_misused_promises/test_allows_promise_spread_when_disabled.ts",
            r#"
function takesNumbers(...values: number[]): void {}
async function load(): Promise<number> {
    return 1;
}

const promisedNumbers = [load()];
takesNumbers(...promisedNumbers);
"#,
        );
        test.result(result).assert_no_lint("no-misused-promises");
    }

    #[test]
    fn test_flags_async_callback_in_sync_position() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_flags_async_callback_in_sync_position.ts",
            r#"
function usePredicate(callback: (value: number) => boolean): boolean {
    return callback(1);
}

usePredicate(async (value: number) => value > 0);
"#,
        );
        test.result(result).assert_lint("no-misused-promises");
    }

    #[test]
    fn test_allows_promise_parameter() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_allows_promise_parameter.ts",
            r#"
function takesPromise(value: Promise<number>): void {}
async function load(): Promise<number> {
    return 1;
}

takesPromise(load());
"#,
        );
        test.result(result).assert_no_lint("no-misused-promises");
    }

    #[test]
    fn test_allows_conditional_when_disabled() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises).with_options(|options| {
            options.correctness.no_misused_promises_check_conditionals = false;
        });
        let result = test.lint_dir(
            "no_misused_promises/test_allows_conditional_when_disabled.ts",
            r#"
async function ready(): Promise<boolean> {
    return true;
}

if (ready()) {
    console.log("ready");
}
"#,
        );
        test.result(result).assert_no_lint("no-misused-promises");
    }

    #[test]
    fn test_allows_callback_when_disabled() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises).with_options(|options| {
            options.correctness.no_misused_promises_check_callbacks = false;
        });
        let result = test.lint_dir(
            "no_misused_promises/test_allows_callback_when_disabled.ts",
            r#"
function usePredicate(callback: (value: number) => boolean): boolean {
    return callback(1);
}

usePredicate(async (value: number) => value > 0);
"#,
        );
        test.result(result).assert_no_lint("no-misused-promises");
    }

    #[test]
    fn test_flags_promise_in_for_condition() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_flags_promise_in_for_condition.ts",
            r#"
async function ready(): Promise<boolean> {
    return true;
}

for (; ready(); ) {
    break;
}
"#,
        );
        test.result(result).assert_lint("no-misused-promises");
    }

    #[test]
    fn test_flags_promise_in_while_condition() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_flags_promise_in_while_condition.ts",
            r#"
async function ready(): Promise<boolean> {
    return true;
}

while (ready()) {
    break;
}
"#,
        );
        test.result(result).assert_lint("no-misused-promises");
    }

    #[test]
    fn test_allows_async_callback_when_promise_return_is_expected() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_allows_async_callback_when_promise_return_is_expected.ts",
            r#"
function useAsyncPredicate(callback: (value: number) => Promise<boolean>): Promise<boolean> {
    return callback(1);
}

useAsyncPredicate(async (value: number) => value > 0);
"#,
        );
        test.result(result).assert_no_lint("no-misused-promises");
    }

    #[test]
    fn test_allows_union_parameter_that_accepts_promise() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_allows_union_parameter_that_accepts_promise.ts",
            r#"
function takesNumberOrPromise(value: number | Promise<number>): void {}
async function load(): Promise<number> {
    return 1;
}

takesNumberOrPromise(load());
"#,
        );
        test.result(result).assert_no_lint("no-misused-promises");
    }

    #[test]
    fn test_allows_async_callback_with_overload_accepting_promise() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_allows_async_callback_with_overload_accepting_promise.ts",
            r#"
function usePredicate(callback: (value: number) => boolean): boolean;
function usePredicate(callback: (value: number) => Promise<boolean>): Promise<boolean>;
function usePredicate(callback: (value: number) => boolean | Promise<boolean>): boolean | Promise<boolean> {
    return callback(1);
}

usePredicate(async (value: number) => value > 0);
"#,
        );
        test.result(result).assert_no_lint("no-misused-promises");
    }

    #[test]
    fn test_flags_promise_in_match_guard() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedPromises);
        let result = test.lint_dir(
            "no_misused_promises/test_flags_promise_in_match_guard.ds",
            r#"
async function ready(): Promise<boolean> {
    return true;
}

match (1) {
    1 if ready() => "one"
    _ => "other"
}
"#,
        );
        test.result(result).assert_lint("no-misused-promises");
    }
}
