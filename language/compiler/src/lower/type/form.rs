use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{ModuleLowerer, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// One indirect layer peeled from a value type.
pub(in crate::lower) struct Indirection {
    /// The type stored behind the indirection.
    pub(in crate::lower) stored: dir::GlobalTypeId,
    /// The access exposed through the indirection.
    pub(in crate::lower) access: mir::Access,
}

impl TypeLowerer<'_, '_> {
    /// Lower one form type through the memory form algebra.
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
            // narrow the access of the next indirect layer inward
            dir::Form::Readonly => match self.lowerer.ty(form.value)? {
                // carry the narrowed access into the layer below
                dir::Type::Form(_) => self.lower_form(form.value, Some(mir::Access::Readonly)),
                // lower readonly over a pure value at the narrowed access
                _ => self.lower_layer_value(form.value, Some(mir::Access::Readonly)),
            },

            // reference the stored payload of managed layers in their referent place
            dir::Form::Managed { place } => {
                // enter the referent place for the duration of the layer
                let saved = self.space;
                if let Some(space) = self.lowerer.place_space(place)? {
                    self.space = ModuleLowerer::mir_space(space);
                }

                let lowered = self.lower_reference(
                    mir::ReferenceKind::Managed,
                    mir::Lifetime::empty(),
                    access.unwrap_or(mir::Access::Mutable),
                    form.value,
                );
                self.space = saved;

                lowered
            }

            // carry the declared lifetime, access, and referent place of borrowed layers
            dir::Form::Borrowed(borrow) => {
                let Some(borrow) = self.lowerer.types(id.module_id)?.borrow_form_maybe(borrow)
                else {
                    return Err(CompilerError::Internal {
                        message: "missing a borrow form".to_string(),
                    });
                };

                // split the declared region into its extent and referent place
                let (region, borrow_access) = (borrow.region, borrow.access);
                let (lifetime, spaces) = match self.lowerer.ty(region)? {
                    dir::Type::Region(pair) => (pair.extent, Some(pair.space)),
                    _ => (region, None),
                };

                // lower the extent and access the borrow declares
                let lifetime = self
                    .lowerer
                    .lower_lifetime(lifetime, self.lifetime_parameters)?;
                let borrow_access = self.lowerer.borrow_access(borrow_access)?;

                // select the reference storage from the referent place
                //  parametric places keep the canonical ambient space
                let saved = self.space;
                if let Some(spaces) = spaces {
                    if let Some(space) = self.lowerer.place_space(spaces)? {
                        self.space = ModuleLowerer::mir_space(space);
                    } else if matches!(self.lowerer.ty(spaces)?, dir::Type::Union(_)) {
                        return Err(CompilerError::Internal {
                            message: "borrow region carries a space join".to_string(),
                        });
                    }
                }

                let lowered = self.lower_reference(
                    mir::ReferenceKind::Borrowed,
                    lifetime,
                    access.unwrap_or(borrow_access),
                    form.value,
                );
                self.space = saved;

                lowered
            }

            // produce process-local machine pointers for raw layers
            dir::Form::Raw => {
                self.lower_pointer(form.value, access.unwrap_or(mir::Access::Mutable))
            }

            // fuse owned fat references into one unique layer
            dir::Form::Owned if self.lowerer.is_reference_representation(form.value)? => self
                .lower_reference(
                    mir::ReferenceKind::Unique,
                    mir::Lifetime::empty(),
                    access.unwrap_or(mir::Access::Exclusive),
                    form.value,
                ),

            // hold storage directly for owned value families
            dir::Form::Owned => self.lower_pointee(form.value),
        }
    }

    /// Lower one reference layer over its stored payload, fusing fat payloads into one descriptor.
    pub(in crate::lower) fn lower_reference(
        &mut self,
        kind: mir::ReferenceKind,
        lifetime: mir::Lifetime,
        access: mir::Access,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // fuse dynamic payload references into the erased descriptor
        if let dir::Type::Dynamic(dynamic) = self.lowerer.ty(payload)? {
            let constraint = self.lower_dynamic_constraint(dynamic.constraint)?;

            return Ok(self.tree.intern_type(mir::Type::Dynamic {
                kind,
                lifetime,
                constraint,
                storage: mir::Storage::heap(self.space),
                access,
                nullability: mir::Nullability::None,
            }));
        }

        // fuse callable environment references into the closure descriptor
        if let dir::Type::Function(function) = self.lowerer.ty(payload)? {
            let signature = self.lower_callable_signature(function.signature)?;
            let multiplicity = self.lowerer.callable_multiplicity(function.receiver)?;

            return Ok(self.tree.intern_type(mir::Type::Function {
                signature,
                multiplicity,
                kind,
                lifetime,
                storage: mir::Storage::heap(self.space),
                access,
                nullability: mir::Nullability::None,
            }));
        }

        // fuse slice values with the layer into one fat descriptor
        if let Some(slice) = self.lowerer.slice_pointee(payload)? {
            let element = self.lower(slice.element)?;

            return Ok(self.tree.intern_type(mir::Type::Slice {
                kind,
                lifetime,
                element,
                storage: mir::Storage::heap(self.space),
                access,
                nullability: mir::Nullability::None,
            }));
        }

        // otherwise reference the lowered pointee thinly
        let pointee = self.lower_pointee(payload)?;

        Ok(self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime,
            storage: mir::Storage::heap(self.space),
            access,
            pointee,
            nullability: mir::Nullability::None,
        }))
    }

    /// Lower one raw form into a process-local machine pointer.
    fn lower_pointer(
        &mut self,
        payload: dir::GlobalTypeId,
        access: mir::Access,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let pointee = self.lower_pointee(payload)?;

        Ok(self.tree.intern_type(mir::Type::Pointer {
            pointee,
            access,
            nullability: mir::Nullability::None,
        }))
    }

    /// Lower one type as the pointee behind a reference or owner.
    pub(in crate::lower) fn lower_pointee(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let ty = self.lowerer.ty(id)?;

        // store reference primitives as their representation classes
        if let Some(item) = ModuleLowerer::representation_item(&ty) {
            let symbol = self.lowerer.language_item_symbol(item)?;
            let source = self.lowerer.symbol_type(symbol)?;

            return Ok(self.lower_nominal(source)?.storage);
        }

        match ty {
            // reject contextual this, which materialization resolves before lowering
            dir::Type::This => Err(CompilerError::Internal {
                message: "a contextual this was never materialized".to_string(),
            }),
            // store nominals as their declared type
            dir::Type::Application(_) => {
                let nominal = self.lower_nominal(id)?;

                Ok(nominal.storage)
            }
            // store bare named nominals as their declared type
            dir::Type::Reference(_) => {
                let nominal = self.lower_nominal(id)?;

                Ok(nominal.storage)
            }
            // store anonymous object types as their concrete struct
            dir::Type::Object(shape) => self.lower_object_struct(&shape, id.module_id),
            // hold value families as their value form
            _ => self.lower(id),
        }
    }

    /// Lower one non-form type at a narrowed access.
    fn lower_layer_value(
        &mut self,
        id: dir::GlobalTypeId,
        access: Option<mir::Access>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // reference families receive the access on their implicit managed layer
        if self.lowerer.has_indirect_representation(id)? {
            return self.lower_reference(
                mir::ReferenceKind::Managed,
                mir::Lifetime::empty(),
                access.unwrap_or(mir::Access::Mutable),
                id,
            );
        }

        self.lower(id)
    }
}

