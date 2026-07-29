use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{ModuleLowerer, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// One reference layer peeled from a value type.
pub(in crate::lower) struct Indirection {
    /// The type the reference stores.
    pub(in crate::lower) stored: dir::GlobalTypeId,
    /// The access exposed through the reference.
    pub(in crate::lower) access: mir::Access,
}

impl TypeLowerer<'_, '_> {
    /// Lower one sealed form type through the memory form algebra.
    pub(in crate::lower) fn lower_form(
        &mut self,
        id: dir::GlobalTypeId,
        access: Option<mir::Access>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let dir::Type::Form(form) = self.lowerer.ty(id)? else {
            return Err(CompilerError::Internal {
                message: "lowering entered the form algebra outside a form type".to_string(),
            });
        };

        match form.form {
            // views narrow the access of the next reference layer inward
            dir::Form::Readonly => match self.lowerer.ty(form.value)? {
                dir::Type::Form(_) => self.lower_form(form.value, Some(mir::Access::Readonly)),
                // readonly over a pure value is a check-side fact only
                _ => self.lower_layer_value(form.value, Some(mir::Access::Readonly)),
            },

            // managed layers reference their stored payload on the local heap
            dir::Form::Managed => self.lower_reference(
                mir::ReferenceKind::Managed,
                mir::Lifetime::empty(),
                access.unwrap_or(mir::Access::Mutable),
                form.value,
            ),

            // borrowed layers carry their declared lifetime and access
            dir::Form::Borrowed(borrow) => {
                let Some(borrow) = self.lowerer.types(id.module_id)?.borrow_form_maybe(borrow)
                else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR is missing a borrow form".to_string(),
                    });
                };
                let (lifetime, borrow_access) = (borrow.lifetime, borrow.access);
                let lifetime = self
                    .lowerer
                    .lower_lifetime(lifetime, self.lifetime_parameters)?;
                let borrow_access = self.lowerer.borrow_access(borrow_access)?;

                self.lower_reference(
                    mir::ReferenceKind::Borrowed,
                    lifetime,
                    access.unwrap_or(borrow_access),
                    form.value,
                )
            }

            // raw layers reference their payload without safety
            dir::Form::Raw => self.lower_reference(
                mir::ReferenceKind::Raw,
                mir::Lifetime::empty(),
                access.unwrap_or(mir::Access::Mutable),
                form.value,
            ),

            // owned means holding the stored value itself
            dir::Form::Owned => self.lower_pointee(form.value),

            dir::Form::Placed { .. } => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an explicit placement".to_string(),
            })?,
        }
    }


    /// Lower one reference layer over its stored payload.
    ///
    /// Slice payloads fuse with the layer into one fat descriptor.
    pub(in crate::lower) fn lower_reference(
        &mut self,
        kind: mir::ReferenceKind,
        lifetime: mir::Lifetime,
        access: mir::Access,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // fuse unsized pointees with the layer into one fat descriptor
        if let Some(slice) = self.lowerer.slice_pointee(payload)? {
            let element = self.lower(slice.element)?;

            return Ok(self.tree.intern_type(mir::Type::Slice {
                kind,
                lifetime,
                element,
                space: mir::Space::Local,
                access,
                nullability: mir::Nullability::None,
            }));
        }
        // otherwise reference the lowered pointee thinly
        let pointee = self.lower_pointee(payload)?;

        Ok(self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime,
            space: mir::Space::Local,
            access,
            pointee,
            nullability: mir::Nullability::None,
        }))
    }

    /// Lower one type as the pointee behind a reference or owner.
    ///
    /// The pointee is the declared storage for nominal families and the
    /// value form for value families.
    pub(in crate::lower) fn lower_pointee(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let ty = self.lowerer.ty(id)?;

        // reference primitives store as their representation classes
        if let Some((item, arguments)) = ModuleLowerer::representation_item(&ty) {
            let symbol = self.lowerer.language_item_symbol(item)?;

            return Ok(self.lower_nominal(symbol, &arguments)?.storage);
        }

        match ty {
            // contextual this stores as the receiver's storage representation
            dir::Type::This => self.lower_receiver_storage(),
            // nominal storage is the declared type, not its value form
            dir::Type::Application(instance) => {
                let arguments = self
                    .lowerer
                    .types(id.module_id)?
                    .type_ids(instance.arguments)
                    .to_vec();
                let nominal = self.lower_nominal(instance.symbol, &arguments)?;

                Ok(nominal.storage)
            }
            // value families hold their value form directly
            _ => self.lower(id),
        }
    }

    /// Lower one non-form type at a narrowed access.
    fn lower_layer_value(
        &mut self,
        id: dir::GlobalTypeId,
        access: Option<mir::Access>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // reference-family bases receive the access on their implicit layer
        if let dir::Type::Application(instance) = self.lowerer.ty(id)?
            && self.lowerer.has_reference_representation(id)?
        {
            let arguments = self
                .lowerer
                .types(id.module_id)?
                .type_ids(instance.arguments)
                .to_vec();
            let pointee = self.lower_nominal(instance.symbol, &arguments)?.storage;

            return Ok(self.insert_reference(
                mir::ReferenceKind::Managed,
                access.unwrap_or(mir::Access::Mutable),
                pointee,
            ));
        }

        self.lower(id)
    }
}

