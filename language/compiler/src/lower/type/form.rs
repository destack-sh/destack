use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{ModuleLowerer, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// One reference layer peeled from a sealed value type.
pub(in crate::lower) struct ReferenceLayer {
    /// The type the reference stores.
    pub(in crate::lower) stored: dir::GlobalTypeId,
    /// The access exposed through the reference.
    pub(in crate::lower) access: mir::Access,
}

impl TypeLowerer<'_, '_> {
    /// Lower one sealed form type through the memory form algebra.
    ///
    /// Forms nest: each ownership constructor wraps one reference layer around
    /// the lowering of its payload, so `&&T` stores a reference to a reference.
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
            dir::Form::Managed => {
                let pointee = self.lower_stored(form.value)?;

                Ok(self.insert_reference(
                    mir::ReferenceKind::Managed,
                    access.unwrap_or(mir::Access::Mutable),
                    pointee,
                ))
            }

            // borrowed layers carry their sealed lifetime and access
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
                let pointee = self.lower_stored(form.value)?;

                Ok(self.tree.intern_type(mir::Type::Reference {
                    kind: mir::ReferenceKind::Borrowed,
                    lifetime,
                    space: mir::Space::Local,
                    access: access.unwrap_or(borrow_access),
                    pointee,
                    nullability: mir::Nullability::None,
                }))
            }

            // raw layers reference their payload without safety
            dir::Form::Raw => {
                let pointee = self.lower_stored(form.value)?;

                Ok(self.insert_reference(
                    mir::ReferenceKind::Raw,
                    access.unwrap_or(mir::Access::Mutable),
                    pointee,
                ))
            }

            // owned means holding the stored value itself
            dir::Form::Owned => self.lower_stored(form.value),

            dir::Form::Placed { .. } => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an explicit placement".to_string(),
            })?,
        }
    }

    /// Lower one type as the storage behind a reference or owner.
    fn lower_stored(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        match self.lowerer.ty(id)? {
            // contextual this stores as the receiver's nominal representation
            dir::Type::This => Ok(self.lower_receiver()?.storage),
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
            // any other type stores as its value carrier
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
    ) -> CompilerResult<Option<ReferenceLayer>> {
        match self.ty(id)? {
            // form layers reference their payload by constructor
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed | dir::Form::Raw => Ok(Some(ReferenceLayer {
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

                    Ok(Some(ReferenceLayer {
                        stored: form.value,
                        access,
                    }))
                }
                // views narrow the layer beneath them
                dir::Form::Readonly => {
                    let layer = self.peel_reference(form.value)?;

                    Ok(layer.map(|layer| ReferenceLayer {
                        access: mir::Access::Readonly,
                        ..layer
                    }))
                }
                // owned values hold their storage directly
                dir::Form::Owned | dir::Form::Placed { .. } => Ok(None),
            },

            // reference-family nominals carry an implicit managed layer
            dir::Type::Application(_) => match self.has_reference_representation(id)? {
                true => Ok(Some(ReferenceLayer {
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
    /// Return whether one sealed type is carried by a reference at runtime.
    pub(in crate::lower) fn type_is_reference(
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
                    self.type_is_reference(form.value)
                }
            },

            // nullable unions ride their reference carrier; tagged unions are values
            dir::Type::Union(union) => match self.decompose_nullish_union(id.module_id, &union)? {
                Some((_, carrier)) => self.type_is_reference(carrier),
                None => Ok(false),
            },

            // bare bases answer by their family default
            other => Ok(self.base_default_ownership(&other)? == dir::Ownership::Managed),
        }
    }

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

            // value families are held directly
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::EnumMember(_)
            | dir::Type::Tuple(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => dir::Ownership::Owned,

            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a default form for the '{}' type", other.variant_name()),
            })?,
        })
    }

    /// Return the carrier storing one union of scalar literals.
    ///
    /// Each member widens through the language's literal widening; the union
    /// stores at the shared widened carrier. Returns None when the members
    /// disagree or are not all literals.
    pub(in crate::lower) fn literal_union_carrier(
        &self,
        module: ModuleId,
        union: &dir::UnionType,
    ) -> CompilerResult<Option<mir::Type>> {
        let mut carrier: Option<dir::Type> = None;
        for id in self.types(module)?.type_ids(union.elements) {
            let dir::Type::Literal(literal) = self.ty(*id)? else {
                return Ok(None);
            };
            let widened = literal.widen();
            match carrier {
                None => carrier = Some(widened),
                Some(agreed) if agreed == widened => {}
                Some(_) => return Ok(None),
            }
        }
        let Some(carrier) = carrier else {
            return Ok(None);
        };

        Ok(Some(self.scalar_type(&carrier)?))
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
