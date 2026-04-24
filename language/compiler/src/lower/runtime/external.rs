use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult, RequirementError};

use crate::lower::ModuleLowerer;

/// Visitor that collects call expressions from a subtree.
struct ExternalCallCollector {
    /// Call expressions found in the subtree.
    calls: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Options for the node visitor.
    options: dir::NodeVisitorOptions,
}

impl ExternalCallCollector {
    /// Create a new collector.
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            options: dir::NodeVisitorOptions::default(),
        }
    }
}

impl dir::NodeVisitor for ExternalCallCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // record direct call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.calls.push(id);
        }

        // continue walking the subtree
        destack_core::ensure_sufficient_stack(|| dir::walk_expression(self, tree, id, expression));
    }
}

pub(crate) struct BindingResolution {
    pub(crate) name: String,
    pub(crate) is_binding: bool,
}

impl ModuleLowerer<'_> {
    /// Declare call targets referenced by an expression subtree.
    pub(crate) fn declare_call_targets_for_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<()> {
        // collect call expressions from the subtree
        let mut collector = ExternalCallCollector::new();
        let expression = self.dir_tree.get(expression_id);
        dir::NodeVisitor::visit_expression(
            &mut collector,
            self.dir_tree,
            expression_id,
            expression,
        );

        // declare each required call target lazily for this subtree
        for call_id in collector.calls {
            self.declare_call_target(call_id)?;
        }

        Ok(())
    }

    /// Declare the function referenced by a call, when needed.
    fn declare_call_target(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<()> {
        // resolve the static call candidate
        let node_id = expression_id.into_global_any(self.module_id);
        let Some(resolution_id) = self.types.get_resolution_for_node(node_id) else {
            return Ok(());
        };
        let resolution = self.types.get_resolution(resolution_id);
        let dir::Resolution::Static { candidate, .. } = resolution else {
            return Ok(());
        };

        // skip local targets and already declared symbols
        let target_symbol = candidate.target_symbol;
        if target_symbol.module_id == self.module_id {
            return Ok(());
        }
        if self.function_for_symbol(target_symbol).is_some() {
            return Ok(());
        }

        // skip intrinsic bindings, they are lowered directly
        if self
            .resolve_intrinsic_binding_name_id(target_symbol)?
            .is_some()
        {
            return Ok(());
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

        // declare the call target on demand
        self.declare_external_function(expression_id, target_symbol, signature)?;

        Ok(())
    }

    /// Declare the MIR function for an external call target.
    fn declare_external_function(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
        signature: &dir::ResolvedSignature,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        // return the existing declaration when present
        if let Some(function_id) = self.function_for_symbol(target_symbol) {
            return Ok(function_id);
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
        let mut parameter_types = Vec::with_capacity(signature.parameters.len());
        for type_id in &signature.parameters {
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

            self.declare_take_platform_error_function(expression_id, binding_info.err_value_type)?;

            if !binding_info.ok_is_void {
                let out_pointer = self.builder.type_reference(
                    mir::ReferenceKind::Raw,
                    binding_info.ok_mir_type,
                    mir::Mutability::Mutable,
                    mir::AddressSpace::Stack,
                    false,
                );
                abi_parameters.push(out_pointer);
            }
            abi_parameters.extend(parameter_types);

            let signature = self
                .builder
                .type_function_signature(abi_parameters.clone(), abi_info.ty);
            let signature_type = self.builder.type_function_pointer(signature);
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
        let signature = self
            .builder
            .type_function_signature(parameter_types.clone(), return_type);
        let signature_type = self.builder.type_function_pointer(signature);
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
        expression_id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> LowerResult<Option<BindingResolution>> {
        // require analysis for the referenced module
        self.require_analyzed_module(symbol.module_id)?;

        // read the symbol entry and binding metadata
        let dir = self.require_analyzed_dir_data(symbol.module_id)?;
        let symbol_entry = dir.symbols.get_symbol(symbol.local_id);
        if let Some(binding) = symbol_entry.decorators.binding.as_ref()
            && let Some(name) = binding.name
        {
            return Ok(Some(BindingResolution {
                name: self.compiler.repository.strings.get(name).to_string(),
                is_binding: true,
            }));
        }
        if let Some(binding) = symbol_entry.decorators.extern_binding.as_ref()
            && let Some(name) = binding.name
        {
            return Ok(Some(BindingResolution {
                name: self.compiler.repository.strings.get(name).to_string(),
                is_binding: false,
            }));
        }

        // fall back to the symbol name
        let default_name = symbol_entry
            .name()
            .map(|name| self.compiler.repository.strings.get(name).to_string())
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
        let result =
            self.compiler
                .require_dir_analyzed(self.context.revision(), module_id, self.profile);
        let Err(error) = result else {
            return Ok(());
        };

        // map task errors to lowering diagnostics
        match error {
            RequirementError::NotReady { requirement } => Err(LowerError::Yield { requirement }),
            RequirementError::Failed { requirement } => {
                Err(LowerError::UnsatisfiedRequirement { requirement })
            }
        }
    }
}
