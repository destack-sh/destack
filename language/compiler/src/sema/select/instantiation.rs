use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, GenericParameterId, Origin, TypeSubstitution, VariableState};

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
                let variable = self.infer.variable(existing)?;
                let is_killed =
                    variable.is_dead || matches!(variable.state, VariableState::Error(_));
                let is_stale_memory =
                    binding.memory_parameter().is_some() && self.open_root(existing)?.is_none();
                if !is_killed && !is_stale_memory {
                    let argument = self.variable_type(existing)?;
                    substitution.bind(parameter, argument)?;

                    continue;
                }
            }

            // open one inference variable for the omitted parameter
            let variable = self.open_omitted_parameter(origin, parameter)?;

            // record the instantiation while the site claims its typing position
            self.infer
                .insert_instantiation(origin_id, parameter, variable);

            // substitute the declared default into the site
            if let Some(default) = binding.default {
                let default = self.substitute_type(default, &substitution)?;
                self.set_variable_default(variable, default)?;
            }

            let argument = self.variable_type(variable)?;
            substitution.bind(parameter, argument)?;
        }

        Ok(Some(substitution))
    }

    /// Instantiate one symbol's callable, the targets binding its type parameters in order.
    pub(in crate::sema) fn instantiate_symbol_call(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        targets: &[dir::GlobalTypeId],
        bound: TypeSubstitution,
    ) -> CompilerResult<Option<dir::Call>> {
        let Some(callable_type) = self.adopt_symbol_type_maybe(symbol)? else {
            return Ok(None);
        };
        let parameters = match self.symbol_template(symbol)? {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let Some(substitution) =
            self.instantiate_parameters(origin, &parameters, targets, bound)?
        else {
            return Ok(None);
        };

        // apply the instantiation to the declared signature
        let signature = match self.ty(callable_type)? {
            dir::Type::FunctionSignature(signature) => {
                self.type_signature(callable_type.module_id, signature)?
            }
            _ => return Ok(None),
        };
        let declared = self.signature_parameters(callable_type.module_id, signature.parameters)?;
        let mut arguments = Vec::with_capacity(declared.len());
        for (index, parameter) in declared.iter().enumerate() {
            let ty = self.substitute_type(parameter.ty, &substitution)?;
            arguments.push(dir::ArgumentBinding {
                coercion: None,
                parameter_type: ty,
                argument_type: ty,
                source: dir::ArgumentSource::Supplied(index as u32),
            });
        }
        let return_type = match signature.return_type {
            Some(return_type) => self.substitute_type(return_type, &substitution)?,
            None => self.intern_type(dir::Type::Void)?,
        };
        // key the instance by its own parameters first, then the owner bindings the caller gave
        let mut bindings = Vec::with_capacity(substitution.bindings.len());
        for parameter in parameters.iter().copied() {
            if let Some(argument) = substitution.argument(parameter) {
                bindings.push(dir::GenericArgumentBinding::new(parameter, argument));
            }
        }
        for binding in substitution.bindings.iter().copied() {
            if !parameters.contains(&binding.parameter) {
                bindings.push(binding);
            }
        }
        let key = dir::InstanceKey::new(symbol, bindings);
        let call = dir::Call {
            regions: self.resolved_region_bindings(&substitution.bindings)?,
            target: dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: None,
                    generic_scope: None,
                    key,
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            callable_type,
            arguments,
            return_type,
        };

        Ok(Some(call))
    }
}
