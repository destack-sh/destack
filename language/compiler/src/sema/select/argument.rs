use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{BodyState, CallableArgument, FlowSite, Origin, PlaceUse, Relation, ValueUse};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Return runtime argument bindings for parameters.
    ///
    /// A rest binding accepts the element type from each packed source;
    /// the packed collection type stays on the signature parameter.
    pub(in crate::sema) fn argument_bindings(
        &mut self,
        origin: Origin,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        parameters: &[dir::FunctionParameterType],
    ) -> CompilerResult<Vec<dir::ArgumentBinding>> {
        let mut argument_index = 0usize;
        let mut bindings = Vec::with_capacity(parameters.len());

        for parameter_type in parameters.iter() {
            let mut accepted = parameter_type.ty;
            let source = if parameter_type.is_rest {
                let elements = arguments[argument_index..]
                    .iter()
                    .map(|argument| argument.into_global_any(module))
                    .collect();
                argument_index = arguments.len();

                // selected signatures are resolved, the element must project
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
        let ty = self.infer_node_type(site, PlaceUse::Read)?;

        Ok(ty)
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

        // preserve source expressions for candidate checking
        for argument in arguments {
            let source = argument.into_global_any(module);
            let is_spread = self.is_spread_argument(source);
            match self.argument_expression(module, *argument) {
                Some(value) => values.push(CallableArgument {
                    source: value,
                    ty: None,
                    relation: Relation::Assignable,
                    use_,
                    is_spread,
                }),
                None => {
                    let ty = self.intern_type(dir::Type::Error)?;
                    values.push(CallableArgument {
                        source,
                        ty: Some(ty),
                        relation: Relation::Assignable,
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
                dir::ArgumentSource::Provided(source) => {
                    values.push(CallableArgument {
                        source: *source,
                        ty: None,
                        relation: Relation::Assignable,
                        use_: ValueUse::Argument,
                        is_spread: self.is_spread_argument(*source),
                    });
                }
                dir::ArgumentSource::Rest { elements, .. } => {
                    for source in elements {
                        values.push(CallableArgument {
                            source: *source,
                            ty: None,
                            relation: Relation::Assignable,
                            use_: ValueUse::Argument,
                            is_spread: self.is_spread_argument(*source),
                        });
                    }
                }
                dir::ArgumentSource::Static(ty) => {
                    values.push(CallableArgument {
                        source,
                        ty: Some(*ty),
                        relation: Relation::Assignable,
                        use_: ValueUse::Argument,
                        is_spread: false,
                    });
                }
                dir::ArgumentSource::Write | dir::ArgumentSource::Omitted => {
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
