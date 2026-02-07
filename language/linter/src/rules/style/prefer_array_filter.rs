use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{expression_method_call, is_array_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest `.filter()` over `forEach` with conditional push.
    ///
    /// Using `filter` is more declarative and avoids manual array mutation.
    #[lint(
        id = "prefer-array-filter",
        code = "LY083",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArrayFilter,
    "Prefer filter() over forEach with conditional push"
}

impl LintRule for PreferArrayFilter {
    fn meta(&self) -> &'static LintMeta {
        PreferArrayFilter::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferArrayFilterVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags forEach with conditional push patterns.
struct PreferArrayFilterVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The array symbol for this module profile.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the forEach method name.
    for_each_name: StringId,
    /// The string id for the push method name.
    push_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferArrayFilterVisitor<'a, 'b> {
    /// Build a visitor for prefer-array-filter checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let for_each_name = ctx.program.strings.intern("forEach");
        let push_name = ctx.program.strings.intern("push");

        Self {
            ctx,
            meta,
            array_symbol,
            for_each_name,
            push_name,
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

    /// Check if this is a forEach with conditional push pattern.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match forEach call
        let Some(call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };
        if call.method_name != self.for_each_name {
            return;
        }

        // check receiver is an array
        if !self.is_array_receiver(call.receiver_id) {
            return;
        }

        // get the callback argument
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            dynamic_arguments, ..
        } = expression
        else {
            return;
        };
        if dynamic_arguments.is_empty() {
            return;
        }

        // get callback expression
        let callback_arg = self.ctx.tree.get(dynamic_arguments[0]);
        let callback_id = callback_arg.value();
        let callback = self.ctx.tree.get(callback_id);

        // match function expression (arrow functions and regular functions are Declaration::Function)
        let body_id = match callback {
            dir::Expression::Declaration { declaration } => {
                let decl = self.ctx.tree.get(*declaration);
                let dir::Declaration::Function { body, .. } = decl else {
                    return;
                };
                let Some(body) = body else {
                    return;
                };
                *body
            }
            _ => return,
        };

        // check if callback body matches the conditional push pattern
        if self.is_conditional_push_body(body_id) {
            self.report(expression_id);
        }
    }

    /// Check if the callback body is a conditional push pattern.
    fn is_conditional_push_body(&self, body_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let body = self.ctx.tree.get(body_id);

        // body can be a block or an if expression directly
        match body {
            // block body: check for single if statement
            dir::Expression::Block { block } => {
                let block_node = self.ctx.tree.get(*block);
                // should have exactly one statement
                if block_node.expressions.len() != 1 {
                    return false;
                }
                self.is_conditional_push_expression(block_node.expressions[0])
            }
            // direct if expression
            dir::Expression::If { .. } => self.is_conditional_push_expression(body_id),
            _ => false,
        }
    }

    /// Unwrap statement expressions to get the inner expression.
    fn unwrap_statement(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> dir::LocalNodeId<dir::Expression> {
        let expression = self.ctx.tree.get(expression_id);
        if let dir::Expression::Statement { statement } = expression {
            *statement
        } else {
            expression_id
        }
    }

    /// Check if expression is an if with push in the body.
    fn is_conditional_push_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        // unwrap statement if present
        let expression_id = self.unwrap_statement(expression_id);
        let expression = self.ctx.tree.get(expression_id);

        // match if expression without else
        let dir::Expression::If {
            then_expression,
            else_expression,
            ..
        } = expression
        else {
            return false;
        };

        // should not have else branch (pure filter pattern)
        if else_expression.is_some() {
            return false;
        }

        // check the then block contains only a push call
        self.is_push_only_block(*then_expression)
    }

    /// Check if block contains only a push call.
    fn is_push_only_block(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression = self.ctx.tree.get(expression_id);

        // match block
        let dir::Expression::Block { block } = expression else {
            // direct push call without block
            return self.is_push_call(expression_id);
        };

        let block_node = self.ctx.tree.get(*block);

        // should have exactly one expression
        if block_node.expressions.len() != 1 {
            return false;
        }

        self.is_push_call(block_node.expressions[0])
    }

    /// Check if expression is a push call.
    fn is_push_call(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // unwrap statement if present
        let expression_id = self.unwrap_statement(expression_id);
        let Some(call) = expression_method_call(self.ctx.tree, expression_id) else {
            return false;
        };

        call.method_name == self.push_name
    }

    /// Return true when the receiver expression is an array type.
    fn is_array_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_array_type(self.ctx.types, type_id, Some(self.array_symbol))
    }

    /// Report a prefer-array-filter match.
    fn report(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                PREFER_ARRAY_FILTER.id,
                PREFER_ARRAY_FILTER.code,
                PREFER_ARRAY_FILTER.category,
                severity,
                "prefer filter() over forEach with conditional push",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use array.filter(...) instead"),
        );
    }
}

impl NodeVisitor for PreferArrayFilterVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_call(id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag forEach with conditional push.
    #[test]
    fn test_flags_foreach_conditional_push() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFilter);
        let result = test.lint_dir(
            "prefer_array_filter/test_flags_foreach_conditional_push.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => {
    if (x > 1) {
        result.push(x);
    }
});
"#,
        );
        test.result(result).assert_lint("prefer-array-filter");
    }

    /// Flag arrow function with block body.
    #[test]
    fn test_flags_arrow_block_body() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFilter);
        let result = test.lint_dir(
            "prefer_array_filter/test_flags_arrow_block_body.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => { if (x > 1) { result.push(x) } });
"#,
        );
        test.result(result).assert_lint("prefer-array-filter");
    }

    /// Allow forEach with unconditional push (that's prefer-array-map).
    #[test]
    fn test_allows_unconditional_push() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFilter);
        let result = test.lint_dir(
            "prefer_array_filter/test_allows_unconditional_push.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => {
    result.push(x * 2);
});
"#,
        );
        test.result(result).assert_no_lint("prefer-array-filter");
    }

    /// Allow forEach with if-else (not a simple filter).
    #[test]
    fn test_allows_if_else() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFilter);
        let result = test.lint_dir(
            "prefer_array_filter/test_allows_if_else.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => {
    if (x > 1) {
        result.push(x);
    } else {
        result.push(0);
    }
});
"#,
        );
        test.result(result).assert_no_lint("prefer-array-filter");
    }

    /// Allow forEach with ternary (has implicit else).
    #[test]
    fn test_allows_ternary() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFilter);
        let result = test.lint_dir(
            "prefer_array_filter/test_allows_ternary.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => x > 1 ? result.push(x) : undefined);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-filter");
    }

    /// Allow forEach with multiple statements (complex logic).
    #[test]
    fn test_allows_multiple_statements() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFilter);
        let result = test.lint_dir(
            "prefer_array_filter/test_allows_multiple_statements.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => {
    let doubled = x * 2;
    if (doubled > 2) {
        result.push(doubled);
    }
});
"#,
        );
        test.result(result).assert_no_lint("prefer-array-filter");
    }

    /// Allow direct filter usage.
    #[test]
    fn test_allows_filter_directly() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayFilter);
        let result = test.lint_dir(
            "prefer_array_filter/test_allows_filter_directly.ds",
            r#"
let items = [1, 2, 3];
let result = items.filter(x => x > 1);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-filter");
    }
}
