use crate::check::{
    Condition, ConditionPredicate, Origin, ReceiverCapture, StaticIfCondition, StaticOperand,
    StaticTerm, VariableId, VariableKind, WalkState,
};
use crate::common::dir::r#static::{StaticContext, StaticFailure};
use destack_dir as dir;

impl WalkState<'_, '_> {
    /// Evaluate the static guard attached to one owner.
    ///
    /// Example:
    /// ```ds
    /// @if(Target.isShared)
    /// function f() {}
    /// ```
    pub(in crate::check) fn evaluate_owner_static_guard(
        &mut self,
        tree: &dir::Tree,
        owner: dir::LocalNodeIdAny,
        receiver: Option<ReceiverCapture>,
    ) -> Condition {
        let invocations = self
            .check
            .decorator_invocations_for_owner(tree.module_id, owner);
        let mut condition = Condition::Always;

        // combine visible static guards in source order
        for invocation in invocations {
            if let Some(decorator) = self
                .check
                .static_if_decorator_from_invocation(tree.module_id, &invocation)
            {
                let StaticIfCondition::Present(condition_expression) = decorator.condition else {
                    self.check
                        .report_invalid_static_guard(tree.module_id, decorator.condition_anchor());

                    return Condition::Never;
                };
                let next = self.evaluate_static_guard(tree, receiver, condition_expression);

                condition = condition.and(next);
                if condition.is_never() {
                    return Condition::Never;
                }
            } else {
                let decorator_node = self
                    .check
                    .module(tree.module_id)
                    .view()
                    .get(invocation.decorator)
                    .clone();

                self.walk_decorator(tree, invocation.decorator, &decorator_node);
            }
        }

        condition
    }

    /// Push one static guard for subsequently walked work.
    pub(in crate::check) fn push_static_guard(&mut self, condition: Condition) {
        self.flow_mut().push_static_guard(condition);
    }

    /// Pop the current static guard.
    pub(in crate::check) fn pop_static_guard(&mut self) {
        self.flow_mut().pop_static_guard();
    }

    /// Push the static guard attached to one owner.
    ///
    /// Example:
    /// ```ds
    /// @if(Enabled)
    /// const value = 1;
    /// ```
    pub(in crate::check) fn push_static_guard_for(
        &mut self,
        tree: &dir::Tree,
        owner: dir::LocalNodeIdAny,
        receiver: Option<ReceiverCapture>,
    ) -> bool {
        let owner_condition = self.evaluate_owner_static_guard(tree, owner, receiver);
        let condition = self.active_static_guard().and(owner_condition.clone());

        // store symbol availability after combining guards
        if let Some(symbol) = self.check.declaration_symbol(tree.module_id, owner) {
            self.check
                .availability_mut(tree.module_id)
                .insert(symbol, condition.clone());
        }

        // just bail if statically never
        if condition.is_never() {
            false
        }
        // actually push and keep going (conditionally)
        else {
            self.push_static_guard(owner_condition);
            true
        }
    }

    /// Return the active static guard.
    pub(in crate::check) fn active_static_guard(&self) -> Condition {
        self.flow().active_static_guard()
    }

