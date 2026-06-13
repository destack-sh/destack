use std::ptr::NonNull;

use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Condition, FlowState, NameLookup, NameTarget, StaticIfCondition, WalkState};
use crate::common::dir::r#static::{StaticContext, StaticFailure, StaticValue};

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
    /// Profile-level conditions decide eagerly; everything else lowers
    /// to predicate types that the solver decides or assumes.
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
                // store availability after combining enclosing guards
                let combined = self.active_static_guard().and(condition.clone());
                if let Some(symbol) = symbol
                    && combined != Condition::Always
                {
                    self.check
                        .module_mut(self.module)
                        .availability
                        .insert(symbol, combined);
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
            Err(StaticFailure::NotBoolean(expression)) => {
                self.check
                    .report_invalid_static_guard(module, expression.into_any());

                Ok(GuardOutcome::Absent)
            }
            // open conditions lower to predicate types for the solver
            Err(StaticFailure::NotStatic(_)) => {
                let predicate = self.lower_static_predicate(condition)?;

                Ok(GuardOutcome::Present(Condition::When(smallvec::smallvec![
                    predicate
                ])))
            }
        }
    }

    /// Lower one static guard expression to a predicate type.
    ///
    /// Example:
    /// ```ds
    /// Mode == "inline"
    /// ```
    pub(in crate::check) fn lower_static_predicate(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
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
        if let Some(value) = evaluated
            && let Some(literal) = self.static_value_literal(value)
        {
            return self.push_type(dir::Type::Literal(literal), source);
        }

        match self.tree.get(expression) {
            // (C)
            dir::Expression::Parenthesized { expression } => {
                let expression = *expression;

                self.lower_static_predicate(expression)
            }
            // type
            dir::Expression::Type { value } => {
                let value = *value;
                if let dir::TypeExpression::Infer {
                    form: dir::InferForm::Hole,
                    ..
                } = self.tree.get(value)
                {
                    return self.node_type_any(value.into_global_any(self.module));
                }

                self.walk_type_expression(value)
            }
            // 1
            dir::Expression::ScalarLiteral(value) => {
                let value = *value;

                self.push_type(dir::Type::Literal(value), source)
            }
            // this
            dir::Expression::This => self.push_type(dir::Type::This, source),
            // names reference comptime parameters and static constants
            dir::Expression::Identifier { name } => {
                let name = *name;
                let lookup =
                    self.check
                        .lookup_name(self.module, source, name, dir::SymbolSpace::Value);
                let symbol = match lookup {
                    NameLookup::Found(candidate) => match candidate.target {
                        NameTarget::Symbol(symbol) => Some(symbol),
                        NameTarget::Namespace(_) => None,
                    },
                    NameLookup::Missing | NameLookup::Ambiguous(_) => None,
                };
                let Some(symbol) = symbol else {
                    self.check.report_invalid_static_guard(self.module, source);

                    return self.push_type(
                        dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                        source,
                    );
                };

                // comptime parameters write their parameter type so
                // instantiation substitution reaches the predicate
                if let Some(parameter) = self.check.generics.parameter_by_symbol(symbol) {
                    return self.push_type(dir::Type::Parameter(parameter), source);
                }

                let reference = dir::Type::Reference(dir::GenericInstance {
                    symbol,
                    arguments: Vec::new(),
                });

                self.push_type(reference, source)
            }
            // C == D, N * 2
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let (operator, left, right) = (*operator, *left, *right);
                let Ok(operator) = dir::StaticBinaryOperator::try_from(operator) else {
                    self.check.report_invalid_static_guard(self.module, source);

                    return self.push_type(
                        dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                        source,
                    );
                };
                let left = self.lower_static_predicate(left)?;
                let right = self.lower_static_predicate(right)?;
                let operation = dir::TypeOperation::StaticBinary(dir::StaticBinaryType {
                    operator,
                    left,
                    right,
                });

                self.push_type(dir::Type::Operation(operation), source)
            }
            // !C
            dir::Expression::Unary { operator, right } => {
                let (operator, target) = (*operator, *right);
                let Ok(operator) = dir::StaticUnaryOperator::try_from(operator) else {
                    self.check.report_invalid_static_guard(self.module, source);

                    return self.push_type(
                        dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                        source,
                    );
                };
                let target = self.lower_static_predicate(target)?;
                let operation =
                    dir::TypeOperation::StaticUnary(dir::StaticUnaryType { operator, target });

                self.push_type(dir::Type::Operation(operation), source)
            }
            // member chains project static members off their owners
            dir::Expression::Member {
                left,
                name: Some(name),
            } => {
                let (left, name) = (*left, *name);
                let owner = self.lower_static_predicate(left)?;
                let member = dir::Type::Member(dir::MemberType {
                    owner,
                    key: dir::StaticKey::Name(name),
                    arguments: Vec::new(),
                });

                self.push_type(member, source)
            }
            _ => {
                self.check.report_invalid_static_guard(self.module, source);

                self.push_type(
                    dir::Type::Literal(dir::ScalarLiteral::Boolean(false)),
                    source,
                )
            }
        }
    }

    /// Return one eagerly evaluated static value as a scalar literal.
    fn static_value_literal(&mut self, value: StaticValue) -> Option<dir::ScalarLiteral> {
        match value {
            StaticValue::Boolean(value) => Some(dir::ScalarLiteral::Boolean(value)),
            StaticValue::String(value) => {
                let id = self.check.module_mut(self.module).strings.intern(&value);

                Some(dir::ScalarLiteral::String(id))
            }
            StaticValue::Scalar(value) => Some(value),
            StaticValue::Undefined => Some(dir::ScalarLiteral::Undefined),
            StaticValue::Object | StaticValue::StringList(_) => None,
        }
    }
}
