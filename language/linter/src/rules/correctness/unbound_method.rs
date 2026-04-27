use destack_ast::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    assign_pattern_contains_expression, call_like_invocation_is_receiver_bound,
    has_non_void_this_parameter_type, member_receiver_text, parent_is_receiver_helper,
    resolution_target_symbols, symbol_primary_declaration_for, symbol_value_type_id_for,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unbound instance methods.
    ///
    /// Referencing an instance method without calling or binding it can lose
    /// the receiver and break `this` dependent logic.
    #[lint(
        id = "unbound-method",
        code = "LC041",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub UnboundMethod,
    "Disallow unbound methods as callbacks"
}

impl LintRule for UnboundMethod {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        UnboundMethod::meta()
    }

    /// Check module DIR nodes for unbound method references.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let bind_name = ctx.repository.strings.intern("bind");
        let call_name = ctx.repository.strings.intern("call");
        let apply_name = ctx.repository.strings.intern("apply");

        // resolve visitor
        let mut visitor = UnboundMethodVisitor::new(ctx, meta, bind_name, call_name, apply_name);
        visitor.run();
    }
}

/// Node visitor for unbound method checks.
struct UnboundMethodVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The interned "bind" name.
    bind_name: StringId,
    /// The interned "call" name.
    call_name: StringId,
    /// The interned "apply" name.
    apply_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> UnboundMethodVisitor<'a, 'b> {
    /// Build a visitor for unbound method checks.
    fn new(
        ctx: &'a mut LintModuleDirContext<'b>,
        meta: &'a LintMeta,
        bind_name: StringId,
        call_name: StringId,
        apply_name: StringId,
    ) -> Self {
        Self {
            ctx,
            meta,
            bind_name,
            call_name,
            apply_name,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // inspect dir roots
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one method reference expression for unbound method usage.
    fn check_method_reference(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        if self.is_safe_method_reference_usage(expression_id) {
            return;
        }

        // resolve global id
        let global_id = expression_id.into_global_any(self.ctx.module_id());
        let Some(resolution_id) = self.ctx.types.get_resolution_for_node(global_id) else {
            return;
        };
        let resolution = self.ctx.types.get_resolution(resolution_id);

        // resolve is unbound method
        let is_unbound_method = resolution_target_symbols(resolution)
            .into_iter()
            .any(|symbol| self.is_this_bound_method_symbol(symbol));
        if !is_unbound_method {
            return;
        }

        // resolve effective lint severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the full method reference span
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            UNBOUND_METHOD.id,
            UNBOUND_METHOD.code,
            UNBOUND_METHOD.category,
            severity,
            "unbound method reference",
            self.ctx.module.file_id,
            span,
        )
        .with_label("bind this method or wrap it in a lambda");
        if self.ctx.include_fixes
            && let Some(fix) = self.unbound_method_fix(expression_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a conservative bind fix for one unbound method reference.
    fn unbound_method_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintFix> {
        let expression = self.ctx.tree.get(expression_id);
        let (left_expression_id, method_name, is_private, method_text_span) = match expression {
            dir::Expression::Member { left, name, .. } => {
                // skip helper names: this expression is itself not a method reference target
                if *name == Some(self.bind_name)
                    || *name == Some(self.call_name)
                    || *name == Some(self.apply_name)
                {
                    return None;
                }

                (*left, *name, false, self.ctx.get_span(expression_id))
            }
            dir::Expression::PrivateMember { left, name, .. } => {
                // skip helper names: this expression is itself not a method reference target
                if *name == Some(self.bind_name)
                    || *name == Some(self.call_name)
                    || *name == Some(self.apply_name)
                {
                    return None;
                }

                (*left, *name, true, self.ctx.get_span(expression_id))
            }
            _ => return None,
        };

        // skip helper chains like `method.call()` and `method.bind()`
        if parent_is_receiver_helper(
            self.ctx.tree,
            expression_id,
            self.bind_name,
            self.call_name,
            self.apply_name,
        ) {
            return None;
        }

        // keep direct expression text for robust source preserving rewrites
        let method_text = self.ctx.get_span_text(method_text_span).to_string();
        let left_text = member_receiver_text(
            self.ctx,
            left_expression_id,
            &method_text,
            method_name?,
            is_private,
        )?;
        if method_text.is_empty() || left_text.is_empty() {
            return None;
        }

        // bind the method to its receiver expression
        let replacement = format!("{method_text}.bind({left_text})");
        let edits = self
            .ctx
            .edit_builder()
            .replace(method_text_span, replacement)
            .into_edits();
        Some(LintFix::r#unsafe("Bind method receiver explicitly").with_edits(edits))
    }

    /// Return true when a method reference is already safely used.
    fn is_safe_method_reference_usage(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let mut current_id = expression_id;

        loop {
            let Some(parent) = self.ctx.tree.get_parent(current_id.id) else {
                return false;
            };
            if parent.ty != dir::NodeType::Expression {
                return false;
            }

            // inspect the next parent expression usage
            let parent_id = parent.into_typed::<dir::Expression>();
            let parent_expression = self.ctx.tree.get(parent_id);
            match parent_expression {
                dir::Expression::Parenthesized { expression } if *expression == current_id => {
                    current_id = parent_id;
                }
                dir::Expression::Maybe { left } | dir::Expression::Must { left }
                    if *left == current_id =>
                {
                    current_id = parent_id;
                }
                dir::Expression::As {
                    operator: _,
                    source: _,
                    expression: value,
                    target_type: _,
                }
                | dir::Expression::Satisfies {
                    expression: value,
                    target_type: _,
                } if *value == current_id => {
                    current_id = parent_id;
                }
                dir::Expression::ValueOf { right, .. }
                | dir::Expression::ReferenceOf { right, .. }
                | dir::Expression::PointerOf { right, .. }
                    if *right == current_id =>
                {
                    current_id = parent_id;
                }
                dir::Expression::Instantiation { left, .. } if *left == current_id => {
                    current_id = parent_id;
                }
                dir::Expression::Call { left, .. } | dir::Expression::New { left, .. }
                    if *left == current_id
                        && call_like_invocation_is_receiver_bound(
                            self.ctx.tree,
                            parent_id,
                            self.bind_name,
                            self.call_name,
                            self.apply_name,
                        ) =>
                {
                    return true;
                }
                dir::Expression::TaggedTemplateExpression { tag, .. } if *tag == current_id => {
                    return true;
                }
                dir::Expression::If { condition, .. }
                    if if_condition_expression_id(condition) == Some(current_id) =>
                {
                    return true;
                }
                dir::Expression::Loop {
                    condition: Some(condition_id),
                    ..
                } if *condition_id == current_id => {
                    return true;
                }
                dir::Expression::For {
                    condition: Some(condition_id),
                    ..
                } if *condition_id == current_id => {
                    return true;
                }
                dir::Expression::Match { value, .. } if *value == current_id => {
                    return true;
                }
                dir::Expression::Delete { value } if *value == current_id => {
                    return true;
                }
                dir::Expression::Unary {
                    operator:
                        dir::UnaryOperator::Not | dir::UnaryOperator::Typeof | dir::UnaryOperator::Void,
                    right,
                } if *right == current_id => {
                    return true;
                }
                dir::Expression::Binary {
                    left,
                    operator,
                    right,
                } if *left == current_id || *right == current_id => {
                    if binary_operator_is_safe_receiver_test(*operator) {
                        return true;
                    }

                    // short circuit and with method reference on left is safe
                    if *operator == dir::BinaryOperator::And && *left == current_id {
                        return true;
                    }

                    // continue through logical operators to inspect outer usage
                    if matches!(
                        operator,
                        dir::BinaryOperator::And
                            | dir::BinaryOperator::Or
                            | dir::BinaryOperator::Coalesce
                    ) {
                        current_id = parent_id;
                        continue;
                    }

                    return false;
                }
                dir::Expression::Assign { left, .. }
                    if assign_pattern_contains_expression(self.ctx.tree, *left, current_id) =>
                {
                    return true;
                }
                dir::Expression::Member { left, name, .. } if *left == current_id => {
                    if *name == Some(self.bind_name)
                        || *name == Some(self.call_name)
                        || *name == Some(self.apply_name)
                    {
                        current_id = parent_id;
                        continue;
                    }

                    return true;
                }
                dir::Expression::PrivateMember { left, name, .. } if *left == current_id => {
                    if *name == Some(self.bind_name)
                        || *name == Some(self.call_name)
                        || *name == Some(self.apply_name)
                    {
                        current_id = parent_id;
                        continue;
                    }

                    return true;
                }
                _ => return false,
            }
        }
    }

    /// Return true when a symbol is a method that needs a bound `this`.
    fn is_this_bound_method_symbol(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        // prefer declarations to identify method symbols and skip static members
        let Some(primary_declaration) = symbol_primary_declaration_for(
            &self.ctx.repository,
            self.ctx.revision,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            symbol_id,
        ) else {
            return self.symbol_has_this_parameter(symbol_id);
        };

        // inspect member declarations and reject static methods
        if primary_declaration.local_id.ty == dir::NodeType::Member {
            let Some(module_dir) = self.ctx.analyzed_dir(primary_declaration.module_id) else {
                return self.symbol_has_this_parameter(symbol_id);
            };
            let member = module_dir
                .tree
                .get(primary_declaration.into_local_typed::<dir::Member>());
            let dir::Member::Method { is_static, .. } = member else {
                return false;
            };

            return !is_static;
        }

        // inspect property declarations for method values
        if primary_declaration.local_id.ty == dir::NodeType::Property {
            let Some(module_dir) = self.ctx.analyzed_dir(primary_declaration.module_id) else {
                return self.symbol_has_this_parameter(symbol_id);
            };
            let property = module_dir
                .tree
                .get(primary_declaration.into_local_typed::<dir::Property>());
            return matches!(property, dir::Property::Method { .. });
        }

        self.symbol_has_this_parameter(symbol_id)
    }

    /// Return true when a symbol value type declares a `this` parameter.
    fn symbol_has_this_parameter(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        let Some(symbol_type_id) = symbol_value_type_id_for(
            &self.ctx.repository,
            self.ctx.revision,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            self.ctx.types,
            symbol_id,
        ) else {
            return false;
        };

        // fast path when the type information is in this module
        if symbol_type_id.module_id == self.ctx.module_id() {
            return has_non_void_this_parameter_type(self.ctx.types, symbol_type_id.type_id);
        }

        // load foreign module types for the `this` parameter check
        let Some(module_dir) = self.ctx.analyzed_dir(symbol_type_id.module_id) else {
            return false;
        };
        has_non_void_this_parameter_type(&module_dir.types, symbol_type_id.type_id)
    }
}

/// Return one expression id when an if condition is a plain expression.
fn if_condition_expression_id(
    condition: &dir::IfCondition,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match condition {
        dir::IfCondition::Expression { condition } => Some(*condition),
        dir::IfCondition::Let { .. } => None,
    }
}

/// Return true when one binary operator uses a value only as a safe receiver test.
fn binary_operator_is_safe_receiver_test(operator: dir::BinaryOperator) -> bool {
    matches!(
        operator,
        dir::BinaryOperator::Equal
            | dir::BinaryOperator::NotEqual
            | dir::BinaryOperator::EqualStrict
            | dir::BinaryOperator::NotEqualStrict
    )
}

impl NodeVisitor for UnboundMethodVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if matches!(
            expression,
            dir::Expression::Member { .. } | dir::Expression::PrivateMember { .. }
        ) {
            self.check_method_reference(id);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag unbound class method references.
    #[test]
    fn test_flags_unbound_class_method_reference() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_flags_unbound_class_method_reference.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
let callback = counter.increment;
"#,
        );
        test.result(result)
            .assert_lint("unbound-method")
            .assert_unsafe_fixed(
                r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
let callback = counter.increment.bind(counter);
"#,
            );
    }

    /// Allow direct method calls.
    #[test]
    fn test_allows_direct_method_call() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_direct_method_call.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
counter.increment();
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow bound methods via bind.
    #[test]
    fn test_allows_bound_method_with_bind() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_bound_method_with_bind.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
let callback = counter.increment.bind(counter);
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow static method references.
    #[test]
    fn test_allows_static_method_reference() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_static_method_reference.ts",
            r#"
class Counter {
    static create(): Counter {
        return new Counter();
    }
}

let factory = Counter.create;
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow function-valued fields that do not use this.
    #[test]
    fn test_allows_function_valued_field_reference() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_function_valued_field_reference.ts",
            r#"
let container = {
    map: (value: number) => value + 1
};

let callback = container.map;
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow direct method invocation through call.
    #[test]
    fn test_allows_method_call_with_call() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_method_call_with_call.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
counter.increment.call(counter);
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow direct method invocation through apply.
    #[test]
    fn test_allows_method_call_with_apply() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_method_call_with_apply.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
counter.increment.apply(counter, []);
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow method references in if conditions.
    #[test]
    fn test_allows_method_reference_in_if_condition() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_method_reference_in_if_condition.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
if (counter.increment) {
    counter.increment();
}
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow method references in equality comparisons.
    #[test]
    fn test_allows_method_reference_in_equality_comparison() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_method_reference_in_equality_comparison.ts",
            r#"
class Counter {
    increment() {}
}

let counter = new Counter();
if (counter.increment === undefined) {}
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow property access on method references.
    #[test]
    fn test_allows_member_access_on_method_reference() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_member_access_on_method_reference.ts",
            r#"
class Counter {
    increment() {}
}

let counter = new Counter();
const functionName = counter.increment.name;
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow method references in left side logical-and checks.
    #[test]
    fn test_allows_method_reference_in_left_logical_and() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_method_reference_in_left_logical_and.ts",
            r#"
class Counter {
    increment() {}
}

let counter = new Counter();
counter.increment && counter.increment();
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow method references in delete expressions.
    #[test]
    fn test_allows_method_reference_in_delete_expression() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_method_reference_in_delete_expression.ts",
            r#"
class Counter {
    increment() {}
}

let counter = new Counter();
delete counter.increment;
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Allow method references as tagged template tags.
    #[test]
    fn test_allows_method_reference_in_tagged_template() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_allows_method_reference_in_tagged_template.ts",
            r#"
class Counter {
    increment(messageParts: string[]): string {
        return messageParts[0];
    }
}

let counter = new Counter();
counter.increment`ok`;
"#,
        );
        test.result(result).assert_no_lint("unbound-method");
    }

    /// Flag call helper invocations that omit a receiver argument.
    #[test]
    fn test_flags_method_call_without_call_receiver() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_flags_method_call_without_call_receiver.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
counter.increment.call();
"#,
        );
        test.result(result).assert_lint("unbound-method");
    }

    /// Flag apply helper invocations that omit a receiver argument.
    #[test]
    fn test_flags_method_call_without_apply_receiver() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_flags_method_call_without_apply_receiver.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
