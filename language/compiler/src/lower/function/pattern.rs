use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::place::{Place, PlaceProjection, PlaceRoot};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Bind every symbol one irrefutable pattern selects from a place.
    pub(in crate::lower) fn lower_pattern_bindings(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        place: &Place,
    ) -> CompilerResult<()> {
        match self.pattern_decision(pattern)? {
            // bind nothing for a wildcard
            dir::PatternDecision::Ignore => Ok(()),

            // bind the whole value, then match any nested pattern over it
            dir::PatternDecision::Bind(binding) => {
                if let Some(symbol) = binding.symbol {
                    let value = self.read_pattern_binding(symbol, place)?;
                    self.bind_symbol(symbol, value)?;
                }

                if let Some(nested) = binding.pattern {
                    let nested = self.pattern_node(nested)?;
                    self.lower_pattern_bindings(nested, place)?;
                }

                Ok(())
            }

            // project each declared field out of the destructured place
            dir::PatternDecision::Destructure(resolution) => {
                let (fields, rest) = match &*resolution {
                    dir::PatternDestructureResolution::Nominal(nominal) => {
                        (&nominal.fields, nominal.rest.as_deref())
                    }
                    dir::PatternDestructureResolution::Object(object) => {
                        (&object.fields, object.rest.as_deref())
                    }
                    dir::PatternDestructureResolution::Tuple(tuple) => (&tuple.fields, None),
                    dir::PatternDestructureResolution::Sequence(sequence) => {
                        (&sequence.fields, sequence.rest.as_deref())
                    }
                };

                // bind each named field
                for field in fields {
                    self.lower_destructured_field(field, place)?;
                }

                // bind the trailing rest field
                if let Some(rest) = rest {
                    self.lower_destructured_field(rest, place)?;
                }

                Ok(())
            }

            // project the place once, then match the nested pattern
            dir::PatternDecision::Project(resolution) => {
                let projected = self.lower_pattern_projection(&resolution.projection, place)?;
                if let Some(nested) = resolution.pattern {
                    let nested = self.pattern_node(nested)?;
                    self.lower_pattern_bindings(nested, &projected)?;
                }

                Ok(())
            }

            // fall back to the default value when the selected value is undefined
            dir::PatternDecision::Default(resolution) => {
                let nested = self.pattern_node(resolution.pattern)?;
                let value = self.lower_defaulted_input(nested, place, resolution.value)?;

                self.lower_pattern_bindings(nested, &value)
            }

            // unwrap the required value, trapping when it is absent
            dir::PatternDecision::Must(resolution) => {
                let nested = self.pattern_node(resolution.pattern)?;
                let value = self.lower_required_input(nested, place)?;

                self.lower_pattern_bindings(nested, &value)
            }

            // reject variant payload patterns
            dir::PatternDecision::Variant(resolution) => {
                if resolution.predicate.projection.is_some() {
                    return Err(self.unsupported("a variant payload pattern"));
                }

                Ok(())
            }

            // accept tested patterns, their tests run before the bindings
            dir::PatternDecision::Test(_) | dir::PatternDecision::Or(_) => Ok(()),
        }
    }

    /// Bind one destructured field through its selected projection.
    fn lower_destructured_field(
        &mut self,
        field: &dir::PatternFieldResolution,
        place: &Place,
    ) -> CompilerResult<()> {
        // project the field out of the destructured place
        let projected = self.lower_pattern_projection(&field.projection, place)?;

        // bind the declared symbol directly for a bare field
        let Some(nested) = field.pattern else {
            let Some(symbol) = self.lower.symbol_declared_at(field.source)? else {
                return Err(CompilerError::Internal {
                    message: "a missing symbol for one destructured field".to_string(),
                });
            };
            let value = self.read_pattern_binding(symbol, &projected)?;

            return self.bind_symbol(symbol, value);
        };

        // otherwise match the nested pattern over the projected place
        let nested = self.pattern_node(nested)?;

        self.lower_pattern_bindings(nested, &projected)
    }

    /// Read one binding's value from its place, a binding declared as a borrow of the place's
    /// value borrowing it.
    fn read_pattern_binding(
        &mut self,
        symbol: dir::GlobalSymbolId,
        place: &Place,
    ) -> CompilerResult<mir::Value> {
        let declared = self.lower.symbol_type(symbol)?;
        let declared = self.lower_type(declared)?;
        let held = self.place_type(place)?;
        let borrows = matches!(
            self.builder.tree().get(declared),
            mir::Type::Reference {
                kind: mir::ReferenceKind::Borrowed,
                ..
            }
        ) && !self
            .builder
            .tree()
            .same_representation(mir::TypeId::from(held), mir::TypeId::from(declared));
        match borrows {
            true => self.borrow_place(place, declared, mir::AddressKind::Borrow),
            false => self.read_place(place),
        }
    }

    /// Project one pattern input place through its selected projection.
    pub(in crate::lower) fn lower_pattern_projection(
        &mut self,
        projection: &dir::ProjectionResolution,
        place: &Place,
    ) -> CompilerResult<Place> {
        let dir::OperationResolution::One(projection) = projection else {
            return Err(self.unsupported("a destructure over a union receiver"));
        };

        match projection {
            // select the layout field
            dir::Projection::Field(field) => self.project_pattern_field(place, field),

            // select the single newtype payload
            dir::Projection::NewtypePayload { .. } => {
                let ty = self.place_type(place)?;
                let ty = self.builder.tree().represented(ty);
                let mir::Type::Newtype { inner, .. } = *self.builder.tree().get(ty) else {
                    return Err(CompilerError::Internal {
                        message: "a newtype projection over a place without a newtype".to_string(),
                    });
                };
                let mut place = place.clone();
                place.path.push(PlaceProjection::Field {
                    field: 0,
                    ty: inner,
                });

                Ok(place)
            }

            // materialize the statically absent field as undefined
            dir::Projection::Absent { ty } => {
                let absent_type = self.lower_type(*ty)?;
                let absent = self.builder.constant(mir::Constant::Zeroed, absent_type);

                Ok(Place::local(self.home(absent)))
            }

            // duplicated and moved inputs keep the place they name
            dir::Projection::Copy { .. } | dir::Projection::Move { .. } => Ok(place.clone()),

            // read through the selected sequence call
            dir::Projection::Call(call) => {
                let value = self.read_place(place)?;
                let read = self.lower_value_target_call(value, call)?;
                let read = read.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a projection call".to_string(),
                })?;

                Ok(Place::local(self.home(read)))
            }

            // read through the selected subscript
            dir::Projection::Subscript(subscript) => match &subscript.target {
                dir::SubscriptTarget::Call(call) => {
                    let value = self.read_place(place)?;
                    let read = self.lower_value_target_call(value, call)?;
                    let read = read.ok_or_else(|| CompilerError::Internal {
                        message: "a void result from a subscript projection".to_string(),
                    })?;

                    Ok(Place::local(self.home(read)))
                }
                dir::SubscriptTarget::Index(read) => {
                    if read.missing.is_some() {
                        return Err(self.internal("an Index read with a missing result"));
                    }
                    let value = self.read_place(place)?;
                    let address = self.lower_value_target_call(value, &read.call)?;
                    let address = address.ok_or_else(|| CompilerError::Internal {
                        message: "a void result from an Index projection".to_string(),
                    })?;
                    let received = self.value_representation(address)?;
                    let Some(access) = self.rooted_access(received) else {
                        return Err(CompilerError::Internal {
                            message: "an Index projection without a reference".to_string(),
                        });
                    };

                    Ok(Place {
                        root: PlaceRoot::Reference {
                            value: address,
                            access,
                        },
                        path: Vec::new(),
                    })
                }
                dir::SubscriptTarget::Member(_) => {
                    Err(self.unsupported("a member subscript projection"))
                }
            },

            // keep the place a view strip dereferences
            dir::Projection::Dereference(dereference) if self.is_view_strip(dereference)? => {
                Ok(place.clone())
            }

            // address the pointee behind a dereferenced input
            dir::Projection::Dereference(_) => {
                let value = self.read_place(place)?;
                let received = self.value_representation(value)?;
                let Some(access) = self.rooted_access(received) else {
                    return Err(CompilerError::Internal {
                        message: "a dereferenced pattern input outside a reference".to_string(),
                    });
                };

                Ok(Place {
                    root: PlaceRoot::Reference { value, access },
                    path: Vec::new(),
                })
            }

            // reject every other pattern projection
            other => Err(self.unsupported(format!("a '{}' pattern projection", other.name()))),
        }
    }

    /// Select one layout field of a pattern input place, projecting behind a held handle.
    fn project_pattern_field(
        &mut self,
        place: &Place,
        field: &dir::FieldResolution,
    ) -> CompilerResult<Place> {
        let index = self.member_field_index(field)?;
        let ty = self.lower_type(field.ty)?;

        // select the union member the field resolves against before projecting into it
        let receiver = field.receiver.ty();
        let member = match self.lower.indirection(receiver, &self.scope)? {
            Some(layer) => layer.stored,
            None => self.lower.stored(receiver)?,
        };
        let place = self.downcast_place(place.clone(), member)?;
        let mut place = self.through_handle(place)?;
        place.path.push(PlaceProjection::Field { field: index, ty });

        Ok(place)
    }

    /// Resolve one optional pattern input to the place of its present payload or default.
    fn lower_defaulted_input(
        &mut self,
        nested: dir::LocalNodeId<dir::Pattern>,
        place: &Place,
        default: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Place> {
        // read the representation the nested pattern was checked at and the default expression
        let exact = self.pattern_representation(nested)?;
        let default = default
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|message| CompilerError::Internal { message })?;
        let value = self.read_place(place)?;
        let value =
            self.lower_absent_fallback(value, exact, |lower| lower.lower_value(default).map(Some))?;

        Ok(Place::local(self.home(value)))
    }

    /// Unwrap one required pattern input into the place of its payload, trapping when absent.
    fn lower_required_input(
        &mut self,
        nested: dir::LocalNodeId<dir::Pattern>,
        place: &Place,
    ) -> CompilerResult<Place> {
        let exact = self.pattern_representation(nested)?;
        let value = self.read_place(place)?;
        let value = self.lower_absent_fallback(value, exact, |lower| {
            lower.builder.unreachable();

            Ok(None)
        })?;

        Ok(Place::local(self.home(value)))
    }

    /// Branch one possibly absent value into its present payload or fallback.
    pub(in crate::lower) fn lower_absent_fallback(
        &mut self,
        value: mir::Value,
        exact: mir::LocalNodeId<mir::Type>,
        fallback: impl FnOnce(&mut Self) -> CompilerResult<Option<mir::Value>>,
    ) -> CompilerResult<mir::Value> {
        // hand an input already at the exact representation through unchanged
        let representation = self.value_representation(value)?;
        if representation == exact {
            return Ok(value);
        }

        // take the fallback for a statically absent input
        if matches!(self.builder.tree().get(representation), mir::Type::Void) {
            return match fallback(self)? {
                Some(value) => Ok(value),
                None => Err(CompilerError::Internal {
                    message: "a statically absent required pattern input".to_string(),
                }),
            };
        }

        // stage the joined result slot and the branch blocks
        let slot = self.builder.local(exact, mir::Mutability::Immutable);
        let present_block = self.builder.block();
        let absent_block = self.builder.block();
        let join = self.builder.block();

        match self.builder.tree().get(representation).clone() {
            // split an optional variant on its undefined case
            mir::Type::Variant { cases, .. } => {
                let Some(absent) = self.absent_case(representation) else {
                    return Err(CompilerError::Internal {
                        message: "an optional pattern input without its undefined case".to_string(),
                    });
                };

                // route the undefined case to the fallback arm
                self.builder.variant_switch(
                    value,
                    Some(present_block),
                    vec![(absent, absent_block)],
                );

                // collect the cases that carry a payload
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
                    // extract the sole payload
                    [sole] => {
                        let payload = self.builder.variant_payload(value, *sole as u32);
                        self.builder.local_set(slot, payload);
                        self.builder.jump(join);
                    }
                    // rebuild the narrowed union case by case
                    _ => {
                        // open one arm block per surviving case
                        let mut arms = Vec::with_capacity(present.len());
                        for _ in &present {
                            arms.push(self.builder.block());
                        }

                        // switch each surviving case into its arm
                        let targets = present
                            .iter()
                            .copied()
                            .zip(arms.iter().copied())
                            .map(|(case, block)| (case as u32, block))
                            .collect();
                        self.builder.variant_switch(value, None, targets);

                        // rebuild each payload at its case in the narrowed variant
                        for (case, block) in present.iter().copied().zip(arms) {
                            self.builder.switch_to_block(block);
                            let payload = self.builder.variant_payload(value, case as u32);
                            let payload_type = cases[case].payload(self.builder.tree());
                            let narrowed = self.payload_case(exact, payload_type)?;
                            let rebuilt = self.builder.variant_new(exact, narrowed, Some(payload));
                            self.builder.local_set(slot, rebuilt);
                            self.builder.jump(join);
                        }
                    }
                }
            }
            // reject every other input shape
            _ => {
                return Err(CompilerError::Internal {
                    message: "an optional pattern input without an absent case".to_string(),
                });
            }
        }

        // fill the fallback arm, which may terminate or join
        self.builder.switch_to_block(absent_block);
        if let Some(value) = fallback(self)? {
            self.builder.local_set(slot, value);
            self.builder.jump(join);
        }

        // continue in the joined block
        self.builder.switch_to_block(join);

        Ok(self.builder.local_get(slot))
    }

    /// Return the lowered representation one pattern node was checked at.
    fn pattern_representation(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let node = pattern.into_global_any(self.source);
        let ty =
            self.source()
                .types
                .get_node_type_id(node)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("a missing type for pattern node {}", node.local_id.id),
                })?;

        self.lower_type(ty)
    }

    /// Return one nested pattern node inside this body's tree.
    pub(in crate::lower) fn pattern_node(
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
