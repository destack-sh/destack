use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerResult, LowerError};

use crate::lower::{FunctionLowerer, LocalBinding, LocalStorage};

impl FunctionLowerer<'_> {
    /// Resolve a reference value for a symbol, upgrading locals to addressable storage.
    pub(crate) fn reference_value_for_symbol(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // prefer local bindings
        if let Some(binding) = self.state.bindings.locals_by_symbol.get(&symbol).copied() {
            let (value, updated) = self.reference_value_for_binding(binding, result_type)?;
            if let Some(updated) = updated {
                self.state.bindings.locals_by_symbol.insert(symbol, updated);
            }
            return Ok(value);
        }

        // resolve `this` when the symbol matches
        if self.symbol_is_this(symbol)
            && let Some(binding) = self.state.bindings.this_binding
        {
            let (value, updated) = self.reference_value_for_binding(binding, result_type)?;
            if let Some(updated) = updated {
                self.state.bindings.this_binding = Some(updated);
            }
            return Ok(value);
        }

        // fall back to globals
        let global = self.global_binding_for_symbol(expression_id, symbol)?;
        let addr_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            global.ty,
            global.mutability,
            global.space.clone(),
            false,
        );
        let addr = self.state.builder.global_addr(global.global, addr_type);
        if addr_type == result_type {
            return Ok(addr);
        }

        Ok(self
            .state
            .builder
            .cast(mir::CastOperator::Bitcast, addr, result_type))
    }

    /// Bind the implicit `this` parameter to a local binding.
    pub(crate) fn bind_this_parameter(
        &mut self,
        node_id: dir::LocalNodeIdAny,
        symbol: Option<dir::GlobalSymbolId>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<()> {
        // reject duplicate bindings when a symbol is provided
        if let Some(symbol) = symbol
            && self.state.bindings.locals_by_symbol.contains_key(&symbol)
        {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    node_id.into_anchored(self.context.module_id, Some(self.context.profile)),
                ),
                message: "duplicate this binding".to_string(),
            }
            .into());
        }

        // create a binding from the implicit parameter value
        let value = self.state.builder.function_parameter(0);
        let binding = if self.this_needs_addressable_local() {
            let local = self.state.builder.local(ty, mir::Mutability::Immutable);
            self.state.builder.local_set(local, value);
            LocalBinding::local(local, ty)
        } else {
            let variable = self.state.builder.variable(ty);
            self.state.builder.define_variable(variable, value);
            LocalBinding::variable(variable, ty)
        };

        // record the binding for `this`
        self.state.bindings.this_binding = Some(binding);
        if let Some(symbol) = symbol {
            self.state.bindings.locals_by_symbol.insert(symbol, binding);
        }

        Ok(())
    }

    /// Resolve a reference value for a local binding, upgrading storage when needed.
    fn reference_value_for_binding(
        &mut self,
        binding: LocalBinding,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(mir::Value, Option<LocalBinding>)> {
        match binding.storage {
            // addressable locals are ready to use
            LocalStorage::Local(local) => {
                let value = self.state.builder.local_addr(local, result_type);
                Ok((value, None))
            }
            // boxed bindings store the reference directly
            LocalStorage::IndirectBinding {
                variable,
                reference_type,
            } => {
                let value = self.state.builder.use_variable(variable);
                if reference_type == result_type {
                    return Ok((value, None));
                }
                let cast = self
                    .state
                    .builder
                    .cast(mir::CastOperator::Bitcast, value, result_type);
                Ok((cast, None))
            }
            // upgrade SSA bindings to addressable storage
            LocalStorage::Variable(_) => {
                let current = self.binding_value(binding);
                let local = self
                    .state
                    .builder
                    .local(binding.ty, mir::Mutability::Mutable);
                self.state.builder.local_set(local, current);
                let updated = LocalBinding::local(local, binding.ty);
                let value = self.state.builder.local_addr(local, result_type);
                Ok((value, Some(updated)))
            }
        }
    }

    /// Check whether a symbol represents `this`.
    fn symbol_is_this(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.state
            .bindings
            .this_symbol
            .is_some_and(|this_symbol| this_symbol == symbol)
    }

    /// Check whether a receiver expression resolves to a namespace symbol.
    pub(crate) fn receiver_is_namespace_reference(
        &self,
        receiver_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let dir::Expression::Path { .. } = self.context.dir_tree.get(receiver_id) else {
            return false;
        };
        let Ok(symbol) = self.resolve_expression_symbol(receiver_id) else {
            return false;
        };

        let Some(dir) = self.artifact_dir_data_if_present(symbol.module_id) else {
            return false;
        };

        dir.bindings.get_symbol(symbol.local_id).role == dir::SymbolRole::Namespace
    }

    /// Check whether a symbol requires an addressable local.
    pub(crate) fn symbol_needs_addressable_local(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.state.bindings.address_taken_locals.contains(&symbol)
    }

    /// Check whether a symbol requires boxed capture storage.
    pub(crate) fn symbol_needs_reference_cell(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.state.bindings.reference_bindings.contains(&symbol)
    }

    /// Check whether `this` requires an addressable local.
    pub(crate) fn this_needs_addressable_local(&self) -> bool {
        self.state.bindings.takes_this_address
    }

    /// Load a local binding value.
    pub(crate) fn binding_value(&mut self, binding: LocalBinding) -> mir::Value {
        match binding.storage {
            LocalStorage::Variable(variable) => self.state.builder.use_variable(variable),
            LocalStorage::Local(local) => self.state.builder.local_get(local),
            LocalStorage::IndirectBinding { variable, .. } => {
                let reference_value = self.state.builder.use_variable(variable);
                self.state.builder.load(reference_value, binding.ty)
            }
        }
    }

    /// Assign a new value to a local binding.
    pub(crate) fn set_binding_value(&mut self, binding: LocalBinding, value: mir::Value) {
        match binding.storage {
            LocalStorage::Variable(variable) => {
                self.state.builder.define_variable(variable, value);
            }
            LocalStorage::Local(local) => {
                self.state.builder.local_set(local, value);
            }
            LocalStorage::IndirectBinding { variable, .. } => {
                let reference = self.state.builder.use_variable(variable);
                self.state.builder.store(reference, value);
            }
        }
    }
}
