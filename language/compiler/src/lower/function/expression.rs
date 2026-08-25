use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

/// One value in flight along a coercion path.
enum CoercionValue {
    /// One source expression that has not been evaluated.
    Expression(dir::LocalNodeId<dir::Expression>),
    /// One materialized value.
    Runtime(mir::Value),
    /// One compile-time scalar literal.
    Literal(dir::Literal),
    /// The unmaterialized null value.
    Null,
    /// The unmaterialized undefined value.
    Undefined,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one expression through its coercion.
    pub(in crate::lower) fn lower_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let Some(coercion) = self.coercion(expression) else {
            return self.lower_expression_value(expression);
        };

        // walk the coercion path from the classified source to its target
        let target = coercion.target();
        let source = self.lowerer.instance_type(self.instance, coercion.source)?;
        let value = self.coercion_source(expression, source)?;
        let value = self.lower_adjustments(value, source, &coercion.adjustments)?;

        self.materialize_coercion_value(value, target)
    }

    /// Classify one expression before applying its coercion path.
    fn coercion_source(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<CoercionValue> {
        // keep sources whose value lives in their type unmaterialized
        let value = match self.lowerer.ty(source)? {
            dir::Type::Literal(literal) => CoercionValue::Literal(literal),
            dir::Type::Null => CoercionValue::Null,
            dir::Type::Undefined => CoercionValue::Undefined,
            _ => return Ok(CoercionValue::Expression(expression)),
        };

        // lower the const expression's remaining runtime evaluation
        self.lower_const_expression(expression)?;

        Ok(value)
    }

    /// Lower one expression whose value lives in its type.
    pub(in crate::lower) fn lower_const_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // skip the expressions already folded into their committed type
        if self.is_folded_expression(expression)? {
            return Ok(());
        }

        // evaluate every remaining const value as an ordinary expression
        // NOTE: value lowering avoids re-entering the coercion path that called here
        self.lower_expression_value(expression)?;

        Ok(())
    }

    /// Return whether one expression's computation folded into its committed type.
    fn is_folded_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        match self.source().tree().get(expression) {
            // literal values compute nothing
            dir::Expression::Literal(_) => Ok(true),

            // builtin operations fold when every operand folds
            dir::Expression::Binary { left, right, .. } => {
                let (left, right) = (*left, *right);
                let is_builtin = self.operator_decision(expression)?.is_builtin();

                Ok(is_builtin
                    && self.is_folded_expression(left)?
                    && self.is_folded_expression(right)?)
            }
            dir::Expression::Unary { right, .. } => {
                let right = *right;
                let is_builtin = self.operator_decision(expression)?.is_builtin();

                Ok(is_builtin && self.is_folded_expression(right)?)
            }

            _ => Ok(false),
        }
    }

    /// Apply one complete adjustment path.
    fn lower_adjustments(
        &mut self,
        mut value: CoercionValue,
        mut source: dir::GlobalTypeId,
        adjustments: &[dir::CoercionAdjustment],
    ) -> CompilerResult<CoercionValue> {
        // apply each adjustment and advance the source type along the path
        for adjustment in adjustments {
            value = self.lower_adjustment(value, source, adjustment)?;
            source = adjustment.target();
        }

        Ok(value)
    }

    /// Apply one adjustment.
    fn lower_adjustment(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        adjustment: &dir::CoercionAdjustment,
    ) -> CompilerResult<CoercionValue> {
        match adjustment {
            dir::CoercionAdjustment::Widen { target } => {
                let value = self.materialize_coercion_value(value, *target)?;

                Ok(CoercionValue::Runtime(value))
            }
            dir::CoercionAdjustment::Borrow { target } => {
                let target = self.lower_type(*target)?;
                let value = match value {
                    CoercionValue::Expression(expression) => {
                        self.lower_borrowed_place(expression, target)?
                    }
                    CoercionValue::Runtime(value)
                        if self.lowerer.has_indirect_representation(source)? =>
                    {
                        self.builder.cast(mir::CastOperator::Bitcast, value, target)
                    }
                    _ => {
                        return Err(CompilerError::Internal {
                            message: "a coercion borrow of a value without a place".to_string(),
                        });
                    }
                };

                Ok(CoercionValue::Runtime(value))
            }
            dir::CoercionAdjustment::Read { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let target = self.lower_type(*target)?;
                let value = self.builder.load(value, target);

                Ok(CoercionValue::Runtime(value))
            }
            dir::CoercionAdjustment::Scalar { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let source = self.value_representation(value)?;
                let target = self.lower_type(*target)?;
                let value = if self.builder.tree().get(source) == self.builder.tree().get(target) {
                    value
                } else {
                    let operator = self.cast_operator(
                        self.builder.tree().get(source),
                        self.builder.tree().get(target),
                    )?;

                    self.builder.cast(operator, value, target)
                };

                Ok(CoercionValue::Runtime(value))
            }
            dir::CoercionAdjustment::Union { target, cases } => {
                let value = self.lower_union_adjustment(value, source, *target, cases)?;

                Ok(CoercionValue::Runtime(value))
            }
            dir::CoercionAdjustment::Erase { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let value = self.lower_erasure(value, source, *target)?;

                Ok(CoercionValue::Runtime(value))
            }
            dir::CoercionAdjustment::Manage { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let target = self.lower_type(*target)?;
                let value = self.builder.cast(mir::CastOperator::Bitcast, value, target);

                Ok(CoercionValue::Runtime(value))
            }
            dir::CoercionAdjustment::Instantiate { target, arguments } => {
                let CoercionValue::Expression(expression) = value else {
                    return Err(CompilerError::Internal {
                        message: "an instantiate coercion of a lowered value".to_string(),
                    });
                };
                let node = expression.into_global_any(self.source);
                let symbol = self.lowerer.resolved_symbol(node)?;
                let value = self.lower_instantiated_value(symbol, *target, arguments)?;

                Ok(CoercionValue::Runtime(value))
            }
            adjustment => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("an implicit {} coercion", adjustment.as_str()),
            }
            .into()),
        }
    }

    /// Lower one expression into a borrow of its place or reference.
    pub(in crate::lower) fn lower_borrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // borrow reference sources as a kind change
        let source = self.node_type_id(expression)?;
        if self.lowerer.has_indirect_representation(source)? {
            let value = self.lower_expression_value(expression)?;

            return Ok(self.builder.cast(mir::CastOperator::Bitcast, value, target));
        }

        // borrow the storage holding value sources
        match self.source().tree().get(expression).clone() {
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lowerer.resolved_symbol(node)?;
                match self.values.get(&symbol.local_id).copied() {
                    Some(Binding::Local(local)) => Ok(self.builder.local_addr(local, target)),
                    // borrow captured bindings at their frame field
                    Some(Binding::Captured { frame, field, .. }) => {
                        Ok(self.builder.field_addr(frame, field, target))
                    }
                    // give borrowed parameters a frame home on first borrow
                    Some(Binding::Value(value)) => {
                        let ty = self.lowerer.symbol_type(symbol)?;
                        let slot = self.lower_type(ty)?;
                        let local = self.builder.local(slot, mir::Mutability::Mutable);
                        self.builder.local_set(local, value);
                        self.values.insert(symbol.local_id, Binding::Local(local));

                        Ok(self.builder.local_addr(local, target))
                    }
                    None => Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a borrow of a module binding".to_string(),
                    }
                    .into()),
                }
            }
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a borrow of a '{}' expression", other.variant_name()),
            }
            .into()),
        }
    }

    /// Adjust one value into its union target representation.
    fn lower_union_adjustment(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        let representation = self.lower_type(target)?;

        // preserve the source case correspondence for indexed variants
        if let mir::Type::Variant { .. } = self.builder.tree().get(representation) {
            return self.lower_variant_adjustment(value, source, target, representation, cases);
        }

        // dispatch union sources before leaving their indexed representation
        if matches!(self.lowerer.ty(source)?, dir::Type::Union(_)) {
            return self.lower_union_exit(value, source, target, representation, cases);
        }

        // materialize nullish sentinels directly at the union's representation
        if matches!(value, CoercionValue::Null | CoercionValue::Undefined) {
            return self.materialize_coercion_value(value, target);
        }

        // convert the singular source through its selected case first
        let (value, source) = match cases {
            [] => (value, source),
            [case] => {
                let value = self.lower_adjustments(value, source, &case.adjustments)?;

                (value, case.target)
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "a singular union injection carries several cases".to_string(),
                });
            }
        };

        // inject the converted source at the union's shared reference representation
        let value = self.materialize_coercion_value(value, source)?;
        if self
            .builder
            .tree()
            .get(representation)
            .is_reference_representation()
        {
            return self.adapt_to_representation(value, representation);
        }

        Ok(value)
    }

    /// Inject one complete source value into an indexed variant case.
    fn lower_variant_adjustment(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        representation: mir::LocalNodeId<mir::Type>,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        let target_members = self.union_members(target)?;

        // convert each runtime case of a union source
        if let dir::Type::Union(source_union) = self.lowerer.ty(source)? {
            let source_members = self
                .lowerer
                .types(source.module_id)?
                .type_ids(source_union.elements)
                .to_vec();
            let value = self.materialize_coercion_value(value, source)?;
            let value_type = self.value_representation(value)?;
            if matches!(
                self.builder.tree().get(value_type),
                mir::Type::Variant { .. }
            ) {
                return self.lower_variant_conversion(
                    source_members,
                    target_members,
                    value,
                    representation,
                    cases,
                );
            }

            // enter a single target case for shared source representations
            let Some(first) = cases.first() else {
                return Err(CompilerError::Internal {
                    message: "a union conversion without source cases".to_string(),
                });
            };
            if cases.iter().any(|case| case.target != first.target) {
                return Err(CompilerError::Internal {
                    message: "a union conversion requiring a missing source discriminant"
                        .to_string(),
                });
            }

            // build the shared target case around the converted payload
            let target_member = first.target;
            require_union_case(&target_members, first.index, target_member)?;
            let payload = self.union_payload(CoercionValue::Runtime(value), target_member)?;

            return Ok(self
                .builder
                .variant_new(representation, first.index, payload));
        }

        // enter the single declared case of a plain injection
        let [case] = cases else {
            return Err(CompilerError::Internal {
                message: "a union injection requiring exactly one source case".to_string(),
            });
        };
        let target_member = case.target;
        require_union_case(&target_members, case.index, target_member)?;

        // convert the source value before inserting its target case
        let value = self.lower_adjustments(value, source, &case.adjustments)?;
        let payload = self.union_payload(value, target_member)?;

        Ok(self
            .builder
            .variant_new(representation, case.index, payload))
    }

    /// Convert one indexed union value into another ordered case set.
    fn lower_variant_conversion(
        &mut self,
        source_members: Vec<dir::GlobalTypeId>,
        target_members: Vec<dir::GlobalTypeId>,
        value: mir::Value,
        representation: mir::LocalNodeId<mir::Type>,
        mappings: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        if mappings.len() != source_members.len() {
            return Err(CompilerError::Internal {
                message: "a union conversion with an incomplete case map".to_string(),
            });
        }

        // dispatch once over the source discriminant
        let result = self
            .builder
            .local(representation, mir::Mutability::Immutable);
        let exit = self.builder.block();
        let mut cases = Vec::with_capacity(mappings.len());
        for index in 0..mappings.len() {
            cases.push((index as u32, self.builder.block()));
        }
        self.builder.variant_switch(value, None, cases.clone());

        // rebuild every source case under its target discriminant
        for (source_index, block) in cases {
            self.builder.switch_to_block(block);
            let source_member = source_members[source_index as usize];
            let mapping = &mappings[source_index as usize];
            let source_value = self.union_case_value(value, source_index, source_member)?;
            let source_value =
                self.lower_adjustments(source_value, source_member, &mapping.adjustments)?;
            let target_member = mapping.target;
            require_union_case(&target_members, mapping.index, target_member)?;
            let payload = self.union_payload(source_value, target_member)?;
            let converted = self
                .builder
                .variant_new(representation, mapping.index, payload);
            self.builder.local_set(result, converted);
            self.builder.jump(exit);
        }

        // resume after the dispatch with the rebuilt value
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(result))
    }

    /// Convert one indexed union value into a non-union representation.
    fn lower_union_exit(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        representation: mir::LocalNodeId<mir::Type>,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        let dir::Type::Union(source_union) = self.lowerer.ty(source)? else {
            return Err(CompilerError::Internal {
                message: "a union exit with a non-union source".to_string(),
            });
        };

        // require one case per source member, all landing on the same target
        let source_members = self
            .lowerer
            .types(source.module_id)?
            .type_ids(source_union.elements)
            .to_vec();
        if cases.len() != source_members.len() {
            return Err(CompilerError::Internal {
                message: "a union exit with an incomplete case map".to_string(),
            });
        }
        if cases.iter().any(|case| case.target != target) {
            return Err(CompilerError::Internal {
                message: "a union exit selecting a different target type".to_string(),
            });
        }

        // materialize the source at its own representation before converting
        let value = self.materialize_coercion_value(value, source)?;
        let value_type = self.value_representation(value)?;

        // dispatch indexed representations and convert each payload independently
        if matches!(
            self.builder.tree().get(value_type),
            mir::Type::Variant { .. }
        ) {
            // dispatch once over the source discriminant
            let result = self
                .builder
                .local(representation, mir::Mutability::Immutable);
            let exit = self.builder.block();
            let mut blocks = Vec::with_capacity(cases.len());
            for index in 0..cases.len() {
                blocks.push((index as u32, self.builder.block()));
            }
            self.builder.variant_switch(value, None, blocks.clone());

            // convert each source case into the shared target representation
            for (index, block) in blocks {
                self.builder.switch_to_block(block);
                let member = source_members[index as usize];
                let member_value = self.union_case_value(value, index, member)?;
                let member_value = self.lower_adjustments(
                    member_value,
                    member,
                    &cases[index as usize].adjustments,
                )?;
                let member_value = self.materialize_coercion_value(member_value, target)?;
                self.builder.local_set(result, member_value);
                self.builder.jump(exit);
            }

            // resume after the dispatch with the converted value
            self.builder.switch_to_block(exit);

            return Ok(self.builder.local_get(result));
        }

        // require one shared conversion for shared representations
        let Some(first) = cases.first() else {
            return Err(CompilerError::Internal {
                message: "a union exit without source cases".to_string(),
            });
        };
        if cases
            .iter()
            .any(|case| case.adjustments != first.adjustments)
        {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a per-arm conversion over an undiscriminated union representation"
                    .to_string(),
            }
            .into());
        }
        let value = self.lower_adjustments(
            CoercionValue::Runtime(value),
            source_members[0],
            &first.adjustments,
        )?;

        self.materialize_coercion_value(value, target)
    }

    /// Return the logical members of one union type behind forms and aliases.
    pub(in crate::lower) fn union_members(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // resolve the union behind owned forms and transparent aliases
        let mut stored = self.lowerer.peel_owned(ty)?;
        while let dir::Type::Application(instance) = self.lowerer.ty(stored)? {
            let defined = match self.lowerer.definition(instance.symbol)? {
                Some(dir::Definition::TypeAlias(alias)) => alias.value,
                _ => break,
            };
            stored = self.lowerer.peel_owned(defined)?;
        }

        // require the resolved type to be a union
        let dir::Type::Union(union) = self.lowerer.ty(stored)? else {
            return Err(CompilerError::Internal {
                message: "union members requested from a non-union type".to_string(),
            });
        };

        Ok(self
            .lowerer
            .types(stored.module_id)?
            .type_ids(union.elements)
            .to_vec())
    }

    /// Return one indexed source case as a coercion value.
    fn union_case_value(
        &mut self,
        value: mir::Value,
        index: u32,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<CoercionValue> {
        Ok(match self.lowerer.ty(member)? {
            dir::Type::Literal(literal) => CoercionValue::Literal(literal),
            dir::Type::Null => CoercionValue::Null,
            dir::Type::Undefined => CoercionValue::Undefined,
            _ => CoercionValue::Runtime(self.builder.variant_payload(value, index)),
        })
    }

    /// Materialize one target union case payload when it has runtime storage.
    fn union_payload(
        &mut self,
        value: CoercionValue,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<mir::Value>> {
        // skip literal and singleton cases, which carry no runtime payload
        if matches!(
            self.lowerer.ty(target)?,
            dir::Type::Literal(_) | dir::Type::Null | dir::Type::Undefined
        ) {
            return Ok(None);
        }

        // materialize every other case into its storage
        let value = self.materialize_coercion_value(value, target)?;

        Ok(Some(value))
    }

    /// Materialize one coercion value at its target representation.
    fn materialize_coercion_value(
        &mut self,
        value: CoercionValue,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        match value {
            CoercionValue::Expression(expression) => self.lower_expression_value(expression),
            CoercionValue::Runtime(value) => Ok(value),
            CoercionValue::Literal(literal) => {
                let target = self.lower_type(target)?;
                let target = self.builder.tree().get(target).clone();

                self.lower_constant(literal, target)
            }
            CoercionValue::Null => {
                let target = self.lower_type(target)?;

                Ok(self.builder.constant(mir::Constant::Null, target))
            }
            CoercionValue::Undefined => {
                let target = self.lower_type(target)?;

                Ok(self.builder.constant(mir::Constant::Undefined, target))
            }
        }
    }

    /// Read the current value of one binding.
    pub(in crate::lower) fn read_binding(&mut self, binding: Binding) -> mir::Value {
        match binding {
            Binding::Value(value) => value,
            Binding::Local(local) => self.builder.local_get(local),
            // load captured bindings through their frame field
            Binding::Captured { frame, field, ty } => {
                let address = self.emit_field_address(frame, field, ty, mir::Access::Mutable);

                self.builder.load(address, ty)
            }
        }
    }

    /// Lower one value expression that resolved to a symbol.
    pub(in crate::lower) fn lower_resolved_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<mir::Value> {
        match self.values.get(&symbol.local_id).copied() {
            Some(binding) => Ok(self.read_binding(binding)),
            // load module constants through their globals
            None => {
                if let Some(global) = self.module_constant_global(symbol)? {
                    return Ok(self.builder.load_global(global));
                }

                // materialize callable declarations as function values
                if let Some(value) = self.lower_function_value(expression, symbol, None)? {
                    return Ok(value);
                }

                Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a module or captured binding".to_string(),
                }
                .into())
            }
        }
    }

    /// Lower one expression to the value it produces.
    fn lower_expression_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        match self.source().tree().get(expression).clone() {
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lowerer.resolved_symbol(node)?;

                self.lower_resolved_value(expression, symbol)
            }

            dir::Expression::Literal(literal) => self.lower_scalar_literal(expression, literal),

            dir::Expression::Declaration(declaration) => {
                let node = declaration.into_global_any(self.source);
                let Some(symbol) = self.lowerer.symbol_declared_at(node)? else {
                    return Err(CompilerError::Internal {
                        message: "a declaration expression without a symbol".to_string(),
                    });
                };

                // bind the declared closure over its captured environment
                let environment = self.capture_environment(symbol)?;
                match self.lower_function_value(expression, symbol, environment)? {
                    Some(value) => Ok(value),
                    None => Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a non-callable declaration expression".to_string(),
                    }
                    .into()),
                }
            }

            // a + b
            dir::Expression::Binary {
                left,
                operator: _,
                right,
            } => {
                let resolution = self.operator_decision(expression)?;
                let dir::OperationResolution::One(application) = resolution else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a binary operator on union operands".to_string(),
                    }
                    .into());
                };
                match application {
                    dir::OperatorApplication::Binary {
                        operator,
                        target: dir::OperatorTarget::Builtin(operands),
                        ..
                    } => match operator {
                        dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                            self.lower_strict_equality(left, operator, right, &operands)
                        }
                        dir::BinaryOperator::And | dir::BinaryOperator::Or => {
                            self.lower_logical(expression, left, operator, right)
                        }
                        dir::BinaryOperator::Coalesce => {
                            self.lower_coalesce(expression, left, right)
                        }
                        operator => self.lower_binary(left, operator, right, &operands),
                    },
                    // dispatch protocol operators as left.method(right)
                    dir::OperatorApplication::Binary {
                        target: dir::OperatorTarget::Call(call),
                        ..
                    } => match &*call {
                        dir::Call {
                            target:
                                dir::CallableTarget::Symbol {
                                    function,
                                    dispatch: dir::FunctionDispatch::Direct,
                                },
                            ..
                        } => self.lower_operator_method(left, &call, function),
                        _ => Err(CompilerError::Internal {
                            message: "a non-callable binary operator".to_string(),
                        }),
                    },
                    dir::OperatorApplication::Unary { .. } => Err(CompilerError::Internal {
                        message: "a unary resolution for a binary expression".to_string(),
                    }),
                }
            }

            // -value
            dir::Expression::Unary { operator: _, right } => {
                let resolution = self.operator_decision(expression)?;
                let dir::OperationResolution::One(application) = resolution else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a unary operator on a union operand".to_string(),
                    }
                    .into());
                };
                match application {
                    dir::OperatorApplication::Unary {
                        operator,
                        target: dir::OperatorTarget::Builtin(operand),
                        ..
                    } => self.lower_unary(operator, right, &operand),
                    // dispatch protocol operators as right.method()
                    dir::OperatorApplication::Unary {
                        target: dir::OperatorTarget::Call(call),
                        ..
                    } => match &*call {
                        dir::Call {
                            target:
                                dir::CallableTarget::Symbol {
                                    function,
                                    dispatch: dir::FunctionDispatch::Direct,
                                },
                            ..
                        } => self.lower_operator_method(right, &call, function),
                        _ => Err(CompilerError::Internal {
                            message: "a non-callable unary operator".to_string(),
                        }),
                    },
                    dir::OperatorApplication::Binary { .. } => Err(CompilerError::Internal {
                        message: "a binary resolution for a unary expression".to_string(),
                    }),
                }
            }

            // match (value) { ... }
            dir::Expression::Match { value, arms } => self.lower_match(expression, value, &arms),

            // cond ? a : b
            dir::Expression::If {
                form: dir::IfForm::Ternary,
                condition,
                then_expression,
                else_expression,
            } => self.lower_ternary(expression, &condition, then_expression, else_expression),

            // { x: 1 }
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.into_iter().collect::<Vec<_>>();

                self.lower_object_expression(expression, &properties)
            }

            // Point { x: 1 }
            dir::Expression::StructExpression { properties, .. } => {
                self.lower_struct_expression(expression, &properties)
            }

            // (a, b)
            dir::Expression::TupleExpression { elements } => {
                self.lower_tuple_expression(expression, &elements)
            }

            // this
            dir::Expression::This => {
                let Some(binding) = self.this else {
                    return Err(CompilerError::Internal {
                        message: "this used outside a method body".to_string(),
                    });
                };

                Ok(self.read_binding(binding))
            }

            // &value, &readonly value
            dir::Expression::BorrowOf { right, .. } => {
                let target = self.lower_type(self.node_type_id(expression)?)?;

                self.lower_borrowed_place(right, target)
            }

            // point.x
            dir::Expression::Member { left, .. } => self.lower_member(expression, left),

            // pair[0]
            dir::Expression::Index { left, index, .. } => {
                self.lower_subscript(expression, left, index)
            }

            // value as T
            dir::Expression::As {
                expression: value, ..
            } => self.lower_as(expression, value),

            // Meters(5)
            dir::Expression::Call { .. }
                if let Some(resolution) = self.construct_decision(expression) =>
            {
                self.lower_construct(expression, &resolution)
            }

            // new Counter(start)
            dir::Expression::New { .. } => {
                let Some(resolution) = self.construct_decision(expression) else {
                    return Err(CompilerError::Internal {
                        message: "missing a construct resolution for one new".to_string(),
                    });
                };

                self.lower_construct(expression, &resolution)
            }

            // [1, 2, 3]
            dir::Expression::ArrayExpression { .. } => {
                let value = self.lower_call(expression)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "an array construction without a value".to_string(),
                })
            }

            // <div .../>
            dir::Expression::TreeExpression { .. } => {
                let resolution = self.tree_decision(expression)?;

                self.lower_tree(&resolution)
            }

            // call(...)
            dir::Expression::Call { .. } => {
                let value = self.lower_call(expression)?;

                // return the value the call produced
                if let Some(value) = value {
                    return Ok(value);
                }

                // yield one dead uninit value behind a diverging call's sealed block
                let ty = self.node_type_id(expression)?;
                if matches!(self.lowerer.ty(ty)?, dir::Type::Never) {
                    let never = self.lower_type(ty)?;

                    return Ok(self.builder.constant(mir::Constant::Uninit, never));
                }

                // yield the unique inhabitant of a zero sized result
                let result_type = self.lower_type(ty)?;
                if matches!(self.builder.tree().get(result_type), mir::Type::Void) {
                    return Ok(self.builder.constant(mir::Constant::Undefined, result_type));
                }

                // reject a call that produced no value in value position
                Err(CompilerError::Internal {
                    message: "a call used as a value produced no value".to_string(),
                })
            }

            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("'{}' expressions", other.variant_name()),
            }
            .into()),
        }
    }

    /// Return the declared global behind one module constant.
    pub(in crate::lower) fn module_constant_global(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Global>>> {
        // read the global this module declared or imported
        match self.lowerer.globals.get(&symbol) {
            Some(Ok(global)) => return Ok(Some(*global)),
            // cascade the declaration failure the declare phase kept
            Some(Err(diagnostic)) => {
                return Err(CompilerError::Diagnostic(Box::new(diagnostic.clone())));
            }
            None => {}
        }

        // skip local constants and non-binding symbols
        if symbol.module_id == self.lowerer.module || !self.lowerer.is_module_binding(symbol)? {
            return Ok(None);
        }

        let path = self.lowerer.symbol_path(symbol)?;

        Err(CompilerError::Internal {
            message: format!("missing a declared global behind the constant '{path}'"),
        })
    }
}

/// Require one selected union case to name its declared member at its index.
fn require_union_case(
    members: &[dir::GlobalTypeId],
    index: u32,
    member: dir::GlobalTypeId,
) -> CompilerResult<()> {
    if members.get(index as usize) != Some(&member) {
        return Err(CompilerError::Internal {
            message: "a union conversion selecting an absent target member".to_string(),
        });
    }

    Ok(())
}
