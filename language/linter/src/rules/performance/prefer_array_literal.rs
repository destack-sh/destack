use std::collections::HashMap;

use destack_core::StringId;
use destack_dir::{
    self as dir, GlobalSymbolId, LocalNodeId, NodeVisitor, NodeVisitorOptions, WellKnownSymbol,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{expand_span_to_statement_terminator, expression_method_call};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer array literal over empty array followed by push.
    ///
    /// Initializing an empty array and then immediately pushing known values
    /// is less efficient and less readable than using an array literal.
    #[lint(
        id = "prefer-array-literal",
        code = "LP013",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArrayLiteral,
    "Prefer array literal over empty array + push"
}

impl LintRule for PreferArrayLiteral {
    fn meta(&self) -> &'static LintMeta {
        PreferArrayLiteral::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferArrayLiteralVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Info about an empty array declaration.
#[derive(Clone)]
struct EmptyArrayDecl {
    /// The expression id of the declaration.
    expression_id: LocalNodeId<dir::Expression>,
    /// The initializer expression id (`[]`).
    initializer_id: LocalNodeId<dir::Expression>,
    /// Whether we've seen a non-push use of this variable.
    has_other_use: bool,
    /// Number of consecutive push calls seen.
    push_count: u32,
    /// Push calls seen for this declaration.
    push_call_ids: Vec<LocalNodeId<dir::Expression>>,
}

/// Visitor that flags empty array + push patterns.
struct PreferArrayLiteralVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The string id for the push method name.
    push_name: StringId,
    /// Tracked empty array declarations.
    empty_arrays: HashMap<GlobalSymbolId, EmptyArrayDecl>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferArrayLiteralVisitor<'a, 'b> {
    /// Build a visitor for prefer-array-literal checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let push_name = ctx.repository.strings.intern("push");

        Self {
            ctx,
            meta,
            push_name,
            empty_arrays: HashMap::new(),
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

        // report any arrays that had only pushes
        self.report_candidates();
    }

    /// Check for empty array declarations.
    fn check_let(
        &mut self,
        expression_id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // match let declarations
        let dir::Expression::Let { declarators, .. } = expression else {
            return;
        };

        // check each declarator
        for declarator_id in declarators {
            let declarator = self.ctx.tree.get(*declarator_id);

            // require an initializer
            let Some(init_id) = declarator.value else {
                continue;
            };

            // check if the initializer is an empty array
            let init = self.ctx.tree.get(init_id);
            let is_empty_array = matches!(
                init,
                dir::Expression::ArrayExpression { elements } if elements.is_empty()
            );
            if !is_empty_array {
                continue;
            }

            // get the declared symbol
            let pattern = self.ctx.tree.get(declarator.pattern);
            let Some(local_symbol) = pattern.symbol() else {
                continue;
            };

            // track this empty array
            let global_symbol = GlobalSymbolId::new(self.ctx.module_id(), local_symbol);
            self.empty_arrays.insert(
                global_symbol,
                EmptyArrayDecl {
                    expression_id,
                    initializer_id: init_id,
                    has_other_use: false,
                    push_count: 0,
                    push_call_ids: Vec::new(),
                },
            );
        }
    }

    /// Check for push calls on tracked arrays.
    fn check_call(&mut self, expression_id: LocalNodeId<dir::Expression>) {
        // match method call pattern
        let Some(method_call) = expression_method_call(self.ctx.tree, expression_id) else {
            return;
        };

        // check if this is push
        if method_call.method_name != self.push_name {
            // mark as other use if the receiver is a tracked array
            self.mark_other_use(method_call.receiver_id);
            return;
        }

        // check if the receiver is a tracked empty array
        let receiver = self.ctx.tree.get(method_call.receiver_id);
        let Some(target_symbol) = receiver.target_symbol() else {
            return;
        };

        // increment the push count
        if let Some(decl) = self.empty_arrays.get_mut(&target_symbol) {
            decl.push_count += 1;
            decl.push_call_ids.push(expression_id);
        }
    }

    /// Mark an expression's target as having a non-push use.
    fn mark_other_use(&mut self, expression_id: LocalNodeId<dir::Expression>) {
        // check if this references a tracked array
        let expression = self.ctx.tree.get(expression_id);
        let Some(target_symbol) = expression.target_symbol() else {
            return;
        };

        // mark as having other uses
        if let Some(decl) = self.empty_arrays.get_mut(&target_symbol) {
            decl.has_other_use = true;
        }
    }

    /// Return true when this expression is only the receiver of a `.push(...)` call.
    fn is_push_receiver_use(&self, expression_id: LocalNodeId<dir::Expression>) -> bool {
        let Some(parent_id) = self.ctx.tree.get_parent_id(expression_id.id) else {
            return false;
        };
        if self.ctx.tree.get_node_type(parent_id) != dir::NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<dir::Expression>::new(parent_id);
        let parent_expression = self.ctx.tree.get(parent_expression_id);
        let dir::Expression::Member { left, name, .. } = parent_expression else {
            return false;
        };
        if *left != expression_id || *name != Some(self.push_name) {
            return false;
        }

        let Some(grandparent_id) = self.ctx.tree.get_parent_id(parent_id) else {
            return false;
        };
        if self.ctx.tree.get_node_type(grandparent_id) != dir::NodeType::Expression {
            return false;
        }

        let grandparent_expression = self
            .ctx
            .tree
            .get(LocalNodeId::<dir::Expression>::new(grandparent_id));
        matches!(
            grandparent_expression,
            dir::Expression::Call {
                left,
                ..
            } if *left == parent_expression_id
        )
    }

    /// Report arrays that could be array literals.
    fn report_candidates(&mut self) {
        let declaration_candidates = self.empty_arrays.values().cloned().collect::<Vec<_>>();
        for decl in declaration_candidates {
            // only report if we have pushes and no other uses
            if decl.push_count == 0 || decl.has_other_use {
                continue;
            }

            // honor per node severity
            let severity = self
                .ctx
                .get_effective_severity(self.meta, decl.expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = self.ctx.get_span(decl.expression_id);
            let mut diagnostic = LintDiagnostic::new(
                PREFER_ARRAY_LITERAL.id,
                PREFER_ARRAY_LITERAL.code,
                PREFER_ARRAY_LITERAL.category,
                severity,
                "prefer array literal over empty array + push",
                self.ctx.module.file_id,
                span,
            )
            .with_label("initialize with values directly");
            if self.ctx.include_fixes
                && let Some(fix) = self.prefer_array_literal_fix(&decl)
            {
                diagnostic = diagnostic.with_fix(fix);
            }
            self.ctx.report(diagnostic);
        }
    }

    /// Build an unsafe fix by moving pushed values into the initializer.
    fn prefer_array_literal_fix(&self, decl: &EmptyArrayDecl) -> Option<LintFix> {
        let (container_id, declaration_item_id) =
            self.container_and_item_expression_id(decl.expression_id)?;
        let declaration_item_span = self.ctx.get_span(declaration_item_id);
        let mut push_statement_ids = Vec::new();
        let mut literal_elements = Vec::new();
        for push_call_id in &decl.push_call_ids {
            let (push_container_id, push_item_id) =
                self.container_and_item_expression_id(*push_call_id)?;
            if push_container_id != container_id {
                return None;
            }

            // keep pushes after declaration in the same container
            let push_item_span = self.ctx.get_span(push_item_id);
            if push_item_span.start <= declaration_item_span.start {
                return None;
            }

            let push_call = self.ctx.tree.get(*push_call_id);
            let dir::Expression::Call {
                generic_arguments,
                dynamic_arguments,
                ..
            } = push_call
            else {
                return None;
            };
            if !generic_arguments.is_empty() || dynamic_arguments.len() != 1 {
                return None;
            }
            let argument = self.ctx.tree.get(dynamic_arguments[0]);
            let dir::Argument::Positional { value, .. } = argument else {
                return None;
            };

            // keep positional push values only
            literal_elements.push(
                self.ctx
                    .get_span_text(self.ctx.get_span(*value))
                    .to_string(),
            );
            push_statement_ids.push(push_item_id);
        }

        // rewrite initializer and remove pushes
        let array_literal = format!("[{}]", literal_elements.join(", "));
        let mut edit_builder = self.ctx.edit_builder();
        edit_builder = edit_builder.replace(self.ctx.get_span(decl.initializer_id), array_literal);
        let file = self.ctx.file.as_ref();
        let source = file.text();
        for push_statement_id in push_statement_ids {
            let statement_span = self.ctx.get_span(push_statement_id);
            let statement_span = expand_span_to_statement_terminator(source, statement_span);
            edit_builder = edit_builder.replace(statement_span, "");
        }
        let edits = edit_builder.into_edits();
        Some(LintFix::r#unsafe("Initialize array with literal and remove pushes").with_edits(edits))
    }

    /// Return a container id and removable expression item for one expression.
    fn container_and_item_expression_id(
        &self,
        expression_id: LocalNodeId<dir::Expression>,
    ) -> Option<(Option<u32>, LocalNodeId<dir::Expression>)> {
        let parent_id = self.ctx.tree.get_parent_id(expression_id.id);
        Some((parent_id, expression_id))
    }
}

impl NodeVisitor for PreferArrayLiteralVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for let declarations
        if matches!(expression, dir::Expression::Let { .. }) {
            self.check_let(id, expression);
        }

        // check for call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_call(id);
        }

        // mark plain symbol uses that are not just `.push(...)` receivers
        if expression.target_symbol().is_some() && !self.is_push_receiver_use(id) {
            self.mark_other_use(id);
        }

        // check for member accesses (like items.length) as other use
        // but exclude .push() which is handled by check_call
        if let dir::Expression::Member { left, name, .. } = expression
            && *name != Some(self.push_name)
        {
            self.mark_other_use(*left);
        }

        // check assignment-like writes to tracked arrays as other use
        if let dir::Expression::Assign { left, .. } | dir::Expression::AssignBinary { left, .. } =
            expression
        {
            self.mark_other_use(*left);
        }

        // check update writes to tracked arrays as other use
        if let dir::Expression::Unary { operator, right } = expression
            && matches!(
                operator,
                dir::UnaryOperator::PreIncrement
                    | dir::UnaryOperator::PostIncrement
                    | dir::UnaryOperator::PreDecrement
                    | dir::UnaryOperator::PostDecrement
            )
        {
            self.mark_other_use(*right);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag empty array with push.
    #[test]
    fn test_flags_empty_array_push() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_flags_empty_array_push.ds",
            r#"
let items: number[] = [];
items.push(1);
items.push(2);
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-literal")
            .assert_unsafe_fixed(
                r#"
let items: number[] = [1, 2];
"#,
            );
    }

    /// Flag single push.
    #[test]
    fn test_flags_single_push() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_flags_single_push.ds",
            r#"
let items: string[] = [];
items.push("hello");
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-literal")
            .assert_unsafe_fixed(
                r#"
let items: string[] = ["hello"];
"#,
            );
    }

