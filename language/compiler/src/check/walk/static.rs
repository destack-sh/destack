use std::ptr::NonNull;

use crate::CompilerResult;
use crate::check::{
    Condition, ConditionPredicate, FlowState, NameLookup, Origin, ReceiverCapture,
    StaticIfCondition, StaticOperand, StaticTerm, VariableId, WalkState,
};
use crate::common::dir::r#static::{StaticContext, StaticFailure};
use destack_dir as dir;

/// One active static guard scope.
pub(in crate::check) struct StaticGuard {
    /// The guarded flow state.
    flow: NonNull<FlowState>,
}

impl StaticGuard {
    /// Return one active static guard scope.
    fn new(flow: &mut FlowState) -> Self {
        Self {
            flow: NonNull::from(flow),
        }
    }
}

impl Drop for StaticGuard {
    fn drop(&mut self) {
        // pop the guard owned by this scope
        unsafe {
            self.flow.as_mut().pop_static_guard();
        }
    }
}

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
        owner: dir::LocalNodeIdAny,
        receiver: Option<ReceiverCapture>,
    ) -> CompilerResult<Condition> {
        let invocations = self
            .check
            .decorator_invocations_for_owner(self.module, owner);
        let mut condition = Condition::Always;

        // combine visible static guards in source order
        for invocation in invocations {
            if let Some(decorator) = self
                .check
                .static_if_decorator_from_invocation(self.module, &invocation)
            {
                let StaticIfCondition::Present(condition_expression) = decorator.condition else {
                    self.check
                        .report_invalid_static_guard(self.module, decorator.condition_anchor());

                    return Ok(Condition::Never);
                };
                let next = self.evaluate_static_guard(receiver, condition_expression)?;

                condition = condition.and(next);
                if condition.is_never() {
                    return Ok(Condition::Never);
                }
            } else {
                let decorator_node = self
                    .check
                    .module(self.module)
                    .view()
                    .get(invocation.decorator)
                    .clone();

                self.walk_decorator(invocation.decorator, &decorator_node)?;
            }
        }

        Ok(condition)
    }

    /// Enter one already evaluated static guard.
    pub(in crate::check) fn enter_static_guard(&mut self, condition: Condition) -> StaticGuard {
        self.flow_mut().push_static_guard(condition);

        StaticGuard::new(self.flow_mut())
    }

    /// Enter the static guard attached to one owner.
    ///
    /// Example:
    /// ```ds
    /// @if(Enabled)
    /// const value = 1;
    /// ```
    pub(in crate::check) fn enter_static_guard_for(
        &mut self,
        owner: dir::LocalNodeIdAny,
        receiver: Option<ReceiverCapture>,
    ) -> CompilerResult<Option<StaticGuard>> {
        let owner_condition = self.evaluate_owner_static_guard(owner, receiver)?;
        let condition = self.active_static_guard().and(owner_condition.clone());

        // store symbol availability after combining guards
        if let Some(symbol) = self.check.module(self.module).declaration_symbol(owner) {
            self.check
                .module_mut(self.module)
                .availability
                .insert(symbol, condition.clone());
        }

        // just bail if statically never
        if condition.is_never() {
            Ok(None)
        }
        // actually push and keep going (conditionally)
        else {
            self.flow_mut().push_static_guard(owner_condition);

            Ok(Some(StaticGuard::new(self.flow_mut())))
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
        receiver: Option<ReceiverCapture>,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Condition> {
        let module = self.module;

        self.with_static_receiver(receiver, |this| {
            let context = StaticContext::new(
                this.check.module(module).view(),
                this.check.module(module).module.as_ref(),
                &this.check.module(module).profile,
                &this.check.module(module).profile.conditions,
                &this.check.module(module).strings,
            );

            match context.evaluate_boolean(condition) {
                Ok(true) => Ok(Condition::Always),
                Ok(false) => Ok(Condition::Never),
                Err(StaticFailure::NotBoolean(expression)) => {
                    this.check
                        .report_invalid_static_guard(module, expression.into_any());

                    Ok(Condition::Never)
                }
                Err(StaticFailure::NotStatic(expression)) => {
                    let condition_guard = this.active_static_guard();
                    let variable =
                        this.allocate_static_expression_variable(condition, condition_guard)?;

                    // walk static guard leaves without runtime condition constraints
                    this.walk_static_expression(expression)?;

                    Ok(Condition::When {
                        conditions: smallvec::smallvec![ConditionPredicate {
                            origin: Origin::Node(condition.into_global_any(module)),
                            operand: variable.into(),
                        }],
                    })
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
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        match self.tree.get(expression) {
            // (C)
            dir::Expression::Parenthesized { expression } => {
                self.walk_static_expression(*expression)?;
            }
            // this
            dir::Expression::This => {
                let source = expression.into_global_any(self.module);
                if let Some(term) = self.lower_this_receiver_type_term(source) {
                    self.bind_node_type(expression, term)?;
                }
            }
            // this.X
            dir::Expression::Member { left, .. } | dir::Expression::PrivateMember { left, .. } => {
                self.walk_static_expression(*left)?;
            }
            // C && D, C == D
            dir::Expression::Binary { left, right, .. } => {
                self.walk_static_expression(*left)?;
                self.walk_static_expression(*right)?;
            }
            // !C
            dir::Expression::Unary { right, .. } => {
                self.walk_static_expression(*right)?;
            }
            // <T extends U>
            dir::Expression::Type { value } => {
                self.walk_static_type_expression(*value)?;
            }
            // C[I]
            dir::Expression::Index { left, index, .. } => {
                self.walk_static_expression(*left)?;
                if let Some(index) = index {
                    self.walk_static_expression(*index)?;
                }
            }
            // literal or non static leaf
            _ => {}
        }

        Ok(())
    }

    /// Walk leaves needed by a type-space static expression.
    ///
    /// Example:
    /// ```ds
    /// T extends Borrowed
    /// ```
    pub(in crate::check) fn walk_static_type_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        match self.tree.get(expression) {
            // (T)
            dir::TypeExpression::Parenthesized { expression } => {
                self.walk_static_type_expression(*expression)?;
            }
            // T extends U
            dir::TypeExpression::Extends { left, right }
            | dir::TypeExpression::Implements { left, right } => {
                self.walk_type_expression(*left, self.tree.get(*left))?;
                self.walk_type_expression(*right, self.tree.get(*right))?;
            }
            // T extends U ? X : Y
            dir::TypeExpression::Conditional {
                left: _,
                extends_type: _,
                then_type: _,
                else_type: _,
            } => {
                self.walk_type_expression(expression, self.tree.get(expression))?;
            }
            // other type expressions are walked normally when directly needed
            _ => self.walk_type_expression(expression, self.tree.get(expression))?,
        }

        Ok(())
    }

    /// Walk one static expression and return its operand.
    ///
    /// Example:
    /// ```ds
    /// <Size + 1>
    /// ```
    pub(in crate::check) fn walk_static_expression_operand(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<StaticOperand> {
        // walk static leaves without keeping expression flow changes
        let before_expression = self.fork_flow();

        self.walk_static_expression(expression)?;
        self.restore_flow(before_expression);

        // allocate the argument local static variable
        let condition = self.active_static_guard();
        let variable =
            self.check
                .create_static_expression_variable(self.module, expression, condition);

        Ok(variable.into())
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
    ) -> CompilerResult<VariableId> {
        let module = self.module;
        let source = id.into_global_any(module);
        let origin = Origin::Node(source);
        let variable = self.check.create_static_variable(module, origin);

        if let Some(operand) = self.lower_direct_static_argument_operand(id)? {
            let condition = self.active_static_guard();

            self.check
                .equate_static(origin, variable, operand, condition);
        }

        Ok(variable)
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
    ) -> CompilerResult<Option<StaticOperand>> {
        let term = match self.tree.get(id) {
            // <(C)>
            dir::TypeExpression::Parenthesized { expression } => {
                return self.lower_direct_static_argument_operand(*expression);
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
                let Some(term) = self.lower_direct_static_object_term(members)? else {
                    return Ok(None);
                };

                term
            }
            // <[1, 2]>
            dir::TypeExpression::Tuple { elements } => {
                let Some(term) = self.lower_direct_static_tuple_term(elements)? else {
                    return Ok(None);
                };

                term
            }
            // <L | R>
            dir::TypeExpression::Union { elements } => StaticTerm::Union {
                elements: {
                    let mut operands = Vec::with_capacity(elements.len());

                    // collect union operands
                    for element in elements {
                        operands.push(self.create_static_argument_variable(*element)?.into());
                    }

                    operands
                },
            },
            // <readonly [1, 2]>
            dir::TypeExpression::ArrayTuple { elements } => {
                let Some(term) = self.lower_direct_static_tuple_term(elements)? else {
                    return Ok(None);
                };

                term
            }
            // <C>
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                return self.lower_static_reference_argument_operand(id, path, generic_arguments);
            }
            // <T.Value>
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let arguments = self.walk_generic_arguments(generic_arguments)?;

                StaticTerm::Member {
                    source: id.into_global_any(self.module),
                    owner: self.allocate_node_type_operand(*left)?,
                    key: dir::StaticKey::Name(*name),
                    arguments: arguments.into_vec(),
                }
            }
            // not directly representable static argument syntax
            _ => return Ok(None),
        };

        Ok(Some(self.check.inference.push_term(term).into()))
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
    ) -> CompilerResult<Option<StaticOperand>> {
        let [name] = path.segments.as_slice() else {
            return Ok(None);
        };

        // bind bare static parameter
        let term = if generic_arguments.is_empty()
            && let Some(operand) = self.static_parameter_argument_operand(id, *name)?
        {
            operand
        }
        // lower static memory intrinsic
        else if let Some(term) =
            self.lower_static_intrinsic_argument_term(id, *name, generic_arguments)?
        {
            self.check.inference.push_term(term).into()
        }
        // not a static reference argument
        else {
            return Ok(None);
        };

        Ok(Some(term))
    }

    /// Return one static parameter argument operand.
    ///
    /// Example:
    /// ```ds
    /// <Size>
    /// ```
    fn static_parameter_argument_operand(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        name: dir::StringId,
    ) -> CompilerResult<Option<StaticOperand>> {
        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_name_by_name(self.module, id.into_any(), name, dir::SymbolSpace::Value)
            .available_under(&guard);
        let symbol = match lookup {
            NameLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    return Ok(None);
                };

                symbol
            }
            NameLookup::Missing => return Ok(None),
            NameLookup::Ambiguous(_) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };

                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), &path);

                return Ok(None);
            }
        };

        if self.check.symbol_kind(symbol) != dir::SymbolKind::GenericValueParameter {
            return Ok(None);
        }

        let operand = self
            .check
            .import_symbol_static_operand(self.module, symbol)?;

        Ok(Some(operand))
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
    ) -> CompilerResult<Option<StaticTerm>> {
        let guard = self.active_static_guard();
        let lookup = self
            .check
            .lookup_name_by_name(self.module, id.into_any(), name, dir::SymbolSpace::Type)
            .available_under(&guard);
        let symbol = match lookup {
            NameLookup::Found(candidate) => {
                let Some(symbol) = candidate.symbol() else {
                    return Ok(None);
                };

                symbol
            }
            NameLookup::Missing => return Ok(None),
            NameLookup::Ambiguous(_) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };

                self.check
                    .report_ambiguous_reference(self.module, id.into_any(), &path);

                return Ok(None);
            }
        };
        let Some(item) = self.check.environment.language.item(symbol) else {
            return Ok(None);
        };

        if !Self::is_memory_static_intrinsic(item) {
            return Ok(None);
        }

        let arguments = self.walk_generic_arguments(generic_arguments)?;

        Ok(Some(StaticTerm::Intrinsic {
            item,
            arguments: arguments.into_vec(),
        }))
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
    ) -> CompilerResult<Option<StaticTerm>> {
        let mut properties = Vec::with_capacity(members.len());

        for member in members {
            match self.tree.get(*member) {
                // <{ key: value }>
                dir::TypeMember::Field {
                    key,
                    declared_type: Some(value),
                    is_static: false,
                    is_optional: false,
                    ..
                } => {
                    let Some(key) = key.static_key(self.tree) else {
                        return Ok(None);
                    };
                    let Some(operand) = self.lower_direct_static_argument_operand(*value)? else {
                        return Ok(None);
                    };
                    let Some(StaticTerm::Literal(value)) = operand.known_static_term(self.check)
                    else {
                        return Ok(None);
                    };

                    properties.push(dir::StaticProperty::Field { key, value });
                }
                // not directly representable object member syntax
                _ => return Ok(None),
            }
        }

        Ok(Some(StaticTerm::Literal(dir::StaticTerm::Object {
            properties,
        })))
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
    ) -> CompilerResult<Option<StaticTerm>> {
        let mut values = Vec::with_capacity(elements.len());

        for element in elements {
            match self.tree.get(*element) {
                // <[value]>
                dir::TupleElement::Element {
                    value,
                    is_optional: false,
                    ..
                } => {
                    let Some(operand) = self.lower_direct_static_argument_operand(*value)? else {
                        return Ok(None);
                    };
                    let Some(StaticTerm::Literal(value)) = operand.known_static_term(self.check)
                    else {
                        return Ok(None);
                    };

                    values.push(value);
                }
                // not directly representable tuple element syntax
                _ => return Ok(None),
            }
        }

        Ok(Some(StaticTerm::Literal(dir::StaticTerm::Tuple {
            elements: values,
        })))
    }
}
