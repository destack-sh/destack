use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

/// One resolved place base.
pub(in crate::lower) enum PlaceRoot {
    /// A mutable local holding the base aggregate.
    Local(mir::LocalNodeId<mir::Local>),
    /// A reference value addressing heap storage.
    Reference {
        /// The reference value.
        value: mir::Value,
        /// The access exposed through the reference.
        access: mir::Access,
    },
}

/// One typed field projection within a place.
pub(in crate::lower) struct PlaceProjection {
    /// The field index within its aggregate.
    pub(in crate::lower) field: u32,
    /// The projected value type.
    pub(in crate::lower) ty: mir::LocalNodeId<mir::Type>,
}

/// One resolved place: a base with its projection path.
pub(in crate::lower) struct Place {
    /// The base holding the outermost aggregate.
    pub(in crate::lower) root: PlaceRoot,
    /// The typed projections from the base to the place.
    pub(in crate::lower) path: Vec<PlaceProjection>,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Return the place selected by one assignment resolution.
    pub(in crate::lower) fn place(
        &mut self,
        assignment: &dir::AssignmentDecision,
    ) -> CompilerResult<Place> {
        // read the target expression the assignment writes
        let Ok(source) = assignment
            .target
            .local_id
            .try_into_typed::<dir::Expression>()
        else {
            return Err(CompilerError::Internal {
                message: "a non-expression node in a place".to_string(),
            });
        };

        match &assignment.write {
            // write a plain binding
            dir::WriteResolution::Binding { symbol, .. } => self.binding_place(symbol.local_id),
            // write a field of the receiver's place
            dir::WriteResolution::Member(resolution) => {
                // require a direct, unadjusted field member
                let dir::OperationResolution::One(access) = resolution else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a field write on a union receiver".to_string(),
                    }
                    .into());
                };
                let dir::MemberTarget::Field(field) = &access.target else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a write through non-field member storage".to_string(),
                    }
                    .into());
                };
                let dir::MemberReceiver::Direct(receiver) = &field.receiver else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a field write through dynamic dispatch".to_string(),
                    }
                    .into());
                };
                if !receiver.adjustments.is_empty() {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a field write through receiver adjustments".to_string(),
                    }
                    .into());
                }

                // lower the storage selection
                let index = self.member_field_index(field)?;
                let ty = self.lower_type(field.ty)?;

                // read the receiver chain through its member resolutions
                let dir::Expression::Member { left, .. } = *self.source().tree().get(source) else {
                    return Err(CompilerError::Internal {
                        message: "a field write on a non-member place".to_string(),
                    });
                };

                // project the written field onto the receiver's place
                let mut place = self.receiver_place(left)?;
                place.path.push(PlaceProjection { field: index, ty });

                Ok(place)
            }
            // reject every other write target
            other => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("a write through {} storage", other.name()),
            }
            .into()),
        }
    }

    /// Return the place behind one receiver expression, peeling its member chain.
    fn receiver_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Place> {
        // root the place at the reference value for reference receivers
        let ty = self.node_type_id(expression)?;
        if let Some(layer) = self.lower.peel_indirection(ty)? {
            let value = self.lower_expression(expression)?;

            return Ok(Place {
                root: PlaceRoot::Reference {
                    value,
                    access: layer.access,
                },
                path: Vec::new(),
            });
        }

        match *self.source().tree().get(expression) {
            // project one more field of the member chain
            dir::Expression::Member { left, .. } => {
                // require a direct field member
                let resolution = self.member_decision(expression)?;
                let dir::OperationResolution::One(access) = &resolution else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a place projection on a union receiver".to_string(),
                    }
                    .into());
                };
                let dir::MemberTarget::Field(field) = &access.target else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a place projection through non-field storage".to_string(),
                    }
                    .into());
                };

                // project this field onto the receiver's place
                let index = self.member_field_index(field)?;
                let ty = self.lower_type(self.node_type_id(expression)?)?;
                let mut place = self.receiver_place(left)?;
                place.path.push(PlaceProjection { field: index, ty });

                Ok(place)
            }
            // root the place at the receiver's home
            dir::Expression::This => {
                let Some(binding) = self.this else {
                    return Err(CompilerError::Internal {
                        message: "a this outside a method body".to_string(),
                    });
                };

                self.binding_home(binding)
            }
            // bind the place base at the identifier
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;

                self.binding_place(symbol.local_id)
            }
            // reject every other place expression
            ref other => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("a write through '{}' expressions", other.variant_name()),
            }
            .into()),
        }
    }

    /// Return the place behind one binding symbol.
    fn binding_place(&self, symbol: dir::LocalSymbolId) -> CompilerResult<Place> {
        match self.values.get(&symbol).copied() {
            Some(binding) => self.binding_home(binding),
            // reject a symbol without a binding
            None => Err(CompilerError::Internal {
                message: "a write through a binding without a mutable home".to_string(),
            }),
        }
    }

    /// Return the writable place rooted at one binding's home.
    fn binding_home(&self, binding: Binding) -> CompilerResult<Place> {
        match binding {
            // mutable locals root their own place
            Binding::Local(local) => Ok(Place {
                root: PlaceRoot::Local(local),
                path: Vec::new(),
            }),
            // captured bindings live behind their frame reference
            Binding::Captured { frame, field, ty } => Ok(Place {
                root: PlaceRoot::Reference {
                    value: frame,
                    access: mir::Access::Mutable,
                },
                path: vec![PlaceProjection { field, ty }],
            }),
            // reject a pure value
            Binding::Value(_) => Err(CompilerError::Internal {
                message: "a write through a binding without a mutable home".to_string(),
            }),
        }
    }

    /// Project one field address through an aggregate reference.
    pub(in crate::lower) fn emit_field_address(
        &mut self,
        reference: mir::Value,
        index: u32,
        field: mir::LocalNodeId<mir::Type>,
        access: mir::Access,
    ) -> mir::Value {
        // interior addresses inherit the base reference's storage
        let mut pointee = field;
        let mut storage = mir::Storage::LocalHeap;
        if let Some(ty) = self.builder.value_type(reference)
            && let Some(base) = self.builder.tree().get(ty).reference_storage()
        {
            storage = base;
        }

        // project an uninitialized field out of an uninitialized aggregate
        if let Some(ty) = self.builder.value_type(reference)
            && let mir::Type::Reference {
                pointee: aggregate, ..
            } = self.builder.tree().get(ty)
            && matches!(
                self.builder.tree().get(*aggregate),
                mir::Type::Uninit { .. }
            )
        {
            pointee = self
                .builder
                .tree_mut()
                .intern_type(mir::Type::Uninit { value: field });
        }

        // borrow interior addresses from the object reference
        let address = self.builder.tree_mut().intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            storage,
            access,
            pointee,
            nullability: mir::Nullability::None,
        });

        self.builder.field_addr(reference, index, address)
    }

    /// Read the current value of one place.
    pub(in crate::lower) fn read_place(&mut self, place: &Place) -> CompilerResult<mir::Value> {
        match place.root {
            // project locals by value
            PlaceRoot::Local(local) => {
                let mut value = self.builder.local_get(local);
                for projection in &place.path {
                    value = self.builder.field_get(value, projection.field);
                }

                Ok(value)
            }
            // project references by address and load the leaf
            PlaceRoot::Reference { value, access } => {
                let (address, leaf) = self.emit_address_path(value, access, &place.path)?;

                Ok(self.builder.load(address, leaf))
            }
        }
    }

    /// Write one value through a place.
    pub(in crate::lower) fn write_place(
        &mut self,
        place: &Place,
        value: mir::Value,
    ) -> CompilerResult<()> {
        match place.root {
            // rebuild the aggregates on the path for locals
            PlaceRoot::Local(local) => {
                self.write_local_place(local, &place.path, value);

                Ok(())
            }
            // store through the leaf address for references
            PlaceRoot::Reference {
                value: reference,
                access,
            } => {
                let (address, _) = self.emit_address_path(reference, access, &place.path)?;
                self.builder.store(address, value);

                Ok(())
            }
        }
    }

    /// Project one address chain to the leaf of a path.
    fn emit_address_path(
        &mut self,
        reference: mir::Value,
        access: mir::Access,
        path: &[PlaceProjection],
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let Some((first, remaining)) = path.split_first() else {
            return Err(CompilerError::Internal {
                message: "a reference place addressed without a field".to_string(),
            });
        };

        // descend one address per path element
        let mut current = self.emit_field_address(reference, first.field, first.ty, access);
        let mut leaf = first.ty;
        for projection in remaining {
            current = self.emit_field_address(current, projection.field, projection.ty, access);
            leaf = projection.ty;
        }

        Ok((current, leaf))
    }

    /// Write one value into a local place, rebuilding the aggregates on the path.
    fn write_local_place(
        &mut self,
        local: mir::LocalNodeId<mir::Local>,
        path: &[PlaceProjection],
        value: mir::Value,
    ) {
        // store bare locals directly
        if path.is_empty() {
            self.builder.local_set(local, value);

            return;
        }

        // load the aggregate at every level above the written field
        let mut loaded = vec![self.builder.local_get(local)];
        for projection in &path[..path.len() - 1] {
            let level = self
                .builder
                .field_get(loaded[loaded.len() - 1], projection.field);
            loaded.push(level);
        }

        // rebuild each level bottom-up around the written value
        let mut value = value;
        for (level, projection) in path.iter().enumerate().rev() {
            value = self
                .builder
                .field_set(loaded[level], projection.field, value);
        }

        // store the rebuilt aggregate back into the local
        self.builder.local_set(local, value);
    }

    /// Lower one expression into a borrow of its place or reference.
    pub(in crate::lower) fn lower_borrowed_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // borrow reference sources as a kind change
        let source = self.node_type_id(expression)?;
        if self.lower.has_indirect_representation(source)? {
            let value = self.lower_expression_value(expression)?;

            return Ok(self.builder.cast(mir::CastOperator::Bitcast, value, target));
        }

        // borrow the storage holding value sources
        match self.source().tree().get(expression).clone() {
            // borrow the binding behind a name
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;
                match self.values.get(&symbol.local_id).copied() {
                    // borrow a mutable local directly
                    Some(Binding::Local(local)) => Ok(self.builder.local_addr(local, target)),
                    // borrow captured bindings at their frame field
                    Some(Binding::Captured { frame, field, .. }) => {
                        Ok(self.builder.field_addr(frame, field, target))
                    }
                    // give borrowed parameters a frame home on first borrow
                    Some(Binding::Value(value)) => {
                        let ty = self.lower.symbol_type(symbol)?;
                        let slot = self.lower_type(ty)?;
                        let local = self.builder.local(slot, mir::Mutability::Mutable);
                        self.builder.local_set(local, value);
                        self.values.insert(symbol.local_id, Binding::Local(local));

                        Ok(self.builder.local_addr(local, target))
                    }
                    // reject a borrow of a module binding
                    None => Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a borrow of a module binding".to_string(),
                    }
                    .into()),
                }
            }
            // reject every other borrow source
            other => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("a borrow of a '{}' expression", other.variant_name()),
            }
            .into()),
        }
    }
}