impl ModuleLowerer<'_> {
    /// Return the outermost reference layer of one sealed value type, when one exists.
    pub(in crate::lower) fn peel_reference(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Indirection>> {
        match self.ty(id)? {
            // form layers reference their payload by constructor
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed | dir::Form::Raw => Ok(Some(Indirection {
                    stored: form.value,
                    access: mir::Access::Mutable,
                })),
                dir::Form::Borrowed(borrow) => {
                    let Some(borrow) = self.types(id.module_id)?.borrow_form_maybe(borrow) else {
                        return Err(CompilerError::Internal {
                            message: "checked DIR is missing a borrow form".to_string(),
                        });
                    };
                    let access = self.borrow_access(borrow.access)?;

                    Ok(Some(Indirection {
                        stored: form.value,
                        access,
                    }))
                }
                // views narrow the layer beneath them
                dir::Form::Readonly => {
                    let layer = self.peel_reference(form.value)?;

                    Ok(layer.map(|layer| Indirection {
                        access: mir::Access::Readonly,
                        ..layer
                    }))
                }
                // owned values hold their storage directly
                dir::Form::Owned | dir::Form::Placed { .. } => Ok(None),
            },

            // reference-family nominals carry an implicit managed layer
            dir::Type::Application(_) => match self.has_reference_representation(id)? {
                true => Ok(Some(Indirection {
                    stored: id,
                    access: mir::Access::Mutable,
                })),
                false => Ok(None),
            },

            // nullable unions reference through their carrier
            dir::Type::Union(union) => match self.decompose_nullish_union(id.module_id, &union)? {
                Some((_, carrier)) => self.peel_reference(carrier),
                None => Ok(None),
            },

            _ => Ok(None),
        }
    }

    /// Return the storage base beneath one value's owner and view layers.
    pub(in crate::lower) fn peel_owned(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.ty(id)? {
            // owners and views store their payload directly
            dir::Type::Form(form) => match form.form {
                dir::Form::Owned | dir::Form::Readonly => self.peel_owned(form.value),
                _ => Ok(id),
            },
            _ => Ok(id),
        }
    }

    /// Return whether one sealed type is carried by a reference at runtime.
    pub(in crate::lower) fn has_reference_representation(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        match self.ty(id)? {
            // form layers answer by their outermost constructor
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed | dir::Form::Borrowed(_) | dir::Form::Raw => Ok(true),
                // owned values hold their storage directly
                dir::Form::Owned => Ok(false),
                // views and placement answer for the layer beneath
                dir::Form::Readonly | dir::Form::Placed { .. } => {
                    self.has_reference_representation(form.value)
                }
            },

            // nullable unions ride their reference carrier; tagged unions are values
            dir::Type::Union(union) => match self.decompose_nullish_union(id.module_id, &union)? {
                Some((_, carrier)) => self.has_reference_representation(carrier),
                None => Ok(false),
            },

            // bare bases answer by their family default
            other => Ok(self.base_default_ownership(&other)? == dir::Ownership::Managed),
        }
    }

    /// Return the default ownership of one base type family.
    fn base_default_ownership(&self, base: &dir::Type) -> CompilerResult<dir::Ownership> {
        Ok(match base {
            // reference families default to managed values
            dir::Type::Shape(_)
            | dir::Type::Array(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Object => dir::Ownership::Managed,

            // nominal defaults follow their declaration family
            dir::Type::Application(instance) => match self.definition(instance.symbol)? {
                Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                    dir::Ownership::Managed
                }
                _ => dir::Ownership::Owned,
            },

            // string and bigint primitives are held through their representation classes
            dir::Type::Primitive(dir::PrimitiveType::String | dir::PrimitiveType::Bigint) => {
                dir::Ownership::Managed
            }

            // value families are held directly
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Tuple(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => dir::Ownership::Owned,

            // precise variants share their owner's representation
            dir::Type::Variant(variant) => {
                let owner = self.ty(variant.owner)?;

                return self.base_default_ownership(&owner);
            }

            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a default form for the '{}' type", other.variant_name()),
            })?,
        })
    }

    /// Split one union into its nullish bits and single reference carrier.
    ///
    /// Returns None when the union does not reduce to one nullable reference.
    pub(in crate::lower) fn decompose_nullish_union(
        &self,
        module: ModuleId,
        union: &dir::UnionType,
    ) -> CompilerResult<Option<(mir::Nullability, dir::GlobalTypeId)>> {
        let mut nullability = (false, false);
        let mut carriers = Vec::new();
        for id in self.types(module)?.type_ids(union.elements) {
            match self.ty(*id)? {
                // nullish members ride the carrier's spare values
                dir::Type::Null => nullability.0 = true,
                dir::Type::Undefined => nullability.1 = true,
                _ => carriers.push(*id),
            }
        }

        // one reference member carries the whole union in its niches
        let [carrier] = carriers.as_slice() else {
            return Ok(None);
        };
        if !self.has_reference_representation(*carrier)? {
            return Ok(None);
        }
        let nullability = match nullability {
            (true, true) => mir::Nullability::NullOrUndefined,
            (true, false) => mir::Nullability::Null,
            (false, true) => mir::Nullability::Undefined,
            (false, false) => {
                return Err(CompilerError::Internal {
                    message: "checked DIR sealed a single-member union".to_string(),
                });
            }
        };

        Ok(Some((nullability, *carrier)))
    }
}

