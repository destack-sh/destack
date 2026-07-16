use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Decision, DecoratorApplication, StaticIfCondition, VariableRole, WalkState, Widening,
};
use crate::r#static::{StaticContext, StaticError};

/// Source presence decided by closed static gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum StaticGate {
    /// The node is absent.
    Absent,
    /// The node is present.
    Present,
}

impl StaticGate {
    /// Return the gate for one presence decision.
    fn from_presence(is_present: bool) -> Self {
        if is_present {
            Self::Present
        } else {
            Self::Absent
        }
    }

    /// Return whether this gate keeps the source node.
    fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }
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
    pub(in crate::check) fn decorated_static_gate(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<StaticGate> {
        let decorated_global = decorated.into_global(self.module);
        if let Some(is_present) = self
            .check
            .module(self.module)
            .static_presence
            .get(&decorated_global)
            .copied()
        {
            return Ok(StaticGate::from_presence(is_present));
        }

        let applications = self.check.decorator_applications(self.module, decorated);

        // record checked decorator applications before any static gate exits
        for application in &applications {
            self.commit_decorator_application(decorated_global, application);
        }

        // decide static gates before walking ordinary decorators
        for application in &applications {
            if let Some(decorator) = self
                .check
                .static_if_decorator_from_application(self.module, application)
            {
                let StaticIfCondition::Present(condition_expression) = decorator.condition else {
                    self.check
                        .report_invalid_static_guard(self.module, decorator.condition_anchor());
                    self.commit_static_gate(decorated_global, StaticGate::Absent);

                    return Ok(StaticGate::Absent);
                };

                match self.evaluate_static_gate(condition_expression)? {
                    StaticGate::Absent => {
                        self.commit_static_gate(decorated_global, StaticGate::Absent);

                        return Ok(StaticGate::Absent);
                    }
                    StaticGate::Present => {}
                }
            }
        }

        // walk ordinary decorators only when the node is present
        for application in applications {
            if self
                .check
                .static_if_decorator_from_application(self.module, &application)
                .is_none()
            {
                self.walk_decorator(application.decorator)?;
            }
        }

        self.commit_static_gate(decorated_global, StaticGate::Present);

        Ok(StaticGate::Present)
    }

    /// Commit one checked decorator application.
    fn commit_decorator_application(
        &mut self,
        decorated: dir::GlobalNodeIdAny,
        application: &DecoratorApplication,
    ) {
        let arguments = application
            .arguments
            .iter()
            .map(|argument| dir::DecoratorArgument {
                source: (*argument).into_global_any(self.module),
                value: None,
            })
            .collect();
        let application = dir::DecoratorApplication {
            source: application.decorator.into_global_any(self.module),
            owner: decorated,
            target: application.target.into_global_any(self.module),
            resolution: self.check.decorator_target(self.module, application),
            arguments,
        };

        self.check
            .module_mut(self.module)
            .decorators
            .insert_application(application);
    }

    /// Commit one static gate decision.
    fn commit_static_gate(&mut self, decorated: dir::GlobalNodeIdAny, gate: StaticGate) {
        self.check
            .module_mut(self.module)
            .static_presence
            .insert(decorated, gate.is_present());
    }

    /// Decide whether one decorated node is present in checked source.
    ///
    /// Example:
    /// ```ds
    /// @if(import.meta.test)
    /// const value = 1;
    /// ```
    pub(in crate::check) fn decide_decorated_presence(
        &mut self,
        decorated: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        let gate = self.decorated_static_gate(decorated)?;
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
            let context = StaticContext::new(
                input.view(),
                input.module.as_ref(),
                &input.profile,
                &input.profile.conditions,
                self.check.strings(),
            );

            context.evaluate_boolean(condition)
        };

        match evaluated {
            Ok(true) => Ok(StaticGate::Present),
            Ok(false) => Ok(StaticGate::Absent),
            Err(StaticError::NotBoolean(expression)) => {
                self.check
                    .report_invalid_static_guard(module, expression.into_any());

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
            let context = StaticContext::new(
                input.view(),
                input.module.as_ref(),
                &input.profile,
                &input.profile.conditions,
                self.check.strings(),
            );

            context.evaluate_expression(expression).ok()
        };
        if let Some(term) = evaluated
            && let Some(literal) = self.static_term_literal(term)
        {
            let ty = self.intern_type(dir::Type::Literal(literal))?;

            return self.bind_static_term(expression, ty);
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
