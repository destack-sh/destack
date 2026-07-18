use destack_dir as dir;
use destack_mir as mir;
use destack_mir::{IntrinsicInstruction, IntrinsicTerminator};

use crate::lower::{AmbientCallable, FunctionLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one call expression through its checked call resolution.
    pub(in crate::lower) fn lower_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        let resolution = self.lowerer.call_resolution(expression)?;

        match &resolution.target {
            // free(...)
            dir::CallTarget::Symbol(candidate) => {
                // sealed intrinsic and binding callables bypass declared functions
                match self.lowerer.ambient_callable(candidate.symbol)? {
                    Some(AmbientCallable::Intrinsic { name }) => {
                        return self.lower_intrinsic_call(name, &resolution);
                    }
                    Some(AmbientCallable::Binding { .. }) => {
                        return self.lower_binding_call(candidate.symbol, &resolution);
                    }
                    None => {}
                }

                // receiver.method(...)
                if candidate.receiver.is_some() || !candidate.adjustments.is_empty() {
                    self.lower_method_call(expression, &resolution, candidate)
                }
                // generic<T>(...) selects its materialized instance
                else if candidate.generic_scope.is_some() {
                    self.lower_generic_call(&resolution)
                } else if self.has_type_generic_arguments(candidate)? {
                    self.lower_instance_call(candidate, &resolution)
                }
                // local or imported (...)
                else {
                    self.lower_function_call(candidate.symbol, &resolution)
                }
            }
            // value(...)
            dir::CallTarget::Expression { .. } => self.lower_indirect_call(&resolution),
            // (a | b).method(...)
            dir::CallTarget::Universal(_) => self.lower_universal_call(&resolution),
        }
    }

    /// Return whether one candidate binds generic arguments beyond lifetimes.
    fn has_type_generic_arguments(&self, candidate: &dir::CallCandidate) -> CompilerResult<bool> {
        for binding in &candidate.generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.lowerer.state(parameter.module_id)?.generics;
            let declared = generics.get_parameter(parameter.local_id);
            if declared.memory_parameter() != Some(dir::MemoryParameter::Lifetime) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Lower one sealed intrinsic call to its MIR operation.
    fn lower_intrinsic_call(
        &mut self,
        name: Option<String>,
        resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        let Some(name) = name else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an unnamed intrinsic callable".to_string(),
            }
            .into());
        };
        // each MIR position owns its own name vocabulary
        if let Ok(operation) = name.parse::<mir::Intrinsic>() {
            let result = self
                .lowerer
                .lower_type_id(self.builder.tree_mut(), resolution.return_type)?;
            let values = self.lower_provided_arguments(resolution)?;

            return Ok(Some(self.builder.intrinsic(operation, result, values)));
        }

        if let Some(instruction) = IntrinsicInstruction::from_name(&name) {
            return match instruction {
                IntrinsicInstruction::AtomicFence => self.lower_atomic_fence(resolution),
                IntrinsicInstruction::AtomicLoad => self.lower_atomic_load(resolution),
                IntrinsicInstruction::AtomicCompareExchange { weak } => {
                    self.lower_atomic_compare_exchange(weak, resolution)
                }
                IntrinsicInstruction::AtomicRmw(operator) => {
                    self.lower_atomic_rmw(operator, resolution)
                }
                IntrinsicInstruction::AtomicStore => self.lower_atomic_store(resolution),
                IntrinsicInstruction::Breakpoint => {
                    self.builder.breakpoint();

                    Ok(None)
                }
                IntrinsicInstruction::Cast(operator) => {
                    self.lower_cast_intrinsic(operator, resolution)
                }
                IntrinsicInstruction::PointerLoad => self.lower_pointer_load(resolution),
                IntrinsicInstruction::PointerStore => self.lower_pointer_store(resolution),
                IntrinsicInstruction::PointerReplace => self.lower_pointer_replace(resolution),
                IntrinsicInstruction::PointerSwap => self.lower_pointer_swap(resolution),
                IntrinsicInstruction::PointerDropInPlace => {
                    self.lower_pointer_drop_in_place(resolution)
                }
                IntrinsicInstruction::SliceFromRaw => self.lower_slice_from_raw(resolution),
                IntrinsicInstruction::SliceGet => self.lower_slice_get(resolution),
                IntrinsicInstruction::SliceLength => self.lower_slice_length(resolution),
                IntrinsicInstruction::SliceSet => self.lower_slice_set(resolution),
                IntrinsicInstruction::SliceView => self.lower_slice_view(resolution),
            };
        }

        if let Some(terminator) = IntrinsicTerminator::from_name(&name) {
            match terminator {
                IntrinsicTerminator::TrapAbort => self.builder.trap_abort(),
                IntrinsicTerminator::Unreachable => self.builder.unreachable(),
                IntrinsicTerminator::Panic => {
                    let values = self.lower_provided_arguments(resolution)?;

                    self.builder.panic(values.into_iter().next());
                }
            }

            // code after a terminator continues in a dead block
            let dead = self.builder.block();
            self.builder.switch_to_block(dead);

            return Ok(None);
        }

        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: format!("the '{name}' intrinsic"),
        }
        .into())
    }

    /// Lower one sealed binding call through its declared dotted extern.
    fn lower_binding_call(
        &mut self,
        symbol: dir::GlobalSymbolId,
        resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        let function = self.function_for(symbol, &[])?;
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Lower the provided arguments of one resolution in parameter order.
    fn lower_provided_arguments(
        &mut self,
        resolution: &dir::CallResolution,
    ) -> CompilerResult<Vec<mir::Value>> {
        let mut values = Vec::with_capacity(resolution.arguments.len());
        for binding in &resolution.arguments {
            let dir::ArgumentSource::Provided(source) = binding.argument else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted or spread argument".to_string(),
                }
                .into());
            };
            values.push(self.lower_argument(source)?);
        }

        Ok(values)
    }

    /// Lower one call to a declared or imported function.
    fn lower_function_call(
        &mut self,
        symbol: dir::GlobalSymbolId,
        resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        // resolve the declared MIR function behind the symbol
        let function = self.function_for(symbol, &[])?;
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one method call through its checked candidate.
    fn lower_method_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::CallResolution,
        candidate: &dir::CallCandidate,
    ) -> CompilerResult<Option<mir::Value>> {
        // the callee member names the receiver expression
        let dir::Expression::Call { left: callee, .. } =
            *self.lowerer.source().tree().get(expression)
        else {
            return Err(CompilerError::Internal {
                message: "checked DIR called a method outside a call expression".to_string(),
            });
        };
        let dir::Expression::Member { left: receiver, .. } =
            *self.lowerer.source().tree().get(callee)
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a method call without a member callee".to_string(),
            }
            .into());
        };

        self.lower_candidate_call(receiver, resolution, candidate)
    }

    /// Lower one candidate call over one explicit receiver expression.
    pub(in crate::lower) fn lower_candidate_call(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::CallResolution,
        candidate: &dir::CallCandidate,
    ) -> CompilerResult<Option<mir::Value>> {
        // sealed adjustments take the receiver through its place
        let receiver = match candidate.adjustments.as_slice() {
            [] => self.lower_expression(receiver)?,
            [dir::Projection::Borrow { ty, .. }] => {
                let target = self.lowerer.lower_type_id(self.builder.tree_mut(), *ty)?;

                self.lower_borrowed_place(receiver, target)?
            }
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a '{other:?}' receiver adjustment"),
                }
                .into());
            }
        };

        // resolve the declared MIR function behind the method symbol
        let function = self.function_for(candidate.symbol, &[])?;

        // bind the arguments after the receiver
        let mut values = vec![receiver];
        values.extend(self.lower_provided_arguments(resolution)?);

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one call instantiating a generic callable.
    fn lower_generic_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a generically scoped call".to_string(),
        }
        .into())
    }

    /// Lower one call to a materialized generic instance.
    fn lower_instance_call(
        &mut self,
        candidate: &dir::CallCandidate,
        resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        // the substituted arguments select the declared instance by carrier
        let arguments = self
            .lowerer
            .instance_arguments(candidate, &self.lowerer.substitution)?;
        let key = self
            .lowerer
            .instance_key(self.builder.tree_mut(), &arguments)?;
        let function = self.function_for(candidate.symbol, &key)?;
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Return the MIR function behind one callable symbol and its instance key.
    fn function_for(
        &self,
        symbol: dir::GlobalSymbolId,
        key: &[mir::Type],
    ) -> CompilerResult<mir::FunctionId> {
        self.lowerer
            .functions
            .get(&(symbol, key.to_vec()))
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR is missing a declared function behind one call symbol"
                    .to_string(),
            })
    }

    /// Lower one call through a function-typed value.
    fn lower_indirect_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "an indirect call".to_string(),
        }
        .into())
    }

    /// Lower one call selected across every union member.
    fn lower_universal_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a universal call".to_string(),
        }
        .into())
    }

    /// Lower one provided argument source to its value.
    pub(in crate::lower) fn lower_argument(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<mir::Value> {
        let expression = self.argument_expression(source)?;

        self.lower_expression(expression)
    }

    /// Return the value expression provided by one checked argument source.
    pub(in crate::lower) fn argument_expression(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // unwrap the provided value from argument nodes
        if let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>() {
            let value = match self.lowerer.source().tree().get(argument) {
                dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => *value,
                dir::Argument::Spread { .. } => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a spread argument".to_string(),
                    }
                    .into());
                }
                dir::Argument::Elision | dir::Argument::Error => {
                    return Err(CompilerError::Internal {
                        message: "checked DIR provided an empty argument".to_string(),
                    });
                }
            };

            return Ok(value);
        }

        // lower the source as the value expression itself
        let Ok(expression) = source.local_id.try_into_typed::<dir::Expression>() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked DIR provided a non-expression argument node {}",
                    source.local_id.id
                ),
            });
        };

        Ok(expression)
    }
}
