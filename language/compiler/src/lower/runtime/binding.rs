use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerResult, LowerError, LowerResult};

use crate::lower::{
    FieldInput, FieldLayoutKind, LayoutPolicy, ModuleLowerer, TypeLowerer, is_void_type,
    resolve_result_union,
};

/// Cached layout metadata for runtime status values.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeStatusLayout {
    /// MIR type id for RuntimeStatus.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
    /// Layout index for the status code field.
    pub(crate) code_field_index: u32,
    /// Layout index for the error id field.
    pub(crate) error_id_field_index: u32,
}

/// Result metadata for binding ABI lowering.
#[derive(Debug, Clone)]
pub(crate) struct BindingResultInfo {
    /// MIR type for the Ok value payload.
    pub(crate) ok_mir_type: mir::LocalNodeId<mir::Type>,
    /// Whether the Ok payload is void.
    pub(crate) ok_is_void: bool,
    /// Error value type id for the binding Result.
    pub(crate) err_value_type: dir::LocalTypeId,
}

impl ModuleLowerer<'_> {
    /// Resolve the binding result metadata for ABI lowering.
    pub(crate) fn binding_result_info(
        &mut self,
        signature: &dir::ResolvedSignature,
        expression_id: dir::LocalNodeId<dir::Expression>,
        _target_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<BindingResultInfo> {
        let Some(return_type_id) = signature.return_type else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "binding return type must be Result<T, PlatformError>".to_string(),
            }
            .into());
        };

        let result =
            resolve_result_union(self.types, &self.strings, return_type_id).ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "binding return type must be Result<T, PlatformError>".to_string(),
                }
            })?;

        let ok_is_void = is_void_type(self.types, result.ok_value_type);
        let ok_mir_type = if ok_is_void {
            self.type_lowerer.ty_void
        } else {
            self.lower_type(
                result.ok_value_type,
                expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
            )?
        };

        let platform_error_symbol = self.platform_error_symbol(expression_id)?;
        if !self.is_platform_error_type(result.err_value_type, platform_error_symbol) {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "binding error type must be PlatformError".to_string(),
            }
            .into());
        }

        Ok(BindingResultInfo {
            ok_mir_type,
            ok_is_void,
            err_value_type: result.err_value_type,
        })
    }

    /// Declare the takePlatformError binding for ABI lowering.
    pub(crate) fn declare_take_platform_error_function(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        err_value_type: dir::LocalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Function>> {
        if let Some(function_id) = self.take_platform_error_function {
            return Ok(function_id);
        }

        let symbol = self.take_platform_error_symbol(expression_id)?;
        if let Some(function_id) = self.function_for_symbol(symbol) {
            self.take_platform_error_function = Some(function_id);
            return Ok(function_id);
        }

        let anchor = expression_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));

        let binding = self
            .binding_name_for_symbol(expression_id, symbol)?
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "missing @binding decorator for takePlatformError".to_string(),
            })?;

        let err_mir_type = self.lower_type(err_value_type, anchor)?;
        let status_layout = self.runtime_status_layout(expression_id)?;
        let out_ptr_type = self.builder.type_reference(
            mir::ReferenceKind::Raw,
            err_mir_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Stack,
            false,
        );
        let error_id_type = self.builder.type_u64();
        let parameters = vec![out_ptr_type, error_id_type];

        let signature = self
            .builder
            .type_function_signature(parameters.clone(), status_layout.ty);
        let signature_type = self.builder.type_function_pointer(signature);
        self.assign_signature_metadata_name(signature_type, symbol, anchor)?;

        let function_id =
            self.builder
                .extern_function(&binding.name, &parameters, status_layout.ty);
        self.register_function_binding_for_symbol(symbol, function_id, signature)?;
        self.binding_symbols.insert(symbol);

        self.take_platform_error_function = Some(function_id);
        Ok(function_id)
    }

    /// Resolve (and cache) the RuntimeStatus layout for binding ABI lowering.
    pub(crate) fn runtime_status_layout(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<RuntimeStatusLayout> {
        if let Some(layout) = self.runtime_status_layout {
            return Ok(layout);
        }

        let code_name = self.builder.intern("code");
        let error_name = self.builder.intern("error_id");
        let code_type = self.builder.type_u32();
        let error_type = self.builder.type_u64();
        let anchor = expression_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));

        let (code_size, code_align) = self
            .type_lowerer
            .size_and_align_of_type(self.builder.tree().get(code_type), self.builder.tree())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "binding layout requires concrete nested types".to_string(),
            })?;
        let (error_size, error_align) = self
            .type_lowerer
            .size_and_align_of_type(self.builder.tree().get(error_type), self.builder.tree())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "binding layout requires concrete nested types".to_string(),
            })?;

        let layout = TypeLowerer::compute_struct_layout(
            vec![
                FieldInput {
                    name: code_name,
                    ty: code_type,
                    size: code_size,
                    alignment: code_align,
                    source_index: Some(0),
                    kind: FieldLayoutKind::Source,
                },
                FieldInput {
                    name: error_name,
                    ty: error_type,
                    size: error_size,
                    alignment: error_align,
                    source_index: Some(1),
                    kind: FieldLayoutKind::Source,
                },
            ],
            LayoutPolicy::C,
        );

        let mir_type = self
            .type_lowerer
            .create_struct_type(&layout, &mut self.builder);
        self.type_lowerer.set_layout(mir_type, layout.clone());
        self.insert_layout_entry(mir_type, mir::LayoutKind::Struct, &layout);

        let Some(code_field_index) = layout.field_index(code_name) else {
            return Err(LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: "RuntimeStatus layout missing code field".to_string(),
            }
            .into());
        };
        let Some(error_id_field_index) = layout.field_index(error_name) else {
            return Err(LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: "RuntimeStatus layout missing error id field".to_string(),
            }
            .into());
        };

        let layout = RuntimeStatusLayout {
            ty: mir_type,
            code_field_index,
            error_id_field_index,
        };
        self.runtime_status_layout = Some(layout);

        Ok(layout)
    }

    fn take_platform_error_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let symbol = self
            .declared_ambient_symbol("takePlatformError", dir::SymbolSpaceOrder::ValueThenType)?
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "takePlatformError binding is not available".to_string(),
            })?;
        Ok(symbol)
    }

    fn platform_error_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let symbol = self
            .declared_ambient_symbol("PlatformError", dir::SymbolSpaceOrder::ValueThenType)?
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "PlatformError type is not available".to_string(),
            })?;
        Ok(symbol)
    }

    fn is_platform_error_type(
        &self,
        type_id: dir::LocalTypeId,
        platform_error_symbol: dir::GlobalSymbolId,
    ) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.is_platform_error_type_inner(type_id, platform_error_symbol, &mut visited)
    }

    fn is_platform_error_type_inner(
        &self,
        type_id: dir::LocalTypeId,
        platform_error_symbol: dir::GlobalSymbolId,
        visited: &mut std::collections::HashSet<dir::LocalTypeId>,
    ) -> bool {
        if !visited.insert(type_id) {
            return false;
        }

        match self.types.get_type(type_id) {
            dir::Type::Reference { symbol, .. } => {
                if *symbol == platform_error_symbol {
                    return true;
                }
                if symbol.ty() == dir::SymbolType::TypeAlias
                    && let Some(target) = self.types.get_alias_target_type_id(*symbol)
                {
                    return self.is_platform_error_type_inner(
                        target,
                        platform_error_symbol,
                        visited,
                    );
                }
                if let Some(instance) = self.types.get_instance_type_id(*symbol) {
                    return self.is_platform_error_type_inner(
                        instance,
                        platform_error_symbol,
                        visited,
                    );
                }
                false
            }
            dir::Type::Value { value } => {
                self.is_platform_error_type_inner(*value, platform_error_symbol, visited)
            }
            _ => false,
        }
    }
}
