use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::body::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

/// One resolved place base.
pub(in crate::lower) enum PlaceRoot {
    /// A mutable local slot holding the base aggregate.
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
        assignment: &dir::AssignmentResolution,
    ) -> CompilerResult<Place> {
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
            // value = x
            dir::WriteResolution::Binding { symbol, .. } => self.binding_place(symbol.local_id),
            // value.field = x
            dir::WriteResolution::Member(resolution) => {
                let dir::OperationResolution::One(access) = resolution else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a field write on a union receiver".to_string(),
                    }
                    .into());
                };
                let dir::MemberTarget::Field(field) = &access.target else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a write through non-field member storage".to_string(),
                    }
                    .into());
                };
                let dir::MemberReceiver::Direct(receiver) = &field.receiver else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a field write through dynamic dispatch".to_string(),
                    }
                    .into());
                };
                if !receiver.adjustments.is_empty() {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
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
                let mut place = self.receiver_place(left)?;
                place.path.push(PlaceProjection { field: index, ty });

                Ok(place)
            }
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a write through {other:?} storage"),
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
        if let Some(layer) = self.lowerer.peel_indirection(ty, &self.type_substitution)? {
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
            // base.field keeps projecting
            dir::Expression::Member { left, .. } => {
                let resolution = self.member_resolution(expression)?;
                let dir::OperationResolution::One(access) = &resolution else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a place projection on a union receiver".to_string(),
                    }
                    .into());
                };
                let dir::MemberTarget::Field(field) = &access.target else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a place projection through non-field storage".to_string(),
                    }
                    .into());
                };
                let index = self.member_field_index(field)?;
                let ty = self.lower_type(self.node_type_id(expression)?)?;
                let mut place = self.receiver_place(left)?;
                place.path.push(PlaceProjection { field: index, ty });

                Ok(place)
            }
            // bind the place base at the identifier
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lowerer.resolved_symbol(node)?;

                self.binding_place(symbol.local_id)
            }
            ref other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a write through '{}' expressions", other.variant_name()),
            }
            .into()),
        }
    }

    /// Return the place behind one binding symbol.
    fn binding_place(&self, symbol: dir::LocalSymbolId) -> CompilerResult<Place> {
        match self.values.get(&symbol).copied() {
            // mutable locals root their own place
            Some(Binding::Local(local)) => Ok(Place {
                root: PlaceRoot::Local(local),
                path: Vec::new(),
            }),
            // captured bindings live behind their frame reference
            Some(Binding::Captured { frame, field, ty }) => Ok(Place {
                root: PlaceRoot::Reference {
                    value: frame,
                    access: mir::Access::Mutable,
                },
                path: vec![PlaceProjection { field, ty }],
            }),
            // pure values and unbound symbols have no writable home
            Some(Binding::Value(_)) | None => Err(CompilerError::Internal {
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
        // borrow interior addresses from the object reference
        let address = self.builder.tree_mut().intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            storage: mir::Storage::Heap(mir::Space::Local),
            access,
            pointee: field,
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
        self.builder.local_set(local, value);
    }
}
