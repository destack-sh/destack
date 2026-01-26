use destack_dir::{GlobalSymbolId, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::item::lower_mutability;
use crate::lower::table::{ClosureEnvField, ClosureEnvLayout};

impl FunctionContext<'_> {
    /// Resolve the closure environment layout for this function when captured.
    pub(crate) fn closure_env_layout(&self) -> Option<&ClosureEnvLayout> {
        self.env.closure_env_layouts.get(&self.env.symbol)
    }

    /// Resolve a captured field definition for a symbol.
    pub(crate) fn capture_field_for_symbol(
        &self,
        symbol: GlobalSymbolId,
    ) -> Option<ClosureEnvField> {
        let layout = self.closure_env_layout()?;
        layout.field_for_symbol(symbol).copied()
    }

    /// Resolve the typed closure environment pointer value.
    pub(crate) fn closure_env_value(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<mir::Value> {
        self.state
            .bindings
            .closure_env
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "missing closure environment".to_string(),
            })
    }

    /// Resolve the address of a captured field inside the closure environment.
    pub(crate) fn capture_field_addr(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        field: &ClosureEnvField,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let env_value = self.closure_env_value(expression_id)?;
        let field_addr_type = self.state.builder.type_reference(
            mir::ReferenceKind::Managed,
            field.ty,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
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
        field: &ClosureEnvField,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let (field_addr, _) = self.capture_field_addr(expression_id, field)?;

        // by value or move: load the field directly
        if matches!(
            field.kind,
            dir::CaptureKind::ByValue | dir::CaptureKind::ByMove
        ) {
            let value = self.state.builder.load(field_addr, field.ty);
            return Ok((value, field.ty));
        }

        // by reference: load the stored pointer, then load the pointee
        let reference_value = self.state.builder.load(field_addr, field.ty);
        let pointee = match self.state.builder.tree().get(field.ty) {
            mir::Type::Reference { pointee, .. } => *pointee,
            _ => {
                return Err(LowerError::Internal {
                    module: self.env.module_id,
                    message: "capture reference field missing reference type".to_string(),
                });
            }
        };
        let value = self.state.builder.load(reference_value, pointee);
        Ok((value, pointee))
    }

    /// Store a value into a captured binding.
    pub(crate) fn store_captured_binding(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        field: &ClosureEnvField,
        value: mir::Value,
    ) -> LowerResult<()> {
        let (field_addr, _) = self.capture_field_addr(expression_id, field)?;

        // by value or move: store directly into the env field
        if matches!(
            field.kind,
            dir::CaptureKind::ByValue | dir::CaptureKind::ByMove
        ) {
            self.state.builder.store(field_addr, value);
            return Ok(());
        }

        // by reference: load the stored pointer, then store into it
        let reference_value = self.state.builder.load(field_addr, field.ty);
        self.state.builder.store(reference_value, value);
        Ok(())
    }

    /// Borrow a captured binding.
    pub(crate) fn borrow_captured_binding(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        field: &ClosureEnvField,
        mutability: Option<dir::Mutability>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // by reference: forward the stored pointer
        if field.kind == dir::CaptureKind::ByReference {
            let (field_addr, _) = self.capture_field_addr(expression_id, field)?;
            let reference_value = self.state.builder.load(field_addr, field.ty);
            return Ok((reference_value, field.ty));
        }

        // by value or move: return a reference to the env field
        let (field_addr, field_addr_type) = self.capture_field_addr(expression_id, field)?;
        let mir_mutability = mutability
            .map(lower_mutability)
            .unwrap_or(mir::Mutability::Immutable);
        let result_type = self.state.builder.type_reference(
            mir::ReferenceKind::Managed,
            field.ty,
            mir_mutability,
            mir::AddressSpace::Generic,
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

    /// Record the closure env type on a function used as a closure value.
    pub(crate) fn set_closure_env_type(
        &mut self,
        expression_id: LocalNodeId<dir::Expression>,
        function_id: mir::LocalNodeId<mir::Function>,
        env_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<()> {
        // update the function metadata in the shared node tree
        let function = self.state.builder.tree_mut().get_mut(function_id);
        match function.closure_env_type {
            Some(existing) if existing != env_type => {
                return Err(self.error(expression_id, "mismatched closure env type"));
            }
            Some(_) => {}
            None => {
                function.closure_env_type = Some(env_type);
            }
        }

        Ok(())
    }
}
