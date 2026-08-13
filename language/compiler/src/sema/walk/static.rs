use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, DecoratorExpression, VariableRole, WalkState, Widening};
use crate::r#static::{StaticError, StaticEvaluator, StaticGuard};

pub(in crate::sema) use dir::StaticPresence;

impl CheckState<'_> {
    /// Return the static key named by one key.
    pub(in crate::sema) fn static_key(
        &mut self,
        key: dir::Key,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        match key {
            dir::Key::Name(name) => Ok(Some(name.static_key())),
            dir::Key::Expression(expression) => {
                self.evaluate_static_key(self.module_id, expression)
            }
        }
    }

    /// Decide the static gates attached to one decorated node.
    ///
    /// Example:
    /// ```ds
    /// @if(import.meta.platform == "windows")
    /// function f() {}
    /// ```
    fn decide_static_gate(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<StaticPresence> {
        // reuse an already decided gate
        if let Some(gate) = self
            .module(self.module_id)
            .static_presence
            .get(&decorated)
            .copied()
        {
            return Ok(gate);
        }

        // trust earlier-stage gate decisions from the table, never re-judge them
        if let Some(gate) = self.module(self.module_id).statics.presence(decorated) {
            return Ok(gate);
        }

        let decorators = self.decorator_expressions(self.module_id, decorated);

        // decide every static gate
        for decorator in decorators {
            let view = self.module_view(self.module_id);
            let guard = StaticGuard::classify(view, self.strings(), decorator.decorator);
            match guard {
                StaticGuard::Ordinary => {}
                StaticGuard::Rejected(error) => {
                    self.report_invalid_static_if_invocation(self.module_id, error.node())?;
                    self.commit_static_gate(decorated, StaticPresence::Absent);

                    return Ok(StaticPresence::Absent);
                }
                StaticGuard::Condition(condition) => match self.evaluate_static_gate(condition)? {
                    StaticPresence::Absent => {
                        self.commit_static_gate(decorated, StaticPresence::Absent);

                        return Ok(StaticPresence::Absent);
                    }
                    StaticPresence::Present => {}
                },
            }
        }

        self.commit_static_gate(decorated, StaticPresence::Present);

        Ok(StaticPresence::Present)
    }

    /// Commit one static gate decision.
    fn commit_static_gate(&mut self, decorated: dir::LocalNodeIdAny, gate: StaticPresence) {
        let module = self.module_mut(self.module_id);
        module.statics_tail.record_presence(decorated, gate);

        // retain the decision for repeated decorator walks
        module.static_presence.insert(decorated, gate);
    }

    /// Decide whether one node is present under its static decorators.
    ///
    /// Example:
    /// ```ds
    /// @if(import.meta.test)
    /// const value = 1;
    /// ```
    pub(in crate::sema) fn decide_static_presence(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        let gate = self.decide_static_gate(decorated)?;
        let symbol = self.module(self.module_id).declaration_symbol(decorated);

        match gate {
            // record absent declarations so name lookup drops them
            StaticPresence::Absent => {
                if let Some(symbol) = symbol {
                    self.module_mut(self.module_id)
                        .absent_symbols
                        .insert(symbol);
                }

                Ok(false)
            }
            StaticPresence::Present => Ok(true),
        }
    }

    /// Decide one node's presence and select its applied decorators.
    fn applied_decorators(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<Vec<DecoratorExpression>>> {
        if !self.decide_static_presence(decorated)? {
            return Ok(None);
        }

        // keep applied decorators, presence is already decided
        let decorators = self.decorator_expressions(self.module_id, decorated);
        let mut ordinary = Vec::with_capacity(decorators.len());
        for decorator in decorators {
            let view = self.module_view(self.module_id);
            let guard = StaticGuard::classify(view, self.strings(), decorator.decorator);
            if matches!(guard, StaticGuard::Ordinary) {
                ordinary.push(decorator);
            }
        }

        Ok(Some(ordinary))
    }

    /// Evaluate one static gate expression.
    ///
    /// Example:
    /// ```ds
    /// Enabled && Target.isShared
    /// ```
    fn evaluate_static_gate(
        &mut self,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<StaticPresence> {
        let module = self.module_id;

        // decide profile-level conditions eagerly
        let evaluated = {
            let input = self.module(module);
            let evaluator = StaticEvaluator::new(
                input.view(),
                input.module.as_ref(),
                input.package.as_ref(),
                self.environment.as_ref(),
                &input.profile,
                self.strings(),
            );

            evaluator.evaluate_boolean(condition)
        };

        match evaluated {
            Ok(true) => Ok(StaticPresence::Present),
            Ok(false) => Ok(StaticPresence::Absent),
            Err(StaticError::NotBoolean(expression)) => {
                self.report_non_boolean_static_guard(module, expression.into_any());

                Ok(StaticPresence::Absent)
            }
            Err(StaticError::NotStatic(_)) => {
                self.report_undecidable_static_guard(module, condition.into_any());

                Ok(StaticPresence::Absent)
            }
        }
    }

    /// Return one eagerly evaluated static term as a scalar literal.
    fn static_term_literal(&self, term: dir::StaticTerm) -> Option<dir::ScalarLiteral> {
        match term {
            dir::StaticTerm::ScalarLiteral { value } => Some(value),
            _ => None,
        }
    }
}

impl WalkState<'_, '_> {
    /// Decide one node's presence and walk its ordinary decorators.
    pub(in crate::sema) fn walk_decorators(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        let Some(decorators) = self.applied_decorators(decorated)? else {
            return Ok(false);
        };

        // walk applied decorators in authored order
        let owner = decorated.into_global(self.module);
        for decorator in decorators {
            self.walk_decorator(decorator, owner)?;
        }

        Ok(true)
    }

    /// Declare decorator applications without walking their values.
    pub(in crate::sema) fn declare_decorators(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        let Some(decorators) = self.applied_decorators(decorated)? else {
            return Ok(false);
        };

        // declare applied decorators in authored order
        let owner = decorated.into_global(self.module);
        for decorator in decorators {
            self.declare_decorator(decorator, owner)?;
        }

        Ok(true)
    }

    /// Walk one expression in static term position.
    ///
    /// Returns the type level term produced by the expression.
    ///
    /// Example:
    /// ```ds
    /// Mode == "inline"
    /// ```
    pub(in crate::sema) fn walk_static_term(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = expression.into_any();

        // embed eagerly evaluable subtrees as literals, covering profile
        //  and module metadata like import.meta inside mixed guards
        let evaluated = {
            let input = self.check.module(self.module);
            let evaluator = StaticEvaluator::new(
                input.view(),
                input.module.as_ref(),
                input.package.as_ref(),
                self.check.environment.as_ref(),
                &input.profile,
                self.check.strings(),
            );

            evaluator.evaluate_expression(expression)
        };
        match evaluated {
            Ok(term) => {
                if let Some(literal) = self.static_term_literal(term) {
                    let ty = self.intern_type(dir::Type::Literal(literal))?;

                    return self.bind_static_term(expression, ty);
                }
            }
            Err(StaticError::NotStatic(_) | StaticError::NotBoolean(_)) => {}
        }

        match self.tree.get(expression) {
            // contextual static hole
            dir::Expression::Infer {
                form: dir::InferForm::Hole,
                name: None,
            } => {
                let ty = match self.reject_declaration_hole(source)? {
                    Some(rejected) => rejected,
                    None => self.open_type_hole(source, Widening::Never, VariableRole::Regular)?,
                };
                self.commit_node_type(expression, ty)?;

                self.bind_static_term(expression, ty)
            }
            // type
            dir::Expression::Type { value } => {
                let ty = if let dir::TypeExpression::Infer {
                    form: dir::InferForm::Hole,
                    ..
                } = self.tree.get(*value)
                {
                    let source = (*value).into_any();
                    let ty = match self.reject_declaration_hole(source)? {
                        Some(rejected) => rejected,
                        None => {
                            self.open_type_hole(source, Widening::Never, VariableRole::Regular)?
                        }
                    };
                    self.commit_node_type(*value, ty)?
                } else {
                    self.walk_type_expression(*value)?
                };

                self.bind_static_term(expression, ty)
            }
            // 1
            dir::Expression::ScalarLiteral(value) => {
                let ty = self.intern_type(dir::Type::Literal(*value))?;

                self.bind_static_term(expression, ty)
            }
            // this
            dir::Expression::This => {
                let ty = self.intern_type(dir::Type::This)?;

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
                        let symbols = self.check.present_symbols(symbols);
                        match symbols.as_slice() {
                            [symbol] => Some(*symbol),
                            _ => None,
                        }
                    }
                    Some(dir::Reference::Missing)
                    | Some(dir::Reference::Namespace { .. })
                    | Some(dir::Reference::Projected { .. })
                    | Some(dir::Reference::Ambiguous(_))
                    | None => None,
                };
                let Some(symbol) = symbol else {
                    self.check
                        .report_undecidable_static_value(self.module, source);
                    let ty = self.intern_type(dir::Type::Error)?;

                    return self.bind_static_term(expression, ty);
                };

                // record the name edge for checked output
                let global_source = expression.into_global_any(self.module);
                self.capture_symbol_reference(symbol);
                self.check
                    .commit_name(global_source, dir::NameResolution::new(symbol))?;

                // comptime parameters write their parameter type so
                //  instantiation substitution reaches the predicate
                if let Some(parameter) = self.check.parameter_by_symbol(symbol) {
                    let ty = self.intern_type(dir::Type::Parameter(parameter))?;

                    return self.bind_static_term(expression, ty);
                }

                if let Some(value) = self.check.static_value(symbol) {
                    return self.bind_static_term(expression, value);
                }

                let reference = dir::Type::Application(dir::GenericApplication {
                    symbol,
                    arguments: dir::TypeListId::EMPTY,
                });
                let ty = self.intern_type(reference)?;

                self.bind_static_term(expression, ty)
            }
            // C == D, N * 2
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let Ok(operator) = dir::StaticBinaryOperator::try_from(*operator) else {
                    self.check
                        .report_undecidable_static_value(self.module, source);
                    let ty = self.intern_type(dir::Type::Error)?;

                    return self.bind_static_term(expression, ty);
                };
                let left = self.walk_static_term(*left)?;
                let right = self.walk_static_term(*right)?;
                let operation = dir::TypeOperation::StaticBinary(dir::StaticBinaryType {
                    operator,
                    left,
                    right,
                });
                let ty = self.intern_operation(operation)?;

                self.bind_static_term(expression, ty)
            }
            // !C
            dir::Expression::Unary { operator, right } => {
                let Ok(operator) = dir::StaticUnaryOperator::try_from(*operator) else {
                    self.check
                        .report_undecidable_static_value(self.module, source);
                    let ty = self.intern_type(dir::Type::Error)?;

                    return self.bind_static_term(expression, ty);
                };
                let target = self.walk_static_term(*right)?;
                let operation =
                    dir::TypeOperation::StaticUnary(dir::StaticUnaryType { operator, target });
                let ty = self.intern_operation(operation)?;

                self.bind_static_term(expression, ty)
            }
            // { role: "button", live: true }
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.clone();

                // collect one static shape field per literal keyed property
                let mut fields = Vec::new();
                for property in properties {
                    match self.tree.get(property).clone() {
                        dir::Property::Field { key, value, .. } => {
                            let ty = self.walk_static_term(value)?;
                            let Some(key) = key.direct_static_key() else {
                                self.check
                                    .report_undecidable_static_value(self.module, source);
                                continue;
                            };
                            fields.push(dir::TypeProperty {
                                key,
                                access: dir::PropertyAccess::Read(ty),
                                is_optional: false,
                            });
                        }
                        _ => {
                            self.check
                                .report_undecidable_static_value(self.module, source);
                        }
                    }
                }

                // intern the collected fields as an object shape
                let properties = self.check.intern_properties(&fields)?;
                let ty = self.intern_type(dir::Type::Object(dir::ShapeType {
                    properties,
                    call_signatures: dir::TypeListId::EMPTY,
                    construct_signatures: dir::TypeListId::EMPTY,
                    index_signatures: dir::TypeListId::EMPTY,
                }))?;

                self.bind_static_term(expression, ty)
            }
            // member chains project static members off their owners
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                let owner = self.walk_static_term(*left)?;
                let ty = self.intern_member(dir::MemberType {
                    owner,
                    key: dir::StaticKey::Name(*name),
                    arguments: dir::TypeListId::EMPTY,
                    qualifier: None,
                })?;

                self.bind_static_term(expression, ty)
            }
            _ => {
                self.check
                    .report_undecidable_static_value(self.module, source);
                let ty = self.intern_type(dir::Type::Error)?;

                self.bind_static_term(expression, ty)
            }
        }
    }

    /// Bind one static expression node to its static term type.
    /// Bind one static expression node to its static term type.
    fn bind_static_term(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.commit_node_type(expression, ty)
    }
}