counter.increment.apply();
"#,
        );
        test.result(result).assert_lint("unbound-method");
    }

    /// Flag bind helper invocations that omit a receiver argument.
    #[test]
    fn test_flags_method_call_without_bind_receiver() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_flags_method_call_without_bind_receiver.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
let callback = counter.increment.bind();
"#,
        );
        test.result(result).assert_lint("unbound-method");
    }

    /// Flag method bind property references that are not invoked.
    #[test]
    fn test_flags_uninvoked_bind_member_reference() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_flags_uninvoked_bind_member_reference.ts",
            r#"
class Counter {
    value: number = 0;

    increment() {
        this.value += 1;
    }
}

let counter = new Counter();
let bindRef = counter.increment.bind;
"#,
        );
        test.result(result)
            .assert_lint("unbound-method")
            .assert_has_no_fix("unbound-method");
    }

    /// Flag unbound private method references.
    #[test]
    fn test_flags_unbound_private_method_reference() {
        let test = TestProgram::for_rule_with_prelude(UnboundMethod);
        let result = test.lint_dir(
            "unbound_method/test_flags_unbound_private_method_reference.ts",
            r#"
class Counter {
    #increment() {
    }

    callback() {
        let ref = this.#increment;
    }
}
"#,
        );
        test.result(result).assert_lint("unbound-method");
    }
}
