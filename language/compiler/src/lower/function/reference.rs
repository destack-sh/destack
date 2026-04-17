use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::{FunctionLowerer, LocalStorage, lower_mutability};

impl FunctionLowerer<'_> {
    /// Lower a borrow expression to a reference value.
    ///
    /// ```ds
    /// function borrow(value: int32): ref<int32, borrowed, readonly> {
    ///     return &value;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: ref<int32, borrowed, readonly> = local.address v0 -> ref<int32, borrowed, readonly>
    /// ```
    pub(crate) fn lower_reference_of_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the reference result type
        let pointee_type = self.lower_type_for_expression(right)?;
        let mir_mutability = mutability
            .map(lower_mutability)
            .unwrap_or(mir::Mutability::Mutable);
        let result_type = self.state.builder.type_reference(
            mir::ReferenceKind::Borrowed,
            pointee_type,
            mir_mutability,
            mir::AddressSpace::Generic,
            false,
        );

        // lower the reference target to an address when possible
        match self.context.dir_tree.get(right) {
            dir::Expression::Parenthesized { expression } => {
                self.lower_reference_of_expression(expression_id, mutability, *expression)
            }
            dir::Expression::LocalReference { target_symbol, .. } => {
                if let Some(field) = self.capture_field_for_symbol(*target_symbol) {
                    return self.borrow_captured_binding(expression_id, &field, mutability);
                }

                let binding = self.local_binding_for_symbol(right, *target_symbol)?;
                match binding.storage {
                    LocalStorage::Local(local) => {
                        let value = self.state.builder.local_addr(local, result_type);
                        Ok((value, result_type))
                    }
                    LocalStorage::IndirectBinding {
                        variable,
                        reference_type,
                    } => {
                        let reference_value = self.state.builder.use_variable(variable);
                        let value = if reference_type == result_type {
                            reference_value
                        } else {
                            self.state.builder.cast(
                                mir::CastOperator::Bitcast,
                                reference_value,
                                result_type,
                            )
                        };
                        Ok((value, result_type))
                    }
                    LocalStorage::Variable(_) => Err(LowerError::Internal {
                        module: self.context.module_id,
                        message: "local borrow requires addressable storage".to_string(),
                    }),
                }
            }
            dir::Expression::This => {
                if let Some(binding) = self.state.bindings.this_binding {
                    return match binding.storage {
                        LocalStorage::Local(local) => {
                            let value = self.state.builder.local_addr(local, result_type);
                            Ok((value, result_type))
                        }
                        LocalStorage::IndirectBinding {
                            variable,
                            reference_type,
                        } => {
                            let reference_value = self.state.builder.use_variable(variable);
                            let value = if reference_type == result_type {
                                reference_value
                            } else {
                                self.state.builder.cast(
                                    mir::CastOperator::Bitcast,
                                    reference_value,
                                    result_type,
                                )
                            };
                            Ok((value, result_type))
                        }
                        LocalStorage::Variable(_) => Err(LowerError::Internal {
                            module: self.context.module_id,
                            message: "this borrow requires addressable storage".to_string(),
                        }),
                    };
                }

                if let Some(this_symbol) = self.state.bindings.this_symbol
                    && let Some(field) = self.capture_field_for_symbol(this_symbol)
                {
                    return self.borrow_captured_binding(expression_id, &field, mutability);
                }

                Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "this reference outside of method context".to_string(),
                })
            }
            dir::Expression::ModuleReference { target_symbol, .. }
            | dir::Expression::GlobalReference { target_symbol, .. } => {
                if let Some(binding) = self
                    .state
                    .bindings
                    .locals_by_symbol
                    .get(target_symbol)
                    .copied()
                {
                    match binding.storage {
                        LocalStorage::Local(local) => {
                            let value = self.state.builder.local_addr(local, result_type);
                            return Ok((value, result_type));
                        }
                        LocalStorage::IndirectBinding {
                            variable,
                            reference_type,
                        } => {
                            let reference_value = self.state.builder.use_variable(variable);
                            let value = if reference_type == result_type {
                                reference_value
                            } else {
                                self.state.builder.cast(
                                    mir::CastOperator::Bitcast,
                                    reference_value,
                                    result_type,
                                )
                            };
                            return Ok((value, result_type));
                        }
                        LocalStorage::Variable(_) => {
                            return Err(LowerError::Internal {
                                module: self.context.module_id,
                                message: "local borrow requires addressable storage".to_string(),
                            });
                        }
                    }
                }

                let global = self.global_binding_for_symbol(expression_id, *target_symbol)?;
                let value = self.state.builder.global_addr(global.global, result_type);
                Ok((value, result_type))
            }
            dir::Expression::Member {
                left,
                name,
                generic_arguments,
            }
            | dir::Expression::PrivateMember {
                left,
                name,
                generic_arguments,
            } => {
                // reject generic arguments on member borrows
                if !generic_arguments.is_empty() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "generic arguments on member borrows are not supported"
                            .to_string(),
                    });
                }

                // lower the aggregate value
                let (aggregate_value, aggregate_type) = self.lower_value_expression(*left)?;

                // emit null checks when enabled
                self.emit_null_check(expression_id, aggregate_value, aggregate_type)?;

                // resolve field index through the type lowerer
                let field_index = self
                    .context
                    .type_lowerer
                    .field_index_for_type(
                        aggregate_type,
                        name.ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                            message: "missing member name".to_string(),
                        })?,
                        self.context.strings,
                        self.state.builder.tree(),
                    )
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "field not found in aggregate type".to_string(),
                    })?;

                // emit field.address
                let value =
                    self.state
                        .builder
                        .field_addr(aggregate_value, field_index as u32, result_type);
                Ok((value, result_type))
            }
            dir::Expression::Index { left, right } => {
                let index_expr = right.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "missing index expression".to_string(),
                })?;

                // lower array and index expressions
                let (array_value, _) = self.lower_value_expression(*left)?;
                let (index_value, _) = self.lower_value_expression(index_expr)?;

                // emit null checks when enabled
                let array_type = self.lower_type_for_expression(*left)?;
                self.emit_null_check(expression_id, array_value, array_type)?;

                // emit bounds checks when enabled
                self.emit_bounds_check(
                    expression_id,
                    array_value,
                    array_type,
                    index_value,
                    index_expr,
                )?;

                // emit element.address
                let value = self
                    .state
                    .builder
                    .element_addr(array_value, index_value, result_type);
                Ok((value, result_type))
            }
            _ => {
                // lower rvalue borrows by spilling into a temporary
                let (value, value_type) = self.lower_value_expression(right)?;
                let local = self.state.builder.local(value_type, mir_mutability);
                self.state.builder.local_set(local, value);
                let value = self.state.builder.local_addr(local, result_type);
                Ok((value, result_type))
            }
        }
    }

    /// Lower an ownership conversion to an owning handle.
    ///
    /// ```ds
    /// function own(value: int32): ref<int32, owned, readonly> {
    ///     return value;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: ref<int32, owned, readonly> = raw.alloc int32
    /// store v1, v0
    /// ```
    pub(crate) fn lower_value_of_expression(
        &mut self,
        _expression_id: dir::LocalNodeId<dir::Expression>,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the owned value expression
        let (value, pointee_type) = self.lower_value_expression(right)?;

        // resolve the owning handle type
        let mutability = mutability
            .map(lower_mutability)
            .unwrap_or(mir::Mutability::Immutable);
        let result_type = self.state.builder.type_reference(
            mir::ReferenceKind::Owned,
            pointee_type,
            mutability,
            mir::AddressSpace::Generic,
            false,
        );

        // allocate owned storage and store the value
        let pointer = self.state.builder.raw_alloc(pointee_type, result_type);
        self.state.builder.store(pointer, value);

        Ok((pointer, result_type))
    }
}
