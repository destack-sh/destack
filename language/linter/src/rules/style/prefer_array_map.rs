use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{expression_method_call, is_array_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest `.map()` over `forEach` with push.
    ///
    /// Using `map` is more declarative and avoids manual array mutation.
    #[lint(
        id = "prefer-array-map",
        code = "LY085",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArrayMap,
    "Prefer map() over forEach with push"
}

impl LintRule for PreferArrayMap {
    fn meta(&self) -> &'static LintMeta {
        PreferArrayMap::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferArrayMapVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags forEach with unconditional push patterns.
struct PreferArrayMapVisitor<'a, 'b> {
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

impl<'a, 'b> PreferArrayMapVisitor<'a, 'b> {
    /// Build a visitor for prefer-array-map checks.
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

    /// Check if this is a forEach with unconditional push pattern.
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

        // check if callback body matches the unconditional push pattern
        if self.is_unconditional_push_body(body_id) {
            self.report(expression_id);
        }
    }

    /// Check if the callback body is an unconditional push pattern.
    fn is_unconditional_push_body(&self, body_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let body = self.ctx.tree.get(body_id);

        // body can be a block or a push call directly
        match body {
            // block body: check for single push statement
            dir::Expression::Block { block } => {
                let block_node = self.ctx.tree.get(*block);
                // should have exactly one statement
                if block_node.expressions.len() != 1 {
                    return false;
                }
                self.is_push_call(block_node.expressions[0])
            }
            // direct push call (concise arrow body)
            dir::Expression::Call { .. } => self.is_push_call(body_id),
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

    /// Report a prefer-array-map match.
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
                PREFER_ARRAY_MAP.id,
                PREFER_ARRAY_MAP.code,
                PREFER_ARRAY_MAP.category,
                severity,
                "prefer map() over forEach with push",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use array.map(...) instead"),
        );
    }
}

impl NodeVisitor for PreferArrayMapVisitor<'_, '_> {
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

    /// Flag forEach with unconditional push.
    #[test]
    fn test_flags_foreach_unconditional_push() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => {
    result.push(x * 2);
});
"#,
        );
        test.result(result).assert_lint("prefer-array-map");
    }

    /// Flag arrow function with block body.
    #[test]
    fn test_flags_arrow_block_body() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => { result.push(x * 2) });
"#,
        );
        test.result(result).assert_lint("prefer-array-map");
    }

    /// Allow forEach with conditional push (that's prefer-array-filter).
    #[test]
    fn test_allows_conditional_push() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "test.ds",
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
        test.result(result).assert_no_lint("prefer-array-map");
    }

    /// Allow forEach with multiple statements (complex logic).
    #[test]
    fn test_allows_multiple_statements() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => {
    let y = x * 2;
    result.push(y);
});
"#,
        );
        test.result(result).assert_no_lint("prefer-array-map");
    }

    /// Allow direct map usage.
    #[test]
    fn test_allows_map_directly() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let result = items.map(x => x * 2);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-map");
    }
}
