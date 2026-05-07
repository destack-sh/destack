use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError};

use crate::lower::{
    FunctionEnvironmentField, FunctionEnvironmentLayout, FunctionLowerer, lower_mutability,
};

impl FunctionLowerer<'_> {
    /// Resolve the function environment layout for this function when captured.
    pub(crate) fn function_environment_layout(&self) -> Option<&FunctionEnvironmentLayout> {
        self.context
            .function_environment_layouts
            .get(&self.context.symbol)
    }

    /// Resolve a captured field definition for a symbol.
    pub(crate) fn capture_field_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<FunctionEnvironmentField> {
        let layout = self.function_environment_layout()?;
        layout.field_for_symbol(symbol).copied()
    }

    /// Resolve the typed function environment pointer value.
    pub(crate) fn function_environment_value(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        self.state
            .bindings
            .environment
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "missing function environment".to_string(),
            })
            .map_err(CompilerError::from)
    }

    /// Build a function environment value for a target function symbol.
    pub(crate) fn build_function_environment_for_symbol(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // allocate and populate the environment when captures exist
        if let Some(env_layout) = self
            .context
            .function_environment_layouts
            .get(&target_symbol)
        {
            let env_ref_type = env_layout.env_pointer_type;
            let env_value = self.state.builder.new_(env_layout.env_type, env_ref_type);
            for field in &env_layout.fields {
                let field_addr_type = self.state.builder.type_reference(
                    mir::ReferenceKind::Managed,
                    field.ty,
                    mir::Mutability::Mutable,
                    mir::AddressSpace::Local,
                    false,
                );
                let field_addr =
                    self.state
                        .builder
                        .field_addr(env_value, field.index, field_addr_type);
                match field.mode {
                    dir::CaptureMode::Copy | dir::CaptureMode::Move => {
                        if self
                            .state
                            .bindings
                            .this_symbol
                            .is_some_and(|this_symbol| this_symbol == field.symbol)
                        {
                            let (value, _) = self.lower_this_expression(expression_id)?;
                            self.state.builder.store(field_addr, value);
                        } else {
                            let (value, _) =
                                self.lower_reference_expression(expression_id, field.symbol)?;
                            self.state.builder.store(field_addr, value);
                        }
                    }
                    dir::CaptureMode::Borrow => {
                        let reference_value =
                            self.reference_value_for_symbol(expression_id, field.symbol, field.ty)?;
                        self.state.builder.store(field_addr, reference_value);
                    }
                }
            }
            return Ok((env_value, env_ref_type));
        }

        // otherwise use the canonical empty function environment pointer
        let env_ref_type = self.context.empty_function_environment_pointer_type;
        let env_value = self.state.builder.null(env_ref_type);
        Ok((env_value, env_ref_type))
    }

    /// Resolve the address of a captured field inside the function environment.
    pub(crate) fn capture_field_addr(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        field: &FunctionEnvironmentField,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let env_value = self.function_environment_value(expression_id)?;
        let field_addr_type = self.state.builder.type_reference(
            mir::ReferenceKind::Managed,
            field.ty,
            mir::Mutability::Mutable,
            mir::AddressSpace::Local,
            false,
        );
        let addr = self
            .state
            .builder
            .field_addr(env_value, field.index, field_addr_type);
        Ok((addr, field_addr_type))
    }

    /// Load the value of a captured binding.
    pub(crate) fn captured_binding_value(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        field: &FunctionEnvironmentField,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let (field_addr, _) = self.capture_field_addr(expression_id, field)?;

        // copy or move: load the field directly
        if matches!(field.mode, dir::CaptureMode::Copy | dir::CaptureMode::Move) {
            let value = self.state.builder.load(field_addr, field.ty);
            return Ok((value, field.ty));
        }

        // borrow: load the stored pointer, then load the pointee
        let reference_value = self.state.builder.load(field_addr, field.ty);
        let pointee = match self.state.builder.tree().get(field.ty) {
            mir::Type::Reference { pointee, .. } => pointee
                .ty()
                .ok_or_else(|| LowerError::Internal {
                    anchor: (self.context.module_id).into(),
                    module: self.context.module_id,
                    message: "capture reference field pointee must be concrete".to_string(),
                })
                .map_err(CompilerError::from)?,
            _ => {
                return Err(LowerError::Internal {
                    anchor: (self.context.module_id).into(),
                    module: self.context.module_id,
                    message: "capture reference field missing reference type".to_string(),
                }
                .into());
            }
        };
        let value = self.state.builder.load(reference_value, pointee);
        Ok((value, pointee))
    }

    /// Store a value into a captured binding.
    pub(crate) fn store_captured_binding(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        field: &FunctionEnvironmentField,
        value: mir::Value,
    ) -> CompilerResult<()> {
        let (field_addr, _) = self.capture_field_addr(expression_id, field)?;

        // copy or move: store directly into the env field
        if matches!(field.mode, dir::CaptureMode::Copy | dir::CaptureMode::Move) {
            self.state.builder.store(field_addr, value);
            return Ok(());
        }

        // borrow: load the stored pointer, then store into it
        let reference_value = self.state.builder.load(field_addr, field.ty);
        self.state.builder.store(reference_value, value);
        Ok(())
    }

    /// Borrow a captured binding.
    pub(crate) fn borrow_captured_binding(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        field: &FunctionEnvironmentField,
        mutability: Option<dir::Mutability>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // borrow: forward the stored pointer
        if field.mode == dir::CaptureMode::Borrow {
            let (field_addr, _) = self.capture_field_addr(expression_id, field)?;
            let reference_value = self.state.builder.load(field_addr, field.ty);
            return Ok((reference_value, field.ty));
        }

        // copy or move: return a reference to the env field
        let (field_addr, field_addr_type) = self.capture_field_addr(expression_id, field)?;
        let mir_mutability = mutability
            .map(lower_mutability)
            .unwrap_or(mir::Mutability::Immutable);
        let result_type = self.state.builder.type_reference(
            mir::ReferenceKind::Managed,
            field.ty,
            mir_mutability,
            mir::AddressSpace::Local,
            false,
        );
        let value = if field_addr_type == result_type {
            field_addr
        } else {
            self.state
                .builder
                .cast(mir::CastOperator::Bitcast, field_addr, result_type)
        };
        Ok((value, result_type))
    }

    /// Record the function environment type on one function used as a closure value.
    pub(crate) fn set_function_environment(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        function_id: mir::LocalNodeId<mir::Function>,
        env_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<()> {
        // update the function metadata in the shared tree
        let function = self.state.builder.tree_mut().get_mut(function_id);
        match function.environment {
            Some(existing) if existing.ty() != Some(env_type) => {
                return Err(self
                    .error(expression_id, "mismatched function environment type")
                    .into());
            }
            Some(_) => {}
            None => {
                function.environment = Some(env_type.into());
            }
        }

        Ok(())
    }
}