impl ModuleLowerer<'_> {
    /// Return the outermost indirect layer of one value type, when one exists.
    pub(in crate::lower) fn peel_indirection(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Indirection>> {
        match self.ty(id)? {
            // form layers select their indirection by constructor
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed { .. } | dir::Form::Raw => Ok(Some(Indirection {
                    stored: form.value,
                    access: mir::Access::Mutable,
                })),
                dir::Form::Borrowed(borrow) => {
                    let Some(borrow) = self.types(id.module_id)?.borrow_form_maybe(borrow) else {
                        return Err(CompilerError::Internal {
                            message: "missing a borrow form".to_string(),
                        });
                    };
                    let access = self.borrow_access(borrow.access)?;

                    Ok(Some(Indirection {
                        stored: form.value,
                        access,
                    }))
                }
                // narrow the layer beneath views
                dir::Form::Readonly => {
                    let layer = self.peel_indirection(form.value)?;

                    Ok(layer.map(|layer| Indirection {
                        access: mir::Access::Readonly,
                        ..layer
                    }))
                }
                // owned fat references carry one exclusive unique layer
                dir::Form::Owned if self.is_reference_representation(form.value)? => {
                    Ok(Some(Indirection {
                        stored: form.value,
                        access: mir::Access::Exclusive,
                    }))
                }
                // owned values hold storage directly
                dir::Form::Owned => Ok(None),
            },

            // nullable unions reference through their representation
            dir::Type::Union(union) => match self.decompose_nullish_union(id.module_id, &union)? {
                Some((_, representation)) => self.peel_indirection(representation),
                None => Ok(None),
            },

            // bare reference families carry an implicit managed layer
            _ => match self.has_indirect_representation(id)? {
                true => Ok(Some(Indirection {
                    stored: id,
                    access: mir::Access::Mutable,
                })),
                false => Ok(None),
            },
        }
    }

    /// Return the storage base beneath one value's owner and view layers.
    pub(in crate::lower) fn peel_owned(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.ty(id)? {
            // store the payload of owners and views directly
            dir::Type::Form(form) => match form.form {
                dir::Form::Owned | dir::Form::Readonly => self.peel_owned(form.value),
                _ => Ok(id),
            },
            _ => Ok(id),
        }
    }

    /// Return whether one type has an indirect runtime representation.
    pub(in crate::lower) fn has_indirect_representation(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        match self.ty(id)? {
            // answer form layers by their outermost constructor
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed { .. } | dir::Form::Borrowed(_) | dir::Form::Raw => Ok(true),
                // intrinsic fat owners retain a unique reference representation
                dir::Form::Owned => self.is_reference_representation(form.value),
                // views answer for the layer beneath
                dir::Form::Readonly => self.has_indirect_representation(form.value),
            },

            // answer every bare base, unions included, by its family default
            other => {
                Ok(self.base_default_ownership(id.module_id, &other)? == dir::Ownership::Managed)
            }
        }
    }

    /// Return the concrete space one place singleton names.
    pub(in crate::lower) fn place_space(
        &self,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Space>> {
        let space = match self.ty(place)? {
            dir::Type::Literal(dir::Literal::String(value)) => dir::Space::from_text(value),
            // induced place parameters ground at the ambient space
            dir::Type::Parameter(parameter) => {
                let binding = self
                    .state(parameter.module_id)?
                    .generics
                    .get_parameter(parameter.local_id);
                let is_induced_place = matches!(
                    binding.induced_memory_parameter(),
                    Some(dir::MemoryParameter::Place | dir::MemoryParameter::Space)
                );

                is_induced_place.then_some(dir::Space::Local)
            }
            _ => None,
        };

        Ok(space)
    }

    /// Return whether one value has an intrinsic reference representation.
    fn is_reference_representation(&self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        let ty = self.ty(id)?;

        Ok(matches!(
            ty,
            dir::Type::Dynamic(_) | dir::Type::Function(_) | dir::Type::Slice(_)
        ))
    }

    /// Return the default ownership one named declaration's family carries.
    fn symbol_default_ownership(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::Ownership> {
        Ok(match self.definition(symbol)? {
            Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                dir::Ownership::Managed
            }
            // follow a transparent alias default to its defined value
            Some(dir::Definition::TypeAlias(alias)) if self.language_item(symbol).is_none() => {
                let value = self.ty(alias.value)?;

                self.base_default_ownership(alias.value.module_id, &value)?
            }
            _ => dir::Ownership::Owned,
        })
    }

    /// Return the default ownership of one base type family declared in one module.
    fn base_default_ownership(
        &self,
        module: ModuleId,
        base: &dir::Type,
    ) -> CompilerResult<dir::Ownership> {
        Ok(match base {
            // default reference families to managed
            dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Slice(_)
            | dir::Type::Object(_)
            | dir::Type::Unknown => dir::Ownership::Managed,

            // follow the declaration family for nominal defaults
            dir::Type::Application(instance) => self.symbol_default_ownership(instance.symbol)?,
            dir::Type::Reference(reference) => self.symbol_default_ownership(reference.symbol)?,

            // hold string and bigint primitives through their representation classes
            dir::Type::Primitive(dir::PrimitiveType::String | dir::PrimitiveType::Bigint) => {
                dir::Ownership::Managed
            }

            // hold value families directly
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Tuple(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => dir::Ownership::Owned,

            // share the owner's representation for precise variants
            dir::Type::Variant(variant) => {
                let owner = self.ty(variant.owner)?;

                return self.base_default_ownership(variant.owner.module_id, &owner);
            }

            // follow the reference representation of a nullish union, hold indexed unions directly
            dir::Type::Union(union) => {
                let Some((_, representation)) = self.decompose_nullish_union(module, union)? else {
                    return Ok(dir::Ownership::Owned);
                };

                match self.has_indirect_representation(representation)? {
                    true => dir::Ownership::Managed,
                    false => dir::Ownership::Owned,
                }
            }

            // reject contextual this, which materialization resolves before lowering
            dir::Type::This => Err(CompilerError::Internal {
                message: "a contextual this was never materialized".to_string(),
            })?,

            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a default form for the '{}' type", other.variant_name()),
            })?,
        })
    }

    /// Split one union into its nullish values and single reference representation.
    pub(in crate::lower) fn decompose_nullish_union(
        &self,
        module: ModuleId,
        union: &dir::UnionType,
    ) -> CompilerResult<Option<(mir::Nullability, dir::GlobalTypeId)>> {
        // split the members into nullish values and reference representations
        let mut nullability = (false, false);
        let mut representations = Vec::new();
        for id in self.types(module)?.type_ids(union.elements) {
            match self.ty(*id)? {
                // take nullish members onto the representation's spare values
                dir::Type::Null => nullability.0 = true,
                dir::Type::Undefined => nullability.1 = true,
                _ => representations.push(*id),
            }
        }

        // require one reference member to carry the union
        let &[representation] = representations.as_slice() else {
            return Ok(None);
        };
        if !self.has_indirect_representation(representation)? {
            return Ok(None);
        }

        // fold the nullish members into the nullability the representation admits
        let nullability = match nullability {
            (true, true) => mir::Nullability::NullOrUndefined,
            (true, false) => mir::Nullability::Null,
            (false, true) => mir::Nullability::Undefined,
            (false, false) => {
                return Err(CompilerError::Internal {
                    message: "a single-member union".to_string(),
                });
            }
        };

        Ok(Some((nullability, representation)))
    }
}

