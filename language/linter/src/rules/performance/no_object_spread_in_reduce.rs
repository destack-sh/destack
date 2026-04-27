use destack_core::StringId;
use destack_dir::{
    self as dir, Declaration, NodeVisitor, NodeVisitorOptions, Property, WellKnownSymbol,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{expression_method_call, is_array_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow object spread in reduce accumulators.
    ///
    /// Using object spread in a reduce accumulator like `{ ...acc, [key]: value }`
    /// causes O(n²) allocations because a new object is created on each iteration.
    /// Use direct mutation with assignment instead.
    #[lint(
        id = "no-object-spread-in-reduce",
        code = "LP008",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoObjectSpreadInReduce,
    "Disallow object spread in reduce accumulators (O(n²))"
}

impl LintRule for NoObjectSpreadInReduce {
    fn meta(&self) -> &'static LintMeta {
        NoObjectSpreadInReduce::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoObjectSpreadInReduceVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags object spread in reduce accumulators.
struct NoObjectSpreadInReduceVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the reduce method name.
    reduce_name: StringId,
    /// The string id for the reduceRight method name.
    reduce_right_name: StringId,
    /// The global Object symbol for this module, when available.
    object_symbol: Option<dir::GlobalSymbolId>,
    /// The string id for the assign method name.
    assign_name: StringId,
    /// The accumulator symbol when inside a reduce callback.
    accumulator_symbol: Option<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoObjectSpreadInReduceVisitor<'a, 'b> {
    /// Build a visitor for no-object-spread-in-reduce checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let reduce_name = ctx.repository.strings.intern("reduce");
        let reduce_right_name = ctx.repository.strings.intern("reduceRight");
        let object_name = ctx.repository.strings.intern("Object");
        let object_symbol = ctx.get_declared_library_symbol(object_name);
        let assign_name = ctx.repository.strings.intern("assign");

        Self {
            ctx,
            meta,
            array_symbol,
            reduce_name,
            reduce_right_name,
            object_symbol,
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

        // extract the accumulator parameter from function declaration
        let local_symbol = match callback {
            dir::Expression::Declaration(declaration) => {
                let decl = tree.get(*declaration);
                match decl {
                    Declaration::Function(declaration) => {
                        if declaration.signature.parameters.is_empty() {
                            return;
                        }
                        let param = tree.get(declaration.signature.parameters[0]);
                        param.symbol()
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

        // visit the callback
        self.visit_expression(tree, callback_id, callback);

        // restore previous context
        self.accumulator_symbol = previous_accumulator;
    }

    /// Check for spread of the accumulator in object expressions.
    fn check_spread(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // only check when we have an accumulator context
        let Some(accumulator_symbol) = self.accumulator_symbol else {
            return;
        };

        // match object expression with spread
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

            // check if this spreads the accumulator
            let spread_expr = self.ctx.tree.get(*value);
            let target_symbol = spread_expr.target_symbol();
            if target_symbol != Some(accumulator_symbol) {
                continue;
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
                    NO_OBJECT_SPREAD_IN_REDUCE.id,
                    NO_OBJECT_SPREAD_IN_REDUCE.code,
                    NO_OBJECT_SPREAD_IN_REDUCE.category,
                    severity,
                    "object spread in reduce causes O(n²) allocations",
                    self.ctx.module.file_id,
                    span,
                )
                .with_label("use direct assignment with mutation instead"),
            );

            return;
        }
    }

    /// Check for `Object.assign({}, acc, ...)` inside reduce callbacks.
    fn check_object_assign(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // only check when we have an accumulator context
        let Some(accumulator_symbol) = self.accumulator_symbol else {
            return;
        };

        // require Object symbol availability
        let Some(object_symbol) = self.object_symbol else {
            return;
        };

        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };
        if method_call.method_name != self.assign_name {
            return;
        }

        // require Object.assign(...)
        let receiver_expression = self.ctx.tree.get(method_call.receiver_id);
        if receiver_expression.target_symbol() != Some(object_symbol) {
            return;
        }

        // require at least two arguments and an empty object seed
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call { arguments, .. } = expression else {
            return;
        };
        if arguments.len() < 2 {
            return;
        }

        let first_argument = self.ctx.tree.get(arguments[0]);
        let dir::Argument::Positional {
            value: first_value, ..
        } = first_argument
        else {
            return;
        };
        let first_value_expression = self.ctx.tree.get(*first_value);
        let dir::Expression::ObjectExpression { ty: _, properties } = first_value_expression else {
            return;
        };
        if !properties.is_empty() {
            return;
        }

        // require accumulator usage in following arguments
        let mut has_accumulator_argument = false;
        for argument_id in arguments.iter().skip(1) {
            let argument = self.ctx.tree.get(*argument_id);
            let dir::Argument::Positional { value, .. } = argument else {
                continue;
            };
            let value_expression = self.ctx.tree.get(*value);
            if value_expression.target_symbol() == Some(accumulator_symbol) {
                has_accumulator_argument = true;
                break;
            }
        }
        if !has_accumulator_argument {
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
                NO_OBJECT_SPREAD_IN_REDUCE.id,
                NO_OBJECT_SPREAD_IN_REDUCE.code,
                NO_OBJECT_SPREAD_IN_REDUCE.category,
                severity,
                "Object.assign with accumulator clone in reduce causes O(n²) allocations",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use direct assignment with mutation instead"),
        );
    }
}

impl NodeVisitor for NoObjectSpreadInReduceVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for reduce calls to enter reduce context
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_reduce_call(tree, id);
            self.check_object_assign(id);
            // still walk children in case of nested reduces
        }

        // check for object spread
        if matches!(expression, dir::Expression::ObjectExpression { .. }) {
            self.check_spread(id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag object spread in reduce.
    #[test]
    fn test_flags_object_spread_in_reduce() {
        let test = TestProgram::for_rule_without_prelude(NoObjectSpreadInReduce);
        let result = test.lint_dir(
            "no_object_spread_in_reduce/test_flags_object_spread_in_reduce.ds",
            r#"
let items = [{ id: 1 }, { id: 2 }];
let byId = items.reduce((acc, item) => ({ ...acc, [item.id]: item }), {});
"#,
        );
        test.result(result)
            .assert_lint("no-object-spread-in-reduce");
    }

    /// Flag object spread in reduceRight accumulators.
    #[test]
    fn test_flags_object_spread_in_reduce_right() {
        let test = TestProgram::for_rule_without_prelude(NoObjectSpreadInReduce);
        let result = test.lint_dir(
            "no_object_spread_in_reduce/test_flags_object_spread_in_reduce_right.ds",
            r#"
let items = [{ id: 1 }, { id: 2 }];
let byId = items.reduceRight((acc, item) => ({ ...acc, [item.id]: item }), {});
"#,
        );
        test.result(result)
            .assert_lint("no-object-spread-in-reduce");
    }

    /// Flag object spread with additional properties.
    #[test]
    fn test_flags_spread_with_properties() {
        let test = TestProgram::for_rule_without_prelude(NoObjectSpreadInReduce);
        let result = test.lint_dir(
            "no_object_spread_in_reduce/test_flags_spread_with_properties.ds",
            r#"
let items = ["a", "b", "c"];
let counts = items.reduce((acc, item) => ({ ...acc, [item]: (acc[item] || 0) + 1 }), {});
"#,
        );
        test.result(result)
            .assert_lint("no-object-spread-in-reduce");
    }

    /// Flag spread in block body.
    #[test]
    fn test_flags_spread_in_block_body() {
        let test = TestProgram::for_rule_without_prelude(NoObjectSpreadInReduce);
        let result = test.lint_dir(
            "no_object_spread_in_reduce/test_flags_spread_in_block_body.ds",
            r#"
let items = [{ id: 1 }, { id: 2 }];
let byId = items.reduce((acc, item) => {
    return { ...acc, [item.id]: item };
}, {});
"#,
        );
        test.result(result)
            .assert_lint("no-object-spread-in-reduce");
    }

    /// Flag Object.assign accumulator clones in reduce.
    #[test]
    fn test_flags_object_assign_clone_in_reduce() {
        let test = TestProgram::for_rule_with_prelude(NoObjectSpreadInReduce);
        let result = test.lint_dir(
            "no_object_spread_in_reduce/test_flags_object_assign_clone_in_reduce.ds",
            r#"
let items = [{ id: 1 }, { id: 2 }];
let byId = items.reduce((acc, item) => Object.assign({}, acc, { [item.id]: item }), {});
"#,
        );
        test.result(result)
            .assert_lint("no-object-spread-in-reduce");
    }

    /// Allow direct assignment in reduce.
    #[test]
    fn test_allows_direct_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoObjectSpreadInReduce);
        let result = test.lint_dir(
            "no_object_spread_in_reduce/test_allows_direct_assignment.ds",
            r#"
let items = [{ id: 1 }, { id: 2 }];
let byId = items.reduce((acc, item) => {
    acc[item.id] = item;
    return acc;
}, {});
"#,
        );
        test.result(result)
            .assert_no_lint("no-object-spread-in-reduce");
    }

    /// Allow object spread outside reduce.
    #[test]
    fn test_allows_spread_outside_reduce() {
        let test = TestProgram::for_rule_without_prelude(NoObjectSpreadInReduce);
        let result = test.lint_dir(
            "no_object_spread_in_reduce/test_allows_spread_outside_reduce.ds",
            r#"
let base = { a: 1 };
let extended = { ...base, b: 2 };
"#,
        );
        test.result(result)
            .assert_no_lint("no-object-spread-in-reduce");
    }

    /// Allow Object.fromEntries instead.
    #[test]
    fn test_allows_from_entries() {
        let test = TestProgram::for_rule_without_prelude(NoObjectSpreadInReduce);
        let result = test.lint_dir(
            "no_object_spread_in_reduce/test_allows_from_entries.ds",
            r#"
let items = [{ id: 1, name: "a" }, { id: 2, name: "b" }];
let byId = Object.fromEntries(items.map((item) => [item.id, item]));
"#,
        );
        test.result(result)
            .assert_no_lint("no-object-spread-in-reduce");
    }
}
