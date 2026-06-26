use std::ptr::NonNull;

use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Condition, Decision, FlowState, StaticIfCondition, WalkState, Widening};
use crate::r#static::{StaticContext, StaticError};

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

/// One walk-time guard evaluation outcome.
pub(in crate::check) enum GuardOutcome {
    /// The guard decided statically false: the node is absent.
    Absent,
    /// The guard holds under the collected condition.
    Present(Condition),
}

impl WalkState<'_, '_> {
    /// Evaluate the static guard attached to one decorated node.
    ///
    /// Profile-level conditions decide eagerly; everything else becomes
    /// predicate types that the solver decides or assumes.
    ///
    /// Example:
    /// ```ds
    /// @if(Target.isShared)
    /// function f() {}
    /// ```
    pub(in crate::check) fn static_guard_condition(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<GuardOutcome> {
        let invocations = self.check.decorator_invocations(self.module, decorated);
        let mut condition = Condition::Always;

        // combine static guards in source order
        for invocation in invocations {
            if let Some(decorator) = self
                .check
                .static_if_decorator_from_invocation(self.module, &invocation)
            {
                let StaticIfCondition::Present(condition_expression) = decorator.condition else {
                    self.check
                        .report_invalid_static_guard(self.module, decorator.condition_anchor());

                    return Ok(GuardOutcome::Absent);
                };

                match self.evaluate_static_guard(condition_expression)? {
                    GuardOutcome::Absent => return Ok(GuardOutcome::Absent),
                    GuardOutcome::Present(next) => condition = condition.and(next),
                }
            } else {
                self.walk_decorator(invocation.decorator)?;
            }
        }

        Ok(GuardOutcome::Present(condition))
    }

    /// Enter one already evaluated static guard.
    pub(in crate::check) fn enter_static_guard(&mut self, condition: Condition) -> StaticGuard {
        self.flow_mut().push_static_guard(condition);

        StaticGuard::new(self.flow_mut())
    }

    /// Enter the static guard attached to one decorated node.
    /// Returns none when the guard decided statically false and the
    /// declaration must not be walked.
    ///
    /// Example:
    /// ```ds
    /// @if(Enabled)
    /// const value = 1;
    /// ```
    pub(in crate::check) fn enter_decorated_static_guard(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<StaticGuard>> {
        let outcome = self.static_guard_condition(decorated)?;
        let symbol = self.check.module(self.module).declaration_symbol(decorated);

        match outcome {
            // record absent declarations so name lookup drops them
            GuardOutcome::Absent => {
                if let Some(symbol) = symbol {
                    self.check
                        .module_mut(self.module)
                        .unavailable
                        .insert(symbol);
                }

                Ok(None)
            }
            GuardOutcome::Present(condition) => {
                // store the symbol's guard predicates after combining enclosing guards
                let combined = self.active_static_guard().and(condition.clone());
                if let Some(symbol) = symbol
                    && let Condition::When(predicates) = combined
                {
                    self.check.set_symbol_condition(symbol, predicates);
                }

                self.flow_mut().push_static_guard(condition);

                Ok(Some(StaticGuard::new(self.flow_mut())))
            }
        }
    }

    /// Return the active static guard.
    pub(in crate::check) fn active_static_guard(&self) -> Condition {
        self.flow().active_static_guard()
    }

    /// Return the active guard as one stored member condition.
    /// Nested guards conjoin into a single predicate written form.
    pub(in crate::check) fn member_condition(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let predicates = self.guard_predicates();

        match predicates.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(*single)),
            // conjoin nested guard predicates into one and-chain
            [first, rest @ ..] => {
                let mut joined = *first;
                for predicate in rest.iter().copied() {
                    let operation = dir::TypeOperation::StaticBinary(dir::StaticBinaryType {
                        operator: dir::StaticBinaryOperator::And,
                        left: joined,
                        right: predicate,
                    });
                    joined = self.push_type(dir::Type::Operation(operation), source)?;
                }

                Ok(Some(joined))
            }
        }
    }

