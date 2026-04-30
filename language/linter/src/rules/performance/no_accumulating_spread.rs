use destack_core::StringId;
use destack_dir::{
    self as dir, Argument, Declaration, NodeVisitor, NodeVisitorOptions, Property, WellKnownSymbol,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_enters_nested_declaration_scope, expression_method_call, expression_target_symbol,
    is_array_type,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow spreading in accumulators.
    ///
    /// Using spread in a reduce accumulator like `[...acc, x]` causes O(n²)
    /// allocations because a new array is created on each iteration. Use
    /// `push` with mutation instead.
    #[lint(
        id = "no-accumulating-spread",
        code = "LP001",
        category = Performance,
        level = Dir,
        requires_all = [
            RequireWellKnownSymbol(WellKnownSymbol::Array),
            RequireWellKnownSymbol(WellKnownSymbol::Object)
        ],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoAccumulatingSpread,
    "Disallow spreading in accumulators (O(n²))"
}

impl LintRule for NoAccumulatingSpread {
    fn meta(&self) -> &'static LintMeta {
        NoAccumulatingSpread::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoAccumulatingSpreadVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags spread in reduce accumulators.
struct NoAccumulatingSpreadVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The well known Object symbol for this module.
    object_symbol: dir::GlobalSymbolId,
    /// The string id for the reduce method name.
    reduce_name: StringId,
    /// The string id for the reduceRight method name.
    reduce_right_name: StringId,
    /// The string id for the assign method name.
    assign_name: StringId,
    /// The accumulator symbol when inside a reduce callback.
    accumulator_symbol: Option<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoAccumulatingSpreadVisitor<'a, 'b> {
    /// Build a visitor for no-accumulating-spread checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let object_symbol = ctx.well_known_symbol(WellKnownSymbol::Object);
        let reduce_name = ctx.string_id("reduce");
        let reduce_right_name = ctx.string_id("reduceRight");
        let assign_name = ctx.string_id("assign");

        Self {
            ctx,
            meta,
            array_symbol,
            object_symbol,
            reduce_name,
            reduce_right_name,
            assign_name,
            accumulator_symbol: None,
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

    /// Check if a call is a reduce on an array and enter reduce context.
    fn check_reduce_call(
        &mut self,
        tree: &dir::Tree,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // match method call pattern
        let Some(method_call) = expression_method_call(tree, expression_id) else {
            return;
        };

        // check if this is reduce or reduceRight
        if method_call.method_name != self.reduce_name
            && method_call.method_name != self.reduce_right_name
        {
            return;
        }

        // check if the receiver is an array
        let Some(type_id) = self.ctx.expression_type_id(method_call.receiver_id) else {
            return;
        };
        if !is_array_type(self.ctx.types, type_id, Some(self.array_symbol)) {
            return;
        }

        // get the callback argument
        let expression = tree.get(expression_id);
        let dir::Expression::Call { arguments, .. } = expression else {
            return;
        };
        if arguments.is_empty() {
            return;
        }

        // get the callback expression
        let callback_arg = tree.get(arguments[0]);
        let callback_id = callback_arg.value();
        let callback = tree.get(callback_id);

        // extract the accumulator parameter and callback body from function declaration
        let (local_symbol, callback_body_id) = match callback {
            dir::Expression::Declaration(declaration) => {
                let decl = tree.get(*declaration);
                match decl {
                    Declaration::Function(declaration) => {
                        let Some(body_id) = declaration.body else {
                            return;
                        };
                        if declaration.signature.parameters.is_empty() {
                            return;
                        }

                        let param = tree.get(declaration.signature.parameters[0]);
                        (param.symbol(), body_id)
                    }
                    _ => return,
                }
            }
            _ => return,
        };

        // set the accumulator context and visit the callback body
        let previous_accumulator = self.accumulator_symbol;
        self.accumulator_symbol =
            Some(dir::GlobalSymbolId::new(self.ctx.module_id(), local_symbol));

        // visit the callback body with accumulator context
        let callback_body = tree.get(callback_body_id);
        self.visit_expression(tree, callback_body_id, callback_body);

        // restore previous context
        self.accumulator_symbol = previous_accumulator;
    }

    /// Check for accumulator spread in array literals.
    fn check_array_spread(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // only check when we have an accumulator context
        let Some(accumulator_symbol) = self.accumulator_symbol else {
            return;
        };

        // match array expression with spread
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::ArrayExpression { elements } = expression else {
            return;
        };

        // check each element for spread of the accumulator
        for element_id in elements {
            let element = self.ctx.tree.get(*element_id);
            let Argument::Spread { value, .. } = element else {
                continue;
            };

            // check if this spreads the accumulator
            let spread_expr = self.ctx.tree.get(*value);
            let target_symbol = spread_expr.target_symbol();
            if target_symbol != Some(accumulator_symbol) {
                continue;
            }

            // report the accumulating spread diagnostic
            self.report_accumulator_diagnostic(
                expression_id,
                "spreading accumulator causes O(n²) allocations",
            );
            return;
        }
    }

    /// Check for accumulator spread in object literals.
    fn check_object_spread(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // only check when we have an accumulator context
        let Some(accumulator_symbol) = self.accumulator_symbol else {
            return;
        };

        // match object expression with properties
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::ObjectExpression { ty: _, properties } = expression else {
            return;
        };

        // check each property for spread of the accumulator
        for property_id in properties {
            let property = self.ctx.tree.get(*property_id);
            let Property::Spread { value, .. } = property else {
                continue;
            };

            let target_symbol = expression_target_symbol(self.ctx.tree, *value);
            if target_symbol != Some(accumulator_symbol) {
                continue;
            }

            // report the accumulating spread diagnostic
            self.report_accumulator_diagnostic(
                expression_id,
                "spreading accumulator causes O(n²) allocations",
            );
            return;
        }
    }

    /// Check for `Object.assign` accumulator cloning in reduce callbacks.
    fn check_object_assign(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // only check when we have an accumulator context
        let Some(accumulator_symbol) = self.accumulator_symbol else {
            return;
        };

        // match object assign method calls
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };
        if method_call.method_name != self.assign_name {
            return;
        }

        let receiver_symbol = expression_target_symbol(self.ctx.tree, method_call.receiver_id);
        if receiver_symbol != Some(self.object_symbol) {
            return;
        }

        // require at least two arguments to inspect the source object
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call { arguments, .. } = expression else {
            return;
        };
        if arguments.len() < 2 {
            return;
        }

        // match Object.assign(target, accumulator, ...)
        let source_argument = self.ctx.tree.get(arguments[1]);
        let source_expression_id = source_argument.value();
        let source_symbol = expression_target_symbol(self.ctx.tree, source_expression_id);
        if source_symbol != Some(accumulator_symbol) {
            return;
        }

        // report the accumulating Object.assign diagnostic
        self.report_accumulator_diagnostic(
            expression_id,
            "Object.assign with accumulator causes O(n²) allocations",
        );
    }

    /// Report a no-accumulating-spread diagnostic for one expression.
    fn report_accumulator_diagnostic(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        message: &'static str,
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report one diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_ACCUMULATING_SPREAD.id,
                NO_ACCUMULATING_SPREAD.code,
                NO_ACCUMULATING_SPREAD.category,
                severity,
                message,
                self.ctx.module.file_id,
                span,
            )
            .with_label("use in-place mutation instead"),
        );
    }
}