impl TypeLowerer<'_, '_> {
    /// Widen one lowered reference type with the nullish values it admits.
    pub(in crate::lower) fn insert_nullable_reference(
        &mut self,
        reference: mir::LocalNodeId<mir::Type>,
        nullability: mir::Nullability,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let mir::Type::Reference {
            kind,
            lifetime,
            space,
            access,
            pointee,
            ..
        } = self.tree.get(reference).clone()
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a union beyond a nullable reference".to_string(),
            })?;
        };

        Ok(self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime,
            space,
            access,
            pointee,
            nullability,
        }))
    }

    /// Insert one reference type over a pointee.
    pub(in crate::lower) fn insert_reference(
        &mut self,
        kind: mir::ReferenceKind,
        access: mir::Access,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Local,
            access,
            pointee,
            nullability: mir::Nullability::None,
        })
    }
}

impl ModuleLowerer<'_> {
    /// Return the access spelled by one sealed borrow access singleton.
    pub(in crate::lower) fn borrow_access(
        &self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Access> {
        let dir::MemoryLiteral::Access(access) =
            self.memory_literal(access, dir::MemoryParameter::Access)?
        else {
            return Err(CompilerError::Internal {
                message: "checked DIR decoded a borrow access in the wrong domain".to_string(),
            });
        };

        Ok(match access {
            dir::Access::Readonly => mir::Access::Readonly,
            dir::Access::Mutable => mir::Access::Mutable,
            dir::Access::Exclusive => mir::Access::Exclusive,
        })
    }
}