    /// Evaluate one static guard expression.
    ///
    /// Example:
    /// ```ds
    /// Enabled && Target.isShared
    /// ```
    fn evaluate_static_guard(
        &mut self,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<GuardOutcome> {
        let module = self.module;

        // decide profile-level conditions eagerly
        let evaluated = {
            let input = self.check.module(module);
            let context = StaticContext::new(
                input.view(),
                input.module.as_ref(),
                &input.profile,
                &input.profile.conditions,
                &input.strings,
            );

            context.evaluate_boolean(condition)
        };

        match evaluated {
            Ok(true) => Ok(GuardOutcome::Present(Condition::Always)),
            Ok(false) => Ok(GuardOutcome::Absent),
            Err(StaticError::NotBoolean(expression)) => {
                self.check
                    .report_invalid_static_guard(module, expression.into_any());

                Ok(GuardOutcome::Absent)
            }
            // turn open conditions into predicate types for the solver
            Err(StaticError::NotStatic(_)) => {
                let predicate = self.walk_static_term(condition)?;

                Ok(GuardOutcome::Present(Condition::When(smallvec::smallvec![
                    predicate
                ])))
            }
        }
    }

    /// Walk one expression in static term position.
    ///
    /// Returns the type level term produced by the expression.
    ///
    /// Example:
    /// ```ds
    /// Mode == "inline"
    /// ```
    pub(in crate::check) fn walk_static_term(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = expression.into_global_any(self.module);
        if let Some(ty) = self.check.node_type_maybe(node) {
            return Ok(ty);
        }

        let source = expression.into_any();

        // embed eagerly evaluable subtrees as literals, covering profile
        // and module metadata like import.meta inside mixed guards
        let evaluated = {
            let input = self.check.module(self.module);
            let context = StaticContext::new(
                input.view(),
                input.module.as_ref(),
                &input.profile,
                &input.profile.conditions,
                &input.strings,
            );

            context.evaluate_expression(expression).ok()
        };
        if let Some(term) = evaluated
            && let Some(literal) = self.static_term_literal(term)
        {
            let ty = self.push_type(dir::Type::Literal(literal), source)?;

            return self.bind_static_term(expression, ty);
        }

        match self.tree.get(expression) {
            // (C)
            dir::Expression::Parenthesized { expression: nested } => {
                let ty = self.walk_static_term(*nested)?;

                self.bind_static_term(expression, ty)
            }
            // type
            dir::Expression::Type { value } => {
                let ty = if let dir::TypeExpression::Infer {
                    form: dir::InferForm::Hole,
                    ..
                } = self.tree.get(*value)
                {
                    self.infer_node_type(*value, Widening::Preserve)?
                } else {
                    self.walk_type_expression(*value)?
                };

                self.bind_static_term(expression, ty)
            }
            // 1
            dir::Expression::ScalarLiteral(value) => {
                let ty = self.push_type(dir::Type::Literal(*value), source)?;

                self.bind_static_term(expression, ty)
            }
            // this
            dir::Expression::This => {
                let ty = self.push_type(dir::Type::This, source)?;

                self.bind_static_term(expression, ty)
            }
            // resolve names to comptime parameters and static constants
            dir::Expression::Identifier { .. } => {
                let reference = self
                    .check
                    .module(self.module)
                    .resolved
                    .references
                    .get(expression.into_global_any(self.module));
                let symbol = match reference {
                    Some(dir::Reference::Bound(symbols)) => {
                        let symbols = self.check.available_symbols(symbols);
                        match symbols.as_slice() {
                            [symbol] => Some(*symbol),
                            _ => None,
                        }
                    }
                    Some(dir::Reference::Missing)
                    | Some(dir::Reference::Namespace(_))
                    | Some(dir::Reference::Projected { .. })
                    | Some(dir::Reference::Ambiguous(_))
                    | None => None,
                };
                let Some(symbol) = symbol else {
                    self.check.report_invalid_static_guard(self.module, source);

                    return self.push_type(
                        dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                        source,
                    );
                };

                // record the name edge for checked output
                let global_source = expression.into_global_any(self.module);
                self.capture_symbol_reference(symbol);
                self.check.record_decision(
                    global_source,
                    Decision::Name(dir::NameResolution::new(symbol)),
                )?;

                // comptime parameters write their parameter type so
                // instantiation substitution reaches the predicate
                if let Some(parameter) = self.check.generics.parameter_by_symbol(symbol) {
                    let ty = self.push_type(dir::Type::Parameter(parameter), source)?;

                    return self.bind_static_term(expression, ty);
                }

                if let Some(value) = self.check.static_value(symbol) {
                    return self.bind_static_term(expression, value);
                }

                let reference = dir::Type::Instance(dir::GenericInstance {
                    symbol,
                    arguments: Vec::new(),
                });
                let ty = self.push_type(reference, source)?;

                self.bind_static_term(expression, ty)
            }
            // C == D, N * 2
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let Ok(operator) = dir::StaticBinaryOperator::try_from(*operator) else {
                    self.check.report_invalid_static_guard(self.module, source);
                    let ty = self.push_type(
                        dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                        source,
                    )?;

                    return self.bind_static_term(expression, ty);
                };
                let left = self.walk_static_term(*left)?;
                let right = self.walk_static_term(*right)?;
                let operation = dir::TypeOperation::StaticBinary(dir::StaticBinaryType {
                    operator,
                    left,
                    right,
                });
                let ty = self.push_type(dir::Type::Operation(operation), source)?;

                self.bind_static_term(expression, ty)
            }
            // !C
            dir::Expression::Unary { operator, right } => {
                let Ok(operator) = dir::StaticUnaryOperator::try_from(*operator) else {
                    self.check.report_invalid_static_guard(self.module, source);
                    let ty = self.push_type(
                        dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                        source,
                    )?;

                    return self.bind_static_term(expression, ty);
                };
                let target = self.walk_static_term(*right)?;
                let operation =
                    dir::TypeOperation::StaticUnary(dir::StaticUnaryType { operator, target });
                let ty = self.push_type(dir::Type::Operation(operation), source)?;

                self.bind_static_term(expression, ty)
            }
            // member chains project static members off their owners
            dir::Expression::Member {
                left,
                name: Some(name),
            } => {
                let owner = self.walk_static_term(*left)?;
                let member = dir::Type::Member(dir::MemberType {
                    owner,
                    key: dir::StaticKey::Name(*name),
                    arguments: Vec::new(),
                });
                let ty = self.push_type(member, source)?;

                self.bind_static_term(expression, ty)
            }
            _ => {
                self.check.report_invalid_static_guard(self.module, source);
                let ty = self.push_type(
                    dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                    source,
                )?;

                self.bind_static_term(expression, ty)
            }
        }
    }

    /// Bind one static expression node to its static term type.
    fn bind_static_term(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.write_node_type(expression, ty)
    }

    /// Return one eagerly evaluated static term as a scalar literal.
    fn static_term_literal(&self, term: dir::StaticTerm) -> Option<dir::ScalarLiteral> {
        match term {
            dir::StaticTerm::ScalarLiteral { value } => Some(value),
            _ => None,
        }
    }
}
