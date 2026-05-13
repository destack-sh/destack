use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{
    ReferencePath, assign_pattern_reference_path, assign_pattern_target_expression,
    expression_enters_nested_declaration_scope, expression_reference_path,
    expression_unwrap_parenthesized, is_string_type,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow string concatenation in loops.
    ///
    /// String concatenation inside loops can lead to quadratic behavior
    /// due to repeated reallocations.
    #[lint(
        id = "no-string-concat-in-loop",
        code = "LP010",
        category = Performance,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::String)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoStringConcatInLoop,
    "Disallow string concatenation inside loops"
}

impl LintRule for NoStringConcatInLoop {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoStringConcatInLoop::meta()
    }

    /// Check module DIR nodes for string concatenation inside loops.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoStringConcatInLoopVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags string concatenation in loops.
struct NoStringConcatInLoopVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// Whether the current traversal is inside a loop.
    is_in_loop: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoStringConcatInLoopVisitor<'a, 'b> {
    /// Build a visitor for string concatenation in loops.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let string_symbol = ctx.language_item(LanguageItem::String);
        Self {
            ctx,
            meta,
            string_symbol,
            is_in_loop: false,
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

    /// Check an expression for string concatenation inside loops.
    fn check_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // only report inside loops
        if !self.is_in_loop {
            return;
        }

        // check for += concatenation
        if let dir::Expression::AssignBinary {
            left,
            operator,
            right,
        } = expression
        {
            if *operator == dir::AssignOperator::AddAssign
                && (self.is_string_expression(*left) || self.is_string_like_expression(*right))
            {
                self.report_match(expression_id);
            }
            return;
        }

        // check for x = x + y concatenation
        let dir::Expression::Assign { left, right } = expression else {
            return;
        };
        let Some(left_expression_id) = assign_pattern_target_expression(self.ctx.tree, *left)
        else {
            return;
        };

        // resolve the left reference path
        let Some(left_path) = assign_pattern_reference_path(self.ctx, *left) else {
            return;
        };

        // require the rhs to contain the left reference
        let has_add_reference = self.add_chain_contains_reference(*right, &left_path);
        let has_template_reference = self.template_concat_contains_reference(*right, &left_path);
        if !has_add_reference && !has_template_reference {
            return;
        }

        // require at least one string operand
        let has_string_operand = self.add_chain_has_string_operand(*right);
        if !self.is_string_expression(left_expression_id) && !has_string_operand {
            return;
        }

        self.report_match(expression_id);
    }

    /// Report a no-string-concat-in-loop match.
    fn report_match(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_STRING_CONCAT_IN_LOOP.id,
                NO_STRING_CONCAT_IN_LOOP.code,
                NO_STRING_CONCAT_IN_LOOP.category,
                severity,
                "string concatenation inside a loop can be costly",
                span,
            )
            .label("consider collecting and joining instead"),
        );
    }

    /// Return true when the expression is a string type.
    fn is_string_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the expression type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_string_type(self.ctx.types, type_id, Some(self.string_symbol))
    }

    /// Return true when the expression should be treated as a string.
    fn is_string_like_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // unwrap parenthesized expressions
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);

        // match string literals or typed strings
        let expression = self.ctx.tree.get(expression_id);
        matches!(
            expression,
            dir::Expression::ScalarLiteral {
                value: dir::ScalarLiteral::String(_),
            } | dir::Expression::TemplateExpression { .. }
        ) || self.is_string_expression(expression_id)
    }

    /// Return true when a + chain contains the target reference.
    fn add_chain_contains_reference(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        reference: &ReferencePath,
    ) -> bool {
        // unwrap parenthesized expressions
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);

        // descend into add chains
        let expression = self.ctx.tree.get(expression_id);
        if let dir::Expression::Binary {
            left,
            operator: dir::BinaryOperator::Add,
            right,
        } = expression
        {
            return self.add_chain_contains_reference(*left, reference)
                || self.add_chain_contains_reference(*right, reference);
        }

        // compare the resolved path
        expression_reference_path(self.ctx, expression_id).is_some_and(|path| path == *reference)
    }

    /// Return true when a + chain contains a string-like operand.
    fn add_chain_has_string_operand(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        // unwrap parenthesized expressions
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);

        // descend into add chains
        let expression = self.ctx.tree.get(expression_id);
        if let dir::Expression::Binary {
            left,
            operator: dir::BinaryOperator::Add,
            right,
        } = expression
        {
            return self.add_chain_has_string_operand(*left)
                || self.add_chain_has_string_operand(*right);
        }

        // fall back to the leaf expression
        self.is_string_like_expression(expression_id)
    }

    /// Return true when a template expression concatenates the target reference.
    fn template_concat_contains_reference(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        reference: &ReferencePath,
    ) -> bool {
        // unwrap parenthesized expressions
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);

        // match interpolated template expressions
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::TemplateExpression {
            value: dir::TemplateLiteral::InterpolatedString { strings, arguments },
        } = expression
        else {
            return false;
        };

        // track whether the template references the assignment target
        let mut has_target_reference = false;
        let mut has_other_content = strings.iter().any(|segment| {
            let segment_text = self.ctx.strings.get(*segment);
            !segment_text.is_empty()
        });

        for argument_id in arguments {
            let argument = self.ctx.tree.get(*argument_id);
            let dir::Argument::Positional {
                value: value_id, ..
            } = argument
            else {
                return false;
            };

            let is_target_reference = expression_reference_path(self.ctx, *value_id)
                .is_some_and(|path| path == *reference);
            if is_target_reference {
                has_target_reference = true;
            } else {
                has_other_content = true;
            }
        }

        has_target_reference && has_other_content
    }

    /// Visit a loop expression with loop context.
    fn visit_loop(
        &mut self,
        tree: &dir::Tree,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit loop condition inside loop context
        if let Some(condition_id) = condition {
            self.is_in_loop = true;
            let condition_expression = tree.get(condition_id);
            self.visit_expression(tree, condition_id, condition_expression);
        }

        // visit loop body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }

    /// Visit a for each loop expression with loop context.
    fn visit_for_each(
        &mut self,
        tree: &dir::Tree,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit binding inside loop context
        self.is_in_loop = true;
        match binding {
            dir::ForEachBinding::Pattern { pattern, .. } => {
                let pattern_node = tree.get(*pattern);
                self.visit_pattern(tree, *pattern, pattern_node);
            }
            dir::ForEachBinding::Using {
                asynchrony: _,
                pattern,
            } => {
                let pattern_node = tree.get(*pattern);
                self.visit_pattern(tree, *pattern, pattern_node);
            }
        }

        // visit iterator outside loop context
        self.is_in_loop = was_in_loop;
        let iterator_expression = tree.get(iterator);
        self.visit_expression(tree, iterator, iterator_expression);

        // visit body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }

    /// Visit a for loop expression with loop context.
    fn visit_for(
        &mut self,
        tree: &dir::Tree,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit initialization outside loop context
        if let Some(initialization_id) = initialization {
            self.is_in_loop = was_in_loop;
            let initialization_expression = tree.get(initialization_id);
            self.visit_expression(tree, initialization_id, initialization_expression);
        }

        // visit condition inside loop context
        if let Some(condition_id) = condition {
            self.is_in_loop = true;
            let condition_expression = tree.get(condition_id);
            self.visit_expression(tree, condition_id, condition_expression);
        }

        // visit increment inside loop context
        if let Some(increment_id) = increment {
            self.is_in_loop = true;
            let increment_expression = tree.get(increment_id);
            self.visit_expression(tree, increment_id, increment_expression);
        }

        // visit body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }
}

impl NodeVisitor for NoStringConcatInLoopVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // avoid leaking loop context into nested declarations
        if self.is_in_loop && expression_enters_nested_declaration_scope(tree, expression) {
            return;
        }

        // check for string concatenation
        self.check_expression(id, expression);

        // handle loop expressions with custom traversal
        match expression {
            dir::Expression::Loop {
                condition, body, ..
            } => {
                self.visit_loop(tree, *condition, *body);
                return;
            }
            dir::Expression::ForEach {
                binding,
                iterator,
                body,
                ..
            } => {
                self.visit_for_each(tree, binding, *iterator, *body);
                return;
            }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
                ..
            } => {
                self.visit_for(tree, *initialization, *condition, *increment, *body);
                return;
            }
            _ => {}
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report += concatenation inside loops.
    #[test]
    fn test_flags_add_assign_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoStringConcatInLoop);
        let result = test.lint_dir(
            "no_string_concat_in_loop/test_flags_add_assign_in_loop.ds",
            r#"
let items = ["a", "b"];
let result = "";
for (let i = 0; i < items.length; i += 1) {
    result += items[i];
}
"#,
        );
        test.result(result).assert_lint("no-string-concat-in-loop");
    }

    /// Report x = x + y concatenation inside loops.
    #[test]
    fn test_flags_self_add_assign_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoStringConcatInLoop);
        let result = test.lint_dir(
            "no_string_concat_in_loop/test_flags_self_add_assign_in_loop.ds",
            r#"
let items = ["a", "b"];
let result = "";
for (const item of items) {
    result = result + item;
}
"#,
        );
        test.result(result).assert_lint("no-string-concat-in-loop");
    }

    /// Report x = y + x concatenation inside loops.
    #[test]
    fn test_flags_reversed_concat_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoStringConcatInLoop);
        let result = test.lint_dir(
            "no_string_concat_in_loop/test_flags_reversed_concat_in_loop.ds",
            r#"
let items = ["a", "b"];
let result = "";
for (const item of items) {
    result = item + result;
}
"#,
        );
        test.result(result).assert_lint("no-string-concat-in-loop");
    }

    /// Allow numeric += inside loops.
    #[test]
    fn test_allows_numeric_add_assign() {
        let test = TestProgram::for_rule_without_prelude(NoStringConcatInLoop);
        let result = test.lint_dir(
            "no_string_concat_in_loop/test_allows_numeric_add_assign.ds",
            r#"
let items = [1, 2];
let total = 0;
for (const item of items) {
    total += item;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-string-concat-in-loop");
    }

    /// Allow concatenation in for-each iterator expressions.
    #[test]
    fn test_allows_concat_in_for_each_iterator() {
        let test = TestProgram::for_rule_without_prelude(NoStringConcatInLoop);
        let result = test.lint_dir(
            "no_string_concat_in_loop/test_allows_concat_in_for_each_iterator.ds",
            r#"
let source = "ab";
let suffix = "cd";
let output = "";
for (const char of source + suffix) {
    output = char;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-string-concat-in-loop");
    }

    /// Report template string concatenation inside loops.
    #[test]
    fn test_flags_template_concat_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoStringConcatInLoop);
        let result = test.lint_dir(
            "no_string_concat_in_loop/test_flags_template_concat_in_loop.ds",
            r#"
let items = ["a", "b"];
let result = "";
for (const item of items) {
    result = `${result}${item}`;
}
"#,
        );
        test.result(result).assert_lint("no-string-concat-in-loop");
    }

    /// Allow template assignments that only re-emit the same value.
    #[test]
    fn test_allows_identity_template_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoStringConcatInLoop);
        let result = test.lint_dir(
            "no_string_concat_in_loop/test_allows_identity_template_in_loop.ds",
            r#"
let items = ["a", "b"];
let result = "";
for (const item of items) {
    result = `${result}`;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-string-concat-in-loop");
    }

    /// Allow concatenation inside functions declared within loops.
    #[test]
    fn test_allows_nested_function_inside_loop() {
        let test = TestProgram::for_rule_without_prelude(NoStringConcatInLoop);
        let result = test.lint_dir(
            "no_string_concat_in_loop/test_allows_nested_function_inside_loop.ds",
            r#"
let suffix = "b";
for (const item of items) {
    function build(prefix) {
        return prefix + suffix;
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-string-concat-in-loop");
    }
}
