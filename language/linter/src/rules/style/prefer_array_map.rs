use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expand_span_to_statement_terminator, expression_method_call, is_array_type,
    member_receiver_text, statement_expression_ancestor,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest `.map()` over `forEach` with push.
    ///
    /// Using `map` is more declarative and avoids manual array mutation.
    #[lint(
        id = "prefer-array-map",
        code = "LY031",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
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

/// Pattern data for one `forEach` callback that only pushes into a target array.
#[derive(Clone, Copy)]
struct MapPattern {
    /// The callback parameter name.
    parameter_name: StringId,
    /// The symbol receiving `.push(...)`.
    push_target_symbol: dir::GlobalSymbolId,
    /// The pushed value expression.
    pushed_value_id: dir::LocalNodeId<dir::Expression>,
}

/// Empty array declaration info immediately before a `forEach` statement.
#[derive(Clone, Copy)]
struct EmptyArrayDeclaration {
    /// The declaration symbol.
    symbol: dir::GlobalSymbolId,
    /// The `[]` initializer expression id.
    initializer_id: dir::LocalNodeId<dir::Expression>,
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
        let for_each_name = ctx.repository.strings.intern("forEach");
        let push_name = ctx.repository.strings.intern("push");

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

        // extract a strict callback push pattern
        let Some(pattern) = self.extract_map_pattern(expression_id) else {
            return;
        };

