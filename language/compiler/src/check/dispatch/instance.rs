use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CallArgument, CheckState, FunctionParameter, FunctionTerm, GenericArgument,
    GenericArgumentDefault, GenericArgumentKey, GenericInstance, GenericParameterId,
    GenericTemplateId, Origin, SubstitutionSet, TermId, TypeOperand, VariableId, VariableKind,
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
        selected_instance: Option<&GenericInstance>,
        function: TermId<FunctionTerm>,
        generic_parameters: &[GenericParameterId],
        generic_arguments: &[GenericArgument],
        arguments: &[CallArgument],
    ) -> CompilerResult<(Option<GenericInstance>, SubstitutionSet)> {
        if generic_parameters.is_empty() {
            let substitution = if let Some(receiver) = receiver {
                SubstitutionSet::with_receiver(receiver)
            } else {
                SubstitutionSet::empty()
            };

            return Ok((None, substitution));
        }
        let mut substitution = SubstitutionSet::empty();
        let target_template =
            target_symbol.and_then(|symbol| self.inference.generic_template_by_symbol(symbol));
        let mut instance_arguments = SmallVec::<[GenericArgument; 2]>::new();
        let mut template_indexes = SmallVec::<[(GenericTemplateId, usize); 2]>::new();

        // build one substitution entry per signature generic
        for parameter in generic_parameters {
            let generic = self.inference.generic_parameter_binding(*parameter)?;
            let kind = generic.kind();
            let template = generic.parameter().template;
            let index = Self::next_template_index(&mut template_indexes, template);
            let explicit_argument = match target_symbol {
                // expression calls apply generic arguments to the whole callable type
                None => generic_arguments.get(index),
                // symbol calls apply generic arguments only to the called symbol template
                Some(_) if target_template == Some(template) => generic_arguments.get(index),
                // enclosing templates come from the selected receiver member
                Some(_) => None,
            };
            let argument = self.instantiate_call_generic_argument(
                module,
                source,
                *parameter,
                kind,
                template,
                index,
                explicit_argument,
                selected_instance,
                function,
                arguments,
            )?;

            // retain a stable instance for the called template only
            if target_template == Some(template) {
                instance_arguments.push(argument);
            }
            substitution.generic(*parameter, argument);
        }

        // include the receiver in the same substitution pass
        if let Some(receiver) = receiver {
            substitution.receiver(receiver);
        }
        let selected_instance =
            target_template.map(|template| GenericInstance::new(template, instance_arguments));

        Ok((selected_instance, substitution))
    }

    /// Return the next parameter index inside one generic template.
    fn next_template_index(
        indexes: &mut SmallVec<[(GenericTemplateId, usize); 2]>,
        template: GenericTemplateId,
    ) -> usize {
        for (candidate, index) in indexes.iter_mut() {
            if *candidate == template {
                let current = *index;

                *index += 1;

                return current;
            }
        }

        indexes.push((template, 1));

        0
    }

    /// Instantiate one generic argument for a call signature parameter.
    fn instantiate_call_generic_argument(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
        kind: VariableKind,
        template: GenericTemplateId,
        index: usize,
        explicit_argument: Option<&GenericArgument>,
        selected_instance: Option<&GenericInstance>,
        function: TermId<FunctionTerm>,
        arguments: &[CallArgument],
    ) -> CompilerResult<GenericArgument> {
        // use explicitly supplied generic arguments
        if let Some(argument) = explicit_argument {
            return argument.specialize(kind, self);
        }
        // reuse the selected receiver or target instance
        else if let Some(argument) = selected_instance
            .filter(|instance| instance.template == template)
            .and_then(|instance| instance.arguments.get(index))
        {
            return argument.specialize(kind, self);
        }
        // bind comptime parameters from their runtime argument expression
        else if let Some(argument) =
            self.static_parameter_argument(parameter, function, arguments)?
        {
            return Ok(argument);
        }
        // otherwise leave an inference variable for solve
        else {
            self.instantiation_argument(module, source, parameter)
        }
    }

    /// Infer omitted call generic arguments from parameter type patterns.
    pub(in crate::check) fn infer_call_arguments(
        &mut self,
        origin: Origin,
        module: ModuleId,
        function: TermId<FunctionTerm>,
        generic_parameters: &[GenericParameterId],
        receiver: Option<TypeOperand>,
        arguments: &[CallArgument],
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<()>> {
        let templates = self.call_generic_templates(generic_parameters)?;
        let function_id = function;
        let function = self.inference.term(function_id);
        let this_parameter = function.this_parameter;

        // match the method receiver against its declared receiver pattern
        if let (Some(receiver), Some(pattern)) = (receiver, this_parameter) {
            let result = self.infer_type_patterns(
                origin,
                module,
                &templates,
                pattern,
                receiver,
                substitution,
            )?;
            if let Answer::Pending(blockers) = result {
                return Ok(Answer::Pending(blockers));
            }
        }

        // match each actual argument against its declared parameter pattern
        for (index, argument) in arguments.iter().copied().enumerate() {
            let Some((parameter_index, parameter)) = self.function_parameter(function_id, index)
            else {
                continue;
            };
            let Answer::Ready(Some(pattern)) = self.call_argument_parameter_type(
                origin,
                index,
                parameter_index,
                argument,
                &parameter,
            )?
            else {
                return Ok(Answer::pending(argument.ty.dependencies(self)));
            };
            let result = self.infer_type_patterns(
                origin,
                module,
                &templates,
                pattern,
                argument.ty,
                substitution,
            )?;
            if let Answer::Pending(blockers) = result {
                return Ok(Answer::Pending(blockers));
            }
        }

        Ok(Answer::Ready(()))
    }

    /// Return the parameter that receives one call argument.
    fn function_parameter(
        &self,
        function: TermId<FunctionTerm>,
        index: usize,
    ) -> Option<(usize, FunctionParameter)> {
        let parameters = &self.inference.term(function).parameters;

        // use the matching positional parameter
        if index < parameters.len() {
            return Some((index, parameters[index]));
        }

        // use the rest parameter for excess arguments
        parameters
            .last()
            .filter(|parameter| parameter.is_rest)
            .copied()
            .map(|parameter| (parameters.len() - 1, parameter))
    }

    /// Infer generic arguments from one declared pattern and actual operand.
    fn infer_type_patterns(
        &mut self,
        origin: Origin,
        module: ModuleId,
        templates: &[GenericTemplateId],
        pattern: TypeOperand,
        actual: TypeOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<()>> {
        let Some(pattern) = self.type_operand_term_id(pattern)? else {
            return Ok(Answer::Ready(()));
        };
        let Answer::Ready(actual) = self.reduce_type_operand(origin, actual)? else {
            return Ok(Answer::pending(actual.dependencies(self)));
        };
        let Some(actual) = self.type_operand_term_id(actual)? else {
            return Ok(Answer::pending(actual.dependencies(self)));
        };

        // infer all templates referenced by this signature
        for template in templates {
            let is_match = self.match_type_pattern_in_template(
                origin,
                module,
                *template,
                pattern,
                actual,
                substitution,
            )?;
            if let Answer::Pending(blockers) = is_match {
                return Ok(Answer::Pending(blockers));
            }
        }

        Ok(Answer::Ready(()))
    }

    /// Apply declared defaults to generic argument variables.
    pub(in crate::check) fn apply_generic_argument_defaults(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameters: &[GenericParameterId],
        explicit_count: usize,
    ) -> CompilerResult<()> {
        for parameter in parameters.iter().skip(explicit_count) {
            self.apply_generic_argument_default(module, substitution, *parameter)?;
        }

        Ok(())
    }

    /// Apply one declared default to a generic argument variable.
    fn apply_generic_argument_default(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameter: GenericParameterId,
    ) -> CompilerResult<()> {
        let generic = self.inference.generic_parameter_binding(parameter)?;
        let type_default = generic.type_default();
        let static_default = generic.static_default();

        match (type_default, static_default) {
            (Some(default), None) => {
                let Some(argument) = self
                    .substitution_type_operand(substitution, parameter)
                    .and_then(|operand| operand.variable())
                else {
                    return Ok(());
                };
                let default = self.substitute_type_operand(module, substitution, default)?;

                let default = GenericArgumentDefault::r#type(parameter, default);

                self.inference
                    .upsert_generic_argument_default(argument, default)?;
            }
            (None, Some(default)) => {
                let Some(argument) = self
                    .substitution_static_operand(substitution, parameter)
                    .and_then(|operand| operand.variable())
                else {
                    return Ok(());
                };
                let default = self.substitute_static_operand(module, substitution, default)?;

                let default = GenericArgumentDefault::r#static(parameter, default);

                self.inference
                    .upsert_generic_argument_default(argument, default)?;
            }
            (None, None) => {}
            (Some(_), Some(_)) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "generic parameter {parameter:?} has both type and static defaults"
                    ),
                });
            }
        };

        Ok(())
    }

    /// Return generic templates referenced by one call signature.
    fn call_generic_templates(
        &self,
        generic_parameters: &[GenericParameterId],
    ) -> CompilerResult<SmallVec<[GenericTemplateId; 2]>> {
        let mut templates = SmallVec::new();

        // collect unique templates in parameter order
        for parameter in generic_parameters {
            let generic = self.inference.generic_parameter_binding(*parameter)?;
            let template = generic.parameter().template;
            if templates.contains(&template) {
                continue;
            }

            templates.push(template);
        }

        Ok(templates)
    }

    /// Create one call instantiation argument.
    fn instantiation_argument(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
    ) -> CompilerResult<GenericArgument> {
        let generic = self.inference.generic_parameter_binding(parameter)?;
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
        parameter: GenericParameterId,
        function: TermId<FunctionTerm>,
        arguments: &[CallArgument],
    ) -> CompilerResult<Option<GenericArgument>> {
        let Some(index) = self
            .inference
            .term(function)
            .parameters
            .iter()
            .position(|candidate| candidate.static_parameter == Some(parameter))
        else {
            return Ok(None);
        };
        let Some(value) = arguments.get(index).copied() else {
            return Ok(None);
        };
        let operand = self.node_static_operand(value.source)?;

        Ok(Some(GenericArgument::Static(operand)))
    }

    /// Return one stable call instantiation argument variable.
    fn instantiation_variable(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
    ) -> CompilerResult<VariableId> {
        let generic = self.inference.generic_parameter_binding(parameter)?;
        let origin = Origin::Node(source);
        let key = GenericArgumentKey::new(source, parameter);
        if let Some(variable) = self.inference.generic_argument(key) {
            return Ok(variable);
        }

        let variable = if generic.is_static() {
            self.push_static_variable(module, origin)
        } else {
            self.push_type_variable(module, origin)
        };
        let variable = self.inference.upsert_generic_argument(key, variable);

        Ok(variable)
    }
}
