use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::body::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Bind every symbol one irrefutable pattern selects from a value.
    pub(in crate::lower) fn lower_pattern_bindings(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        value: mir::Value,
        mutability: dir::Mutability,
    ) -> CompilerResult<()> {
        match self.pattern_decision(pattern)? {
            dir::PatternDecision::Ignore => Ok(()),

            // bind the whole value, then match any nested pattern over it
            dir::PatternDecision::Bind(binding) => {
                if let Some(symbol) = binding.symbol {
                    self.bind_pattern_symbol(symbol, value, mutability)?;
                }
                if let Some(nested) = binding.pattern {
                    let nested = self.pattern_node(nested)?;
                    self.lower_pattern_bindings(nested, value, mutability)?;
                }

                Ok(())
            }

            // project each declared field out of the destructured value
            dir::PatternDecision::Destructure(resolution) => {
                let (fields, rest) = match &*resolution {
                    dir::PatternDestructureResolution::Nominal(nominal) => {
                        (&nominal.fields, nominal.rest.as_deref())
                    }
                    dir::PatternDestructureResolution::Object(object) => {
                        (&object.fields, object.rest.as_deref())
                    }
                    dir::PatternDestructureResolution::Tuple(tuple) => (&tuple.fields, None),
                    dir::PatternDestructureResolution::Sequence(_) => {
                        return Err(LowerError::Unsupported {
                            anchor: self.lowerer.module.into(),
                            construct: "a sequence destructure".to_string(),
                        }
                        .into());
                    }
                };
                for field in fields {
                    self.lower_destructured_field(field, value, mutability)?;
                }
                if let Some(rest) = rest {
                    self.lower_destructured_field(rest, value, mutability)?;
                }

                Ok(())
            }

            // project the value once, then match the nested pattern
            dir::PatternDecision::Project(resolution) => {
                let projected = self.lower_pattern_projection(&resolution.projection, value)?;
                if let Some(nested) = resolution.pattern {
                    let nested = self.pattern_node(nested)?;
                    self.lower_pattern_bindings(nested, projected, mutability)?;
                }

                Ok(())
            }

            // fall back to the default value when the selected value is undefined
            dir::PatternDecision::Default(resolution) => {
                let nested = self.pattern_node(resolution.pattern)?;
                let value = self.lower_defaulted_input(nested, value, resolution.value)?;

                self.lower_pattern_bindings(nested, value, mutability)
            }

            // unwrap the required value, trapping when it is absent
            dir::PatternDecision::Must(resolution) => {
                let nested = self.pattern_node(resolution.pattern)?;
                let value = self.lower_required_input(nested, value)?;

                self.lower_pattern_bindings(nested, value, mutability)
            }

            // dispatch already selected the case; payload-free variants bind nothing
            dir::PatternDecision::Variant(resolution) => {
                if resolution.predicate.projection.is_some() {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a variant payload pattern".to_string(),
                    }
                    .into());
                }

                Ok(())
            }

            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a '{other:?}' binding pattern"),
            }
            .into()),
        }
    }

    /// Bind one destructured field through its selected projection.
    fn lower_destructured_field(
        &mut self,
        field: &dir::PatternFieldResolution,
        value: mir::Value,
        mutability: dir::Mutability,
    ) -> CompilerResult<()> {
        let projected = self.lower_pattern_projection(&field.projection, value)?;

        // a field without a nested pattern binds the symbol it declares
        let Some(nested) = field.pattern else {
            let Some(symbol) = self.lowerer.symbol_declared_at(field.source)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one destructured field".to_string(),
                });
            };

            return self.bind_pattern_symbol(symbol, projected, mutability);
        };
        let nested = self.pattern_node(nested)?;

        self.lower_pattern_bindings(nested, projected, mutability)
    }

    /// Project one pattern input value through its selected projection.
    fn lower_pattern_projection(
        &mut self,
        projection: &dir::ProjectionResolution,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let dir::OperationResolution::One(projection) = projection else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a destructure over a union receiver".to_string(),
            }
            .into());
        };

        match projection {
            // read the selected layout field
            dir::Projection::Field(field) => self.project_pattern_field(value, field),

            // unwrap the single newtype payload
            dir::Projection::NewtypePayload { .. } => Ok(self.builder.field_get(value, 0)),

            // materialize the statically absent field as undefined
            dir::Projection::Absent { ty } => {
                let carrier = self.lower_type(*ty)?;

                Ok(self.builder.constant(mir::Constant::Undefined, carrier))
            }

            // duplicated and moved inputs keep the value they carry
            dir::Projection::Copy { .. } | dir::Projection::Move { .. } => Ok(value),

            // load the pointee behind a dereferenced input
            dir::Projection::Dereference(_) => {
                let carrier = self.value_carrier(value)?;
                let mir::Type::Reference { pointee, .. } = self.builder.tree().get(carrier) else {
                    return Err(CompilerError::Internal {
                        message: "a dereferenced pattern input outside a reference".to_string(),
                    });
                };
                let pointee = *pointee;

                Ok(self.builder.load(value, pointee))
            }

            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a '{other:?}' pattern projection"),
            }
            .into()),
        }
    }

    /// Read one selected layout field out of a pattern input value.
    fn project_pattern_field(
        &mut self,
        value: mir::Value,
        field: &dir::FieldResolution,
    ) -> CompilerResult<mir::Value> {
        let receiver = field.receiver.ty();
        let index = self.member_field_index(field)?;
        let result = self.lower_type(field.ty)?;

        // load fields through addresses for reference receivers
        if let Some(layer) = self.lowerer.peel_indirection(receiver)? {
            let address = self.emit_field_address(value, index, result, layer.access);

            return Ok(self.builder.load(address, result));
        }

        Ok(self.builder.field_get(value, index))
    }

    /// Resolve one optional pattern input to its present payload or default.
    fn lower_defaulted_input(
        &mut self,
        nested: dir::LocalNodeId<dir::Pattern>,
        value: mir::Value,
        default: dir::GlobalNodeIdAny,
    ) -> CompilerResult<mir::Value> {
        let exact = self.pattern_carrier(nested)?;
        let default = default
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|message| CompilerError::Internal { message })?;

        self.lower_absent_fallback(value, exact, |lowerer| {
            lowerer.lower_expression(default).map(Some)
        })
    }

    /// Unwrap one required pattern input, trapping when it is absent.
    fn lower_required_input(
        &mut self,
        nested: dir::LocalNodeId<dir::Pattern>,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let exact = self.pattern_carrier(nested)?;

        self.lower_absent_fallback(value, exact, |lowerer| {
            lowerer.builder.unreachable();

            Ok(None)
        })
    }

    /// Branch one possibly absent value into its present payload or fallback.
    pub(in crate::lower) fn lower_absent_fallback(
        &mut self,
        value: mir::Value,
        exact: mir::LocalNodeId<mir::Type>,
        fallback: impl FnOnce(&mut Self) -> CompilerResult<Option<mir::Value>>,
    ) -> CompilerResult<mir::Value> {
        let carrier = self.value_carrier(value)?;
        if carrier == exact {
            return Ok(value);
        }

        // a statically absent input always takes the fallback
        if matches!(self.builder.tree().get(carrier), mir::Type::Void) {
            return match fallback(self)? {
                Some(value) => Ok(value),
                None => Err(CompilerError::Internal {
                    message: "a required pattern input is statically absent".to_string(),
                }),
            };
        }

        let slot = self.builder.local(exact, mir::Mutability::Immutable);
        let present_block = self.builder.block();
        let absent_block = self.builder.block();
        let join = self.builder.block();
        match self.builder.tree().get(carrier).clone() {
            mir::Type::Variant { cases, .. } => {
                let Some(mir::NullishCase::Case(absent)) =
                    self.builder.tree().undefined_case(carrier)
                else {
                    return Err(CompilerError::Internal {
                        message: "an optional pattern input without its undefined case".to_string(),
                    });
                };
                let absent = absent as usize;
                self.builder.variant_switch(
                    value,
                    Some(present_block),
                    vec![(absent as u32, absent_block)],
                );

                // extract the sole payload or rebuild the narrowed union
                self.builder.switch_to_block(present_block);
                let present: Vec<_> = cases
                    .iter()
                    .enumerate()
                    .filter(|(_, case)| {
                        !matches!(self.builder.tree().get(case.ty), mir::Type::Void)
                    })
                    .map(|(index, _)| index)
                    .collect();
                match present.as_slice() {
                    [sole] => {
                        let payload = self.builder.variant_payload(value, *sole as u32);
                        let payload = self.adapt_to_carrier(payload, exact)?;
                        self.builder.local_set(slot, payload);
                        self.builder.jump(join);
                    }
                    _ => {
                        let mut arms = Vec::with_capacity(present.len());
                        for _ in &present {
                            arms.push(self.builder.block());
                        }
                        let targets = present
                            .iter()
                            .copied()
                            .zip(arms.iter().copied())
                            .map(|(case, block)| (case as u32, block))
                            .collect();
                        self.builder.variant_switch(value, None, targets);
                        for (narrowed, (case, block)) in
                            present.iter().copied().zip(arms).enumerate()
                        {
                            self.builder.switch_to_block(block);
                            let payload = self.builder.variant_payload(value, case as u32);
                            let rebuilt =
                                self.builder
                                    .variant_new(exact, narrowed as u32, Some(payload));
                            self.builder.local_set(slot, rebuilt);
                            self.builder.jump(join);
                        }
                    }
                }
            }
            other if other.is_reference_carrier() => {
                let undefined = self.builder.constant(mir::Constant::Undefined, carrier);
                let is_absent =
                    self.builder
                        .binary_op(mir::BinaryOperator::Equal, value, undefined);
                self.builder.branch(is_absent, absent_block, present_block);
                self.builder.switch_to_block(present_block);
                let kept = self.builder.cast(mir::CastOperator::Bitcast, value, exact);
                self.builder.local_set(slot, kept);
                self.builder.jump(join);
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "an optional pattern input without an absent case".to_string(),
                });
            }
        }

        // fill the fallback arm, which may terminate instead of joining
        self.builder.switch_to_block(absent_block);
        if let Some(value) = fallback(self)? {
            self.builder.local_set(slot, value);
            self.builder.jump(join);
        }
        self.builder.switch_to_block(join);

        Ok(self.builder.local_get(slot))
    }

    /// Bind one pattern symbol at its home, mirroring let bindings.
    fn bind_pattern_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: mir::Value,
        mutability: dir::Mutability,
    ) -> CompilerResult<()> {
        // give lifted bindings their frame home ahead of local storage
        if self.bind_lifted(symbol, value)? {
            return Ok(());
        }

        // keep immutable bindings as pure values; give mutable ones a local
        let binding = match mutability {
            dir::Mutability::Immutable => Binding::Value(value),
            _ => {
                let ty = self.lowerer.symbol_type(symbol)?;
                let ty = self.lower_type(ty)?;
                let value = self.adapt_to_carrier(value, ty)?;
                let local = self.builder.local(ty, mir::Mutability::Mutable);
                self.builder.local_set(local, value);

                Binding::Local(local)
            }
        };
        self.values.insert(symbol.local_id, binding);

        Ok(())
    }

    /// Return the lowered carrier one pattern node was checked at.
    fn pattern_carrier(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let node = pattern.into_global_any(self.source);
        let ty =
            self.source()
                .types
                .get_node_type_id(node)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("missing a type for pattern node {}", node.local_id.id),
                })?;

        self.lower_type(ty)
    }

    /// Return one nested pattern node inside this body's tree.
    fn pattern_node(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::LocalNodeId<dir::Pattern>> {
        if node.module_id != self.source {
            return Err(CompilerError::Internal {
                message: "a nested pattern outside its declaring module".to_string(),
            });
        }

        node.local_id
            .try_into_typed::<dir::Pattern>()
            .map_err(|message| CompilerError::Internal { message })
    }
}
