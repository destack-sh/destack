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
        /// The referenced aggregate type.
        pointee: mir::LocalNodeId<mir::Type>,
        /// The access exposed through the reference.
        access: mir::Access,
    },
}

/// One resolved place: a base with its projection path.
pub(in crate::lower) struct Place {
    /// The base holding the outermost aggregate.
    pub(in crate::lower) root: PlaceRoot,
    /// The field indices projecting from the base to the place.
    pub(in crate::lower) path: Vec<u32>,
}

impl FunctionLowerer<'_, '_> {
    /// Return the place selected by one checked place resolution.
    pub(in crate::lower) fn place(
        &mut self,
        place: &dir::PlaceResolution,
    ) -> CompilerResult<Place> {
        let Ok(source) = place.source.local_id.try_into_typed::<dir::Expression>() else {
            return Err(CompilerError::Internal {
                message: "checked DIR placed a non-expression node".to_string(),
            });
        };

        match &place.storage {
            // value = x
            dir::Storage::Binding { symbol } => {
                let local = self.place_base(symbol.local_id)?;

                Ok(Place {
                    root: PlaceRoot::Local(local),
                    path: Vec::new(),
                })
            }
            // value.field = x
            dir::Storage::Field { receiver, field } => {
                // the final projection comes from the checked storage behind any form
                let stored = match self.lowerer.peel_reference(*receiver)? {
                    Some(layer) => layer.stored,
                    None => self.lowerer.peel_owned(*receiver)?,
                };
                let dir::Type::Instance(instance) = self.lowerer.ty(stored)? else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a write into a structural receiver".to_string(),
                    }
                    .into());
                };
                let index = self.projection_field_index(&instance, field)?;

