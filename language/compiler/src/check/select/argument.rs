use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, BoundMode, CallableArgument, Cause, CauseKind, Constraint, Expectation,
    ExpectedType, FlowSite, Origin, PlaceUse, Relation, ValueUse, answer,
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

    /// Queue final argument checks for one accepted signature.
    pub(in crate::check) fn check_arguments(
        &mut self,
        site: FlowSite,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        bindings: &[dir::ArgumentBinding],
    ) -> CompilerResult<()> {
        let module = site.node.module_id;
        for argument in argument_nodes.iter().copied() {
            let argument_node = argument.into_global_any(module);
            let Some(binding) = bindings.iter().find(|binding| {
                matches!(
                    &binding.argument,
                    dir::ArgumentSource::Provided(source) if *source == argument_node
                ) || matches!(
                    &binding.argument,
                    dir::ArgumentSource::Rest(sources) if sources.contains(&argument_node)
                )
            }) else {
                continue;
            };
            let Some(value) = self.argument_expression(module, argument) else {
                continue;
            };
            let value_site = self.node_site(value)?;
            let origin = Origin::Node(value, site.scope);

            // barred parameters settle from the unbarred arguments alone
            let mut expected = binding.ty;
            let is_barred = self.contains_inference_barrier(expected)?;
            if is_barred {
                for variable in self.type_variables(expected)? {
                    let _ = self.check.solve_variable(variable, BoundMode::Strong)?;
                }
                expected = self.erase_inference_barriers(module, expected)?;
            }
            // use the same relation selected during candidate matching
            let argument = CallableArgument::Expression(value);
            let relation = self.argument_relation(argument);

            let cause = self.check.intern_cause(Cause::root(
                origin,
                CauseKind::Argument {
                    call: site.node,
                    index: binding.parameter as u32,
                },
            ));

            // verify values against open barred targets once they close
            if is_barred && !self.type_variables(expected)?.is_empty() {
                let checked = self.check_node(value_site, PlaceUse::Read, None)?;
                let value_origin = self.intern_origin(origin);
                self.push_constraint(Constraint::check_only_value(
                    relation,
                    checked.ty,
                    expected,
                    value_origin,
                    cause,
                    Some(ValueUse::Argument),
                ));

                continue;
            }

            let expectation = Expectation {
                expected: ExpectedType::Type(expected),
                relation,
                cause,
                use_: ValueUse::Argument,
            };
            self.check_node(value_site, PlaceUse::Read, Some(expectation))?;
        }

        Ok(())
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
    ) -> CompilerResult<SmallVec<[CallableArgument; 4]>> {
        let mut values = SmallVec::<[CallableArgument; 4]>::new();

        // preserve argument positions even for malformed argument nodes
        for argument in arguments {
            let source = argument.into_global_any(module);
            match self.argument_expression(module, *argument) {
                Some(value) => values.push(CallableArgument::Expression(value)),
                None => {
                    let ty = self.intern_type(module, dir::Type::Error)?;
                    values.push(CallableArgument::Typed { source, ty });
                }
            }
        }

        Ok(values)
    }

    /// Return callable arguments carried by selected argument sources.
    pub(in crate::check) fn source_callable_arguments(
        &mut self,
        origin: Origin,
        arguments: &[dir::GlobalTypeId],
        sources: &[dir::ArgumentSource],
    ) -> CompilerResult<SmallVec<[CallableArgument; 4]>> {
        let source = self
            .origin_source_node(origin)?
            .into_global(origin.module());
        let mut index = 0usize;
        let mut values = SmallVec::<[CallableArgument; 4]>::new();

        // map each binding source to the type supplied by its caller
        for argument in sources {
            match argument {
                dir::ArgumentSource::Provided(source) => {
                    if arguments.get(index).is_none() {
                        return Err(CompilerError::Internal {
                            message: "typed argument source has no type".to_string(),
                        });
                    }
                    index += 1;
                    values.push(CallableArgument::Expression(*source));
                }
                dir::ArgumentSource::Rest(sources) => {
                    for source in sources {
                        if arguments.get(index).is_none() {
                            return Err(CompilerError::Internal {
                                message: "typed rest argument source has no type".to_string(),
                            });
                        }
                        index += 1;
                        values.push(CallableArgument::Expression(*source));
                    }
                }
                dir::ArgumentSource::Static(ty) => {
                    let ty = match arguments.get(index).copied() {
                        Some(ty) => {
                            index += 1;
                            ty
                        }
                        None => *ty,
                    };
                    values.push(CallableArgument::Typed { source, ty });
                }
                dir::ArgumentSource::Omitted => {
                    if let Some(ty) = arguments.get(index).copied() {
                        index += 1;
                        values.push(CallableArgument::Typed { source, ty });
                    }
                }
            }
        }

        // reject mismatched typed argument metadata loudly
        if index != arguments.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "typed call supplied {} types but consumed {index}",
                    arguments.len()
                ),
            });
        }

        Ok(values)
    }

    /// Return the relation used to match and check one argument.
    pub(in crate::check) fn argument_relation(&self, argument: CallableArgument) -> Relation {
        let CallableArgument::Expression(mut value) = argument else {
            return Relation::Assignable;
        };
        let module = value.module_id;

        // follow satisfies expressions that preserve the literal value
        loop {
            match self
                .module(module)
                .view()
                .get(value.local_id.into_typed::<dir::Expression>())
            {
                dir::Expression::ObjectExpression { .. }
                | dir::Expression::ArrayExpression { .. }
                | dir::Expression::TupleExpression { .. } => return Relation::Writable,
                dir::Expression::Satisfies { expression, .. } => {
                    value = expression.into_global_any(module);
                }
                _ => return Relation::Assignable,
            }
        }
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
