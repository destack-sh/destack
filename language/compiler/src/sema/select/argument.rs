use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    ArgumentValue, CallableArgument, CheckState, FlowSite, Origin, PlaceUse, Relation, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return runtime argument bindings for parameters.
    pub(in crate::sema) fn argument_bindings(
        &mut self,
        origin: Origin,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        parameters: &[dir::FunctionParameterType],
    ) -> CompilerResult<Vec<dir::ArgumentBinding>> {
        let mut argument_index = 0usize;
        let mut bindings = Vec::with_capacity(parameters.len());

        // bind each declared parameter to its written argument
        for parameter_type in parameters.iter() {
            let mut accepted = parameter_type.ty;
            let source = if parameter_type.is_rest {
                let elements = arguments[argument_index..]
                    .iter()
                    .map(|argument| argument.into_global_any(module))
                    .collect();
                argument_index = arguments.len();

                // require the element to project from the resolved signature
                let element = self.rest_element_type(origin, parameter_type.ty)?;
                accepted = element.unwrap_or(accepted);

                dir::ArgumentSource::Rest {
                    elements,
                    pack: self.rest_pack_selection(origin, parameter_type.ty, accepted)?,
                }
            } else if let Some(argument) = arguments.get(argument_index).copied() {
                argument_index += 1;

                dir::ArgumentSource::Provided(argument.into_global_any(module))
            } else {
                dir::ArgumentSource::Omitted
            };

            bindings.push(dir::ArgumentBinding {
                parameter_type: parameter_type.ty,
                argument_type: accepted,
                source,
            });
        }

        Ok(bindings)
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
            match self.argument_expression(module, *argument) {
                Some(value) => {
                    let site = self.visit_site(value)?;
                    let supplied = match !is_spread && self.is_composite_node(value) {
                        true => ArgumentValue::Composite,
                        false => ArgumentValue::Typed(self.infer_node_type(site, PlaceUse::Read)?),
                    };
                    values.push(CallableArgument {
                        source: value,
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
                        value,
                        relation: Relation::Storable,
                        use_: ValueUse::Argument,
                        is_spread: self.is_spread_argument(*source),
                    });
                }
                // type every element a rest argument collects
                dir::ArgumentSource::Rest { elements, .. } => {
                    for source in elements {
                        let site = self.visit_site(*source)?;
                        let value =
                            ArgumentValue::Typed(self.infer_node_type(site, PlaceUse::Read)?);
                        values.push(CallableArgument {
                            source: *source,
                            value,
                            relation: Relation::Storable,
                            use_: ValueUse::Argument,
                            is_spread: self.is_spread_argument(*source),
                        });
                    }
                }
                // take a generated argument's written type
                dir::ArgumentSource::Static(ty) => {
                    values.push(CallableArgument {
                        source,
                        value: ArgumentValue::Typed(*ty),
                        relation: Relation::Storable,
                        use_: ValueUse::Argument,
                        is_spread: false,
                    });
                }
                // defer a written value to the parameter selection pairs it with
                dir::ArgumentSource::Supplied => {
                    values.push(CallableArgument {
                        source,
                        value: ArgumentValue::Deferred,
                        relation: Relation::Storable,
                        use_: ValueUse::Argument,
                        is_spread: false,
                    });
                }
                // fail on a source that supplies no argument
                dir::ArgumentSource::Omitted => {
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
