use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_> {
    /// Lower one call expression through its checked call resolution.
    pub(in crate::lower) fn lower_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        let resolution = self.lowerer.call_resolution(expression)?;

        match &resolution.target {
            // free(...)
            dir::CallTarget::Symbol(candidate) => {
                // receiver.method(...)
                if candidate.receiver.is_some() || !candidate.adjustments.is_empty() {
                    self.lower_method_call(&resolution)
                }
                // generic<T>(...)
                else if candidate.generic_scope.is_some()
                    || !candidate.generic_arguments.is_empty()
                {
                    self.lower_generic_call(&resolution)
                }
                // imported(...)
                else if candidate.symbol.module_id != self.lowerer.module {
                    self.lower_imported_call(&resolution)
                }
                // local(...)
                else {
                    self.lower_function_call(candidate.symbol.local_id, &resolution)
                }
            }
            // value(...)
            dir::CallTarget::Expression { .. } => self.lower_indirect_call(&resolution),
            // (a | b).method(...)
            dir::CallTarget::Universal(_) => self.lower_universal_call(&resolution),
            // reject operator targets: they never apply through call expressions
            dir::CallTarget::Builtin(_) => Err(CompilerError::Internal {
                message: "checked DIR selected a builtin operator for one call expression"
                    .to_string(),
            }),
        }
    }

    /// Lower one call to a function declared in this module.
    fn lower_function_call(
        &mut self,
        symbol: dir::LocalSymbolId,
        resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        // resolve the declared MIR function behind the symbol
        let Some(function) = self.lowerer.functions.get(&symbol).copied() else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a declared function behind one call symbol"
                    .to_string(),
            });
        };

        // lower the bound arguments in parameter order
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

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one method call through its receiver adjustments.
    fn lower_method_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a method call".to_string(),
        }
        .into())
    }

    /// Lower one call instantiating a generic callable.
    fn lower_generic_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a generic call".to_string(),
        }
        .into())
    }

    /// Lower one call to a function declared in another module.
    fn lower_imported_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a call into another module".to_string(),
        }
        .into())
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
    fn lower_argument(&mut self, source: dir::GlobalNodeIdAny) -> CompilerResult<mir::Value> {
        // unwrap the provided value from argument nodes
        if let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>() {
            let value = match self.lowerer.tree.get(argument) {
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

            return self.lower_expression(value);
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

        self.lower_expression(expression)
    }
}