impl NodeVisitor for NoAccumulatingSpreadVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // avoid leaking reduce accumulator context into nested declarations
        if self.accumulator_symbol.is_some()
            && expression_enters_nested_declaration_scope(tree, expression)
        {
            return;
        }

        // check for reduce calls to enter reduce context
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_reduce_call(tree, id);
            self.check_object_assign(id);
        }

        // check for array spread
        if matches!(expression, dir::Expression::ArrayExpression { .. }) {
            self.check_array_spread(id);
        }

        // check for object spread
        if matches!(expression, dir::Expression::ObjectExpression { .. }) {
            self.check_object_spread(id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag spread accumulator in reduce.
    #[test]
    fn test_flags_spread_in_reduce() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_flags_spread_in_reduce.ds",
            r#"
let items = [1, 2, 3];
let doubled = items.reduce((acc, x) => [...acc, x * 2], []);
"#,
        );
        test.result(result).assert_lint("no-accumulating-spread");
    }

    /// Flag spread accumulator in reduceRight.
    #[test]
    fn test_flags_spread_in_reduce_right() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_flags_spread_in_reduce_right.ds",
            r#"
let items = [1, 2, 3];
let doubled = items.reduceRight((acc, x) => [...acc, x * 2], []);
"#,
        );
        test.result(result).assert_lint("no-accumulating-spread");
    }

    /// Flag spread accumulator at end of array.
    #[test]
    fn test_flags_spread_at_end() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_flags_spread_at_end.ds",
            r#"
