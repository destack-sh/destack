use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::body::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

/// One value in flight along a coercion path.
enum CoercionValue {
    /// One source expression that has not been evaluated.
    Expression(dir::LocalNodeId<dir::Expression>),
    /// One materialized value.
    Runtime(mir::Value),
    /// One compile-time scalar literal.
    Literal(dir::ScalarLiteral),
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
        let target = coercion.target();
        let value = self.coercion_source(expression, coercion.source)?;
        let value = self.lower_adjustments(value, coercion.source, &coercion.adjustments)?;

        self.materialize_coercion_value(value, target)
    }

    /// Classify one expression before applying its coercion path.
    fn coercion_source(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<CoercionValue> {
        let source = self.lowerer.reduced_type(source)?;

        Ok(match self.lowerer.ty(source)? {
            dir::Type::Literal(literal) => CoercionValue::Literal(literal),
            dir::Type::Null => CoercionValue::Null,
            dir::Type::Undefined => CoercionValue::Undefined,
            _ => CoercionValue::Expression(expression),
        })
    }

    /// Apply one complete adjustment path.
    fn lower_adjustments(
        &mut self,
        mut value: CoercionValue,
        mut source: dir::GlobalTypeId,
        adjustments: &[dir::CoercionAdjustment],
    ) -> CompilerResult<CoercionValue> {
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
                        if self
                            .lowerer
                            .has_indirect_representation(source, &self.type_substitution)? =>
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
                let Some(source) = self.builder.value_type(value) else {
                    return Err(CompilerError::Internal {
                        message: "the lowered scalar coercion source has no type".to_string(),
                    });
                };
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
            dir::CoercionAdjustment::Existential { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let value = self.lower_existential(value, source, *target)?;

                Ok(CoercionValue::Runtime(value))
            }
            dir::CoercionAdjustment::Instantiate { target, arguments } => {
                // materialize the reference at its selected concrete instance
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
        if self
            .lowerer
            .has_indirect_representation(source, &self.type_substitution)?
        {
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

    /// Adjust one value into its union target carrier.
    fn lower_union_adjustment(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        let carrier = self.lower_type(target)?;

        // preserve the source case correspondence for indexed variants
        if let mir::Type::Variant { .. } = self.builder.tree().get(carrier) {
            return self.lower_variant_adjustment(value, source, target, carrier, cases);
        }

        // dispatch union sources before leaving their indexed carrier
        if matches!(self.lowerer.ty(source)?, dir::Type::Union(_)) {
            return self.lower_union_exit(value, source, target, carrier, cases);
        }

        self.materialize_coercion_value(value, target)
    }

    /// Inject one complete source value into an indexed variant case.
    fn lower_variant_adjustment(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        carrier: mir::LocalNodeId<mir::Type>,
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
            let Some(value_type) = self.builder.value_type(value) else {
                return Err(CompilerError::Internal {
                    message: "the lowered union coercion source has no type".to_string(),
                });
            };
            if matches!(
                self.builder.tree().get(value_type),
                mir::Type::Variant { .. }
            ) {
                return self.lower_variant_conversion(
                    source_members,
                    target_members,
                    value,
                    carrier,
                    cases,
                );
            }

            // enter a single target case for shared source carriers
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
            let target_member = first.target;
            let Some(target_index) = target_members
                .iter()
                .position(|member| *member == target_member)
            else {
                return Err(CompilerError::Internal {
                    message: "a union conversion selecting an absent target member".to_string(),
                });
            };
            let payload = self.union_payload(CoercionValue::Runtime(value), target_member)?;

            return Ok(self
                .builder
                .variant_new(carrier, target_index as u32, payload));
        }
        let [case] = cases else {
            return Err(CompilerError::Internal {
                message: "a union injection requiring exactly one source case".to_string(),
            });
        };
        let target_member = case.target;
        let Some(target_index) = target_members
            .iter()
            .position(|member| *member == target_member)
        else {
            return Err(CompilerError::Internal {
                message: "a union injection selecting an absent target member".to_string(),
            });
        };

        // convert the source value before inserting its target case
        let value = self.lower_adjustments(value, source, &case.adjustments)?;
        let payload = self.union_payload(value, target_member)?;

        Ok(self
            .builder
            .variant_new(carrier, target_index as u32, payload))
    }

    /// Convert one indexed union value into another ordered case set.
    fn lower_variant_conversion(
        &mut self,
        source_members: Vec<dir::GlobalTypeId>,
        target_members: Vec<dir::GlobalTypeId>,
        value: mir::Value,
        carrier: mir::LocalNodeId<mir::Type>,
        mappings: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        if mappings.len() != source_members.len() {
            return Err(CompilerError::Internal {
                message: "a union conversion with an incomplete case map".to_string(),
            });
        }

        // dispatch once and rebuild the value under the target discriminant
        let result = self.builder.local(carrier, mir::Mutability::Immutable);
        let exit = self.builder.block();
        let mut cases = Vec::with_capacity(mappings.len());
        for index in 0..mappings.len() {
            cases.push((index as u32, self.builder.block()));
        }
        self.builder.variant_switch(value, None, cases.clone());
        for (source_index, block) in cases {
            self.builder.switch_to_block(block);
            let source_member = source_members[source_index as usize];
            let mapping = &mappings[source_index as usize];
            let source_value = self.union_case_value(value, source_index, source_member)?;
            let source_value =
                self.lower_adjustments(source_value, source_member, &mapping.adjustments)?;
            let target_member = mapping.target;
            let Some(target_index) = target_members
                .iter()
                .position(|member| *member == target_member)
            else {
                return Err(CompilerError::Internal {
                    message: "a union conversion selecting an absent target member".to_string(),
                });
            };
            let payload = self.union_payload(source_value, target_member)?;
            let converted = self
                .builder
                .variant_new(carrier, target_index as u32, payload);
            self.builder.local_set(result, converted);
            self.builder.jump(exit);
        }
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(result))
    }

    /// Convert one indexed union value into a non-union carrier.
    fn lower_union_exit(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        carrier: mir::LocalNodeId<mir::Type>,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        let dir::Type::Union(source_union) = self.lowerer.ty(source)? else {
            return Err(CompilerError::Internal {
                message: "a union exit with a non-union source".to_string(),
            });
        };
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
        let value = self.materialize_coercion_value(value, source)?;
        let Some(value_type) = self.builder.value_type(value) else {
            return Err(CompilerError::Internal {
                message: "the lowered union exit source has no type".to_string(),
            });
        };

        // dispatch indexed carriers and convert each payload independently
        if matches!(
            self.builder.tree().get(value_type),
            mir::Type::Variant { .. }
        ) {
            let result = self.builder.local(carrier, mir::Mutability::Immutable);
            let exit = self.builder.block();
            let mut blocks = Vec::with_capacity(cases.len());
            for index in 0..cases.len() {
                blocks.push((index as u32, self.builder.block()));
            }
            self.builder.variant_switch(value, None, blocks.clone());
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
            self.builder.switch_to_block(exit);

            return Ok(self.builder.local_get(result));
        }

        // require one shared conversion for shared carriers
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
                construct: "a per-arm conversion over an undiscriminated union carrier".to_string(),
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

    /// Return the logical members of one union type.
    pub(in crate::lower) fn union_members(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let dir::Type::Union(union) = self.lowerer.ty(ty)? else {
            return Err(CompilerError::Internal {
                message: "a union adjustment targeting a non-union type".to_string(),
            });
        };

        Ok(self
            .lowerer
            .types(ty.module_id)?
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
        let member = self.lowerer.reduced_type(member)?;

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
        let reduced = self.lowerer.reduced_type(target)?;
        if matches!(
            self.lowerer.ty(reduced)?,
            dir::Type::Literal(_) | dir::Type::Null | dir::Type::Undefined
        ) {
            return Ok(None);
        }
        let value = self.materialize_coercion_value(value, target)?;

        Ok(Some(value))
    }

    /// Materialize one coercion value at its target carrier.
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

    /// Lower one value expression by form.
    /// Lower one expression that resolved to a symbol.
    pub(in crate::lower) fn lower_resolved_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<mir::Value> {
        match self.values.get(&symbol.local_id).copied() {
            Some(Binding::Value(value)) => Ok(value),
            Some(Binding::Local(local)) => Ok(self.builder.local_get(local)),
            // load captured bindings through their frame field
            Some(Binding::Captured { frame, field, ty }) => {
                let address = self.emit_field_address(frame, field, ty, mir::Access::Mutable);

                Ok(self.builder.load(address, ty))
            }
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

    fn lower_expression_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // materialize comptime-folded values directly as constants
        if let dir::Type::Literal(literal) = self.node_type(expression)? {
            return self.lower_scalar_literal(expression, literal);
        }

        match self.source().tree().get(expression).clone() {
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lowerer.resolved_symbol(node)?;

                self.lower_resolved_value(expression, symbol)
            }

            dir::Expression::ScalarLiteral(literal) => {
                self.lower_scalar_literal(expression, literal)
            }

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
                        operator => self.lower_binary(left, operator, right, &operands),
                    },
                    // dispatch protocol operators as left.method(right)
                    dir::OperatorApplication::Binary {
                        target: dir::OperatorTarget::Call(call),
                        ..
                    } => match &*call {
                        dir::Call {
                            target:
                                dir::CallTarget::Symbol {
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
                                dir::CallTarget::Symbol {
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
            dir::Expression::This => self.this.ok_or_else(|| CompilerError::Internal {
                message: "this used outside a method body".to_string(),
            }),

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
                self.lower_construct(&resolution)
            }

            // new Counter(start)
            dir::Expression::New { .. } => {
                let Some(resolution) = self.construct_decision(expression) else {
                    return Err(CompilerError::Internal {
                        message: "missing a construct resolution for one new".to_string(),
                    });
                };

                self.lower_construct(&resolution)
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

                // reject void calls in value position
                Err(CompilerError::Internal {
                    message: "a void call used as a value".to_string(),
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
