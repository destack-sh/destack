use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError};

use crate::lower::{
    FunctionLowerer, LocalStorage, access_for_storage_mutability, lower_mutability,
};

impl FunctionLowerer<'_> {
    /// Build one borrowed reference type in the given space.
    fn borrowed_reference_type(
        &mut self,
        pointee_type: mir::LocalNodeId<mir::Type>,
        access: mir::Access,
        space: mir::Space,
    ) -> mir::LocalNodeId<mir::Type> {
        self.state.builder.type_reference(
            mir::ReferenceKind::Borrowed,
            pointee_type,
            access,
            space,
            mir::Nullability::None,
        )
    }

    /// Return the space carried by one MIR reference type.
    fn reference_space(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        reference_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Space> {
        let mir::Type::Reference { space, .. } = self.state.builder.tree().get(reference_type)
        else {
            return Err(LowerError::Internal {
                anchor: (self.context.module_id).into(),
                module: self.context.module_id,
                message: format!("non-reference type in borrow lowering: {expression_id:?}"),
            }
            .into());
        };

        Ok(space.clone())
    }

    /// Return the space produced when borrowing one expression.
    fn borrow_space(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Space> {
        match self.context.dir_tree.get(expression) {
            // nested borrows preserve the inner storage space
            dir::Expression::Parenthesized { expression } => {
                self.borrow_space(expression_id, *expression)
            }

            // resolved paths borrow from their storage owner
            dir::Expression::Identifier { .. } | dir::Expression::QualifiedReference { .. } => {
                let target_symbol = self.resolve_expression_symbol(expression)?;
                if let Some(field) = self.capture_field_for_symbol(target_symbol) {
                    if field.mode == dir::CaptureMode::Borrow {
                        return self.reference_space(expression_id, field.ty);
                    }

                    return Ok(mir::Space::Local);
                }

                if let Some(binding) = self
                    .state
                    .bindings
                    .locals_by_symbol
                    .get(&target_symbol)
                    .copied()
                {
                    return match binding.storage {
                        LocalStorage::Local(_) => Ok(mir::Space::Frame),
                        LocalStorage::IndirectBinding { reference_type, .. } => {
                            self.reference_space(expression_id, reference_type)
                        }
                        LocalStorage::Variable(_) => {
                            if matches!(
                                self.state.builder.tree().get(binding.ty),
                                mir::Type::Reference { .. }
                            ) {
                                return self.reference_space(expression_id, binding.ty);
                            }

                            Err(LowerError::Internal {
                                anchor: (self.context.module_id).into(),
                                module: self.context.module_id,
                                message: "local borrow requires addressable storage".to_string(),
                            }
                            .into())
                        }
                    };
                }

                let global = self.global_binding_for_symbol(expression_id, target_symbol)?;

                Ok(global.space)
            }

            // 'this' follows the same storage rules as ordinary locals
            dir::Expression::This => self.this_borrow_space(expression_id),

            // field and element borrows preserve the aggregate storage space
            dir::Expression::Member { left, .. }
            | dir::Expression::PrivateMember { left, .. }
            | dir::Expression::Index { left, .. } => self.borrow_space(expression_id, *left),

            // rvalue borrows spill into a temporary local slot first
            _ => Ok(mir::Space::Frame),
        }
    }

    /// Return the space produced when borrowing `this`.
    fn this_borrow_space(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Space> {
        // direct method receiver storage
        if let Some(binding) = self.state.bindings.this_binding {
            return match binding.storage {
                LocalStorage::Local(_) => Ok(mir::Space::Frame),
                LocalStorage::IndirectBinding { reference_type, .. } => {
                    self.reference_space(expression_id, reference_type)
                }
                LocalStorage::Variable(_) => {
                    if matches!(
                        self.state.builder.tree().get(binding.ty),
                        mir::Type::Reference { .. }
                    ) {
                        return self.reference_space(expression_id, binding.ty);
                    }

                    Err(LowerError::Internal {
                        anchor: (self.context.module_id).into(),
                        module: self.context.module_id,
                        message: "this borrow requires addressable storage".to_string(),
                    }
                    .into())
                }
            };
        }

        // captured method receiver storage
        if let Some(this_symbol) = self.state.bindings.this_symbol
            && let Some(field) = self.capture_field_for_symbol(this_symbol)
        {
            if field.mode == dir::CaptureMode::Borrow {
                return self.reference_space(expression_id, field.ty);
            }

            return Ok(mir::Space::Local);
        }

        Err(LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(
                expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            ),
            message: "this reference outside of method context".to_string(),
        }
        .into())
    }

    /// Lower a borrow of `this` to a reference value.
    fn lower_reference_of_this_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        mutability: Option<dir::Mutability>,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // direct method receiver storage
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
                LocalStorage::Variable(_) => {
                    if matches!(
                        self.state.builder.tree().get(binding.ty),
                        mir::Type::Reference { .. }
                    ) {
                        let value = self.binding_value(binding);
                        return Ok((value, result_type));
                    }

                    Err(LowerError::Internal {
                        anchor: (self.context.module_id).into(),
                        module: self.context.module_id,
                        message: "this borrow requires addressable storage".to_string(),
                    }
                    .into())
                }
            };
        }

        // captured method receiver storage
        if let Some(this_symbol) = self.state.bindings.this_symbol
            && let Some(field) = self.capture_field_for_symbol(this_symbol)
        {
            return self.borrow_captured_binding(expression_id, &field, mutability);
        }

        Err(LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(
                expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            ),
            message: "this reference outside of method context".to_string(),
        }
        .into())
    }

    /// Lower a borrow expression to a reference value.
    ///
    /// ```ds
    /// function borrow(value: int32): ref<int32, borrowed, readonly> {
    ///     return &value;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: ref<int32, borrowed, readonly, space(frame)> = local.address v0
    ///     -> ref<int32, borrowed, readonly, space(frame)>
    /// ```
    pub(crate) fn lower_reference_of_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the reference result type
        let pointee_type = self.lower_type_for_expression(right)?;
        let access = mutability
            .map(lower_mutability)
            .map(access_for_storage_mutability)
            .unwrap_or(mir::Access::Mutable);
        let space = self.borrow_space(expression_id, right)?;
        let result_type = self.borrowed_reference_type(pointee_type, access, space);

        // lower the reference target to an address when possible
        match self.context.dir_tree.get(right) {
            dir::Expression::Parenthesized { expression } => {
                self.lower_reference_of_expression(expression_id, mutability, *expression)
            }
            dir::Expression::Identifier { .. } | dir::Expression::QualifiedReference { .. } => {
                let target_symbol = self.resolve_expression_symbol(right)?;
                if let Some(field) = self.capture_field_for_symbol(target_symbol) {
                    return self.borrow_captured_binding(expression_id, &field, mutability);
                }

                if let Some(binding) = self
                    .state
                    .bindings
                    .locals_by_symbol
                    .get(&target_symbol)
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
                            if matches!(
                                self.state.builder.tree().get(binding.ty),
                                mir::Type::Reference { .. }
                            ) {
                                let value = self.binding_value(binding);
                                return Ok((value, result_type));
                            }

                            return Err(LowerError::Internal {
                                anchor: (self.context.module_id).into(),
                                module: self.context.module_id,
                                message: "local borrow requires addressable storage".to_string(),
                            }
                            .into());
                        }
                    }
                }

                let global = self.global_binding_for_symbol(expression_id, target_symbol)?;
                let value = self.state.builder.global_addr(global.global, result_type);
                Ok((value, result_type))
            }
            dir::Expression::This => {
                self.lower_reference_of_this_expression(expression_id, mutability, result_type)
            }
            dir::Expression::Member { left, name }
            | dir::Expression::PrivateMember { left, name } => {
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
                            anchor: self.diagnostic_anchor(
                                expression_id
                                    .into_global_any(self.context.module_id)
                                    .into_anchored(Some(self.context.profile)),
                            ),
                            message: "missing member name".to_string(),
                        })
                        .map_err(CompilerError::from)?,
                        self.context.strings,
                        self.state.builder.tree(),
                    )
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "field not found in aggregate type".to_string(),
                    })
                    .map_err(CompilerError::from)?;

                // emit field.address
                let value =
                    self.state
                        .builder
                        .field_addr(aggregate_value, field_index as u32, result_type);
                Ok((value, result_type))
            }
            dir::Expression::Index { left, index, .. } => {
                let index = index
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "missing index expression".to_string(),
                    })
                    .map_err(CompilerError::from)?;

                // lower array and index expressions
                let (array_value, _) = self.lower_value_expression(*left)?;
                let (index_value, _) = self.lower_value_expression(index)?;

                // emit null checks when enabled
                let array_type = self.lower_type_for_expression(*left)?;
                self.emit_null_check(expression_id, array_value, array_type)?;

                // emit bounds checks when enabled
                self.emit_bounds_check(expression_id, array_value, array_type, index_value, index)?;

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
                let mir_mutability = mutability
                    .map(lower_mutability)
                    .unwrap_or(mir::Mutability::Mutable);
                let local = self.state.builder.local(value_type, mir_mutability);
                self.state.builder.local_set(local, value);
                let value = self.state.builder.local_addr(local, result_type);
                Ok((value, result_type))
            }
        }
    }

    /// Lower an ownership conversion.
    ///
    /// ```ds
    /// function own(value: int32): int32 {
    ///     return value;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v0: int32
    /// ```
    pub(crate) fn lower_value_of_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        _mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let (value, _) = self.lower_value_expression(right)?;
        let result_type = self.lower_type_for_expression(expression_id)?;

        Ok((value, result_type))
    }
}
