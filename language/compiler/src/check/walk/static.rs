use destack_dir as dir;

use crate::check::{CheckState, Decision, VariableRole, WalkState, Widening};
use crate::r#static::{StaticError, StaticEvaluator, StaticGuard};
use crate::{CompilerError, CompilerResult};

/// Source presence decided by closed static gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum StaticGate {
    /// The node is absent.
    Absent,
    /// The node is present.
    Present,
}

impl WalkState<'_, '_> {
    /// Return the static key named by one key.
    pub(in crate::check) fn static_key(
        &self,
        key: dir::Key,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        match key {
            dir::Key::Name(name) => Ok(Some(name.static_key())),
            dir::Key::Expression(expression) => self
                .check
                .static_key_from_expression(self.module, expression),
        }
    }

    /// Decide the static gates attached to one decorated node.
    ///
    /// Example:
    /// ```ds
    /// @if(import.meta.platform == "windows")
    /// function f() {}
    /// ```
    fn decide_static_gate(&mut self, decorated: dir::LocalNodeIdAny) -> CompilerResult<StaticGate> {
        let decorated_global = decorated.into_global(self.module);
        if let Some(gate) = self
            .check
            .module(self.module)
            .static_presence
            .get(&decorated_global)
            .copied()
        {
            return Ok(gate);
        }

        let decorators = self.check.decorator_expressions(self.module, decorated);

        // decide every static gate
        for decorator in decorators {
            let view = self.check.module_view(self.module);
            let guard = StaticGuard::classify(view, self.check.strings(), decorator.decorator);
            match guard {
                StaticGuard::Ordinary => {}
                StaticGuard::Rejected(error) => {
                    self.check
                        .report_invalid_static_if_invocation(self.module, error.node())?;
                    self.commit_static_gate(decorated_global, StaticGate::Absent);

                    return Ok(StaticGate::Absent);
                }
                StaticGuard::Condition(condition) => match self.evaluate_static_gate(condition)? {
                    StaticGate::Absent => {
                        self.commit_static_gate(decorated_global, StaticGate::Absent);

                        return Ok(StaticGate::Absent);
                    }
                    StaticGate::Present => {}
                },
            }
        }

        self.commit_static_gate(decorated_global, StaticGate::Present);

        Ok(StaticGate::Present)
    }

    /// Commit one static gate decision.
    fn commit_static_gate(&mut self, decorated: dir::GlobalNodeIdAny, gate: StaticGate) {
        self.check
            .module_mut(self.module)
            .static_presence
            .insert(decorated, gate);
    }

    /// Decide whether one node is present under its static decorators.
    ///
    /// Example:
    /// ```ds
    /// @if(import.meta.test)
    /// const value = 1;
    /// ```
    pub(in crate::check) fn decide_static_presence(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        let gate = self.decide_static_gate(decorated)?;
        let symbol = self.check.module(self.module).declaration_symbol(decorated);

        match gate {
            // record absent declarations so name lookup drops them
            StaticGate::Absent => {
                if let Some(symbol) = symbol {
                    self.check
                        .module_mut(self.module)
                        .absent_symbols
                        .insert(symbol);
                }

                Ok(false)
            }
            StaticGate::Present => Ok(true),
        }
    }

    /// Decide one node's presence and walk its ordinary decorators.
    pub(in crate::check) fn walk_decorators(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        if !self.decide_static_presence(decorated)? {
            return Ok(false);
        }
        let decorated_global = decorated.into_global(self.module);
        let decorators = self.check.decorator_expressions(self.module, decorated);

        // walk ordinary decorators in authored order
        for decorator in decorators {
            let view = self.check.module_view(self.module);
            let guard = StaticGuard::classify(view, self.check.strings(), decorator.decorator);
            if matches!(guard, StaticGuard::Ordinary) {
                self.walk_decorator(decorator, decorated_global)?;
            }
        }

        Ok(true)
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
    ) -> CompilerResult<StaticGate> {
        let module = self.module;

        // decide profile-level conditions eagerly
        let evaluated = {
            let input = self.check.module(module);
            let evaluator = StaticEvaluator::new(
                input.view(),
                input.module.as_ref(),
                input.package.as_ref(),
                self.check.environment.as_ref(),
                &input.profile,
                self.check.strings(),
            );

            evaluator.evaluate_boolean(condition)
        };

        match evaluated {
            Ok(true) => Ok(StaticGate::Present),
            Ok(false) => Ok(StaticGate::Absent),
            Err(StaticError::NotBoolean(expression)) => {
                self.check
                    .report_non_boolean_static_guard(module, expression.into_any());

                Ok(StaticGate::Absent)
            }
            Err(StaticError::NotStatic(_)) => {
                self.check
                    .report_undecidable_static_guard(module, condition.into_any());

                Ok(StaticGate::Absent)
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
                let ty = self.open_type_hole(source, Widening::Never, VariableRole::Regular)?;
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
                    let ty = self.open_type_hole(
                        (*value).into_any(),
                        Widening::Never,
                        VariableRole::Regular,
                    )?;
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
                    | Some(dir::Reference::Namespace(_))
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
                self.check.commit_decision(
                    global_source,
                    Decision::Name(dir::NameResolution::new(symbol)),
                )?;

                // comptime parameters write their parameter type so
                //  instantiation substitution reaches the predicate
                if let Some(parameter) = self.check.generics.parameter_by_symbol(symbol) {
                    let ty = self.intern_type(dir::Type::Parameter(parameter))?;

                    return self.bind_static_term(expression, ty);
                }

                if let Some(value) = self.check.static_value(symbol) {
                    return self.bind_static_term(expression, value);
                }

                let reference = dir::Type::Instance(dir::GenericInstance {
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
    fn bind_static_term(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.commit_node_type(expression, ty)
    }

    /// Return one eagerly evaluated static term as a scalar literal.
    fn static_term_literal(&self, term: dir::StaticTerm) -> Option<dir::ScalarLiteral> {
        match term {
            dir::StaticTerm::ScalarLiteral { value } => Some(value),
            _ => None,
        }
    }
}

impl CheckState<'_> {
    /// Return the static gate committed for one decorated node.
    pub(in crate::check) fn static_gate(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<StaticGate> {
        self.module(node.module_id)
            .static_presence
            .get(&node)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check node {} has no static gate", self.node_label(node)),
            })
    }
}
