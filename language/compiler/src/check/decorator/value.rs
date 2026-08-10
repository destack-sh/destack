use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorApplication, SelectedDecorator};
use crate::r#static::{StaticError, StaticEvaluator};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Evaluate one selected decorator application.
    pub(in crate::check) fn evaluate_decorator(
        &mut self,
        selection: SelectedDecorator,
    ) -> CompilerResult<
        Result<
            (
                DecoratorApplication,
                dir::DecoratorResolution,
                dir::StaticTerm,
            ),
            StaticError,
        >,
    > {
        let SelectedDecorator {
            application,
            resolution,
        } = selection;
        let module = application.owner.module_id;

        let value = match &resolution.selection {
            dir::DecoratorSelection::Newtype {
                newtype, arguments, ..
            } => {
                let expression = self
                    .module_view(module)
                    .get(application.expression.decorator)
                    .expression;
                match self.evaluate_selected_static_newtype(
                    module,
                    expression,
                    arguments,
                    newtype.backing,
                    resolution.ty,
                )? {
                    Ok(value) => value,
                    Err(error) => return Ok(Err(error)),
                }
            }
            dir::DecoratorSelection::Derive { providers } => {
                let mut elements = Vec::with_capacity(providers.len());

                // evaluate selected providers in argument order
                for provider in providers {
                    match self.evaluate_derive_provider(provider)? {
                        Ok(value) => elements.push(value),
                        Err(error) => return Ok(Err(error)),
                    }
                }

                dir::StaticTerm::Newtype {
                    ty: resolution.ty,
                    value: Box::new(dir::StaticTerm::Tuple { elements }),
                }
            }
        };

        Ok(Ok((application, resolution, value)))
    }

    /// Evaluate one selected compiler-owned derive provider.
    fn evaluate_derive_provider(
        &mut self,
        provider: &dir::DeriveProvider,
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        let module = provider.argument.module_id;
        let argument = provider.argument.local_id;
        let view = self.module_view(module);
        let Some(expression) = view.get(argument).value() else {
            return Err(CompilerError::Internal {
                message: format!("derive provider argument {argument:?} has no value"),
            });
        };

        // capability interfaces carry no configuration value
        let is_capability = self
            .environment_bound
            .language
            .item(provider.newtype.symbol)
            .and_then(dir::AutoInterface::from_language_item)
            .is_some_and(dir::AutoInterface::is_derivable);
        if is_capability {
            return Ok(Ok(dir::StaticTerm::Type { ty: provider.ty }));
        }

        // evaluate a configured provider through its construction resolution
        if matches!(view.get(expression), dir::Expression::Call { .. }) {
            self.evaluate_static_expression(module, expression)
        }
        // otherwise evaluate a bare provider through the backing derive selected
        else {
            self.evaluate_selected_static_newtype(
                module,
                expression,
                &[],
                provider.newtype.backing,
                provider.ty,
            )
        }
    }

    /// Evaluate one selected newtype backing into a nominal static value.
    fn evaluate_selected_static_newtype(
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
    fn evaluate_static_arguments(
        &mut self,
        module: ModuleId,
        anchor: dir::LocalNodeId<dir::Expression>,
        bindings: &[dir::ArgumentBinding],
    ) -> CompilerResult<Result<Vec<dir::StaticTerm>, StaticError>> {
        let mut values = Vec::with_capacity(bindings.len());

        for binding in bindings {
            match &binding.source {
                dir::ArgumentSource::Provided(argument) => {
                    let value = match self.evaluate_static_argument(module, anchor, *argument)? {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };
                    values.extend(value);
                }
                dir::ArgumentSource::Static(ty) => {
                    let value = self.evaluate_static_type(*ty)?;
                    values.push(value);
                }
                dir::ArgumentSource::Omitted => {
                    values.push(dir::ScalarLiteral::Undefined.into());
                }
                dir::ArgumentSource::Write => {
                    return Err(CompilerError::Internal {
                        message: "decorator call contains an implicit write argument".to_string(),
                    });
                }
                dir::ArgumentSource::Rest(arguments) => {
                    for argument in arguments {
                        let value =
                            match self.evaluate_static_argument(module, anchor, *argument)? {
                                Ok(value) => value,
                                Err(error) => return Ok(Err(error)),
                            };
                        values.extend(value);
                    }
                }
            }
        }

        Ok(Ok(values))
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
            dir::Type::Static(value) => self.r#static(value).clone(),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("inserted argument {ty:?} is not static"),
                });
            }
        };

        Ok(value)
    }

    /// Evaluate one expression into a static value.
    pub(in crate::check) fn evaluate_static_expression(
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
                    target: dir::ConstructTarget::Newtype(candidate),
                    arguments,
                    return_type,
                }) = resolution
                {
                    self.evaluate_selected_static_newtype(
                        module,
                        expression,
                        &arguments,
                        candidate.backing,
                        return_type,
                    )
                }
                // evaluate calls over module and profile metadata
                else {
                    Ok(self.evaluate_static_subset(module, expression))
                }
            }
            dir::Expression::Type { value } => self.evaluate_static_type_expression(module, value),
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
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        let mut values = Vec::with_capacity(properties.len());

        // preserve authored object property order
        for property in properties {
            let property = self.module_view(module).get(*property).clone();
            let value = match property {
                dir::Property::Field { key, value, .. } => {
                    let key = match key {
                        dir::Key::Name(name) => Some(name.static_key()),
                        dir::Key::Expression(expression) => {
                            self.evaluate_static_key(module, expression)?
                        }
                    };
                    let Some(key) = key else {
                        return Ok(Err(StaticError::NotStatic(expression)));
                    };
                    let value = match self.evaluate_static_expression(module, value)? {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };

                    dir::StaticProperty::Field { key, value }
                }
                dir::Property::Spread { value } => {
                    let value = match self.evaluate_static_expression(module, value)? {
                        Ok(value) => value,
                        Err(error) => return Ok(Err(error)),
                    };

                    dir::StaticProperty::Spread { value }
                }
                dir::Property::Method { .. } | dir::Property::Error => {
                    return Ok(Err(StaticError::NotStatic(expression)));
                }
            };
            values.push(value);
        }

        Ok(Ok(dir::StaticTerm::Object { properties: values }))
    }

    /// Evaluate one first-class type expression.
    fn evaluate_static_type_expression(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Result<dir::StaticTerm, StaticError>> {
        let source = value.into_global_any(module);
        let ty = self.require_node_type(source)?;
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
        let Some(symbol) = self.reference_symbol(source) else {
            return Ok(Err(StaticError::NotStatic(expression)));
        };
        let term = if let Some(value) = self.static_value(symbol) {
            let value = self.shallow_resolve(value)?;

            match self.ty(value)? {
                dir::Type::Literal(value) => dir::StaticTerm::ScalarLiteral { value },
                dir::Type::Static(value) => self.r#static(value).clone(),
                _ => return Ok(Err(StaticError::NotStatic(expression))),
            }
        }
        // otherwise read a type declaration as a first-class value,
        //  skipping foreign kinds while declaring
        else if self
            .symbol_kind_maybe(symbol)?
            .is_some_and(|kind| kind.can_be_used_as_type())
        {
            let ty = self.require_node_type(source)?;
            let ty = self.shallow_resolve(ty)?;

            dir::StaticTerm::Type { ty }
        }
        // reject runtime bindings, which hold no static value
        else {
            return Ok(Err(StaticError::NotStatic(expression)));
        };

        Ok(Ok(term))
    }
}