impl TypeLowerer<'_, '_> {
    /// Widen one lowered reference-like type with the nullish values it admits.
    pub(in crate::lower) fn insert_nullability(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        nullability: mir::Nullability,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // widen through lifetime applications onto their base
        if let mir::Type::Application { base, lifetimes } = self.tree.get(ty) {
            let (base, lifetimes) = (*base, lifetimes.clone());
            let base = self.insert_nullability(base, nullability)?;

            return Ok(self.tree.intern_type(mir::Type::Application {
                base: mir::TypeId::from(base),
                lifetimes,
            }));
        }

        // widen the lowered type in place
        let mut ty = self.tree.get(ty).clone();
        if !ty.set_nullability(nullability) {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a nullable union over the '{ty:?}' representation"),
            }
            .into());
        }

        Ok(self.tree.intern_type(ty))
    }

    /// Insert one reference type over a pointee in the ambient space.
    pub(in crate::lower) fn insert_reference(
        &mut self,
        kind: mir::ReferenceKind,
        access: mir::Access,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime: mir::Lifetime::empty(),
            storage: mir::Storage::heap(self.space),
            access,
            pointee,
            nullability: mir::Nullability::None,
        })
    }
}

impl ModuleLowerer<'_> {
    /// Return the access of one borrow access singleton.
    pub(in crate::lower) fn borrow_access(
        &self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Access> {
        let access = self
            .memory_text(access)?
            .and_then(dir::Access::from_text)
            .ok_or_else(|| CompilerError::Internal {
                message: "a borrow access in the wrong domain".to_string(),
            })?;

        Ok(match access {
            dir::Access::Readonly => mir::Access::Readonly,
            dir::Access::Mutable => mir::Access::Mutable,
            dir::Access::Exclusive => mir::Access::Exclusive,
        })
    }
}

/// Intern one reference type over a pointee in one storage.
pub(in crate::lower) fn insert_reference_type(
    tree: &mut mir::Tree,
    kind: mir::ReferenceKind,
    access: mir::Access,
    storage: mir::Storage,
    pointee: mir::LocalNodeId<mir::Type>,
) -> mir::LocalNodeId<mir::Type> {
    tree.intern_type(mir::Type::Reference {
        kind,
        lifetime: mir::Lifetime::empty(),
        storage,
        access,
        pointee,
        nullability: mir::Nullability::None,
    })
}