    /// Allow arrays whose empty state is observed before the pushes.
    #[test]
    fn test_allows_plain_value_use_before_pushes() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_plain_value_use_before_pushes.ds",
            r#"
let items: number[] = [];
consume(items);
items.push(1);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-literal");
    }

    /// Allow array literal.
    #[test]
    fn test_allows_array_literal() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_array_literal.ds",
            r#"
let items = [1, 2, 3];
"#,
        );
        test.result(result).assert_no_lint("prefer-array-literal");
    }

    /// Allow empty array with other uses.
    #[test]
    fn test_allows_empty_array_with_other_uses() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_empty_array_with_other_uses.ds",
            r#"
let items: number[] = [];
items.push(1);
console.log(items.length);
items.push(2);
"#,
        );
        test.result(result).assert_no_lint("prefer-array-literal");
    }

    /// Allow empty array for dynamic population.
    #[test]
    fn test_allows_dynamic_population() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_dynamic_population.ds",
            r#"
let items: number[] = [];
for (let i = 0; i < 10; i += 1) {
    items.push(i);
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-literal")
            .assert_has_no_fix("prefer-array-literal");
    }

    /// Allow const arrays.
    #[test]
    fn test_allows_const_array() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_const_array.ds",
            r#"
const items: number[] = [];
"#,
        );
        test.result(result).assert_no_lint("prefer-array-literal");
    }

    /// Flag const empty arrays that are only populated with push calls.
    #[test]
    fn test_flags_const_array_with_pushes() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_flags_const_array_with_pushes.ds",
            r#"
const items: number[] = [];
items.push(1);
items.push(2);
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-literal")
            .assert_unsafe_fixed(
                r#"
const items: number[] = [1, 2];
"#,
            );
    }

    /// Keep no fix when push value uses named arguments.
    #[test]
    fn test_no_fix_for_non_positional_push_argument() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_no_fix_for_non_positional_push_argument.ds",
            r#"
let items: number[] = [];
const values = [1];
items.push(...values);
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-literal")
            .assert_has_no_fix("prefer-array-literal");
    }

    /// Allow reassigned arrays after push usage.
    #[test]
    fn test_allows_array_reassignment_after_push() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayLiteral);
        let result = test.lint_dir(
            "prefer_array_literal/test_allows_array_reassignment_after_push.ds",
            r#"
let items: number[] = [];
items.push(1);
items = [2, 3];
"#,
        );
        test.result(result).assert_no_lint("prefer-array-literal");
    }
}
