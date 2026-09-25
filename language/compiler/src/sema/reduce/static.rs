use tspp_core::FxIndexMap;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::CheckState;
use crate::r#static::{StaticError, StaticEvaluator};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Evaluate one selected newtype backing into a nominal static value.
    pub(in crate::sema) fn evaluate_selected_static_newtype(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::ArgumentBinding],
        backing: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        // evaluate exact parameter bindings
        let elements = match self.evaluate_static_arguments(module, expression, arguments)? {
            Ok(elements) => elements,
            Err(error) => return Ok(Err(error)),
        };

        // evaluate the selected backing alternative
        let backing = self.shallow_resolve(backing)?;
        let value = if matches!(self.ty(backing)?, dir::Type::Tuple(_)) {
            dir::StaticTerm::Tuple { elements }
        } else {
            if elements.len() != 1 {
                return Err(CompilerError::Internal {
                    message: format!(
                        "scalar static newtype {expression:?} has {} arguments",
                        elements.len()
                    ),
                });
            }

            elements
                .into_iter()
                .next()
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("scalar static newtype {expression:?} lost its argument"),
                })?
        };
        let value = dir::StaticTerm::Newtype {
            ty,
            value: Box::new(value),
        };

        Ok(Ok(value))
    }

    /// Evaluate parameter bindings in parameter order.
    pub(in crate::sema) fn evaluate_static_arguments(
        &mut self,
        module: ModuleId,
        anchor: dir::LocalNodeId<dir::Expression>,
        bindings: &[dir::ArgumentBinding],
    ) -> CompilerResult<Result<Vec<dir::StaticTerm>, StaticError>> {
        let mut values = Vec::with_capacity(bindings.len());

        for binding in bindings {
            let arguments =
                match self.evaluate_static_argument_source(module, anchor, &binding.source)? {
                    Ok(arguments) => arguments,
                    Err(error) => return Ok(Err(error)),
                };
            values.extend(arguments);
        }

        Ok(Ok(values))
    }

    /// Evaluate the values contributed by one argument source.
    fn evaluate_static_argument_source(
        &mut self,
        module: ModuleId,
        anchor: dir::LocalNodeId<dir::Expression>,
        source: &dir::ArgumentSource,
    ) -> CompilerResult<Result<Vec<dir::StaticTerm>, StaticError>> {
        match source {
            dir::ArgumentSource::Provided(argument) => {
                self.evaluate_static_argument(module, anchor, *argument)
            }
            dir::ArgumentSource::Spread(spread) => {
                self.evaluate_static_argument_source(module, anchor, &spread.value.source)
            }
            dir::ArgumentSource::Rest { elements, .. } => {
                self.evaluate_static_arguments(module, anchor, elements)
            }
            dir::ArgumentSource::Static(ty) => Ok(Ok(vec![self.evaluate_static_type(*ty)?])),
            dir::ArgumentSource::Omitted => Ok(Ok(vec![dir::Literal::Undefined.into()])),
            dir::ArgumentSource::Supplied(_) | dir::ArgumentSource::Error => {
                Err(CompilerError::Internal {
                    message: "a runtime argument in a static call".to_string(),
                })
            }
        }
    }

    /// Evaluate one source argument.
    fn evaluate_static_argument(
        &mut self,
        module: ModuleId,
        anchor: dir::LocalNodeId<dir::Expression>,
        argument: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Result<Vec<dir::StaticTerm>, StaticError>> {
        if argument.module_id != module || argument.local_id.ty != dir::NodeType::Argument {
            return Err(CompilerError::Internal {
                message: format!("static argument binding has invalid source {argument:?}"),
            });
        }
        let argument = argument.into_typed::<dir::Argument>().local_id;
        let node = self.module_view(module).get(argument).clone();
        let Some(expression) = node.value() else {
            return Ok(Err(StaticError::NotStatic(anchor)));
        };
        let value = match self.evaluate_static_expression(module, expression)? {
            Ok(value) => value,
            Err(error) => return Ok(Err(error)),
        };

        // flatten spread arguments into their selected parameter sequence
        if matches!(node, dir::Argument::Spread { .. }) {
            let elements = match value {
                dir::StaticTerm::Array { elements } | dir::StaticTerm::Tuple { elements } => {
                    elements
                }
                _ => return Ok(Err(StaticError::NotStatic(anchor))),
            };

            Ok(Ok(elements))
        } else {
            Ok(Ok(vec![value]))
        }
    }

    /// Evaluate one static type selected as an inserted argument.
    fn evaluate_static_type(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<dir::StaticTerm> {
        let ty = self.shallow_resolve(ty)?;
        let value = match self.ty(ty)? {
            dir::Type::Literal(value) => value.into(),
            dir::Type::Static(value) => self.r#static(value)?.clone(),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("inserted argument {ty:?} is not static"),
                });
            }
        };

        Ok(value)
    }
    /// Evaluate one expression into a static value.
    pub(in crate::sema) fn evaluate_static_expression(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        let node = self.module_view(module).get(expression).clone();

        // evaluate compound values structurally
        match node {
            dir::Expression::ArrayExpression { elements } => {
                self.evaluate_static_sequence(module, expression, &elements, false)
            }
            dir::Expression::TupleExpression { elements } => {
                self.evaluate_static_sequence(module, expression, &elements, true)
            }
            dir::Expression::ObjectExpression { properties } => {
                self.evaluate_static_object(module, expression, &properties)
            }
            dir::Expression::Call { .. } => {
                let source = expression.into_global_any(module);
                let resolution = self.decisions(module).construct_decision(source).cloned();

                // evaluate selected newtype constructors nominally
                if let Some(dir::ConstructDecision {
                    target: dir::ConstructTarget::Newtype { backing, .. },
                    arguments,
                    return_type,
                    ..
                }) = resolution
                {
                    self.evaluate_selected_static_newtype(
                        module,
                        expression,
                        &arguments,
                        backing,
                        return_type,
                    )
                }
                // evaluate calls over module and profile metadata
                else {
                    Ok(self.evaluate_static_subset(module, expression))
                }
            }
            dir::Expression::StructExpression { properties, .. } => {
                self.evaluate_static_struct(module, expression, &properties)
            }
            dir::Expression::Type { value } => {
                self.evaluate_static_type_expression(module, expression, value)
            }
            dir::Expression::Identifier { .. } => {
                self.evaluate_static_identifier(module, expression)
            }
            _ => Ok(self.evaluate_static_subset(module, expression)),
        }
    }

    /// Evaluate one expression in the shared static subset.
    fn evaluate_static_subset(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::StaticTerm, StaticError> {
        let input = self.module(module);
        let evaluator = StaticEvaluator::new(
            input.view(),
            input.module.as_ref(),
            input.package.as_ref(),
            self.environment.as_ref(),
            &input.profile,
            self.strings(),
        );

        evaluator.evaluate_expression(expression)
    }

    /// Evaluate one static array or tuple value.
    fn evaluate_static_sequence(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
        is_tuple: bool,
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        let mut values = Vec::with_capacity(elements.len());

        // evaluate elements and flatten spreads in source order
        for element in elements {
            let source = element.into_global_any(module);
            let value = match self.evaluate_static_argument(module, expression, source)? {
                Ok(value) => value,
                Err(error) => return Ok(Err(error)),
            };
            values.extend(value);
        }

        let value = if is_tuple {
            dir::StaticTerm::Tuple { elements: values }
        } else {
            dir::StaticTerm::Array { elements: values }
        };

        Ok(Ok(value))
    }

    /// Evaluate one static object value.
    fn evaluate_static_object(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        source_properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        let mut fields = FxIndexMap::<dir::StaticKey, dir::StaticTerm>::default();

        // evaluate fields and flatten spreads in source order, later keys overriding earlier ones
        for property in source_properties {
            let property = self.module_view(module).get(*property).clone();
            match property {
                dir::Property::Field { name, value, .. } => {
                    let value = match self.evaluate_static_expression(module, value)? {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };
                    fields.insert(name.into(), value);
                }
                dir::Property::Spread { value } => {
                    let value = match self.evaluate_static_expression(module, value)? {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };
                    let dir::StaticTerm::Object { properties } = value else {
                        return Ok(Err(StaticError::NotStatic(expression)));
                    };
                    for property in properties {
                        match property {
                            dir::StaticProperty::Field { key, value } => {
                                fields.insert(key, value);
                            }
                            dir::StaticProperty::Method { .. } => {
                                return Ok(Err(StaticError::NotStatic(expression)));
                            }
                        }
                    }
                }
                dir::Property::Method { .. } | dir::Property::Error => {
                    return Ok(Err(StaticError::NotStatic(expression)));
                }
            }
        }
        // order fields by name so written order never splits term identity
        let mut fields: Vec<_> = fields.into_iter().collect();
        fields.sort_by_cached_key(|(key, _)| match key {
            dir::StaticKey::Name(name) => (self.strings().get(*name).to_string(), 0),
            dir::StaticKey::Index(index) => (String::new(), *index + 1),
        });
        let properties = fields
            .into_iter()
            .map(|(key, value)| dir::StaticProperty::Field { key, value })
            .collect();

        Ok(Ok(dir::StaticTerm::Object { properties }))
    }

    /// Commit one module constant's static term at its checked declarator.
    ///
    /// Same-module const generic arguments read the term before the write
    /// pass runs; a term the open inference cannot resolve yet stays for the
    /// write pass to evaluate over the solved types.
    pub(in crate::sema) fn commit_constant_term(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // commit module-scope constants only
        let state = self.module(symbol.module_id);
        if state.symbol(symbol.local_id).scope.id != state.bindings.module_scope().id {
            return Ok(());
        }

        // keep the first committed term
        if self
            .module(symbol.module_id)
            .statics_tail
            .get_symbol_static_id(symbol)
            .is_some()
        {
            return Ok(());
        }

        // keep constants outside the static subset unsettled
        let Ok(term) = self.evaluate_static_expression(symbol.module_id, value)? else {
            return Ok(());
        };

        let state = self.module_mut(symbol.module_id);
        let id = state.statics_tail.push_static(term);
        state
            .statics_tail
            .set_symbol_static(symbol, id.into_global(symbol.module_id));

        Ok(())
    }

    /// Evaluate one nominal struct value.
    fn evaluate_static_struct(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        source_properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        // name the constructed type off the checked expression, unchecked stays dynamic
        let source = expression.into_global_any(module);
        let Some(ty) = self.committed_node_type(source) else {
            return Ok(Err(StaticError::NotStatic(expression)));
        };
        let ty = self.shallow_resolve(ty)?;

        // evaluate fields in source order
        let mut fields = Vec::with_capacity(source_properties.len());
        for property in source_properties {
            let property = self.module_view(module).get(*property).clone();
            match property {
                dir::Property::Field { name, value, .. } => {
                    let value = match self.evaluate_static_expression(module, value)? {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };
                    fields.push((dir::StaticKey::from(name), value));
                }
                dir::Property::Spread { .. }
                | dir::Property::Method { .. }
                | dir::Property::Error => {
                    return Ok(Err(StaticError::NotStatic(expression)));
                }
            }
        }

        // order fields by name so written order never splits term identity
        fields.sort_by_cached_key(|(key, _)| match key {
            dir::StaticKey::Name(name) => (self.strings().get(*name).to_string(), 0),
            dir::StaticKey::Index(index) => (String::new(), *index + 1),
        });
        let properties = fields
            .into_iter()
            .map(|(key, value)| dir::StaticProperty::Field { key, value })
            .collect();

        Ok(Ok(dir::StaticTerm::Struct { ty, properties }))
    }

    /// Evaluate one first-class type expression.
    fn evaluate_static_type_expression(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        // read the checked type, unchecked stays dynamic
        let source = value.into_global_any(module);
        let Some(ty) = self.committed_node_type(source) else {
            return Ok(Err(StaticError::NotStatic(expression)));
        };
        let ty = self.shallow_resolve(ty)?;

        Ok(Ok(dir::StaticTerm::Type { ty }))
    }

    /// Evaluate one static declaration reference.
    fn evaluate_static_identifier(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        let source = expression.into_global_any(module);
        let Some(symbol) = self.reference_symbol(source)? else {
            return Ok(Err(StaticError::NotStatic(expression)));
        };
        // read the static value the symbol already settled
        let term = if let Some(value) = self.static_value(symbol)? {
            let value = self.shallow_resolve(value)?;

            match self.ty(value)? {
                dir::Type::Literal(value) => dir::StaticTerm::Literal { value },
                dir::Type::Static(value) => self.r#static(value)?.clone(),
                _ => return Ok(Err(StaticError::NotStatic(expression))),
            }
        }
        // leave declarations without a static value unevaluated
        else {
            return Ok(Err(StaticError::NotStatic(expression)));
        };

        Ok(Ok(term))
    }
}
