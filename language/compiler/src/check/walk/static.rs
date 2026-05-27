use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Decorator, ReceiverCapture, StaticCondition, StaticIfCondition, StaticPredicate,
    StaticTerm, TypeTerm, VariableId,
};
use crate::common::dir::r#static::{StaticContext, StaticFailure};

impl CheckState<'_> {
    /// Return the static condition attached to one owner.
    pub(in crate::check) fn static_condition(
        &mut self,
        tree: &dir::Tree,
        owner: dir::LocalNodeIdAny,
        receiver: Option<ReceiverCapture>,
    ) -> StaticCondition {
        let decorators = self.decorators_for_owner(tree.module_id, owner);
        let mut condition = StaticCondition::Always;

        // combine visible static guards in source order
        for decorator in decorators {
            match decorator {
                Decorator::StaticIf(decorator) => {
                    let StaticIfCondition::Present(condition_expression) = decorator.condition
                    else {
                        self.report_invalid_static_condition(
                            tree.module_id,
                            decorator.condition_anchor(),
                        );

                        return StaticCondition::Never;
                    };
                    let next = self.evaluate_static_guard(tree, receiver, condition_expression);

                    condition = condition.and(next);
                    if condition.is_never() {
                        return StaticCondition::Never;
                    }
                }
                Decorator::Other(call) => {
                    let decorator_node = self
                        .input(tree.module_id)
                        .view()
                        .get(call.decorator)
                        .clone();

                    self.walk_decorator(tree, call.decorator, &decorator_node);
                }
                Decorator::LanguageItem(_)
                | Decorator::Intrinsic(_)
                | Decorator::Representation(_)
                | Decorator::Capture(_)
                | Decorator::Diagnostic(_)
                | Decorator::Foreign(_)
                | Decorator::Restriction(_)
                | Decorator::System(_)
                | Decorator::Stability(_)
                | Decorator::Taint(_)
                | Decorator::Macro(_) => {}
            }
        }

        condition
    }

    /// Push one static condition for subsequently walked work.
    pub(in crate::check) fn push_static_condition(
        &mut self,
        module: ModuleId,
        condition: StaticCondition,
    ) {
        self.flow_mut(module).push_static_condition(condition);
    }

    /// Pop the current static condition.
    pub(in crate::check) fn pop_static_condition(&mut self, module: ModuleId) {
        self.flow_mut(module).pop_static_condition();
    }

    /// Push the static condition attached to one owner.
    pub(in crate::check) fn push_static_condition_for(
        &mut self,
        tree: &dir::Tree,
        owner: dir::LocalNodeIdAny,
        receiver: Option<ReceiverCapture>,
    ) -> bool {
        let owner_condition = self.static_condition(tree, owner, receiver);
        let condition = self
            .active_static_condition(tree.module_id)
            .and(owner_condition.clone());

        // record symbol availability after combining guards
        if let Some(symbol) = self.declaration_symbol(tree.module_id, owner) {
            self.availability_mut(tree.module_id)
                .insert(symbol, condition.clone());
        }

        if condition.is_never() {
            return false;
        }
        self.push_static_condition(tree.module_id, owner_condition);

        true
    }

    /// Return the currently active static condition.
    pub(in crate::check) fn active_static_condition(&self, module: ModuleId) -> StaticCondition {
        self.flow(module).current_static_condition()
    }

    /// Evaluate one static guard condition.
    fn evaluate_static_guard(
        &mut self,
        tree: &dir::Tree,
        receiver: Option<ReceiverCapture>,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> StaticCondition {
        self.with_static_receiver(tree.module_id, receiver, |this| {
            let context = StaticContext::new(
                this.input(tree.module_id).view(),
                this.input(tree.module_id).module.as_ref(),
                &this.input(tree.module_id).profile,
                &this.input(tree.module_id).profile.conditions,
                &this.input(tree.module_id).strings,
            );

            match context.evaluate_boolean(condition) {
                Ok(true) => StaticCondition::Always,
                Ok(false) => StaticCondition::Never,
                Err(StaticFailure::NotBoolean(expression)) => {
                    this.report_invalid_static_condition(tree.module_id, expression.into_any());

                    StaticCondition::Never
                }
                Err(StaticFailure::NotStatic(expression)) => {
                    if this.is_deferred_static_guard(tree, condition) {
                        let variable =
                            this.define_static_expression_variable(tree.module_id, condition);

                        // define static guard leaves without runtime condition constraints
                        this.walk_static_expression(tree, condition);

                        StaticCondition::When {
                            conditions: smallvec::smallvec![StaticPredicate {
                                module: tree.module_id,
                                term: StaticTerm::Variable(variable),
                            }],
                        }
                    } else {
                        this.walk_expression(tree, expression, tree.get(expression));
                        this.report_invalid_static_condition(tree.module_id, expression.into_any());

                        StaticCondition::Never
                    }
                }
            }
        })
    }

    /// Run one callback with a static receiver visible.
    fn with_static_receiver<R>(
        &mut self,
        module: ModuleId,
        receiver: Option<ReceiverCapture>,
        walk: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let has_receiver = receiver.is_some();
        if let Some(receiver) = receiver {
            self.flow_mut(module).push_receiver(receiver);
        }
        let result = walk(self);
        if has_receiver {
            self.flow_mut(module).pop_receiver();
        }

        result
    }

    /// Walk leaves needed to build one static expression.
    pub(in crate::check) fn walk_static_expression(
        &mut self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
    ) {
        match tree.get(expression) {
            // (C)
            dir::Expression::Parenthesized { expression } => {
                self.walk_static_expression(tree, *expression);
            }
            // this
            dir::Expression::This => {
                let source = expression.into_global_any(tree.module_id);
                if let Some(receiver) = self.define_this_receiver_variable(source) {
                    let variable = self.intern_local_type_variable(tree.module_id, expression);

                    self.define_type(tree.module_id, variable, TypeTerm::Variable(receiver));
                }
            }
            // this.X
            dir::Expression::Member { left, .. } | dir::Expression::PrivateMember { left, .. } => {
                self.walk_static_expression(tree, *left);
            }
            // C && D, C == D
            dir::Expression::Binary { left, right, .. } => {
                self.walk_static_expression(tree, *left);
                self.walk_static_expression(tree, *right);
            }
            // !C
            dir::Expression::Unary { right, .. } => {
                self.walk_static_expression(tree, *right);
            }
            // <T extends U>
            dir::Expression::Type { value } => {
                self.walk_static_type_expression(tree, *value);
            }
            // C[I]
            dir::Expression::Index { left, index, .. } => {
                self.walk_static_expression(tree, *left);
                if let Some(index) = index {
                    self.walk_static_expression(tree, *index);
                }
            }
            // literal or unsupported static leaf
            _ => {}
        }
    }

    /// Walk leaves needed by a type-space static expression.
    fn walk_static_type_expression(
        &mut self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::TypeExpression>,
    ) {
        match tree.get(expression) {
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                self.walk_static_type_expression(tree, *expression);
            }
            // T extends U
            dir::TypeExpression::Extends { left, right }
            | dir::TypeExpression::Implements { left, right } => {
                self.walk_type_expression(tree, *left, tree.get(*left));
                self.walk_type_expression(tree, *right, tree.get(*right));
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left: _,
                extends_type: _,
                then_type: _,
                else_type: _,
            } => {
                self.walk_type_expression(tree, expression, tree.get(expression));
            }
            // other type expressions are walked normally when directly needed
            _ => self.walk_type_expression(tree, expression, tree.get(expression)),
        }
    }

    /// Return whether one guard belongs to solver-known static state.
    fn is_deferred_static_guard(
        &self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match tree.get(expression) {
            // (C)
            dir::Expression::Parenthesized { expression } => {
                self.is_deferred_static_guard(tree, *expression)
            }
            // this or this.X
            dir::Expression::This => true,
            // C && D, C == D, and similar static compositions
            dir::Expression::Binary { left, right, .. } => {
                self.is_deferred_static_guard(tree, *left)
                    || self.is_deferred_static_guard(tree, *right)
            }
            // !C
            dir::Expression::Unary { right, .. } => self.is_deferred_static_guard(tree, *right),
            // type relation
            dir::Expression::Type { value } => self.is_deferred_static_type_guard(tree, *value),
            // C.X
            dir::Expression::Member { left, .. } => self.is_deferred_static_guard(tree, *left),
            // C[I]
            dir::Expression::Index { left, index, .. } => {
                self.is_deferred_static_guard(tree, *left)
                    || index.is_some_and(|index| self.is_deferred_static_guard(tree, index))
            }
            // not solver-known static guard syntax
            _ => false,
        }
    }

    /// Return whether one type expression belongs to solver-known static state.
    fn is_deferred_static_type_guard(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> bool {
        match tree.get(id) {
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                self.is_deferred_static_type_guard(tree, *expression)
            }
            // T
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                self.type_reference_is_generic_parameter(tree.module_id, id, path)
                    || generic_arguments
                        .iter()
                        .any(|argument| self.is_deferred_static_generic_argument(tree, *argument))
            }
            // T.Item
            dir::TypeExpression::Member {
                left,
                generic_arguments,
                ..
            } => {
                self.is_deferred_static_type_guard(tree, *left)
                    || generic_arguments
                        .iter()
                        .any(|argument| self.is_deferred_static_generic_argument(tree, *argument))
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                self.is_deferred_static_type_guard(tree, *left)
                    || self.is_deferred_static_type_guard(tree, *extends_type)
                    || self.is_deferred_static_type_guard(tree, *then_type)
                    || self.is_deferred_static_type_guard(tree, *else_type)
            }
            // T extends U
            dir::TypeExpression::Extends { left, right }
            // T implements U
            | dir::TypeExpression::Implements { left, right } => {
                self.is_deferred_static_type_guard(tree, *left)
                    || self.is_deferred_static_type_guard(tree, *right)
            }
            // readonly T, local T, and similar unary type forms
            dir::TypeExpression::Readonly { target_type }
            | dir::TypeExpression::Local { target_type }
            | dir::TypeExpression::Shared { target_type }
            | dir::TypeExpression::KeyOf { target_type }
            | dir::TypeExpression::Must { target_type }
            | dir::TypeExpression::Not { target_type }
            | dir::TypeExpression::OwnedOf { target_type, .. }
            | dir::TypeExpression::BorrowedOf { target_type, .. }
            | dir::TypeExpression::PointerOf { target_type, .. } => {
                self.is_deferred_static_type_guard(tree, *target_type)
            }
            // T | U
            dir::TypeExpression::Union { elements }
            // T & U
            | dir::TypeExpression::Intersection { elements } => elements
                .iter()
                .any(|element| self.is_deferred_static_type_guard(tree, *element)),
            // T[]
            dir::TypeExpression::Array { element }
            // [T]
            | dir::TypeExpression::Slice { element } => {
                self.is_deferred_static_type_guard(tree, *element)
            }
            // [T; N]
            dir::TypeExpression::FixedArray { element, length } => {
                self.is_deferred_static_type_guard(tree, *element)
                    || self.is_deferred_static_guard(tree, *length)
            }
            // typeof value
            dir::TypeExpression::TypeOfValue { value } => self.is_deferred_static_guard(tree, *value),
            // T[K]
            dir::TypeExpression::Index { left, index } => {
                self.is_deferred_static_type_guard(tree, *left)
                    || self.is_deferred_static_type_guard(tree, *index)
            }
            // infer T extends U
            dir::TypeExpression::Infer { constraint, .. } => constraint
                .is_some_and(|constraint| self.is_deferred_static_type_guard(tree, constraint)),
            // value is T
            dir::TypeExpression::Predicate { target, .. } => target
                .is_some_and(|target| self.is_deferred_static_type_guard(tree, target)),
            // type forms without static generic leaves
            _ => false,
        }
    }

    /// Return whether one generic argument belongs to solver-known static state.
    fn is_deferred_static_generic_argument(
        &self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericArgument>,
    ) -> bool {
        match tree.get(id) {
            // <T>
            dir::GenericArgument::Type { value }
            // <...T>
            | dir::GenericArgument::SpreadType { value } => {
                self.is_deferred_static_type_guard(tree, *value)
            }
            // <C>
            dir::GenericArgument::Value { value }
            // <...C>
            | dir::GenericArgument::SpreadValue { value } => {
                self.is_deferred_static_guard(tree, *value)
            }
            // ignore damaged syntax
            dir::GenericArgument::Error => false,
        }
    }

    /// Return whether one reference names a generic parameter.
    fn type_reference_is_generic_parameter(
        &self,
        module: ModuleId,
        source: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
    ) -> bool {
        let [name] = path.segments.as_slice() else {
            return false;
        };

        if let Some(symbol) = self
            .lookup_symbol_by_name(module, source.into_any(), *name, dir::SymbolSpace::Type)
            .unique_symbol()
            && self.symbol_kind(module, symbol) == Some(dir::SymbolKind::GenericTypeParameter)
        {
            return true;
        }

        self.lookup_symbol_by_name(module, source.into_any(), *name, dir::SymbolSpace::Value)
            .unique_symbol()
            .is_some_and(|symbol| {
                self.symbol_kind(module, symbol) == Some(dir::SymbolKind::GenericValueParameter)
            })
    }

    /// Return one static argument variable.
    pub(in crate::check) fn static_argument_variable(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> VariableId {
        let module = tree.module_id;
        let source = id.into_global_any(module);
        let variable = self.intern_node_static_variable(module, source);

        if let Some(term) = self.build_static_argument_term(id, tree) {
            self.define_static(module, variable, term);
        }

        variable
    }

    /// Return one static argument term.
    fn build_static_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> Option<StaticTerm> {
        let term = match tree.get(id) {
            // <(C)>
            dir::TypeExpression::Parenthesized { expression } => {
                return self.build_static_argument_term(*expression, tree);
            }
            // <1>
            dir::TypeExpression::ScalarLiteral { value } => {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: value.clone(),
                })
            }
            // <"name">
            dir::TypeExpression::Literal { value } => {
                StaticTerm::Literal(dir::StaticTerm::TypeLiteral {
                    value: value.clone(),
                })
            }
            // <{ name: "value" }>
            dir::TypeExpression::Object { members } => {
                self.build_static_object_term(members, tree)?
            }
            // <[1, 2]>
            dir::TypeExpression::Tuple { elements } => {
                self.build_static_tuple_term(elements, tree)?
            }
            // <L | R>
            dir::TypeExpression::Union { elements } => StaticTerm::Join {
                elements: elements
                    .iter()
                    .map(|element| self.static_argument_variable(*element, tree))
                    .collect(),
            },
            // <readonly [1, 2]>
            dir::TypeExpression::ArrayTuple { elements } => {
                self.build_static_tuple_term(elements, tree)?
            }
            // <C>
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                let [name] = path.segments.as_slice() else {
                    return None;
                };
                if generic_arguments.is_empty()
                    && let Some(symbol) = self
                        .lookup_symbol_by_name(
                            tree.module_id,
                            id.into_any(),
                            *name,
                            dir::SymbolSpace::Value,
                        )
                        .unique_symbol()
                    && self.symbol_kind(tree.module_id, symbol)
                        == Some(dir::SymbolKind::GenericValueParameter)
                {
                    let variable = self.intern_symbol_static_variable(tree.module_id, symbol);

                    StaticTerm::Variable(variable)
                }
                // <LifetimeOf<T>>
                else if let Some(symbol) = self
                    .lookup_symbol_by_name(
                        tree.module_id,
                        id.into_any(),
                        *name,
                        dir::SymbolSpace::Type,
                    )
                    .unique_symbol()
                    && let Some(item) = self.input(tree.module_id).environment.language.item(symbol)
                    && Self::memory_static_intrinsic_item(item)
                {
                    let arguments =
                        self.build_generic_arguments_for_owner(symbol, generic_arguments, tree);

                    StaticTerm::Intrinsic {
                        item,
                        arguments: arguments.into(),
                    }
                }
                // unsupported reference syntax
                else {
                    return None;
                }
            }
            // <T.Value>
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let arguments = self.build_generic_arguments(generic_arguments, tree);

                StaticTerm::Member {
                    source: Some(id.into_global_any(tree.module_id)),
                    owner: self.intern_local_type_variable(tree.module_id, *left),
                    key: dir::StaticKey::Name(*name),
                    arguments,
                }
            }
            // unsupported static argument syntax
            _ => return None,
        };

        Some(term)
    }

    /// Return one static object term.
    fn build_static_object_term(
        &mut self,
        members: &[dir::LocalNodeId<dir::TypeMember>],
        tree: &dir::Tree,
    ) -> Option<StaticTerm> {
        let mut properties = Vec::with_capacity(members.len());

        for member in members {
            match tree.get(*member) {
                // <{ key: value }>
                dir::TypeMember::Field {
                    key,
                    declared_type: Some(value),
                    is_static: false,
                    is_optional: false,
                    ..
                } => {
                    let key = key.static_key(tree)?;
                    let StaticTerm::Literal(value) =
                        self.build_static_argument_term(*value, tree)?
                    else {
                        return None;
                    };

                    properties.push(dir::StaticProperty::Field { key, value });
                }
                // unsupported object member syntax
                _ => return None,
            }
        }

        Some(StaticTerm::Literal(dir::StaticTerm::Object { properties }))
    }

    /// Return one static tuple term.
    fn build_static_tuple_term(
        &mut self,
        elements: &[dir::LocalNodeId<dir::TupleElement>],
        tree: &dir::Tree,
    ) -> Option<StaticTerm> {
        let mut values = Vec::with_capacity(elements.len());

        for element in elements {
            match tree.get(*element) {
                // <[value]>
                dir::TupleElement::Element {
                    value,
                    is_optional: false,
                    ..
                } => {
                    let StaticTerm::Literal(value) =
                        self.build_static_argument_term(*value, tree)?
                    else {
                        return None;
                    };

                    values.push(value);
                }
                // unsupported tuple element syntax
                _ => return None,
            }
        }

        Some(StaticTerm::Literal(dir::StaticTerm::Tuple {
            elements: values,
        }))
    }
}
