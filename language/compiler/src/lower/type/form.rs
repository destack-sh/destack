use tspp_dir as dir;
use tspp_mir as mir;

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
    ) -> CompilerResult<mir::TypeId> {
        let dir::Type::Form(form) = self.lower.ty(id)? else {
            return Err(CompilerError::Internal {
                message: "a non-form type in the form algebra".to_string(),
            });
        };

        match form.form {
            // narrow the access of the next indirect layer inward
            dir::Form::Readonly => match self.lower.ty(form.value)? {
                // pass the narrowed access to the layer below
                dir::Type::Form(_) => self.lower_form(form.value, Some(mir::Access::Readonly)),
                // lower readonly over a pure value at the narrowed access
                _ => self.lower_layer_value(form.value, Some(mir::Access::Readonly)),
            },

            // keep the declared lifetime and access of borrowed layers
            dir::Form::Borrowed(borrow) => {
                let Some(borrow) = self
                    .lower
                    .types(id.module_id)?
                    .borrow_form_maybe(borrow)
                    .copied()
                else {
                    return Err(CompilerError::Internal {
                        message: "a missing borrow form".to_string(),
                    });
                };

                // lower the complete region and access
                let lifetime = self
                    .lower
                    .lower_lifetime(self.tree, borrow.region, self.scope)?;
                let borrow_access = self.lower_access(borrow.access)?;

                self.lower_reference(
                    mir::Reference::Borrowed,
                    lifetime,
                    access.unwrap_or(borrow_access),
                    form.value,
                )
            }

            // fuse raw fat references into one raw layer
            dir::Form::Raw if self.lower.is_reference_representation(form.value)? => self
                .lower_reference(
                    mir::Reference::Raw,
                    mir::Lifetime::empty(),
                    access.unwrap_or(mir::Access::Mutable),
                    form.value,
                ),

            // produce process-local machine pointers for raw layers
            dir::Form::Raw => {
                self.lower_pointer(form.value, access.unwrap_or(mir::Access::Mutable))
            }

            // fuse owned fat references into one unique layer
            dir::Form::Owned if self.lower.is_reference_representation(form.value)? => self
                .lower_reference(
                    mir::Reference::Unique,
                    mir::Lifetime::empty(),
                    access.unwrap_or(mir::Access::Mutable),
                    form.value,
                ),

            // hold the object an owned value stores
            dir::Form::Owned => {
                let pointee = self.lower_pointee(form.value)?;

                Ok(mir::referent_of(self.tree, pointee))
            }
        }
    }

    /// Return the object one alias names when it erases behind its signatures.
    fn erasing_alias_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(value) = self.lower.alias_value(symbol)? else {
            return Ok(None);
        };

        Ok(match self.lower.ty(value)? {
            dir::Type::Object(shape) if shape.declares_signatures() => Some(value),
            _ => None,
        })
    }

    /// Return the element of one payload represented as a slice, through newtype layers.
    fn slice_element(&mut self, payload: dir::GlobalTypeId) -> CompilerResult<Option<mir::TypeId>> {
        let element = match self.lower.ty(payload)? {
            dir::Type::Slice(slice) => Some(slice.element),
            dir::Type::Application(instance) => {
                let arguments = self
                    .lower
                    .types(payload.module_id)?
                    .type_ids(instance.arguments)
                    .to_vec();

                self.lower
                    .newtype_slice_element(instance.symbol, &arguments)?
            }
            dir::Type::Reference(reference) => {
                self.lower.newtype_slice_element(reference.symbol, &[])?
            }
            _ => None,
        };

        element.map(|element| self.lower(element)).transpose()
    }

    /// Lower one reference layer over its stored payload, fusing fat payloads into one descriptor.
    pub(in crate::lower) fn lower_managed_reference(
        &mut self,
        access: mir::Access,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        let space = self.managed_space(payload)?;

        self.lower_reference(
            mir::Reference::Managed(space),
            mir::Lifetime::empty(),
            access,
            payload,
        )
    }

    /// Return the heap space a managed handle to one payload addresses, local unless declared.
    pub(in crate::lower) fn managed_space(
        &mut self,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Space> {
        let space = match self.lower.nominal_symbol(payload)? {
            Some(symbol) => self.lower.nominal_space(symbol)?,
            None => None,
        };

        Ok(space.map_or(mir::Space::Local, ModuleLowerer::mir_space))
    }

    /// Lower one reference form over a payload.
    pub(in crate::lower) fn lower_reference(
        &mut self,
        kind: mir::Reference,
        lifetime: mir::Lifetime,
        access: mir::Access,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        // lend the object an open value or the receiver stands for
        if let dir::Type::Parameter(_) | dir::Type::This = self.lower.ty(payload)? {
            let target = self.lower(payload)?;

            return Ok(mir::lend(self.tree, kind, lifetime, access, target));
        }

        // fuse dynamic payload references into the erased descriptor
        if let dir::Type::Dynamic(dynamic) = self.lower.ty(payload)? {
            let constraint = self.lower_dynamic_constraint(dynamic.constraint)?;

            return Ok(self.tree.intern_type(mir::Type::Dynamic {
                kind,
                lifetime,
                constraint,
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
                access,
            }));
        }

        // fuse slice values with the layer into one fat descriptor
        if let Some(element) = self.slice_element(payload)? {
            return Ok(self.tree.intern_type(mir::Type::Slice {
                kind,
                lifetime,
                element,
                access,
            }));
        }

        // fuse an interface or signature-declaring payload into the erased descriptor
        let erased = match self.lower.ty(payload)? {
            dir::Type::Application(instance)
                if matches!(
                    self.lower.definition(instance.symbol)?,
                    Some(dir::Definition::Interface(_))
                ) =>
            {
                Some(payload)
            }
            dir::Type::Application(instance) => self.erasing_alias_value(instance.symbol)?,
            dir::Type::Reference(reference) => self.erasing_alias_value(reference.symbol)?,
            dir::Type::Object(shape) if shape.declares_signatures() => Some(payload),
            dir::Type::Unknown => Some(payload),
            _ => None,
        };
        if let Some(erased) = erased {
            let constraint = self.lower_dynamic_constraint(erased)?;

            return Ok(self.tree.intern_type(mir::Type::Dynamic {
                kind,
                lifetime,
                constraint,
                access,
            }));
        }

        // fuse a pointee represented as a fat reference with the layer
        let pointee = self.lower_pointee(payload)?;
        let mut fat = self.tree.get(pointee).clone();
        if let mir::Type::Function {
            kind: fat_kind,
            lifetime: fat_lifetime,
            access: fat_access,
            ..
        }
        | mir::Type::Dynamic {
            kind: fat_kind,
            lifetime: fat_lifetime,
            access: fat_access,
            ..
        }
        | mir::Type::Slice {
            kind: fat_kind,
            lifetime: fat_lifetime,
            access: fat_access,
            ..
        } = &mut fat
        {
            *fat_kind = kind;
            *fat_lifetime = lifetime;
            *fat_access = access;

            return Ok(self.tree.intern_type(fat));
        }

        // otherwise reference the lowered pointee thinly
        Ok(self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime,
            access,
            pointee,
        }))
    }

    /// Lower one raw form into a process-local machine pointer.
    fn lower_pointer(
        &mut self,
        payload: dir::GlobalTypeId,
        access: mir::Access,
    ) -> CompilerResult<mir::TypeId> {
        let pointee = self.lower_pointee(payload)?;

        Ok(self
            .tree
            .intern_type(mir::Type::Pointer { pointee, access }))
    }

    /// Lower one type as the pointee behind a reference or owner.
    pub(in crate::lower) fn lower_pointee(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        // store a dependent pointee as its parameter
        if self.scope.dependent_index(self.lower, id)?.is_some() {
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
    ) -> CompilerResult<mir::TypeId> {
        // reference families receive the access on their implicit managed layer
        if self.lower.indirection(id, self.scope)?.is_some() {
            return self.lower_managed_reference(access.unwrap_or(mir::Access::Mutable), id);
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
                dir::Form::Raw => Ok(Some(Indirection {
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
                // give owned fat references one exclusive unique layer
                dir::Form::Owned if self.is_reference_representation(form.value)? => {
                    Ok(Some(Indirection {
                        stored: form.value,
                        access: mir::Access::Mutable,
                    }))
                }
                // owned values hold storage directly
                dir::Form::Owned => Ok(None),
            },

            // give bare reference families an implicit managed layer
            _ => match self.ownership(id)? {
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

    /// Return the ownership of one type's outermost layer.
    pub(in crate::lower) fn ownership(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Ownership> {
        match self.ty(id)? {
            dir::Type::Form(form) => match form.form {
                dir::Form::Owned => Ok(dir::Ownership::Owned),
                // views answer for the layer beneath
                dir::Form::Readonly => self.ownership(form.value),
                dir::Form::Borrowed(_) => Ok(dir::Ownership::Borrowed),
                dir::Form::Raw => Ok(dir::Ownership::Raw),
            },
            // take the one form an intersection's represented operands agree on
            dir::Type::Intersection(intersection) => {
                let operands = self
                    .types(id.module_id)?
                    .type_ids(intersection.elements)
                    .to_vec();
                let mut agreed = None;
                for operand in operands {
                    if self.is_interface_operand(operand)? {
                        continue;
                    }
                    let ownership = self.ownership(operand)?;
                    if agreed.is_some_and(|known| known != ownership) {
                        return Err(CompilerError::Internal {
                            message: "an intersection of operands in different forms".to_string(),
                        });
                    }
                    agreed = Some(ownership);
                }

                agreed.ok_or_else(|| CompilerError::Internal {
                    message: "an intersection of interface operands alone".to_string(),
                })
            }
            other => self.base_default_ownership(&other),
        }
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

    /// Return the default ownership of one named declaration's family.
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

                Err(CompilerError::Internal {
                    message: format!("a union of '{path}' without a union head"),
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
            // read the reduction recorded at this head, past the families it lowers through
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
    /// Insert one reference type over a pointee.
    pub(in crate::lower) fn insert_reference(
        &mut self,
        kind: mir::Reference,
        access: mir::Access,
        pointee: mir::TypeId,
    ) -> mir::TypeId {
        self.tree.intern_type(mir::Type::Reference {
            kind,
            lifetime: mir::Lifetime::empty(),
            access,
            pointee,
        })
    }

    /// Insert one managed reference over a pointee type in a space.
    pub(in crate::lower) fn insert_managed_reference(
        &mut self,
        space: mir::Space,
        pointee: mir::TypeId,
    ) -> mir::TypeId {
        self.insert_reference(
            mir::Reference::Managed(space),
            mir::Access::Mutable,
            pointee,
        )
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
            .and_then(|access| dir::Access::from_text(self.strings.get(access)))
            .ok_or_else(|| CompilerError::Internal {
                message: "an unknown borrow access value".to_string(),
            })?;

        Ok(ModuleLowerer::mir_access(access))
    }

    /// Return the MIR access one access type names, grounded where the scope is.
    pub(in crate::lower) fn access_in(
        &mut self,
        access: dir::GlobalTypeId,
        scope: &GenericScope,
    ) -> CompilerResult<mir::Access> {
        let index = match scope.dependent_index(self, access)? {
            Some(index) => Some(index),
            None => match self.ty(access)? {
                dir::Type::Parameter(parameter) => scope.parameter_index(parameter),
                _ => None,
            },
        };
        let access = match index {
            Some(index) => mir::Access::Parameter(index),
            None => self.borrow_access(access)?,
        };

        Ok(mir::substitute_access(&scope.grounding, access))
    }

    /// Return the MIR access one declared access names.
    pub(in crate::lower) fn mir_access(access: dir::Access) -> mir::Access {
        match access {
            dir::Access::Readonly => mir::Access::Readonly,
            dir::Access::Mutable => mir::Access::Mutable,
            dir::Access::Immutable => mir::Access::Immutable,
            dir::Access::Exclusive => mir::Access::Exclusive,
        }
    }
}
