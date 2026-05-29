use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Condition, ConditionPredicate, Decorator, Origin, ReceiverCapture,
    StaticIfCondition, StaticTerm, TypeTerm, VariableId,
};
use crate::common::dir::r#static::{StaticContext, StaticFailure};

impl CheckState<'_> {
    /// Return the static condition attached to one owner.
    pub(in crate::check) fn static_condition(
        &mut self,
        tree: &dir::Tree,
        owner: dir::LocalNodeIdAny,
        receiver: Option<ReceiverCapture>,
    ) -> Condition {
        let decorators = self.decorators_for_owner(tree.module_id, owner);
        let mut condition = Condition::Always;

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

                        return Condition::Never;
                    };
                    let next = self.evaluate_static_guard(tree, receiver, condition_expression);

                    condition = condition.and(next);
                    if condition.is_never() {
                        return Condition::Never;
                    }
                }
                Decorator::Other(call) => {
                    let decorator_node = self
                        .module(tree.module_id)
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
        condition: Condition,
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

        // store symbol availability after combining guards
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
    pub(super) fn active_static_condition(&self, module: ModuleId) -> Condition {
        self.flow(module).current_static_condition()
    }

    /// Evaluate one static guard condition.
    fn evaluate_static_guard(
        &mut self,
        tree: &dir::Tree,
        receiver: Option<ReceiverCapture>,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> Condition {
        self.with_static_receiver(tree.module_id, receiver, |this| {
            let context = StaticContext::new(
                this.module(tree.module_id).view(),
                this.module(tree.module_id).module.as_ref(),
                &this.module(tree.module_id).profile,
                &this.module(tree.module_id).profile.conditions,
                &this.module(tree.module_id).strings,
            );

            match context.evaluate_boolean(condition) {
                Ok(true) => Condition::Always,
                Ok(false) => Condition::Never,
                Err(StaticFailure::NotBoolean(expression)) => {
                    this.report_invalid_static_condition(tree.module_id, expression.into_any());

                    Condition::Never
                }
                Err(StaticFailure::NotStatic(expression)) => {
                    let condition_guard = this.active_static_condition(tree.module_id);
                    let variable = this.define_static_expression_variable(
                        tree.module_id,
                        condition,
                        condition_guard,
                    );

                    // define static guard leaves without runtime condition constraints
                    this.walk_static_expression(tree, expression);

                    Condition::When {
                        conditions: smallvec::smallvec![ConditionPredicate {
                            origin: Origin::Node(condition.into_global_any(tree.module_id)),
                            term: StaticTerm::Variable(variable),
                        }],
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
                    let variable = self.intern_local_node_type_variable(tree.module_id, expression);
                    let condition = self.active_static_condition(tree.module_id);

                    self.add_type_definition(variable, TypeTerm::Variable(receiver), condition);
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
            let condition = self.active_static_condition(module);

            self.add_static_definition(variable, term, condition);
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
                    && let Some(item) = self.environment.language.item(symbol)
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
                    source: id.into_global_any(tree.module_id),
                    owner: self.intern_local_node_type_variable(tree.module_id, *left),
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