    /// Evaluate one static guard expression.
    ///
    /// Example:
    /// ```ds
    /// Enabled && Target.isShared
    /// ```
    fn evaluate_static_guard(
        &mut self,
        tree: &dir::Tree,
        receiver: Option<ReceiverCapture>,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> Condition {
        self.with_static_receiver(receiver, |this| {
            let context = StaticContext::new(
                this.check.module(tree.module_id).view(),
                this.check.module(tree.module_id).module.as_ref(),
                &this.check.module(tree.module_id).profile,
                &this.check.module(tree.module_id).profile.conditions,
                &this.check.module(tree.module_id).strings,
            );

            match context.evaluate_boolean(condition) {
                Ok(true) => Condition::Always,
                Ok(false) => Condition::Never,
                Err(StaticFailure::NotBoolean(expression)) => {
                    this.check
                        .report_invalid_static_guard(tree.module_id, expression.into_any());

                    Condition::Never
                }
                Err(StaticFailure::NotStatic(expression)) => {
                    let condition_guard = this.active_static_guard();
                    let variable = this.check.reserve_static_expression(
                        tree.module_id,
                        condition,
                        condition_guard,
                    );

                    // walk static guard leaves without runtime condition constraints
                    this.walk_static_expression(tree, expression);

                    Condition::When {
                        conditions: smallvec::smallvec![ConditionPredicate {
                            origin: Origin::Node(condition.into_global_any(tree.module_id)),
                            operand: variable.into(),
                        }],
                    }
                }
            }
        })
    }

    /// Run one callback with a static receiver visible.
    fn with_static_receiver<R>(
        &mut self,
        receiver: Option<ReceiverCapture>,
        walk: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let has_receiver = receiver.is_some();
        if let Some(receiver) = receiver {
            self.flow_mut().push_receiver(receiver);
        }
        let result = walk(self);
        if has_receiver {
            self.flow_mut().pop_receiver();
        }

        result
    }

    /// Walk leaves needed by one static expression.
    ///
    /// Example:
    /// ```ds
    /// this.Place == local
    /// ```
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
                if let Some(term) = self.lower_this_receiver_type_term(source) {
                    self.publish_node_type(tree.module_id, expression, term);
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
            // literal or non static leaf
            _ => {}
        }
    }

