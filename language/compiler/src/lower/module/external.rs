use destack_dir as dir;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use destack_source::ModuleId;

use crate::{LowerError, LowerResult, TaskDependencyError};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Predeclare external functions referenced by this module.
    pub(crate) fn predeclare_external_calls(&mut self) -> LowerResult<()> {
        // scan call expressions for external targets
        for (expression_id, expression) in self.dir_tree.iter_nodes_of_type::<dir::Expression>() {
            // skip non call expressions
            let dir::Expression::Call { .. } = expression else {
                continue;
            };

            // resolve the static call candidate
            let node_id = expression_id.into_global_any(self.module_id);
            let Some(resolution_id) = self.types.get_resolution_for_node(node_id) else {
                continue;
            };
            let resolution = self.types.get_resolution(resolution_id);
            let dir::Resolution::Static { candidate, .. } = resolution else {
                continue;
            };

            // skip local targets and already declared symbols
            let target_symbol = candidate.target_symbol;
            if target_symbol.module_id == self.module_id {
                continue;
            }
            if self.functions_by_symbol.contains_key(&target_symbol) {
                continue;
            }

            // require a resolved signature
            let Some(signature) = candidate.resolved_signature.as_ref() else {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "missing resolved signature for external call".to_string(),
                });
            };

            // declare the external function
            self.ensure_external_function(expression_id, target_symbol, signature)?;
        }

        // return once all externs are declared
        Ok(())
    }

    /// Ensure an external function is declared for a call target.
    fn ensure_external_function(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        signature: &dir::ResolvedSignature,
    ) -> LowerResult<()> {
        // skip already declared externs
        if self.functions_by_symbol.contains_key(&target_symbol) {
            return Ok(());
        }

        // resolve the extern symbol name
        let extern_name = self
            .extern_name_for_symbol(expression_id, target_symbol)?
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "missing @extern binding for call target".to_string(),
            })?;

        // anchor diagnostic spans for type lowering
        let anchor = expression_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));

        // lower parameter types
        let mut parameter_types = Vec::with_capacity(signature.dynamic_parameters.len());
        for type_id in &signature.dynamic_parameters {
            // lower the parameter type
            let parameter_type = self.type_lowerer.lower_type(
                self.types,
                *type_id,
                self.module_id,
                anchor,
                &mut self.builder,
            )?;
            parameter_types.push(parameter_type);
        }

        // lower the return type
        let return_type = match signature.return_type {
            Some(return_type) => self.type_lowerer.lower_type(
                self.types,
                return_type,
                self.module_id,
                anchor,
                &mut self.builder,
            )?,
            None => self.type_lowerer.ty_void,
        };

        // create a signature type for direct callsites
        let signature_type = self
            .builder
            .type_function_pointer(parameter_types.clone(), return_type);
        // attach a metadata name for the signature type
        self.assign_signature_metadata_name(signature_type, target_symbol, anchor)?;

        // declare the extern function
        let function_id = self
            .builder
            .extern_function(&extern_name, &parameter_types, return_type);
        self.functions_by_symbol.insert(target_symbol, function_id);
        self.function_signature_types
            .insert(function_id, signature_type);

        // return after declaration
        Ok(())
    }

    /// Resolve the extern binding name for a symbol, if any.
    fn extern_name_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        symbol: GlobalSymbolId,
    ) -> LowerResult<Option<String>> {
        // require analysis for the referenced module
        self.require_analyzed_module(symbol.module_id)?;

        // read the symbol entry and extern binding
        let module = self.compiler.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(self.profile);
        let symbols = dir.symbols.read();
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let Some(binding) = symbol_entry.decorators.extern_binding.as_ref() else {
            return Ok(None);
        };

        // prefer the explicit extern binding name
        if let Some(name) = binding.name {
            return Ok(Some(self.compiler.program.strings.get(name).to_string()));
        }

        // fall back to the symbol name
        let default_name = symbol_entry
            .name()
            .map(|name| self.compiler.program.strings.get(name).to_string())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "extern symbol is missing a name".to_string(),
            })?;

        // return the resolved binding
        Ok(Some(default_name))
    }

    /// Ensure the module has been analyzed for this profile.
    fn require_analyzed_module(&self, module_id: ModuleId) -> LowerResult<()> {
        // request analysis for the target module
        let result = self
            .compiler
            .require_analyze_module(module_id, self.profile);
        let Err(error) = result else {
            return Ok(());
        };

        // map task errors to lowering diagnostics
        match error {
            TaskDependencyError::NotReady { dependency } => Err(LowerError::Yield { dependency }),
            TaskDependencyError::Failed { dependency } => {
                Err(LowerError::UnsatisfiedDependency { dependency })
            }
        }
    }
}
