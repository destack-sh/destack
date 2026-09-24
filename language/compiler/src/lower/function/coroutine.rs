use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Synthesize one coroutine entry: create the carrier around the extracted body.
    pub(in crate::lower) fn lower_coroutine_entry(
        &mut self,
        symbol: dir::GlobalSymbolId,
        parameters: &[dir::LocalSymbolId],
        body: mir::FunctionId,
    ) -> CompilerResult<()> {
        // read the recorded creation call at the declaring node
        let state = self.lower.state(self.source)?;
        let node = state.bindings.get_symbol(symbol.local_id).declaration;
        let Some(dir::OperationResolution::One(create)) =
            node.and_then(|node| state.decisions.call_decision(node).cloned())
        else {
            return Err(CompilerError::Internal {
                message: "a coroutine body without its recorded creation call".to_string(),
            });
        };

        // intern the environment types and open its values with the captures
        let (pointee, reference) = self.coroutine_environment_types(symbol, parameters)?;
        let mut values = Vec::with_capacity(parameters.len() + 2);
        if let Some(captures) = self.capture_environment_type(symbol)? {
            values.push(self.builder.function_environment_current(captures));
        }

        // add the receiver the signature declares
        if self.coroutine_receiver_type(symbol)?.is_some() {
            let Some(binding) = self.this else {
                return Err(CompilerError::Internal {
                    message: "a coroutine receiver slot without a receiver binding".to_string(),
                });
            };
            values.push(self.read_binding(binding)?);
        }

        // add each parameter the extracted body reads
        for symbol in parameters {
            let Some(binding) = self.values.get(symbol).copied() else {
                return Err(CompilerError::Internal {
                    message: "a coroutine parameter without a home".to_string(),
                });
            };
            values.push(self.read_binding(binding)?);
        }

        // build the environment the extracted body unpacks
        let aggregate = self.builder.aggregate(pointee, values);
        let environment = self.builder.new_complete(aggregate, reference);

        // find the closure argument the creation call expects
        let Some(binding) = create
            .arguments
            .iter()
            .find(|binding| matches!(binding.source, dir::ArgumentSource::Supplied(_)))
        else {
            return Err(CompilerError::Internal {
                message: "a creation call without its body closure slot".to_string(),
            });
        };
        let dir::CallableTarget::Symbol { function, .. } = &create.target else {
            return Err(CompilerError::Internal {
                message: "a creation call outside a direct symbol target".to_string(),
            });
        };

        // bind the extracted body over the environment as that closure
        let bindings = self.lower.selection_bindings(&function.key)?;
        let closure_type = self.lower_type(binding.argument_type)?;
        let closure = self
            .builder
            .function_bind(body, Vec::new(), closure_type, environment);

        // run the creation call over that closure
        let selection = dir::InstanceKey::new(function.key.symbol, bindings);
        let create_function = self.resolve_callee_of(function.key.symbol, &selection)?;
        let carrier = self.call(&create_function, vec![closure]);

        // return the carrier the creation call produces
        let Some(carrier) = carrier else {
            return Err(CompilerError::Internal {
                message: "a creation call producing no carrier".to_string(),
            });
        };
        self.return_value(Some(carrier))
    }

    /// Rebind one extracted coroutine body's parameters from its environment.
    pub(in crate::lower) fn bind_coroutine_parameters(
        &mut self,
        symbol: dir::GlobalSymbolId,
        parameters: &[dir::LocalSymbolId],
    ) -> CompilerResult<()> {
        // take the environment the entry built and release its allocation
        let (pointee, reference) = self.coroutine_environment_types(symbol, parameters)?;
        let environment = self.builder.function_environment_current(reference);
        let place = mir::Place::value(environment).with_projection(mir::Projection::Deref);
        let taken = self.load_place(place, pointee);

        self.release_emptied(environment, reference);

        // unpack the captures the enclosing function packed
        let mut index = 0;
        if self.capture_environment_type(symbol)?.is_some() {
            self.captures = Some(self.builder.field_get(taken, index));
            index += 1;
        }

        // unpack the receiver the signature declares
        if self.coroutine_receiver_type(symbol)?.is_some() {
            let value = self.builder.field_get(taken, index);
            let local = self.home(value);
            self.this = Some(Binding::Local(local));
            index += 1;
        }

        // rebind each parameter to its unpacked value
        for symbol in parameters {
            let value = self.builder.field_get(taken, index);
            let local = self.home(value);
            self.values.insert(*symbol, Binding::Local(local));
            index += 1;
        }

        Ok(())
    }

    /// Intern the struct and unique reference types of one coroutine environment.
    fn coroutine_environment_types(
        &mut self,
        symbol: dir::GlobalSymbolId,
        parameters: &[dir::LocalSymbolId],
    ) -> CompilerResult<(mir::TypeId, mir::TypeId)> {
        // list the capture, receiver, and parameter types the environment holds
        let mut slots = Vec::with_capacity(parameters.len() + 2);
        slots.extend(self.capture_environment_type(symbol)?);
        slots.extend(self.coroutine_receiver_type(symbol)?);
        for symbol in parameters {
            let declared = self.lower.symbol_type(symbol.into_global(self.source))?;
            slots.push(self.lower_type(declared)?);
        }

        Ok(self.environment_reference_types(&slots, mir::Reference::Unique))
    }

    /// Return the lowered receiver representation the coroutine's signature declares.
    fn coroutine_receiver_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::TypeId>> {
        let declared = self.lower.symbol_type(symbol)?;
        let (signature, owner) = self.lower.signature(declared)?;
        let this = self.lower.types(owner)?.signature(signature).this_parameter;
        let Some(this) = this else {
            return Ok(None);
        };

        Ok(Some(self.lower_type(this)?))
    }
}