    /// Walk leaves needed by a type-space static expression.
    ///
    /// Example:
    /// ```ds
    /// T extends Borrowed
    /// ```
    pub(in crate::check) fn walk_static_type_expression(
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

    /// Lower one static expression operand without committing a checked node operand.
    ///
    /// Example:
    /// ```ds
    /// <Size + 1>
    /// ```
    pub(in crate::check) fn lower_static_expression_operand(
        &mut self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> StaticOperand {
        // walk static leaves without keeping expression flow changes
        let before_expression = self.fork_flow();

        self.walk_static_expression(tree, expression);
        self.restore_flow(before_expression);

        // allocate the argument local static variable
        let condition = self.active_static_guard();
        let variable =
            self.check
                .allocate_static_expression_variable(tree.module_id, expression, condition);

        variable.into()
    }

    /// Create one static variable from a type-space static argument.
    ///
    /// Example:
    /// ```ds
    /// <Size>
    /// ```
    pub(in crate::check) fn create_static_argument_variable(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> VariableId {
        let module = tree.module_id;
        let source = id.into_global_any(module);
        let origin = Origin::Node(source);
        let variable = self
            .check
            .allocate_variable(module, VariableKind::Static, origin);

        if let Some(operand) = self.lower_direct_static_argument_operand(id, tree) {
            let condition = self.active_static_guard();

            self.check
                .equate_static(origin, variable, operand, condition);
        }

        variable
    }

    /// Lower one directly representable static argument term.
    ///
    /// Example:
    /// ```ds
    /// <{ a: 2, b: [1, 2, 3] }>
    /// ```
    fn lower_direct_static_argument_operand(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::Tree,
    ) -> Option<StaticOperand> {
        let term = match tree.get(id) {
            // <(C)>
            dir::TypeExpression::Parenthesized { expression } => {
                return self.lower_direct_static_argument_operand(*expression, tree);
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
                self.lower_direct_static_object_term(members, tree)?
            }
            // <[1, 2]>
            dir::TypeExpression::Tuple { elements } => {
                self.lower_direct_static_tuple_term(elements, tree)?
            }
            // <L | R>
            dir::TypeExpression::Union { elements } => StaticTerm::Union {
                elements: elements
                    .iter()
                    .map(|element| self.create_static_argument_variable(*element, tree))
                    .map(StaticOperand::from)
                    .collect(),
            },
            // <readonly [1, 2]>
            dir::TypeExpression::ArrayTuple { elements } => {
                self.lower_direct_static_tuple_term(elements, tree)?
            }
            // <C>
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                return self.lower_static_reference_argument_operand(
                    id,
                    path,
                    generic_arguments,
                    tree,
                );
            }
            // <T.Value>
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let arguments = self.lower_generic_arguments(None, generic_arguments, tree);

                StaticTerm::Member {
                    source: id.into_global_any(tree.module_id),
                    owner: self.check.require_local_node_type(tree.module_id, *left),
                    key: dir::StaticKey::Name(*name),
                    arguments: arguments.into_vec(),
                }
            }
            // not directly representable static argument syntax
            _ => return None,
        };

        Some(self.check.push_term(term).into())
    }

    /// Lower one static reference argument operand.
    ///
    /// Example:
    /// ```ds
    /// <LifetimeOf<T>>
    /// ```
    fn lower_static_reference_argument_operand(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        path: &dir::Path,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<StaticOperand> {
        let [name] = path.segments.as_slice() else {
            return None;
        };

        // ensure bare static parameter
        let term = if generic_arguments.is_empty()
            && let Some(operand) = self.ensure_static_parameter_argument_operand(id, *name, tree)
        {
            operand
        }
        // lower static memory intrinsic
        else if let Some(term) =
            self.lower_static_intrinsic_argument_term(id, *name, generic_arguments, tree)
        {
            self.check.push_term(term).into()
        }
        // not a static reference argument
        else {
            return None;
        };

        Some(term)
    }

    /// Ensure one static parameter argument operand.
    ///
    /// Example:
    /// ```ds
    /// <Size>
    /// ```
    fn ensure_static_parameter_argument_operand(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        name: dir::StringId,
        tree: &dir::Tree,
    ) -> Option<StaticOperand> {
        let guard = self.active_static_guard();
        let symbol = self
            .check
            .lookup_name_by_name(tree.module_id, id.into_any(), name, dir::SymbolSpace::Value)
            .available_under(&guard)
            .unique_symbol()?;

        if self.check.symbol_kind(tree.module_id, symbol)
            != Some(dir::SymbolKind::GenericValueParameter)
        {
            return None;
        }

        let variable = self
            .check
            .ensure_symbol_static_variable(tree.module_id, symbol);

        Some(variable.into())
    }

    /// Lower one static intrinsic argument term.
    ///
    /// Example:
    /// ```ds
    /// <LifetimeOf<T>>
    /// ```
    fn lower_static_intrinsic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        name: dir::StringId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        tree: &dir::Tree,
    ) -> Option<StaticTerm> {
        let guard = self.active_static_guard();
        let symbol = self
            .check
            .lookup_name_by_name(tree.module_id, id.into_any(), name, dir::SymbolSpace::Type)
            .available_under(&guard)
            .unique_symbol()?;
        let item = self.check.environment.language.item(symbol)?;

        if !Self::is_memory_static_intrinsic(item) {
            return None;
        }

        let arguments = self.lower_generic_arguments(Some(symbol), generic_arguments, tree);

        Some(StaticTerm::Intrinsic {
            item,
            arguments: arguments.into_vec(),
        })
    }

    /// Lower one directly representable static object term.
    ///
    /// Example:
    /// ```ds
    /// { a: 2, b: [1, 2, 3] }
    /// ```
    fn lower_direct_static_object_term(
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
                    let Some(StaticTerm::Literal(value)) = self
                        .lower_direct_static_argument_operand(*value, tree)?
                        .known_static_term(self.check)
                    else {
                        return None;
                    };

                    properties.push(dir::StaticProperty::Field { key, value });
                }
                // not directly representable object member syntax
                _ => return None,
            }
        }

        Some(StaticTerm::Literal(dir::StaticTerm::Object { properties }))
    }

    /// Lower one directly representable static tuple term.
    ///
    /// Example:
    /// ```ds
    /// [1, 2, 3]
    /// ```
    fn lower_direct_static_tuple_term(
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
                    let Some(StaticTerm::Literal(value)) = self
                        .lower_direct_static_argument_operand(*value, tree)?
                        .known_static_term(self.check)
                    else {
                        return None;
                    };

                    values.push(value);
                }
                // not directly representable tuple element syntax
                _ => return None,
            }
        }

        Some(StaticTerm::Literal(dir::StaticTerm::Tuple {
            elements: values,
        }))
    }
}
