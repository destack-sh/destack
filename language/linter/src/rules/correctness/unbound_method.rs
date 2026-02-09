use destack_ast::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    resolution_target_symbols, symbol_primary_declaration_for, symbol_value_type_id_for,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

const DEFAULT_RELATION_CACHE_KEY: u64 = 0;

declare_lint! {
    /// Disallow unbound instance methods.
    ///
    /// Referencing an instance method without calling or binding it can lose
    /// the receiver and break `this` dependent logic.
    #[lint(
        id = "unbound-method",
        code = "LC055",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub UnboundMethod,
    "Disallow unbound methods as callbacks"
}

impl LintRule for UnboundMethod {
    fn meta(&self) -> &'static LintMeta {
        UnboundMethod::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let bind_name = ctx.program.strings.intern("bind");
        let call_name = ctx.program.strings.intern("call");
        let apply_name = ctx.program.strings.intern("apply");

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

        let global_id = expression_id.into_global_any(self.ctx.module_id());
        let Some(resolution_id) = self.ctx.types.get_resolution_for_node(global_id) else {
            return;
        };
        let resolution = self.ctx.types.get_resolution(resolution_id);

        let is_unbound_method = resolution_target_symbols(resolution)
            .into_iter()
            .any(|symbol| self.is_this_bound_method_symbol(symbol));
        if !is_unbound_method {
            return;
        }

        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                UNBOUND_METHOD.id,
                UNBOUND_METHOD.code,
                UNBOUND_METHOD.category,
                severity,
                "unbound method reference",
                self.ctx.module.file_id,
                span,
            )
            .with_label("bind this method or wrap it in a lambda"),
        );
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
                dir::Expression::Cast { value, .. }
                | dir::Expression::OwnershipCast { value, .. }
                    if *value == current_id =>
                {
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
                    if *left == current_id && self.is_safe_call_like_invocation(parent_id) =>
                {
                    return true;
                }
                dir::Expression::Member { left, name, .. } if *left == current_id => {
                    if *name == self.bind_name
                        || *name == self.call_name
                        || *name == self.apply_name
                    {
                        current_id = parent_id;
                        continue;
                    }

                    return false;
                }
                dir::Expression::PrivateMember { left, name, .. } if *left == current_id => {
                    if *name == self.bind_name
                        || *name == self.call_name
                        || *name == self.apply_name
                    {
                        current_id = parent_id;
                        continue;
                    }

                    return false;
                }
                _ => return false,
            }
        }
    }

    /// Return true when a call-like invocation safely binds method receivers.
    fn is_safe_call_like_invocation(
        &self,
        call_like_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression = self.ctx.tree.get(call_like_id);
        let (callee_id, dynamic_arguments) = match expression {
            dir::Expression::Call {
                left,
                dynamic_arguments,
                ..
            }
            | dir::Expression::New {
                left,
                dynamic_arguments,
                ..
            } => (*left, dynamic_arguments.as_slice()),
            _ => return false,
        };

        let callee = self.ctx.tree.get(callee_id);
        let uses_receiver_helper = matches!(
            callee,
            dir::Expression::Member { name, .. } | dir::Expression::PrivateMember { name, .. }
                if *name == self.bind_name || *name == self.call_name || *name == self.apply_name
        );

        if !uses_receiver_helper {
            return true;
        }

        !dynamic_arguments.is_empty()
    }

    /// Return true when a symbol is a method that needs a bound `this`.
    fn is_this_bound_method_symbol(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        // prefer declarations to identify method symbols and skip static members
        let Some(primary_declaration) = symbol_primary_declaration_for(
            &self.ctx.program,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            symbol_id,
        ) else {
            return self.symbol_has_this_parameter(symbol_id);
        };

        if primary_declaration.local_id.ty == dir::NodeType::Member {
            let module_ref = self.ctx.program.modules.get(primary_declaration.module_id);
            let module = module_ref.read();
            let Some(module_dir) = module.dir_maybe(self.ctx.profile_id) else {
                return self.symbol_has_this_parameter(symbol_id);
            };
            let tree = module_dir.tree.read();
            let member = tree.get(primary_declaration.into_local_typed::<dir::Member>());
            let dir::Member::Method { modifiers, .. } = member else {
                return false;
            };

            return !modifiers
                .as_ref()
                .and_then(|modifier| modifier.anchor)
                .is_some_and(|anchor| anchor == dir::BindingAnchor::Static);
        }

        if primary_declaration.local_id.ty == dir::NodeType::Property {
            let module_ref = self.ctx.program.modules.get(primary_declaration.module_id);
            let module = module_ref.read();
            let Some(module_dir) = module.dir_maybe(self.ctx.profile_id) else {
                return self.symbol_has_this_parameter(symbol_id);
            };
            let tree = module_dir.tree.read();
            let property = tree.get(primary_declaration.into_local_typed::<dir::Property>());
            return matches!(property, dir::Property::Method { .. });
        }

        self.symbol_has_this_parameter(symbol_id)
    }

    /// Return true when a symbol value type declares a `this` parameter.
    fn symbol_has_this_parameter(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        let Some(symbol_type_id) = symbol_value_type_id_for(
            &self.ctx.program,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            self.ctx.types,
            symbol_id,
        ) else {
            return false;
        };

        if symbol_type_id.module_id == self.ctx.module_id() {
            return type_has_this_parameter(self.ctx.types, symbol_type_id.type_id);
        }

        let module_ref = self.ctx.program.modules.get(symbol_type_id.module_id);
        let module = module_ref.read();
        let Some(module_dir) = module.dir_maybe(self.ctx.profile_id) else {
            return false;
        };
        let types = module_dir.types.read();
        type_has_this_parameter(&types, symbol_type_id.type_id)
    }
}

impl NodeVisitor for UnboundMethodVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
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

/// Return true when a type declares a `this` parameter.
fn type_has_this_parameter(types: &dir::TypeTable, type_id: dir::LocalTypeId) -> bool {
    let normalized_type_id = types
        .normalized_type(
            dir::NormalizationMode::Flow,
            DEFAULT_RELATION_CACHE_KEY,
            type_id,
        )
        .map(|entry| entry.normalized_type)
        .unwrap_or(type_id);

    let mut visited = Vec::new();
    type_has_this_parameter_inner(types, normalized_type_id, &mut visited)
}

/// Return true when a type declares a `this` parameter with cycle protection.
fn type_has_this_parameter_inner(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
    visited: &mut Vec<dir::LocalTypeId>,
) -> bool {
    if visited.contains(&type_id) {
        return false;
    }
    visited.push(type_id);

    match types.get_type(type_id) {
        dir::Type::Function { this_parameter, .. } => this_parameter.is_some(),
        dir::Type::Value { value } => type_has_this_parameter_inner(types, *value, visited),
        dir::Type::ValueOf { right, .. }
        | dir::Type::ReferenceOf { right, .. }
        | dir::Type::PointerOf { right, .. } => {
            type_has_this_parameter_inner(types, *right, visited)
        }
        dir::Type::Union { elements } | dir::Type::Intersection { elements } => elements
            .iter()
            .any(|element| type_has_this_parameter_inner(types, *element, visited)),
        dir::Type::Reference { symbol, .. } => {
            if let Some(instance_type_id) = types.get_instance_type_id(*symbol) {
                return type_has_this_parameter_inner(types, instance_type_id, visited);
            }
            if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                return type_has_this_parameter_inner(types, value_type_id, visited);
            }

            false
        }
        _ => false,
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
        test.result(result).assert_lint("unbound-method");
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
        test.result(result).assert_lint("unbound-method");
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