                // the receiver chain reads through its checked member resolutions
                let dir::Expression::Member { left, .. } = *self.lowerer.source().tree().get(source) else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR fielded a non-member place".to_string(),
                    });
                };
                let mut place = self.receiver_place(left)?;
                place.path.push(index);

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
        // reference receivers root the place at their reference value
        let ty = self.lowerer.coerced_type_id(expression)?;
        if let Some(layer) = self.lowerer.peel_reference(ty)? {
            let pointee = self.reference_pointee(layer.stored)?;
            let value = self.lower_expression(expression)?;

            return Ok(Place {
                root: PlaceRoot::Reference {
                    value,
                    pointee,
                    access: layer.access,
                },
                path: Vec::new(),
            });
        }

        match *self.lowerer.source().tree().get(expression) {
            // base.field keeps projecting
            dir::Expression::Member { left, .. } => {
                let index = self.member_field_index(expression)?;
                let mut place = self.receiver_place(left)?;
                place.path.push(index);

                Ok(place)
            }
            // the identifier names the base binding
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.lowerer.source);
                let symbol = self.lowerer.resolved_symbol(node)?;
                let local = self.place_base(symbol.local_id)?;

                Ok(Place {
                    root: PlaceRoot::Local(local),
                    path: Vec::new(),
                })
            }
            ref other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a write through '{}' expressions", other.variant_name()),
            }
            .into()),
        }
    }

    /// Return the lowered pointee behind one reference base type.
    fn reference_pointee(
        &self,
        base: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let dir::Type::Instance(ref instance) = self.lowerer.ty(base)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a write through a structural reference".to_string(),
            }
            .into());
        };

        Ok(self.lowerer.nominal(instance)?.ty)
    }

    /// Return the mutable local behind one place base symbol.
    fn place_base(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Local>> {
        let Some(Binding::Local(local)) = self.values.get(&symbol).copied() else {
            return Err(CompilerError::Internal {
                message: "checked DIR wrote through a binding without a mutable local".to_string(),
            });
        };

        Ok(local)
    }

    /// Return the declaration field index behind one checked projection.
    fn projection_field_index(
        &self,
        instance: &dir::GenericInstance,
        field: &dir::ProjectionField,
    ) -> CompilerResult<u32> {
        let nominal = self.lowerer.nominal(instance)?;
        let index = match field {
            dir::ProjectionField::Key(key) => {
                nominal.fields.iter().position(|field| field.key == *key)
            }
            dir::ProjectionField::Member(symbol) => {
                let symbol = symbol.local_id;

                nominal
                    .fields
                    .iter()
                    .position(|field| field.symbol == symbol)
            }
        };

        index
            .map(|index| index as u32)
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR projected a field missing from its nominal".to_string(),
            })
    }

    /// Project one field address and its type through an aggregate reference.
    pub(in crate::lower) fn field_address(
        &mut self,
        reference: mir::Value,
        pointee: mir::LocalNodeId<mir::Type>,
        index: u32,
        access: mir::Access,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // the pointee struct names the projected field type
        let mir::Type::Struct { fields, .. } = self.builder.tree().get(pointee) else {
            return Err(CompilerError::Internal {
                message: "lowered MIR addressed a field outside a struct pointee".to_string(),
            });
        };
        let Some(field) = fields.get(index as usize).copied() else {
            return Err(CompilerError::Internal {
                message: "lowered MIR addressed a field missing from its pointee".to_string(),
            });
        };
        let field = self.builder.tree().get(field).ty;

        // interior addresses borrow from the object reference
        let address = self.builder.tree_mut().insert(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Local,
            access,
            pointee: field,
            nullability: mir::Nullability::None,
        });
        let address = self.builder.field_addr(reference, index, address);

        Ok((address, field))
    }

    /// Read the current value of one place.
    pub(in crate::lower) fn read_place(&mut self, place: &Place) -> CompilerResult<mir::Value> {
        match place.root {
            // locals project by value
            PlaceRoot::Local(local) => {
                let mut value = self.builder.local_get(local);
                for index in &place.path {
                    value = self.builder.field_get(value, *index);
                }

                Ok(value)
            }
            // references project by address and load the leaf
            PlaceRoot::Reference {
                value,
                pointee,
                access,
            } => {
                let (address, leaf) = self.address_path(value, pointee, access, &place.path)?;

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
            // locals rebuild the aggregates on the path
            PlaceRoot::Local(local) => {
                self.write_local_place(local, &place.path, value);

                Ok(())
            }
            // references store through the leaf address
            PlaceRoot::Reference {
                value: reference,
                pointee,
                access,
            } => {
                let (address, _) = self.address_path(reference, pointee, access, &place.path)?;
                self.builder.store(address, value);

                Ok(())
            }
        }
    }

    /// Project one address chain to the leaf of a path.
    fn address_path(
        &mut self,
        reference: mir::Value,
        pointee: mir::LocalNodeId<mir::Type>,
        access: mir::Access,
        path: &[u32],
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        if path.is_empty() {
            return Err(CompilerError::Internal {
                message: "lowered MIR addressed a reference place without a field".to_string(),
            });
        }

        // descend one address per path element
        let mut current = (reference, pointee);
        for index in path {
            current = self.field_address(current.0, current.1, *index, access)?;
        }

        Ok(current)
    }

    /// Write one value into a local place, rebuilding the aggregates on the path.
    fn write_local_place(
        &mut self,
        local: mir::LocalNodeId<mir::Local>,
        path: &[u32],
        value: mir::Value,
    ) {
        // bare locals store directly
        if path.is_empty() {
            self.builder.local_set(local, value);

            return;
        }

        // load the aggregate at every level above the written field
        let mut loaded = vec![self.builder.local_get(local)];
        for index in &path[..path.len() - 1] {
            let inner = self.builder.field_get(loaded[loaded.len() - 1], *index);
            loaded.push(inner);
        }

        // rebuild each level bottom-up around the written value
        let mut value = value;
        for (level, index) in path.iter().enumerate().rev() {
            value = self.builder.field_set(loaded[level], *index, value);
        }
        self.builder.local_set(local, value);
    }
}
