use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    ArgumentValue, CallableArgument, Cause, CauseId, CauseKind, CheckFailure, CheckState,
    Expectation, FlowSite, InferMode, NodeForm, Origin, PlaceUse, Relation, Value, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Forward a generated function's parameters to the called signature.
    pub(in crate::sema) fn forward_parameters(
        &mut self,
        site: FlowSite,
        parameters: &[dir::FunctionParameterType],
        supplied: &[dir::FunctionParameterType],
    ) -> CompilerResult<Option<Vec<dir::ArgumentBinding>>> {
        // forward compatible rest collections at the same parameter position
        let (parameters, supplied, forwarded) = if let Some((parameter, prefix)) =
            parameters.split_last()
            && let Some((argument, supplied_prefix)) = supplied.split_last()
            && parameter.is_rest
            && argument.is_rest
            && prefix.len() == supplied_prefix.len()
            && self
                .decide_relation(site.origin(), Relation::Subtype, argument.ty, parameter.ty)?
                .holds()
        {
            let binding = dir::ArgumentBinding {
                parameter_type: parameter.ty,
                argument_type: argument.ty,
                source: dir::ArgumentSource::Supplied(prefix.len() as u32),
                coercion: None,
            };

            (prefix, supplied_prefix, Some(binding))
        } else {
            (parameters, supplied, None)
        };

        // expand remaining rest parameters and bind positional arguments
        let sources = supplied
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let source = dir::ArgumentSource::Supplied(index as u32);
                if parameter.is_rest {
                    self.select_spread_argument(
                        site,
                        source,
                        Value {
                            ty: parameter.ty,
                            node: None,
                            place: None,
                            is_fresh: false,
                        },
                    )
                } else {
                    Ok(source)
                }
            })
            .collect::<CompilerResult<Vec<_>>>()?;
        let Some(mut arguments) = self.bind_arguments(site.origin(), parameters, &sources)? else {
            return Ok(None);
        };
        arguments.extend(forwarded);

        Ok(Some(arguments))
    }

    /// Bind one prepared source to its complete parameter and argument types.
    fn bind_argument(
        &mut self,
        source: &dir::ArgumentSource,
        parameter_type: dir::GlobalTypeId,
        argument_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::ArgumentBinding> {
        // select an authored spread after its contextual type has been checked
        let source = match source {
            dir::ArgumentSource::Provided(source)
                if source.local_id.ty == dir::NodeType::Argument =>
            {
                self.argument_source(source.into_typed(), None)?
            }
            _ => source.clone(),
        };

        Ok(dir::ArgumentBinding {
            parameter_type,
            argument_type,
            source,
            coercion: None,
        })
    }

    /// Return the value, spread, or omission written at one argument.
    pub(in crate::sema) fn argument_source(
        &mut self,
        argument: dir::GlobalNodeId<dir::Argument>,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::ArgumentSource> {
        match self
            .module(argument.module_id)
            .view()
            .get(argument.local_id)
        {
            dir::Argument::Positional { .. } => {
                Ok(dir::ArgumentSource::Provided(argument.into_any()))
            }
            dir::Argument::Spread { value } => {
                // select iteration from the source value, inferred under the context
                let value = value.into_global_any(argument.module_id);
                let site = self.visit_site(value)?;
                let ty = self.infer_node_in(site, PlaceUse::Read, InferMode::Regular, context)?;
                let ty = self.flow_type_at(site, ty)?;
                let value = self.expression_value(site, ty)?;
                let source = dir::ArgumentSource::Provided(argument.into_any());

                self.select_spread_argument(site, source, value)
            }
            dir::Argument::Elision => Ok(dir::ArgumentSource::Omitted),
            dir::Argument::Error => Ok(dir::ArgumentSource::Error),
        }
    }

    /// Select the collection read and iteration of a spread argument.
    pub(in crate::sema) fn select_spread_argument(
        &mut self,
        site: FlowSite,
        source: dir::ArgumentSource,
        value: Value,
    ) -> CompilerResult<dir::ArgumentSource> {
        // select iteration through the source's declared protocol
        let origin = site.origin();
        let Some((element, iteration)) =
            self.select_iteration(origin, value, dir::Asynchrony::Sync)?
        else {
            self.report_source_not_iterable(site.node);

            return Ok(dir::ArgumentSource::Error);
        };
        let value = dir::ArgumentBinding {
            parameter_type: value.ty,
            argument_type: value.ty,
            source,
            coercion: None,
        };

        Ok(dir::ArgumentSource::Spread(Box::new(dir::SpreadArgument {
            value,
            element,
            iteration,
        })))
    }

    /// Bind authored or generated sources when every parameter has an argument type.
    pub(in crate::sema) fn bind_arguments(
        &mut self,
        origin: Origin,
        parameters: &[dir::FunctionParameterType],
        sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Option<Vec<dir::ArgumentBinding>>> {
        let mut index = 0;
        let mut bindings = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            // collect each trailing argument into the rest parameter
            let (source, argument_type) = if parameter.is_rest {
                let trailing = &sources[index..];
                index = sources.len();
                let Some(element) = self.rest_element_type(origin, parameter.ty)? else {
                    return Ok(None);
                };
                let elements: Vec<_> = trailing
                    .iter()
                    .map(|source| self.bind_argument(source, element, element))
                    .collect::<CompilerResult<_>>()?;
                self.check_spread_arguments(origin, &elements)?;
                let pack = self.rest_pack_selection(origin, parameter.ty, element)?;

                (dir::ArgumentSource::Rest { elements, pack }, element)
            }
            // bind one positional source or record omission
            else {
                let source = match sources.get(index) {
                    Some(source) => {
                        index += 1;
                        source.clone()
                    }
                    None => dir::ArgumentSource::Omitted,
                };

                (source, parameter.ty)
            };
            bindings.push(self.bind_argument(&source, parameter.ty, argument_type)?);
        }

        Ok(Some(bindings))
    }

    /// Check each authored spread element against its destination type.
    pub(in crate::sema) fn check_spread_arguments(
        &mut self,
        origin: Origin,
        arguments: &[dir::ArgumentBinding],
    ) -> CompilerResult<()> {
        for (index, argument) in arguments.iter().enumerate() {
            // check the yielded value at its authored argument
            if let dir::ArgumentSource::Spread(spread) = &argument.source
                && let dir::ArgumentSource::Provided(source) = spread.value.source
            {
                let site = self.visit_site(source)?;
                let cause = self.intern_cause(Cause::root(
                    origin,
                    CauseKind::Element {
                        index: index as u32,
                    },
                ));
                let value = Value {
                    ty: spread.element,
                    node: None,
                    place: None,
                    is_fresh: false,
                };

                // record the conversion for expression writeback
                let expectation =
                    Expectation::assignable(argument.parameter_type, cause, ValueUse::Argument);
                let conversion = self.convert_value(
                    site,
                    cause,
                    expectation.relation,
                    value,
                    expectation.target,
                    expectation.use_,
                    expectation.mode,
                )?;
                self.commit_value_conversion(site, value.ty, expectation, conversion)?;
            }
        }

        Ok(())
    }

    /// Convert a generated argument and retain its selected coercion.
    pub(in crate::sema) fn convert_argument(
        &mut self,
        site: FlowSite,
        cause: CauseId,
        argument: &mut dir::ArgumentBinding,
        supplied: &[dir::GlobalTypeId],
    ) -> CompilerResult<Result<(), CheckFailure>> {
        // convert nested arguments and read this argument's supplied type
        let ty = match &mut argument.source {
            dir::ArgumentSource::Rest { elements, .. } => {
                for element in elements {
                    if let Err(failure) = self.convert_argument(site, cause, element, supplied)? {
                        return Ok(Err(failure));
                    }
                }

                return Ok(Ok(()));
            }
            dir::ArgumentSource::Supplied(index) => supplied
                .get(*index as usize)
                .copied()
                .ok_or_else(|| CompilerError::Internal {
                    message: "an argument binding without its supplied type".to_string(),
                })?,
            dir::ArgumentSource::Spread(spread) => {
                if let Err(failure) =
                    self.convert_argument(site, cause, &mut spread.value, supplied)?
                {
                    return Ok(Err(failure));
                }

                spread.element
            }
            dir::ArgumentSource::Static(ty) => *ty,
            dir::ArgumentSource::Omitted | dir::ArgumentSource::Error => return Ok(Ok(())),
            dir::ArgumentSource::Provided(_) => {
                return Err(CompilerError::Internal {
                    message: "a generated argument conversion contains an authored expression"
                        .to_string(),
                });
            }
        };

        // select the conversion of the supplied value or yielded element
        let value = Value {
            ty,
            node: None,
            place: None,
            is_fresh: false,
        };
        argument.coercion = match self.convert_closed_value(
            site,
            site.origin(),
            cause,
            value,
            argument.parameter_type,
            ValueUse::Argument,
        )? {
            Ok(coercion) => coercion,
            Err(failure) => return Ok(Err(failure)),
        };

        Ok(Ok(()))
    }

    /// Infer the type supplied by one runtime argument.
    pub(in crate::sema) fn infer_argument_type(
        &mut self,
        site: FlowSite,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = site.node.module_id;
        // fall back to the error type when the argument has no parsed expression
        let Some(value) = self.argument_expression(module, argument) else {
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(error);
        };

        let site = self.visit_site(value)?;

        self.infer_node_type(site, PlaceUse::Read)
    }

    /// Infer the types supplied by runtime arguments.
    pub(in crate::sema) fn infer_argument_types(
        &mut self,
        site: FlowSite,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 4]>> {
        let mut types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in arguments {
            let ty = self.infer_argument_type(site, *argument)?;
            types.push(ty);
        }

        Ok(types)
    }

    /// Return callable arguments read from source argument nodes.
    pub(in crate::sema) fn callable_arguments(
        &mut self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        use_: ValueUse,
    ) -> CompilerResult<SmallVec<[CallableArgument; 4]>> {
        let mut values = SmallVec::<[CallableArgument; 4]>::new();

        // type every argument once ahead of the candidates that relate to it
        for argument in arguments {
            let source = argument.into_global_any(module);
            let is_spread = self.is_spread_argument(source);
            let mut prepared = dir::ArgumentSource::Provided(source);
            match self.argument_expression(module, *argument) {
                Some(value) => {
                    let site = self.visit_site(value)?;
                    let supplied = if self.is_contextually_typed(value) {
                        // check a function value's closed body ahead of the candidates
                        if matches!(self.node_form(value), NodeForm::FunctionValue) {
                            self.commit_function_value(value)?;
                            self.queue_check_function_body(value)?;
                        }

                        ArgumentValue::Contextual
                    } else if is_spread {
                        prepared = self.argument_source(argument.into_global(module), None)?;
                        let ty = match &prepared {
                            dir::ArgumentSource::Spread(spread) => spread.element,
                            dir::ArgumentSource::Error => self.intern_type(dir::Type::Error)?,
                            _ => {
                                return Err(CompilerError::Internal {
                                    message: "a spread argument without its iteration".to_string(),
                                });
                            }
                        };

                        ArgumentValue::Typed(ty)
                    } else {
                        ArgumentValue::Typed(self.infer_node_type(site, PlaceUse::Read)?)
                    };
                    values.push(CallableArgument {
                        source: value,
                        argument: prepared,
                        value: supplied,
                        relation: Relation::Storable,
                        use_,
                        is_spread,
                    });
                }
                None => {
                    let value = ArgumentValue::Typed(self.intern_type(dir::Type::Error)?);
                    values.push(CallableArgument {
                        source,
                        argument: prepared,
                        value,
                        relation: Relation::Storable,
                        use_,
                        is_spread,
                    });
                }
            }
        }

        Ok(values)
    }

    /// Return callable arguments read from selected argument sources.
    pub(in crate::sema) fn source_callable_arguments(
        &mut self,
        origin: Origin,
        sources: &[dir::ArgumentSource],
    ) -> CompilerResult<SmallVec<[CallableArgument; 4]>> {
        let source = self
            .origin_source_node(origin)?
            .into_global(origin.module());
        let mut values = SmallVec::<[CallableArgument; 4]>::new();

        // map authored and generated values directly
        for argument in sources {
            match argument {
                // type an authored argument at its own source node
                dir::ArgumentSource::Provided(source) => {
                    let site = self.visit_site(*source)?;
                    let value = ArgumentValue::Typed(self.infer_node_type(site, PlaceUse::Read)?);
                    values.push(CallableArgument {
                        source: *source,
                        argument: argument.clone(),
                        value,
                        relation: Relation::Storable,
                        use_: ValueUse::Argument,
                        is_spread: self.is_spread_argument(*source),
                    });
                }
                // type every element a rest argument collects
                dir::ArgumentSource::Rest { elements, .. } => {
                    for element in elements {
                        values.extend(self.source_callable_arguments(
                            origin,
                            std::slice::from_ref(&element.source),
                        )?);
                    }
                }
                dir::ArgumentSource::Spread(spread) => {
                    values.push(CallableArgument {
                        source,
                        argument: argument.clone(),
                        value: ArgumentValue::Typed(spread.element),
                        relation: Relation::Storable,
                        use_: ValueUse::Argument,
                        is_spread: true,
                    });
                }
                // take a generated argument's written type
                dir::ArgumentSource::Static(ty) => {
                    values.push(CallableArgument {
                        source,
                        argument: argument.clone(),
                        value: ArgumentValue::Typed(*ty),
                        relation: Relation::Storable,
                        use_: ValueUse::Argument,
                        is_spread: false,
                    });
                }
                // defer a written value to the parameter selection pairs it with
                dir::ArgumentSource::Supplied(_) => {
                    values.push(CallableArgument {
                        source,
                        argument: argument.clone(),
                        value: ArgumentValue::Deferred,
                        relation: Relation::Storable,
                        use_: ValueUse::Argument,
                        is_spread: false,
                    });
                }
                // fail on a source that supplies no argument
                dir::ArgumentSource::Omitted | dir::ArgumentSource::Error => {
                    return Err(CompilerError::Internal {
                        message: format!("{argument:?} is not a callable argument source"),
                    });
                }
            }
        }

        Ok(values)
    }

    /// Return whether one source node spreads a sequence.
    pub(in crate::sema) fn is_spread_argument(&self, node: dir::GlobalNodeIdAny) -> bool {
        let Ok(argument) = node.local_id.try_into_typed::<dir::Argument>() else {
            return false;
        };

        // read whether the argument spreads
        matches!(
            self.module(node.module_id).view().get(argument),
            dir::Argument::Spread { .. }
        )
    }

    /// Return the expression of one argument node.
    pub(in crate::sema) fn argument_expression(
        &self,
        module: ModuleId,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> Option<dir::GlobalNodeIdAny> {
        self.module(module)
            .view()
            .get(argument)
            .value()
            .map(|value| value.into_global_any(module))
    }
}
