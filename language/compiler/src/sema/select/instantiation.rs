use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, GenericParameterId, Origin, TypeSubstitution, VariableKind};

impl CheckState<'_> {
    /// Bind one written argument to one parameter.
    fn bind_written_argument(
        &mut self,
        binding: &dir::GenericParameterBinding,
        written: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // pass through arguments other than symbolic value bindings
        let dir::Type::Reference(reference) = self.ty(written)? else {
            return Ok(Some(written));
        };
        if !self.symbol_kind(reference.symbol)?.is_binding() {
            return Ok(Some(written));
        }

        // resolve a const slot's binding to its committed static value
        if binding.is_const {
            if let Some(value) = self.static_value(reference.symbol)? {
                return Ok(Some(value));
            }

            // keep the reference symbolic while declaring
            if self.is_declaring() {
                return Ok(Some(written));
            }
        }

        Ok(None)
    }

    /// Bind explicit arguments and declared defaults to the remaining parameters.
    pub(in crate::sema) fn bind_explicit_arguments(
        &mut self,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
        mut substitution: TypeSubstitution,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        if written.len() > self.writable_parameter_count(parameters)? {
            return Ok(None);
        }

        // bind writable parameters and fill omitted defaults
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            let binding = self.require_generic_parameter(parameter)?.clone();
            let argument = if binding.is_writable()
                && cursor < written.len()
                && self.argument_fills_parameter(&binding, written[cursor])?
            {
                // interpret the written argument by its parameter kind
                let Some(argument) = self.bind_written_argument(&binding, written[cursor])? else {
                    return Ok(None);
                };
                cursor += 1;

                Some(argument)
            } else {
                // evaluate defaults against the arguments already bound
                binding
                    .default
                    .map(|default| self.substitute_type(default, &substitution))
                    .transpose()?
            };

            // retain a fixed argument or require an explicit parameter's argument
            if let Some(argument) = argument {
                substitution.bind(parameter, argument)?;
            } else if matches!(binding.origin, dir::GenericParameterOrigin::Explicit) {
                return Ok(None);
            }
        }

        Ok((cursor == written.len()).then_some(substitution))
    }

    /// Instantiate one parameter list, opening every omitted parameter.
    pub(in crate::sema) fn instantiate_parameters(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
        mut substitution: TypeSubstitution,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        self.counters.instantiations += 1;

        // reject more written arguments than the template can take
        let mut writable = 0;
        for parameter in parameters {
            if substitution.argument(*parameter).is_none()
                && self
                    .generic_parameter(*parameter)?
                    .is_some_and(dir::GenericParameterBinding::is_writable)
            {
                writable += 1;
            }
        }
        if written.len() > writable {
            return Ok(None);
        }

        // bind written parameters and open omitted inference parameters
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            // keep the parameters the caller already bound
            if substitution.argument(parameter).is_some() {
                continue;
            }

            let binding = self.require_generic_parameter(parameter)?.clone();
            if binding.is_writable()
                && cursor < written.len()
                && self.argument_fills_parameter(&binding, written[cursor])?
            {
                // interpret a value binding argument by the slot's kind
                let Some(argument) = self.bind_written_argument(&binding, written[cursor])? else {
                    return Ok(None);
                };
                substitution.bind(parameter, argument)?;
                cursor += 1;

                continue;
            }

            // reuse a parameter opened earlier at this typing position
            let origin_id = self.infer.intern_origin(origin);
            if let Some(existing) = self.infer.instantiation(origin_id, parameter) {
                let is_stale_memory =
                    binding.memory_parameter().is_some() && self.open_root(existing)?.is_none();
                if !is_stale_memory {
                    let argument = self.variable_type(existing)?;
                    substitution.bind(parameter, argument)?;

                    continue;
                }
            }

            // open one inference variable for the omitted parameter
            let memory_kind = match (binding.memory_parameter(), binding.constraint) {
                (Some(kind), _) => Some(kind),
                (None, Some(constraint)) => self.memory_kind(constraint)?,
                (None, None) => None,
            };
            let kind = memory_kind.map_or(VariableKind::Type, VariableKind::Memory);
            let variable = self.open_instantiation(origin, parameter, kind)?;

            // record the instantiation while the site claims its typing position
            self.infer
                .insert_instantiation(origin_id, parameter, variable);

            // keep the declared default for dry inference
            if let Some(kind) = memory_kind {
                let default = self.elided_memory_default(kind)?;
                self.set_variable_default(variable, default)?;
            } else if let Some(default) = binding.default {
                let default = self.substitute_type(default, &substitution)?;
                self.set_variable_default(variable, default)?;
            }

            let argument = self.variable_type(variable)?;
            substitution.bind(parameter, argument)?;
        }

        Ok(Some(substitution))
    }
}
