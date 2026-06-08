use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CheckState, Condition, FunctionParameter, FunctionTerm, GenericArgument, GenericInstance,
    GenericInstanceKey, GenericParameterBinding, GenericParameterId, GenericTemplateId, Origin,
    StaticTerm, SubstitutionSet, TypeOperand, VariableId,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Instantiate one selected call signature.
    pub(in crate::check) fn instantiate_call_signature(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        target_symbol: Option<dir::GlobalSymbolId>,
        receiver: Option<TypeOperand>,
        selected_instance: Option<GenericInstance>,
        function: &FunctionTerm,
        generic_arguments: &[GenericArgument],
        arguments: &[dir::GlobalNodeId<dir::Expression>],
    ) -> CompilerResult<(Option<GenericInstance>, SubstitutionSet)> {
        if function.generic_parameters.is_empty() {
            let substitution = if let Some(receiver) = receiver {
                SubstitutionSet::with_receiver(receiver)
            } else {
                SubstitutionSet::empty()
            };

            return Ok((selected_instance, substitution));
        }
        let mut substitution = SubstitutionSet::empty();
        let target_template =
            target_symbol.and_then(|symbol| self.inference.symbol_generic_template(symbol));
        let mut instance_arguments =
            SmallVec::<[(GenericTemplateId, SmallVec<[GenericArgument; 2]>); 2]>::new();

        // build one substitution entry per signature generic
        for (index, parameter) in function.generic_parameters.iter().enumerate() {
            let generic = self.inference.require_generic_parameter(*parameter).clone();
            let template = generic.parameter().template;
            let instance_key = GenericInstanceKey::new(source, template);
            let argument = self.instantiate_call_generic_argument(
                module,
                source,
                *parameter,
                &generic,
                index,
                generic_arguments,
                selected_instance.as_ref(),
                instance_key,
                &function.parameters,
                arguments,
            )?;

            substitution.generic(*parameter, argument.clone());
            // group instance arguments by declaring template
            if let Some((_, arguments)) = instance_arguments
                .iter_mut()
                .find(|(candidate, _)| *candidate == template)
            {
                arguments.push(argument);
            } else {
                instance_arguments.push((template, smallvec::smallvec![argument]));
            }
        }

        // include the receiver in the same substitution pass
        if let Some(receiver) = receiver {
            substitution.receiver(receiver);
        }
        let mut selected_instance = selected_instance;

        // retain stable instances by generic template
        for (template, arguments) in instance_arguments {
            let key = GenericInstanceKey::new(source, template);
            let instance = self.inference.upsert_generic_instance(key, arguments);

            if target_template == Some(template) && selected_instance.is_none() {
                selected_instance = Some(instance);
            }
        }

        Ok((selected_instance, substitution))
    }

    /// Instantiate one generic argument for a call signature parameter.
    fn instantiate_call_generic_argument(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
        generic: &GenericParameterBinding,
        index: usize,
        explicit_arguments: &[GenericArgument],
        selected_instance: Option<&GenericInstance>,
        instance_key: GenericInstanceKey,
        parameters: &[FunctionParameter],
        arguments: &[dir::GlobalNodeId<dir::Expression>],
    ) -> CompilerResult<GenericArgument> {
        // use explicitly supplied generic arguments
        if let Some(argument) = explicit_arguments.get(index) {
            return self.resolved_generic_argument(generic, argument);
        }
        // reuse the selected receiver or target instance
        else if let Some(argument) = selected_instance
            .filter(|instance| instance.template == generic.parameter().template)
            .and_then(|instance| instance.arguments.get(index))
        {
            return self.resolved_generic_argument(generic, argument);
        }
        // reuse a durable source instance from an earlier reduction pass
        else if let Some(argument) = self
            .inference
            .generic_instance(instance_key)
            .and_then(|instance| instance.arguments.get(index))
        {
            return Ok(argument.clone());
        }
        // bind comptime parameters from their runtime argument expression
        else if let Some(argument) =
            self.static_parameter_argument(module, parameter, parameters, arguments)?
        {
            return Ok(argument);
        }
        // otherwise leave an inference variable for solve
        else {
            self.instantiation_argument(module, source, parameter)
        }
    }

    /// Return one already resolved generic argument.
    fn resolved_generic_argument(
        &self,
        parameter: &GenericParameterBinding,
        argument: &GenericArgument,
    ) -> CompilerResult<GenericArgument> {
        let argument = match (parameter, argument) {
            (
                GenericParameterBinding::Type { .. } | GenericParameterBinding::VariadicType { .. },
                GenericArgument::TypeOrStatic { source },
            ) => {
                let operand = self.node_type_operand(source.value.clone().into_any())?;

                GenericArgument::Type(operand)
            }
            (
                GenericParameterBinding::Type { .. } | GenericParameterBinding::VariadicType { .. },
                GenericArgument::SpreadTypeOrStatic { source },
            ) => {
                let operand = self.node_type_operand(source.value.clone().into_any())?;

                GenericArgument::SpreadType(operand)
            }
            (
                GenericParameterBinding::Static { .. }
                | GenericParameterBinding::VariadicStatic { .. },
                GenericArgument::TypeOrStatic { .. } | GenericArgument::SpreadTypeOrStatic { .. },
            ) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "static generic argument for {:?} reached dispatch without static lowering",
                        parameter.parameter().template
                    ),
                });
            }
            (_, argument) => argument.clone(),
        };

        Ok(argument)
    }

    /// Create one omitted call instantiation argument.
    fn instantiation_argument(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
    ) -> CompilerResult<GenericArgument> {
        let generic = self.inference.require_generic_parameter(parameter);
        let is_static = generic.is_static();
        let variable = self.instantiation_variable(module, source, parameter)?;
        let argument = if is_static {
            GenericArgument::Static(variable.into())
        } else {
            GenericArgument::Type(variable.into())
        };

        Ok(argument)
    }

    /// Return the runtime argument supplied to one comptime parameter.
    fn static_parameter_argument(
        &mut self,
        module: ModuleId,
        parameter: GenericParameterId,
        parameters: &[FunctionParameter],
        arguments: &[dir::GlobalNodeId<dir::Expression>],
    ) -> CompilerResult<Option<GenericArgument>> {
        let Some(index) = parameters
            .iter()
            .position(|candidate| candidate.static_parameter == Some(parameter))
        else {
            return Ok(None);
        };
        let Some(value) = arguments.get(index).cloned() else {
            return Ok(None);
        };
        let origin = Origin::Node(value.clone().into_any());
        let variable = self.create_static_variable(module, origin);
        let term = StaticTerm::Expression(value);
        let term = self.inference.push_term(term);

        self.equate_static(origin, variable, term, Condition::Always);

        Ok(Some(GenericArgument::Static(variable.into())))
    }

    /// Return one stable omitted call instantiation argument variable.
    fn instantiation_variable(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
    ) -> CompilerResult<VariableId> {
        let generic = self.inference.require_generic_parameter(parameter);
        let origin = Origin::Node(source);

        let variable = if generic.is_static() {
            self.create_static_variable(module, origin)
        } else {
            self.create_type_variable(module, origin)
        };

        Ok(variable)
    }
}
