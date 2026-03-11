use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use crate::{BuildRequirementError, LowerError, LowerResult};

use crate::lower::ModuleLowerer;

pub(crate) struct BindingResolution {
    pub(crate) name: String,
    pub(crate) is_binding: bool,
}

impl ModuleLowerer<'_> {
    /// Declare external functions referenced by this module.
    pub(crate) fn lower_external_calls(&mut self) -> LowerResult<()> {
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

            // skip intrinsic bindings, they are lowered directly
            if self
                .resolve_intrinsic_binding_name_id(target_symbol)?
                .is_some()
            {
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
            self.external_function_for_symbol(expression_id, target_symbol, signature)?;
        }

        // return once all externs are declared
        Ok(())
    }

    /// Return the external function declared for a call target.
    fn external_function_for_symbol(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        signature: &dir::ResolvedSignature,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        // skip already declared externs
        if let Some(function_id) = self.functions_by_symbol.get(&target_symbol) {
            return Ok(*function_id);
        }

        // resolve the extern/binding symbol name
        let binding = self
            .binding_name_for_symbol(expression_id, target_symbol)?
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "missing @binding or @extern decorator for call target".to_string(),
            })?;
        let extern_name = binding.name;

        // anchor diagnostic spans for type lowering
        let anchor = expression_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));

        // lower parameter types
        let mut parameter_types = Vec::with_capacity(signature.dynamic_parameters.len());
        for type_id in &signature.dynamic_parameters {
            // lower the parameter type
            let parameter_type = self.lower_type(*type_id, anchor)?;
            parameter_types.push(parameter_type);
        }

        // lower the return type
        let return_type = match signature.return_type {
            Some(return_type) => self.lower_type(return_type, anchor)?,
            None => self.type_lowerer.ty_void,
        };

        if binding.is_binding && self.binding_abi_lowering {
            let binding_info = self.binding_result_info(signature, expression_id, target_symbol)?;
            let abi_info = self.runtime_status_layout(expression_id)?;
            let mut abi_parameters = Vec::with_capacity(parameter_types.len() + 1);

            self.ensure_take_platform_error_function(expression_id, binding_info.err_value_type)?;

            if !binding_info.ok_is_void {
                let out_pointer = self.builder.type_reference(
                    mir::ReferenceKind::Raw,
                    binding_info.ok_mir_type,
                    mir::Mutability::Mutable,
                    mir::AddressSpace::Generic,
                    false,
                );
                abi_parameters.push(out_pointer);
            }
            abi_parameters.extend(parameter_types);

            let signature_type = self
                .builder
                .type_function_pointer(abi_parameters.clone(), abi_info.ty);
            self.assign_signature_metadata_name(signature_type, target_symbol, anchor)?;

            let function_id =
                self.builder
                    .extern_function(&extern_name, &abi_parameters, abi_info.ty);
            self.register_function_binding_for_symbol(target_symbol, function_id, signature_type)?;
            self.binding_symbols.insert(target_symbol);
            if extern_name == "destack.error.takePlatformError" {
                self.take_platform_error_function = Some(function_id);
            }

            return Ok(function_id);
        }

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
        // register function binding
        self.register_function_binding_for_symbol(target_symbol, function_id, signature_type)?;
        if binding.is_binding {
            self.binding_symbols.insert(target_symbol);
        }

        // return after declaration
        Ok(function_id)
    }

    /// Resolve the extern or binding name for a symbol, if any.
    pub(crate) fn binding_name_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        symbol: GlobalSymbolId,
    ) -> LowerResult<Option<BindingResolution>> {
        // require analysis for the referenced module
        self.require_analyzed_module(symbol.module_id)?;

        // read the symbol entry and binding metadata
        let module = self.compiler.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(self.profile);
        let symbols = dir.symbols.read();
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        if let Some(binding) = symbol_entry.decorators.binding.as_ref()
            && let Some(name) = binding.name
        {
            return Ok(Some(BindingResolution {
                name: self.compiler.program.strings.get(name).to_string(),
                is_binding: true,
            }));
        }
        if let Some(binding) = symbol_entry.decorators.extern_binding.as_ref()
            && let Some(name) = binding.name
        {
            return Ok(Some(BindingResolution {
                name: self.compiler.program.strings.get(name).to_string(),
                is_binding: false,
            }));
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

        let is_binding = symbol_entry.decorators.binding.is_some();

        // return the resolved binding
        Ok(Some(BindingResolution {
            name: default_name,
            is_binding,
        }))
    }

    /// Ensure the module has been analyzed for this profile.
    pub(crate) fn require_analyzed_module(&self, module_id: ModuleId) -> LowerResult<()> {
        // request analysis for the target module
        let result = self.compiler.require_dir_analyzed(module_id, self.profile);
        let Err(error) = result else {
            return Ok(());
        };

        // map task errors to lowering diagnostics
        match error {
            BuildRequirementError::NotReady { requirement } => {
                Err(LowerError::Yield { requirement })
            }
            BuildRequirementError::Failed { requirement } => {
                Err(LowerError::UnsatisfiedRequirement { requirement })
            }
        }
    }
}
