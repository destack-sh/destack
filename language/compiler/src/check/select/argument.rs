use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Constraint, FlowSite, Origin, PlaceUse, Relation, ValueUse, answer,
};

impl CheckState<'_> {
    /// Return parameter types.
    pub(in crate::check) fn parameter_types(
        parameters: &[dir::FunctionParameterType],
    ) -> Vec<dir::GlobalTypeId> {
        parameters.iter().map(|parameter| parameter.ty).collect()
    }

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

    /// Return generated argument bindings for parameters.
    pub(in crate::check) fn generated_argument_bindings(
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

    /// Push final argument constraints for one accepted signature.
    pub(in crate::check) fn push_argument_constraints(
        &mut self,
        site: FlowSite,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        bindings: &[dir::ArgumentBinding],
    ) -> CompilerResult<Answer<()>> {
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
            let Some(value) = self.argument_value_node(module, argument) else {
                continue;
            };
            let source = answer!(self.argument_value_type(site, argument)?);

            self.push_constraint(Constraint::value(
                Relation::Assignable,
                source,
                binding.ty,
                Origin::Node(value),
                ValueUse::Argument,
            ));
        }

        Ok(Answer::Ready(()))
    }

    /// Return the type flowing through one runtime argument value.
    pub(in crate::check) fn argument_value_type(
        &mut self,
        site: FlowSite,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let module = site.node.module_id;
        let Some(value) = self.argument_value_node(module, argument) else {
            let error = self.push_type(module, dir::Type::Error, argument.into_any())?;

            return Ok(Answer::Ready(error));
        };

        let site = self.node_site(value)?;
        self.infer_node_type(site, PlaceUse::Read)
    }

    /// Return the value expression carried by one argument node.
    pub(in crate::check) fn argument_value_node(
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

    /// Return value source nodes carried by runtime arguments.
    pub(in crate::check) fn argument_value_sources(
        &self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> SmallVec<[dir::GlobalNodeIdAny; 4]> {
        arguments
            .iter()
            .filter_map(|argument| self.argument_value_node(module, *argument))
            .collect()
    }
}
