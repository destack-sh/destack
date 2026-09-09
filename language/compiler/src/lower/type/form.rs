use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{GenericScope, ModuleLowerer, TypeLowerer};
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
        let dir::Type::Form(form) = self.lower.ty(id)? else {
            return Err(CompilerError::Internal {
                message: "a non-form type in the form algebra".to_string(),
            });
        };

        match form.form {
            // narrow the access of the next indirect layer inward
            dir::Form::Readonly => match self.lower.ty(form.value)? {
                // carry the narrowed access into the layer below
                dir::Type::Form(_) => self.lower_form(form.value, Some(mir::Access::Readonly)),
                // lower readonly over a pure value at the narrowed access
                _ => self.lower_layer_value(form.value, Some(mir::Access::Readonly)),
            },

            // reference the stored payload of managed layers in their referent place
            dir::Form::Managed { place } => {
                // enter the referent place for the duration of the layer
                let saved = self.space;
                if let Some(space) = self.lower_space(place)? {
                    self.space = space;
                }

                // lower the reference and restore the ambient space
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
                let Some(borrow) = self.lower.types(id.module_id)?.borrow_form_maybe(borrow) else {
                    return Err(CompilerError::Internal {
                        message: "a missing borrow form".to_string(),
                    });
                };

                // split the declared region into its extent and referent place
                let (region, access_argument) = (borrow.region, borrow.access);
                let (lifetime, spaces) = match self.lower.ty(region)? {
                    dir::Type::Region(pair) => (pair.extent, Some(pair.space)),
                    _ => (region, None),
                };

                // lower the extent and access the borrow declares
                let lifetime = self.lower.lower_lifetime(lifetime, self.scope)?;
                let borrow_access = self.lower_access(access_argument)?;

                // select the reference storage from the referent place, a region parameter of a
                // type declaration naming both coordinates, keeping the ambient space for an
                // induced place
                let saved = self.space;
                if let Some(spaces) = spaces {
                    if let Some(space) = self.lower_space(spaces)? {
                        self.space = space;
                    } else if matches!(self.lower.ty(spaces)?, dir::Type::Union(_)) {
                        return Err(CompilerError::Internal {
                            message: "borrow region carries a space join".to_string(),
                        });
                    }
                } else if let dir::Type::Parameter(parameter) = self.lower.ty(region)?
                    && let Some(index) = self.scope.parameter_index(parameter)
                {
                    self.space = mir::Space::Parameter(index);
                }

                // lower the reference and restore the ambient space
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
            dir::Form::Owned if self.lower.is_reference_representation(form.value)? => self
                .lower_reference(
                    mir::ReferenceKind::Unique,
                    mir::Lifetime::empty(),
                    access.unwrap_or(mir::Access::Mutable),
                    form.value,
                ),

            // hold storage directly for owned value families
            dir::Form::Owned => self.lower_pointee(form.value),
        }
    }

    /// Return the element of one payload represented as a slice, through newtype layers.
    fn slice_element(
        &mut self,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // read a slice payload's element as written, a nominal payload's through its representation
        let lowered = match self.lower.ty(payload)? {
            dir::Type::Slice(slice) => return Ok(Some(self.lower(slice.element)?)),
            dir::Type::Application(_) | dir::Type::Reference(_) => self.lower(payload)?,
            _ => return Ok(None),
        };
        let mut current = mir::TypeId::from(lowered);
        loop {
            let represented = self.tree.represented(current);
            match self.tree.get(represented) {
                mir::Type::Slice { element, .. } => return Ok(Some(*element)),
                mir::Type::Newtype { inner, .. } => current = *inner,
                _ => return Ok(None),
            }
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
        if let dir::Type::Dynamic(dynamic) = self.lower.ty(payload)? {
            let constraint = self.lower_dynamic_constraint(dynamic.constraint)?;

            return Ok(self.tree.intern_type(mir::Type::Dynamic {
                kind,
                lifetime,
                constraint,
                storage: mir::Storage::heap(self.space),
                access,
            }));
        }

        // fuse callable environment references into the closure descriptor
        if let dir::Type::Function(function) = self.lower.ty(payload)? {
            let signature = self.lower_callable_signature(function.signature)?;
            let multiplicity = self.lower.callable_multiplicity(function.receiver)?;

            return Ok(self.tree.intern_type(mir::Type::Function {
                signature,
                multiplicity,
                kind,
                lifetime,
                storage: mir::Storage::heap(self.space),
                access,
            }));
        }

        // fuse slice values with the layer into one fat descriptor
        if let Some(element) = self.slice_element(payload)? {
            return Ok(self.tree.intern_type(mir::Type::Slice {
                kind,
                lifetime,
                element,
                storage: mir::Storage::heap(self.space),
                access,
            }));
        }

        // fuse a pointee represented as a fat reference with the layer
        let pointee = self.lower_pointee(payload)?;
        let mut fat = self.tree.get(pointee).clone();
        if let mir::Type::Function {
            kind: fat_kind,
            lifetime: fat_lifetime,
            storage,
            access: fat_access,
            ..
        }
        | mir::Type::Dynamic {
            kind: fat_kind,
            lifetime: fat_lifetime,
            storage,
            access: fat_access,
            ..
        }
        | mir::Type::Slice {
            kind: fat_kind,
            lifetime: fat_lifetime,
            storage,
            access: fat_access,
            ..
        } = &mut fat
        {
            *fat_kind = kind;
            *fat_lifetime = lifetime;
            *storage = mir::Storage::heap(self.space);
            *fat_access = access;

            return Ok(self.tree.intern_type(fat));
        }

        // otherwise reference the lowered pointee thinly
        Ok(self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime,
            storage: mir::Storage::heap(self.space),
            access,
            pointee,
        }))
    }

    /// Lower one raw form into a process-local machine pointer.
    fn lower_pointer(
        &mut self,
        payload: dir::GlobalTypeId,
        access: mir::Access,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let pointee = self.lower_pointee(payload)?;

        Ok(self
            .tree
            .intern_type(mir::Type::Pointer { pointee, access }))
    }

    /// Lower one type as the pointee behind a reference or owner.
    pub(in crate::lower) fn lower_pointee(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // store a dependent pointee as its parameter
        if self.scope.dependent_index(id).is_some() {
            return self.lower(id);
        }

        // store reference primitives as their representation classes
        let ty = self.lower.ty(id)?;
        if let Some(item) = ModuleLowerer::representation_item(&ty) {
            let symbol = self.lower.language_item_symbol(item)?;
            let source = self.lower.symbol_type(symbol)?;

            return Ok(self.lower_nominal(source)?.storage);
        }

        match ty {
            // store an interface receiver as its parameter
            dir::Type::This => self.lower(id),
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
        if self.lower.indirection(id, self.scope)?.is_some() {
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
    pub(in crate::lower) fn indirection(
        &mut self,
        id: dir::GlobalTypeId,
        scope: &GenericScope,
    ) -> CompilerResult<Option<Indirection>> {
        match self.ty(id)? {
            // form layers select their indirection by constructor
            dir::Type::Form(form) => match form.form {
                // managed and raw layers expose mutable access
                dir::Form::Managed { .. } | dir::Form::Raw => Ok(Some(Indirection {
                    stored: form.value,
                    access: mir::Access::Mutable,
                })),
                // borrowed layers expose their declared access
                dir::Form::Borrowed(borrow) => {
                    let Some(borrow) = self.types(id.module_id)?.borrow_form_maybe(borrow).cloned()
                    else {
                        return Err(CompilerError::Internal {
                            message: "a missing borrow form".to_string(),
                        });
                    };
                    let access = self.access_in(borrow.access, scope)?;

                    Ok(Some(Indirection {
                        stored: form.value,
                        access,
                    }))
                }
                // narrow the layer beneath views
                dir::Form::Readonly => {
                    let layer = self.indirection(form.value, scope)?;

                    Ok(layer.map(|layer| Indirection {
                        access: mir::Access::Readonly,
                        ..layer
                    }))
                }
                // owned fat references carry one exclusive unique layer
                dir::Form::Owned if self.is_reference_representation(form.value)? => {
                    Ok(Some(Indirection {
                        stored: form.value,
                        access: mir::Access::Mutable,
                    }))
                }
                // owned values hold storage directly
                dir::Form::Owned => Ok(None),
            },

            // bare reference families carry an implicit managed layer
            other => match self.base_default_ownership(&other)? {
                dir::Ownership::Managed => Ok(Some(Indirection {
                    stored: id,
                    access: mir::Access::Mutable,
                })),
                _ => Ok(None),
            },
        }
    }

    /// Return the storage base beneath one value's owner and view layers.
    pub(in crate::lower) fn stored(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.ty(id)? {
            dir::Type::Form(form) => match form.form {
                dir::Form::Owned | dir::Form::Readonly => self.stored(form.value),
                _ => Ok(id),
            },
            _ => Ok(id),
        }
    }

    /// Return the ownership one type's outermost layer carries.
    pub(in crate::lower) fn ownership(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Ownership> {
        match self.ty(id)? {
            dir::Type::Form(form) => match form.form {
                dir::Form::Owned => Ok(dir::Ownership::Owned),
                // views answer for the layer beneath
                dir::Form::Readonly => self.ownership(form.value),
                dir::Form::Managed { .. } => Ok(dir::Ownership::Managed),
                dir::Form::Borrowed(_) => Ok(dir::Ownership::Borrowed),
                dir::Form::Raw => Ok(dir::Ownership::Raw),
            },
            other => self.base_default_ownership(&other),
        }
    }

    /// Return the concrete space one place singleton names.
    pub(in crate::lower) fn place_space(
        &mut self,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Space>> {
        let space = match self.ty(place)? {
            // read the space a place literal names
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
    fn is_reference_representation(&mut self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        Ok(match self.ty(id)? {
            dir::Type::Dynamic(_) | dir::Type::Function(_) | dir::Type::Slice(_) => true,
            // the callable newtype represents itself as the callable reference
            dir::Type::Application(application) => {
                self.language_item(application.symbol) == Some(dir::LanguageItem::Function)
            }
            _ => false,
        })
    }

    /// Return the default ownership one named declaration's family carries.
    fn symbol_default_ownership(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::Ownership> {
        Ok(match self.definition(symbol)?.cloned() {
            // classes and interfaces default to managed
            Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                dir::Ownership::Managed
            }
            // follow a transparent alias default to its defined value
            Some(dir::Definition::TypeAlias(_)) if self.language_item(symbol).is_none() => {
                let value = self.symbol_type(symbol)?;
                let value = self.ty(value)?;

                self.base_default_ownership(&value)?
            }
            // every other declaration defaults to owned
            _ => dir::Ownership::Owned,
        })
    }

    /// Return the default ownership of one base type family.
    fn base_default_ownership(&mut self, base: &dir::Type) -> CompilerResult<dir::Ownership> {
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

                return self.base_default_ownership(&owner);
            }

            // hold unions directly as their variant
            dir::Type::Union(_) => dir::Ownership::Owned,

            // own a parameter's value and an interface receiver until their arguments say otherwise
            dir::Type::Parameter(_) | dir::Type::This => dir::Ownership::Owned,

            // reject every other head
            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("a default form for the '{}' type", other.variant_name()),
            })?,
        })
    }

    /// Return the members one written type lowers as a union of, through forms and bare names.
    pub(in crate::lower) fn union_members(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        match self.union_members_maybe(id)? {
            Some(members) => Ok(members),
            None => {
                let path = self.state(id.module_id)?.path.clone();
                let ty = self.ty(id)?;

                Err(CompilerError::Internal {
                    message: format!("a union {ty:?} of '{path}' without a union head"),
                })
            }
        }
    }

    /// Return the members a written type lowers as a union of, through forms and bare names.
    pub(in crate::lower) fn union_members_maybe(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        let mut current = ty;
        loop {
            // read the reduction recorded at this written head, following it past the union
            // families it lowers through
            if let Some(reduced) = self.types(current.module_id)?.reduction(current) {
                current = reduced;

                continue;
            }
            match self.ty(current)? {
                dir::Type::Union(union) => {
                    return Ok(Some(
                        self.types(current.module_id)?
                            .type_ids(union.elements)
                            .to_vec(),
                    ));
                }
                // step beneath the forms a stored receiver reads through
                dir::Type::Form(form)
                    if matches!(
                        form.form,
                        dir::Form::Owned | dir::Form::Readonly | dir::Form::Borrowed(_)
                    ) =>
                {
                    current = form.value;
                }
                // read a bare alias or newtype name through its body
                dir::Type::Application(application) if application.arguments.is_empty() => {
                    current = match self.definition(application.symbol)? {
                        Some(dir::Definition::TypeAlias(_)) => {
                            self.symbol_type(application.symbol)?
                        }
                        Some(dir::Definition::Newtype(newtype)) => newtype.backing,
                        _ => return Ok(None),
                    };
                }
                _ => return Ok(None),
            }
        }
    }
}

impl TypeLowerer<'_, '_> {
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
        })
    }

    /// Insert one managed local reference over a pointee type.
    pub(in crate::lower) fn insert_managed_reference(
        &mut self,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.insert_reference(mir::ReferenceKind::Managed, mir::Access::Mutable, pointee)
    }
}

impl ModuleLowerer<'_> {
    /// Return the access of one borrow access singleton.
    pub(in crate::lower) fn borrow_access(
        &mut self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Access> {
        let access = self
            .memory_text(access)?
            .and_then(dir::Access::from_text)
            .ok_or_else(|| CompilerError::Internal {
                message: "an unknown borrow access value".to_string(),
            })?;

        Ok(ModuleLowerer::mir_access(access))
    }

    /// Return the MIR access one access type names, a parameter by its index.
    pub(in crate::lower) fn access_in(
        &mut self,
        access: dir::GlobalTypeId,
        scope: &GenericScope,
    ) -> CompilerResult<mir::Access> {
        if let dir::Type::Parameter(parameter) = self.ty(access)?
            && let Some(index) = scope.parameter_index(parameter)
        {
            return Ok(mir::Access::Parameter(index));
        }

        self.borrow_access(access)
    }

    /// Return the MIR access one declared access names.
    pub(in crate::lower) fn mir_access(access: dir::Access) -> mir::Access {
        match access {
            dir::Access::Readonly => mir::Access::Readonly,
            dir::Access::Mutable => mir::Access::Mutable,
        }
    }
}