let items = [1, 2, 3];
let reversed = items.reduce((acc, x) => [x, ...acc], []);
"#,
        );
        test.result(result).assert_lint("no-accumulating-spread");
    }

    /// Flag spread in arrow function with block body.
    #[test]
    fn test_flags_spread_in_block_body() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_flags_spread_in_block_body.ds",
            r#"
let items = [1, 2, 3];
let doubled = items.reduce((acc, x) => {
    return [...acc, x * 2];
}, []);
"#,
        );
        test.result(result).assert_lint("no-accumulating-spread");
    }

    /// Allow push mutation in reduce.
    #[test]
    fn test_allows_push_mutation() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_allows_push_mutation.ds",
            r#"
let items = [1, 2, 3];
let doubled = items.reduce((acc, x) => {
    acc.push(x * 2);
    return acc;
}, []);
"#,
        );
        test.result(result).assert_no_lint("no-accumulating-spread");
    }

    /// Allow spread outside reduce.
    #[test]
    fn test_allows_spread_outside_reduce() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_allows_spread_outside_reduce.ds",
            r#"
let items = [1, 2, 3];
let more = [0, ...items, 4];
"#,
        );
        test.result(result).assert_no_lint("no-accumulating-spread");
    }

    /// Allow map instead of reduce with spread.
    #[test]
    fn test_allows_map() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_allows_map.ds",
            r#"
let items = [1, 2, 3];
let doubled = items.map((x) => x * 2);
"#,
        );
        test.result(result).assert_no_lint("no-accumulating-spread");
    }

    /// Flag spread accumulator in object reduce.
    #[test]
    fn test_flags_object_spread_in_reduce() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_flags_object_spread_in_reduce.ds",
            r#"
let items = ["a", "b", "c"];
let mapping = items.reduce((acc, value) => ({ ...acc, [value]: true }), {});
"#,
        );
        test.result(result).assert_lint("no-accumulating-spread");
    }

    /// Flag Object.assign accumulator cloning in reduce.
    #[test]
    fn test_flags_object_assign_accumulator_clone() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_flags_object_assign_accumulator_clone.ds",
            r#"
let items = ["a", "b", "c"];
let mapping = items.reduce((acc, value) => Object.assign({}, acc, { [value]: true }), {});
"#,
        );
        test.result(result).assert_lint("no-accumulating-spread");
    }

    /// Allow Object.assign mutation style reducers.
    #[test]
    fn test_allows_object_assign_mutation() {
        let test = TestProgram::for_rule_without_prelude(NoAccumulatingSpread);
        let result = test.lint_dir(
            "no_accumulating_spread/test_allows_object_assign_mutation.ds",
            r#"
let items = ["a", "b", "c"];
let mapping = items.reduce((acc, value) => Object.assign(acc, { [value]: true }), {});
"#,
        );
        test.result(result).assert_no_lint("no-accumulating-spread");
    }
}
