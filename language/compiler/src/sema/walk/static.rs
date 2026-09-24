use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, DecoratorExpression, Origin, VariableKind, WalkState};
use crate::r#static::{StaticError, StaticEvaluator, StaticGuard};

pub(in crate::sema) use dir::StaticPresence;

impl CheckState<'_> {
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

        // trust the earlier-stage gate decisions in the table
        if let Some(gate) = self.module(self.module_id).statics.presence(decorated) {
            return Ok(gate);
        }

        // read the decorators written on the declaration
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
        module.statics_tail.commit_presence(decorated, gate);

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

        // record the presence the gate decided
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

        // read the presence the condition evaluates to
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
    fn static_term_literal(&self, term: dir::StaticTerm) -> Option<dir::Literal> {
        match term {
            dir::StaticTerm::Literal { value } => Some(value),
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

    /// Reject one expression as a static value, its node typed as the error type.
    fn reject_static_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check
            .report_undecidable_static_value(self.module, source);
        let ty = self.intern_type(dir::Type::Error)?;

        self.commit_node_type(expression, ty)
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

        // embed an eagerly evaluable subtree as a literal
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

                    return self.commit_node_type(expression, ty);
                }
            }
            Err(StaticError::NotStatic(_) | StaticError::NotBoolean(_)) => {}
        }

        // walk the expression the term left unevaluated
        match self.tree.get(expression) {
            // contextual static hole
            dir::Expression::Infer {
                form: dir::InferForm::Hole,
                name: None,
            } => {
                let ty = match self.report_declaration_hole(source)? {
                    Some(rejected) => rejected,
                    None => self.open_type_hole(source, VariableKind::Type)?,
                };
                self.commit_node_type(expression, ty)
            }
            // type
            dir::Expression::Type { value } => {
                let ty = if let dir::TypeExpression::Infer {
                    form: dir::InferForm::Hole,
                    ..
                } = self.tree.get(*value)
                {
                    let source = (*value).into_any();
                    let ty = match self.report_declaration_hole(source)? {
                        Some(rejected) => rejected,
                        None => self.open_type_hole(source, VariableKind::Type)?,
                    };
                    self.commit_node_type(*value, ty)?
                } else {
                    self.walk_type_expression(*value)?
                };

                self.commit_node_type(expression, ty)
            }
            // 1
            dir::Expression::Literal(value) => {
                let ty = self.intern_type(dir::Type::Literal(*value))?;

                self.commit_node_type(expression, ty)
            }
            // this
            dir::Expression::This => {
                let ty = self.intern_type(dir::Type::This)?;

                self.commit_node_type(expression, ty)
            }
            // resolve names to const parameters and static constants
            dir::Expression::Identifier { .. } => {
                let reference = self
                    .check
                    .module(self.module)
                    .resolved
                    .references
                    .get(expression.into_global_any(self.module));

                // read a type literal name as its denoted type
                if let Some(dir::Reference::TypeLiteral(literal)) = reference {
                    let literal = literal.clone();
                    let ty = self.intern_type(dir::Type::from(literal))?;

                    return self.commit_node_type(expression, ty);
                }
                let symbol = match reference {
                    Some(dir::Reference::Bound(symbols)) => {
                        let symbols = self.check.present_symbols(symbols);
                        match symbols.as_slice() {
                            [symbol] => Some(*symbol),
                            _ => None,
                        }
                    }
                    Some(dir::Reference::TypeLiteral(_))
                    | Some(dir::Reference::Missing)
                    | Some(dir::Reference::Namespace { .. })
                    | Some(dir::Reference::Projected { .. })
                    | Some(dir::Reference::Ambiguous(_))
                    | None => None,
                };
                let Some(symbol) = symbol else {
                    return self.reject_static_value(expression, source);
                };

                // record the name edge
                let global_source = expression.into_global_any(self.module);
                self.capture_symbol_reference(global_source, symbol)?;
                self.check
                    .commit_name(global_source, dir::NameResolution::new(symbol))?;

                // const parameters write their parameter type
                if let Some(parameter) = self.check.parameter_by_symbol(symbol)? {
                    let ty = self.intern_type(dir::Type::Parameter(parameter))?;

                    return self.commit_node_type(expression, ty);
                }

                if let Some(value) = self.check.static_value(symbol)? {
                    return self.commit_node_type(expression, value);
                }

                let reference = dir::Type::Reference(dir::TypeReference::new(symbol));
                let ty = self.intern_type(reference)?;

                self.commit_node_type(expression, ty)
            }
            // C == D, N * 2
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let Ok(operator) = dir::StaticBinaryOperator::try_from(*operator) else {
                    return self.reject_static_value(expression, source);
                };
                let left = self.walk_static_term(*left)?;
                let right = self.walk_static_term(*right)?;
                let operation = dir::TypeOperation::StaticBinary(dir::StaticBinaryType {
                    operator,
                    left,
                    right,
                });
                let ty = self.intern_operation(operation)?;

                self.commit_node_type(expression, ty)
            }
            // !C
            dir::Expression::Unary { operator, right } => {
                let Ok(operator) = dir::StaticUnaryOperator::try_from(*operator) else {
                    return self.reject_static_value(expression, source);
                };
                let target = self.walk_static_term(*right)?;
                let operation =
                    dir::TypeOperation::StaticUnary(dir::StaticUnaryType { operator, target });
                let ty = self.intern_operation(operation)?;

                self.commit_node_type(expression, ty)
            }
            // { role: "button", live: true }
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.clone();

                // collect one static shape field per literal keyed property
                let mut fields = Vec::new();
                for property in properties {
                    match self.tree.get(property).clone() {
                        dir::Property::Field { name, value, .. } => {
                            let ty = self.walk_static_term(value)?;
                            fields.push(dir::TypeProperty {
                                key: name.into(),
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
                let ty = self.intern_type(dir::Type::Object(dir::ObjectType {
                    properties,
                    call_signatures: dir::TypeListId::EMPTY,
                    construct_signatures: dir::TypeListId::EMPTY,
                    index_signatures: dir::TypeListId::EMPTY,
                }))?;

                self.commit_node_type(expression, ty)
            }
            // SocketFlags(1) commits a nominal newtype constant
            dir::Expression::Call {
                position: dir::PostfixPosition::Direct,
                left,
                generic_arguments,
                arguments,
                is_optional: false,
            } if generic_arguments.is_empty() => {
                let left = *left;
                let arguments = arguments.clone();

                // require a resolved newtype head
                let Some(symbol) = self.check.written_newtype_head(self.module, left) else {
                    return self.reject_static_value(expression, source);
                };

                // record the head's name edge
                let head_source = left.into_global_any(self.module);
                self.capture_symbol_reference(head_source, symbol)?;
                self.check
                    .commit_name(head_source, dir::NameResolution::new(symbol))?;

                // evaluate each positional argument to a static term
                let mut elements = Vec::with_capacity(arguments.len());
                for argument in &arguments {
                    let dir::Argument::Positional { value } = self.tree.get(*argument) else {
                        return self.reject_static_value(expression, source);
                    };
                    let ty = self.walk_static_term(*value)?;
                    let origin = Origin::Node(
                        (*value).into_global_any(self.module),
                        self.flow().template_scope(),
                    );
                    let ty = self.check.normalize_computation(origin, ty)?;
                    let term = match self.check.ty(ty)? {
                        dir::Type::Literal(value) => dir::StaticTerm::Literal { value },
                        dir::Type::Static(value) => self.check.r#static(value)?.clone(),
                        _ => {
                            return self.reject_static_value(expression, source);
                        }
                    };
                    elements.push(term);
                }

                // wrap the values as the nominal newtype term
                let value = match <[dir::StaticTerm; 1]>::try_from(elements) {
                    Ok([value]) => value,
                    Err(elements) => dir::StaticTerm::Tuple { elements },
                };
                let head = self.intern_type(dir::Type::Application(dir::GenericApplication {
                    symbol,
                    arguments: dir::TypeListId::EMPTY,
                }))?;
                let term = dir::StaticTerm::Newtype {
                    ty: head,
                    value: Box::new(value),
                };
                let id = self
                    .check
                    .module_mut(self.module)
                    .statics_tail
                    .push_static(term);
                let id = id.into_global(self.module);
                let ty = self.intern_type(dir::Type::Static(id))?;

                self.commit_node_type(expression, ty)
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

                self.commit_node_type(expression, ty)
            }
            // every other expression
            _ => self.reject_static_value(expression, source),
        }
    }
}