        self.report(expression_id, pattern);
    }

    /// Extract a strict map pattern from a forEach call.
    fn extract_map_pattern(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<MapPattern> {
        let call = expression_method_call(self.ctx.tree, expression_id)?;

        // keep simple callback-only forEach calls
        if !call.generic_arguments.is_empty() || call.arguments.len() != 1 {
            return None;
        }

        // keep positional callback expressions
        let callback_argument = self.ctx.tree.get(call.arguments[0]);
        let dir::Argument::Positional {
            value: callback_id, ..
        } = callback_argument
        else {
            return None;
        };

        // keep inline sync callbacks with one named parameter
        let callback_expression = self.ctx.tree.get(*callback_id);
        let dir::Expression::Declaration(declaration) = callback_expression else {
            return None;
        };
        let callback_declaration = self.ctx.tree.get(*declaration);
        let dir::Declaration::Function(declaration) = callback_declaration else {
            return None;
        };
        let body_id = declaration.body?;
        if declaration.signature.asynchrony != dir::Asynchrony::Sync
            || declaration.signature.parameters.len() != 1
        {
            return None;
        }

        // keep one plain named callback parameter
        let parameter = self.ctx.tree.get(declaration.signature.parameters[0]);
        let dir::Parameter::Named {
            visibility: None,
            is_readonly: false,
            is_optional: false,
            name,
            default: None,
            ..
        } = parameter
        else {
            return None;
        };

        // keep one push call in callback body
        let push_call_id = self.push_call_from_callback_body(body_id)?;
        let push_call = expression_method_call(self.ctx.tree, push_call_id)?;
        if push_call.method_name != self.push_name {
            return None;
        }

        // keep one positional pushed value
        if !push_call.generic_arguments.is_empty() || push_call.arguments.len() != 1 {
            return None;
        }
        let pushed_argument = self.ctx.tree.get(push_call.arguments[0]);
        let dir::Argument::Positional {
            value: pushed_value_id,
            ..
        } = pushed_argument
        else {
            return None;
        };

        // keep push targets that resolve to a symbol
        let push_receiver_expression = self.ctx.tree.get(push_call.receiver_id);
        let push_target_symbol = push_receiver_expression.target_symbol()?;

        Some(MapPattern {
            parameter_name: *name,
            push_target_symbol,
            pushed_value_id: *pushed_value_id,
        })
    }

    /// Keep callback bodies with exactly one push call.
    fn push_call_from_callback_body(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let expression = self.ctx.tree.get(expression_id);

        // block callbacks: keep single body expression
        if let dir::Expression::Block(block) = expression {
            let block = self.ctx.tree.get(*block);
            if block.len() != 1 {
                return None;
            }
            return Some(self.unwrap_statement_expression(block.first_expression().unwrap()));
        }

        // concise callbacks: body must already be a call
        if matches!(expression, dir::Expression::Call { .. }) {
            return Some(expression_id);
        }

        None
    }

    /// Unwrap one statement wrapper when present.
    fn unwrap_statement_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> dir::LocalNodeId<dir::Expression> {
        expression_id
    }

    /// Resolve the statement wrapper id for one expression.
    fn statement_expression_id(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        statement_expression_ancestor(self.ctx.tree, expression_id)
    }

    /// Resolve the immediate previous expression in the same container.
    fn previous_expression_in_container(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        // check block containers first
        if let Some(parent_id) = self.ctx.tree.get_parent_id(expression_id.id)
            && self.ctx.tree.get_node_type(parent_id) == dir::NodeType::Block
        {
            let block_id = dir::LocalNodeId::<dir::Block>::new(parent_id);
            let block = self.ctx.tree.get(block_id);
            let expression_ids = block.iter_expressions().collect::<Vec<_>>();
            let index = expression_ids
                .iter()
                .position(|item| *item == expression_id)?;
            if index == 0 {
                return None;
            }

            return Some(expression_ids[index - 1]);
        }

        // then check top-level roots
        let root_index = self
            .ctx
            .roots
            .iter()
            .position(|item| *item == expression_id)?;
        if root_index == 0 {
            return None;
        }

        Some(self.ctx.roots[root_index - 1])
    }

    /// Extract one empty array declaration shape.
    fn empty_array_declaration(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<EmptyArrayDeclaration> {
        let expression_id = self.unwrap_statement_expression(expression_id);
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Let { declarators, .. } = expression else {
            return None;
        };
        if declarators.len() != 1 {
            return None;
        }

        let declarator = self.ctx.tree.get(declarators[0]);
        let initializer_id = declarator.value?;
        let initializer = self.ctx.tree.get(initializer_id);
        if !matches!(
            initializer,
            dir::Expression::ArrayExpression { elements } if elements.is_empty()
        ) {
            return None;
        }

        let pattern = self.ctx.tree.get(declarator.pattern);
        let symbol = pattern.symbol()?.into_global(self.ctx.module_id());
        Some(EmptyArrayDeclaration {
            symbol,
            initializer_id,
        })
    }

    /// Build an unsafe map rewrite when an empty target declaration is adjacent.
    fn map_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        pattern: MapPattern,
    ) -> Option<LintFix> {
        let statement_id = self.statement_expression_id(expression_id)?;
        let previous_expression_id = self.previous_expression_in_container(statement_id)?;
        let declaration = self.empty_array_declaration(previous_expression_id)?;
        if declaration.symbol != pattern.push_target_symbol {
            return None;
        }

        // read receiver text for `receiver.map(...)`
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call { left, .. } = expression else {
            return None;
        };
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member {
            left: receiver_expression_id,
            name,
            ..
        } = member_expression
        else {
            return None;
        };
        if *name != Some(self.for_each_name) {
            return None;
        }
        let member_span = self.ctx.get_span(*left);
        let member_text = self.ctx.get_span_text(member_span);
        let receiver_text = member_receiver_text(
            self.ctx,
            *receiver_expression_id,
            member_text,
            (*name)?,
            false,
        )?;

        // read pushed value text for mapper callback expression
        let pushed_value_text = self
            .ctx
            .get_span_text(self.ctx.get_span(pattern.pushed_value_id))
            .to_string();
        let parameter_name = self.ctx.repository.strings.get(pattern.parameter_name);
        let replacement = format!(
            "{receiver}.map(({parameter}) => ({value}))",
            receiver = receiver_text,
            parameter = parameter_name.as_ref(),
            value = pushed_value_text,
        );

        // replace initializer and remove the old forEach statement
        let mut edit_builder = self.ctx.edit_builder();
        edit_builder =
            edit_builder.replace(self.ctx.get_span(declaration.initializer_id), replacement);
        let file = self.ctx.file.as_ref();
        let source = file.text();
        let statement_span = self.ctx.get_span(statement_id);
        let statement_span = expand_span_to_statement_terminator(source, statement_span);
        edit_builder = edit_builder.replace(statement_span, "");
        let edits = edit_builder.into_edits();
        Some(LintFix::r#unsafe("Rewrite forEach push loop as map assignment").with_edits(edits))
    }

    /// Return true when the receiver expression is an array type.
    fn is_array_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_array_type(self.ctx.types, type_id, Some(self.array_symbol))
    }

    /// Report a prefer-array-map match.
    fn report(&mut self, expression_id: dir::LocalNodeId<dir::Expression>, pattern: MapPattern) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_ARRAY_MAP.id,
            PREFER_ARRAY_MAP.code,
            PREFER_ARRAY_MAP.category,
            severity,
            "prefer map() over forEach with push",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use array.map(...) instead");
        if self.ctx.include_fixes
            && let Some(fix) = self.map_fix(expression_id, pattern)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
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
            "prefer_array_map/test_flags_foreach_unconditional_push.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach(x => {
    result.push(x * 2);
});
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-map")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
let result: number[] = items.map((x) => (x * 2));
"#,
            );
    }

    /// Flag arrow function with block body.
    #[test]
    fn test_flags_arrow_block_body() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "prefer_array_map/test_flags_arrow_block_body.ds",
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
            "prefer_array_map/test_allows_conditional_push.ds",
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
            "prefer_array_map/test_allows_multiple_statements.ds",
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
            "prefer_array_map/test_allows_map_directly.ds",
            r#"
let items = [1, 2, 3];
let result = items.map(x => x * 2);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-map");
    }

    /// Allow index callback patterns, this rule intentionally skips them.
    #[test]
    fn test_allows_index_callback_parameter() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "prefer_array_map/test_allows_index_callback_parameter.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
items.forEach((x, index) => {
    result.push(index + x);
});
"#,
        );
        test.result(result).assert_no_lint("prefer-array-map");
    }

    /// Keep no fix without an adjacent empty array declaration.
    #[test]
    fn test_no_fix_without_adjacent_empty_array_declaration() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "prefer_array_map/test_no_fix_without_adjacent_empty_array_declaration.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
console.log(items.length);
items.forEach((x) => {
    result.push(x * 2);
});
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-map")
            .assert_has_no_fix("prefer-array-map");
    }

    /// Keep no fix when callback pushes into a different target.
    #[test]
    fn test_no_fix_for_mismatched_push_target() {
        let test = TestProgram::for_rule_with_prelude(PreferArrayMap);
        let result = test.lint_dir(
            "prefer_array_map/test_no_fix_for_mismatched_push_target.ds",
            r#"
let items = [1, 2, 3];
let result: number[] = [];
let other: number[] = [];
items.forEach((x) => {
    result.push(x * 2);
});
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-map")
            .assert_has_no_fix("prefer-array-map");
    }
}
