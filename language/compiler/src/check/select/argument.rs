use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CallableArgument, FlowSite, Origin, PlaceUse, Relation, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Return runtime argument bindings for parameters.
    pub(in crate::check) fn argument_bindings(
        &self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        parameters: &[dir::FunctionParameterType],
    ) -> Vec<dir::ArgumentBinding> {
        let mut argument_index = 0usize;
        let mut bindings = Vec::with_capacity(parameters.len());

        for (parameter, parameter_type) in parameters.iter().enumerate() {
            let argument = if parameter_type.is_rest {
                let rest = arguments[argument_index..]
                    .iter()
                    .map(|argument| argument.into_global_any(module))
                    .collect();
                argument_index = arguments.len();

                dir::ArgumentSource::Rest(rest)
            } else if let Some(argument) = arguments.get(argument_index).copied() {
                argument_index += 1;

                dir::ArgumentSource::Provided(argument.into_global_any(module))
            } else {
                dir::ArgumentSource::Omitted
            };

            bindings.push(dir::ArgumentBinding {
                parameter,
                ty: parameter_type.ty,
                argument,
            });
        }

        bindings
    }

    /// Return argument bindings from already selected argument sources.
    pub(in crate::check) fn source_argument_bindings(
        arguments: &[dir::ArgumentSource],
        parameters: &[dir::FunctionParameterType],
    ) -> Vec<dir::ArgumentBinding> {
        parameters
            .iter()
            .enumerate()
            .map(|(parameter, parameter_type)| {
                let argument = arguments
                    .get(parameter)
                    .cloned()
                    .unwrap_or(dir::ArgumentSource::Omitted);

                dir::ArgumentBinding {
                    parameter,
                    ty: parameter_type.ty,
                    argument,
                }
            })
            .collect()
    }

    /// Infer the type supplied by one runtime argument.
    pub(in crate::check) fn infer_argument_type(
        &mut self,
        site: FlowSite,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let module = site.node.module_id;
        let Some(value) = self.argument_expression(module, argument) else {
            let error = self.intern_type(module, dir::Type::Error)?;

            return Ok(Answer::Ready(error));
        };

        let site = self.node_site(value)?;
        let ty = answer!(self.infer_node_type(site, PlaceUse::Read)?);

        Ok(Answer::Ready(ty))
    }

    /// Infer the types supplied by runtime arguments.
    pub(in crate::check) fn infer_argument_types(
        &mut self,
        site: FlowSite,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Answer<SmallVec<[dir::GlobalTypeId; 4]>>> {
        let mut types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in arguments {
            let ty = answer!(self.infer_argument_type(site, *argument)?);
            types.push(ty);
        }

        Ok(Answer::Ready(types))
    }

    /// Return callable arguments carried by source argument nodes.
    pub(in crate::check) fn callable_arguments(
        &mut self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        use_: ValueUse,
    ) -> CompilerResult<SmallVec<[CallableArgument; 4]>> {
        let mut values = SmallVec::<[CallableArgument; 4]>::new();

        // preserve source expressions for candidate checking
        for argument in arguments {
            let source = argument.into_global_any(module);
            match self.argument_expression(module, *argument) {
                Some(value) => values.push(CallableArgument {
                    source: value,
                    ty: None,
                    relation: Relation::Assignable,
                    use_,
                }),
                None => {
                    let ty = self.intern_type(module, dir::Type::Error)?;
                    values.push(CallableArgument {
                        source,
                        ty: Some(ty),
                        relation: Relation::Assignable,
                        use_,
                    });
                }
            }
        }

        Ok(values)
    }

    /// Return callable arguments carried by selected argument sources.
    pub(in crate::check) fn source_callable_arguments(
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
                    });
                }
                dir::ArgumentSource::Rest(sources) => {
                    for source in sources {
                        values.push(CallableArgument {
                            source: *source,
                            ty: None,
                            relation: Relation::Assignable,
                            use_: ValueUse::Argument,
                        });
                    }
                }
                dir::ArgumentSource::Static(ty) => {
                    values.push(CallableArgument {
                        source,
                        ty: Some(*ty),
                        relation: Relation::Assignable,
                        use_: ValueUse::Argument,
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

    /// Return the expression carried by one argument node.
    pub(in crate::check) fn argument_expression(
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
